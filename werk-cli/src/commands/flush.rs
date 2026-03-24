//! Flush command handler.
//!
//! Writes the tension tree state to a git-trackable JSON file at the workspace root.
//! The output is deterministic: same state produces identical file content.

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::TensionStatus;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;

/// The flush output file name, placed at workspace root.
const FLUSH_FILENAME: &str = "tensions.json";

/// Top-level flush output structure.
#[derive(Serialize)]
struct FlushState {
    flushed_at: String,
    summary: FlushSummary,
    tensions: Vec<FlushTension>,
}

/// Summary counts.
#[derive(Serialize)]
struct FlushSummary {
    active: usize,
    released: usize,
    resolved: usize,
    total: usize,
}

/// A single tension in the flush output.
/// Fields are alphabetically ordered in the struct to match sorted JSON keys.
#[derive(Serialize)]
struct FlushTension {
    actual: String,
    created_at: String,
    desired: String,
    horizon: Option<String>,
    id: String,
    parent_id: Option<String>,
    position: Option<i32>,
    short_code: Option<i32>,
    status: String,
}

/// Write tensions.json to the workspace root. Returns (path, count).
fn flush_to_file(workspace: &Workspace) -> Result<(PathBuf, usize), WerkError> {
    let store = workspace.open_store()?;
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let now = Utc::now();

    let mut sorted = tensions;
    sorted.sort_by(|a, b| match (a.short_code, b.short_code) {
        (Some(sa), Some(sb)) => sa.cmp(&sb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });

    let active = sorted
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .count();
    let resolved = sorted
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .count();
    let released = sorted
        .iter()
        .filter(|t| t.status == TensionStatus::Released)
        .count();

    let flush_tensions: Vec<FlushTension> = sorted
        .iter()
        .map(|t| FlushTension {
            actual: t.actual.clone(),
            created_at: t.created_at.to_rfc3339(),
            desired: t.desired.clone(),
            horizon: t.horizon.as_ref().map(|h| h.to_string()),
            id: t.id.clone(),
            parent_id: t.parent_id.clone(),
            position: t.position,
            short_code: t.short_code,
            status: t.status.to_string(),
        })
        .collect();

    let count = sorted.len();

    // SAFETY: refuse to overwrite tensions.json if tension count drops dramatically.
    // This prevents a corrupt/empty DB from destroying good data.
    let flush_path = workspace.root().join(FLUSH_FILENAME);
    if flush_path.exists() {
        if let Ok(existing) = std::fs::read_to_string(&flush_path) {
            if let Ok(existing_val) = serde_json::from_str::<serde_json::Value>(&existing) {
                if let Some(old_total) = existing_val["summary"]["total"].as_u64() {
                    let old_total = old_total as usize;
                    if old_total > 0 && count == 0 {
                        return Err(WerkError::IoError(format!(
                            "SAFETY: refusing to overwrite tensions.json with 0 tensions (was {}). \
                             If intentional, delete tensions.json first.", old_total
                        )));
                    }
                    if old_total > 5 && count < old_total / 2 {
                        return Err(WerkError::IoError(format!(
                            "SAFETY: refusing to overwrite tensions.json: count dropped from {} to {}. \
                             If intentional, delete tensions.json first.", old_total, count
                        )));
                    }
                }
            }
        }
    }

    // Back up tensions.json before overwriting
    if flush_path.exists() {
        let backup_dir = workspace.root().join(".werk").join("backups");
        let _ = std::fs::create_dir_all(&backup_dir);
        let timestamp = now.format("%Y%m%dT%H%M%SZ");
        let backup_path = backup_dir.join(format!("tensions.{}.json", timestamp));
        let _ = std::fs::copy(&flush_path, &backup_path);
    }

    let state = FlushState {
        flushed_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        summary: FlushSummary {
            active,
            released,
            resolved,
            total: count,
        },
        tensions: flush_tensions,
    };

    let json = serde_json::to_string_pretty(&state)
        .map_err(|e| WerkError::IoError(format!("failed to serialize state: {}", e)))?;

    let mut file = std::fs::File::create(&flush_path).map_err(|e| {
        WerkError::IoError(format!("failed to create {}: {}", flush_path.display(), e))
    })?;
    file.write_all(json.as_bytes()).map_err(|e| {
        WerkError::IoError(format!("failed to write {}: {}", flush_path.display(), e))
    })?;
    file.write_all(b"\n").map_err(|e| {
        WerkError::IoError(format!("failed to write {}: {}", flush_path.display(), e))
    })?;

    Ok((flush_path, count))
}

pub fn cmd_flush(output: &Output) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let (flush_path, count) = flush_to_file(&workspace)?;

    if output.is_structured() {
        #[derive(Serialize)]
        struct FlushResult {
            path: String,
            tensions: usize,
        }
        let result = FlushResult {
            path: flush_path.to_string_lossy().to_string(),
            tensions: count,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!(
                "Flushed {} tensions to {}",
                count,
                flush_path.display()
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    Ok(())
}

/// Autoflush: silently write tensions.json if `flush.auto` config is set to true.
/// Errors are silently ignored — autoflush should never break a mutation command.
pub fn autoflush() {
    let Ok(workspace) = Workspace::discover() else {
        return;
    };
    let Ok(config) = werk_shared::config::Config::load(&workspace) else {
        return;
    };
    if config.get("flush.auto").map(|v| v.as_str()) != Some("true") {
        return;
    }
    let _ = flush_to_file(&workspace);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flush_tension_serialization_key_order() {
        let t = FlushTension {
            actual: "reality".to_string(),
            created_at: "2026-03-23T00:00:00+00:00".to_string(),
            desired: "desire".to_string(),
            horizon: None,
            id: "test-id".to_string(),
            parent_id: None,
            position: Some(1),
            short_code: Some(1),
            status: "Active".to_string(),
        };
        let json = serde_json::to_string(&t).unwrap();
        let actual_pos = json.find("\"actual\"").unwrap();
        let created_pos = json.find("\"created_at\"").unwrap();
        let desired_pos = json.find("\"desired\"").unwrap();
        let id_pos = json.find("\"id\"").unwrap();
        let status_pos = json.find("\"status\"").unwrap();
        assert!(actual_pos < created_pos);
        assert!(created_pos < desired_pos);
        assert!(desired_pos < id_pos);
        assert!(id_pos < status_pos);
    }

    #[test]
    fn test_flush_summary_serialization_key_order() {
        let s = FlushSummary {
            active: 5,
            released: 1,
            resolved: 2,
            total: 8,
        };
        let json = serde_json::to_string(&s).unwrap();
        let active_pos = json.find("\"active\"").unwrap();
        let released_pos = json.find("\"released\"").unwrap();
        let resolved_pos = json.find("\"resolved\"").unwrap();
        let total_pos = json.find("\"total\"").unwrap();
        assert!(active_pos < released_pos);
        assert!(released_pos < resolved_pos);
        assert!(resolved_pos < total_pos);
    }
}
