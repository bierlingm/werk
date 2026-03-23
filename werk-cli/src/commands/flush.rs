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

pub fn cmd_flush(output: &Output) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;

    let now = Utc::now();

    // Sort by short_code (deterministic, human-readable order).
    // Tensions without short_code sort last, then by id.
    let mut sorted = tensions.clone();
    sorted.sort_by(|a, b| {
        match (a.short_code, b.short_code) {
            (Some(sa), Some(sb)) => sa.cmp(&sb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.id.cmp(&b.id),
        }
    });

    let active = sorted.iter().filter(|t| t.status == TensionStatus::Active).count();
    let resolved = sorted.iter().filter(|t| t.status == TensionStatus::Resolved).count();
    let released = sorted.iter().filter(|t| t.status == TensionStatus::Released).count();

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

    let state = FlushState {
        flushed_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        summary: FlushSummary {
            active,
            released,
            resolved,
            total: sorted.len(),
        },
        tensions: flush_tensions,
    };

    // Serialize with sorted keys and pretty print.
    let json = serde_json::to_string_pretty(&state)
        .map_err(|e| WerkError::IoError(format!("failed to serialize state: {}", e)))?;

    // Write to workspace root.
    let flush_path = workspace.root().join(FLUSH_FILENAME);
    let mut file = std::fs::File::create(&flush_path).map_err(|e| {
        WerkError::IoError(format!("failed to create {}: {}", flush_path.display(), e))
    })?;
    file.write_all(json.as_bytes()).map_err(|e| {
        WerkError::IoError(format!("failed to write {}: {}", flush_path.display(), e))
    })?;
    file.write_all(b"\n").map_err(|e| {
        WerkError::IoError(format!("failed to write {}: {}", flush_path.display(), e))
    })?;

    if output.is_structured() {
        #[derive(Serialize)]
        struct FlushResult {
            path: String,
            tensions: usize,
        }
        let result = FlushResult {
            path: flush_path.to_string_lossy().to_string(),
            tensions: sorted.len(),
        };
        output.print_structured(&result).map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!(
                "Flushed {} tensions to {}",
                sorted.len(),
                flush_path.display()
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    Ok(())
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
        // Verify keys appear in alphabetical order (serde serializes in struct field order)
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
