//! Shared flush logic for writing `tensions.json` at the workspace root.
//!
//! Used by both `werk flush` (CLI) and the MCP autoflush helper so that both
//! surfaces produce byte-identical output and share the same idempotency
//! contract.
//!
//! ## Idempotency
//!
//! The output is deterministic *and* idempotent: if the tension state has not
//! changed since the last flush, the file is left untouched. The `flushed_at`
//! timestamp alone never triggers a rewrite. This prevents spurious dirty
//! states in git from no-op flushes during pre-commit hooks (critical under
//! GitButler, where `git add` in hooks bypasses the virtual-branch staging
//! model).
//!
//! ## Safety
//!
//! Refuses to overwrite `tensions.json` if the tension count drops to zero
//! (from a non-empty previous state), or if it drops to less than half of
//! the previous count (when the previous count exceeded 5). These guards
//! protect against a corrupt or empty database destroying good data.

use crate::error::{Result, WerkError};
use crate::workspace::Workspace;
use chrono::Utc;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use werk_core::TensionStatus;

/// The flush output file name, placed at workspace root.
pub const FLUSH_FILENAME: &str = "tensions.json";

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

/// Result of a flush operation.
pub struct FlushOutcome {
    /// Path to the flushed file.
    pub path: PathBuf,
    /// Number of tensions serialized.
    pub count: usize,
    /// `true` if the file was actually written, `false` if it was a no-op
    /// (content was already equivalent to the current state).
    pub wrote: bool,
}

/// Flush the current tension state to `tensions.json` at the workspace root.
///
/// Returns a [`FlushOutcome`] describing what happened. When `outcome.wrote`
/// is `false`, the file on disk is byte-identical to what it was before the
/// call — no mtime update, no git dirtying.
pub fn flush_to_file(workspace: &Workspace) -> Result<FlushOutcome> {
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
    let flush_path = workspace.root().join(FLUSH_FILENAME);

    // SAFETY: refuse to overwrite tensions.json if tension count drops dramatically.
    // This prevents a corrupt/empty DB from destroying good data.
    if flush_path.exists()
        && let Ok(existing) = std::fs::read_to_string(&flush_path)
        && let Ok(existing_val) = serde_json::from_str::<serde_json::Value>(&existing)
        && let Some(old_total) = existing_val["summary"]["total"].as_u64()
    {
        let old_total = old_total as usize;
        if old_total > 0 && count == 0 {
            return Err(WerkError::IoError(format!(
                "SAFETY: refusing to overwrite tensions.json with 0 tensions (was {}). \
                 If intentional, delete tensions.json first.",
                old_total
            )));
        }
        if old_total > 5 && count < old_total / 2 {
            return Err(WerkError::IoError(format!(
                "SAFETY: refusing to overwrite tensions.json: count dropped from {} to {}. \
                 If intentional, delete tensions.json first.",
                old_total, count
            )));
        }
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

    // IDEMPOTENCY: if the existing file is structurally identical (ignoring the
    // `flushed_at` timestamp), leave it untouched. This is what lets the pre-commit
    // hook run `werk flush` on every commit without producing spurious dirty files.
    if flush_path.exists()
        && let Ok(existing) = std::fs::read_to_string(&flush_path)
        && content_equivalent(&existing, &json)
    {
        return Ok(FlushOutcome {
            path: flush_path,
            count,
            wrote: false,
        });
    }

    // Back up tensions.json before overwriting (only when we're actually writing).
    if flush_path.exists() {
        let backup_dir = workspace.root().join(".werk").join("backups");
        let _ = std::fs::create_dir_all(&backup_dir);
        let timestamp = now.format("%Y%m%dT%H%M%SZ");
        let backup_path = backup_dir.join(format!("tensions.{}.json", timestamp));
        let _ = std::fs::copy(&flush_path, &backup_path);
    }

    let mut file = std::fs::File::create(&flush_path).map_err(|e| {
        WerkError::IoError(format!("failed to create {}: {}", flush_path.display(), e))
    })?;
    file.write_all(json.as_bytes()).map_err(|e| {
        WerkError::IoError(format!("failed to write {}: {}", flush_path.display(), e))
    })?;
    file.write_all(b"\n").map_err(|e| {
        WerkError::IoError(format!("failed to write {}: {}", flush_path.display(), e))
    })?;

    Ok(FlushOutcome {
        path: flush_path,
        count,
        wrote: true,
    })
}

/// Returns `true` if two `tensions.json` payloads are structurally equivalent,
/// ignoring the `flushed_at` timestamp.
pub fn content_equivalent(existing: &str, new: &str) -> bool {
    let (Ok(mut e), Ok(mut n)) = (
        serde_json::from_str::<serde_json::Value>(existing),
        serde_json::from_str::<serde_json::Value>(new),
    ) else {
        return false;
    };
    if let Some(obj) = e.as_object_mut() {
        obj.remove("flushed_at");
    }
    if let Some(obj) = n.as_object_mut() {
        obj.remove("flushed_at");
    }
    e == n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_equivalent_ignores_flushed_at() {
        let a = r#"{"flushed_at":"2026-04-10T12:00:00Z","summary":{"active":1,"released":0,"resolved":0,"total":1},"tensions":[]}"#;
        let b = r#"{"flushed_at":"2026-04-11T13:00:00Z","summary":{"active":1,"released":0,"resolved":0,"total":1},"tensions":[]}"#;
        assert!(content_equivalent(a, b));
    }

    #[test]
    fn test_content_equivalent_detects_substantive_change() {
        let a = r#"{"flushed_at":"2026-04-10T12:00:00Z","summary":{"active":1,"released":0,"resolved":0,"total":1},"tensions":[]}"#;
        let b = r#"{"flushed_at":"2026-04-10T12:00:00Z","summary":{"active":2,"released":0,"resolved":0,"total":2},"tensions":[]}"#;
        assert!(!content_equivalent(a, b));
    }

    #[test]
    fn test_content_equivalent_handles_invalid_json() {
        assert!(!content_equivalent("not json", r#"{"flushed_at":"x"}"#));
        assert!(!content_equivalent(r#"{"flushed_at":"x"}"#, "not json"));
    }

    #[test]
    fn test_content_equivalent_missing_flushed_at() {
        let a = r#"{"summary":{"active":1,"released":0,"resolved":0,"total":1},"tensions":[]}"#;
        let b = r#"{"flushed_at":"2026-04-10T12:00:00Z","summary":{"active":1,"released":0,"resolved":0,"total":1},"tensions":[]}"#;
        assert!(content_equivalent(a, b));
    }

    #[test]
    fn test_content_equivalent_detects_tension_change() {
        let a = r#"{"flushed_at":"2026-04-10T12:00:00Z","summary":{"active":1,"released":0,"resolved":0,"total":1},"tensions":[{"id":"a","desired":"old"}]}"#;
        let b = r#"{"flushed_at":"2026-04-10T12:00:00Z","summary":{"active":1,"released":0,"resolved":0,"total":1},"tensions":[{"id":"a","desired":"new"}]}"#;
        assert!(!content_equivalent(a, b));
    }
}
