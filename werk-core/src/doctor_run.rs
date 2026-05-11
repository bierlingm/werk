//! Per-run artifact tree for `werk doctor` (R-004).
//!
//! Every doctor invocation that touches user state creates a fresh run
//! directory at `.werk/.doctor/runs/<ULID>/` and records the run's
//! findings, the per-file backups it took before mutating, and an append-only
//! log of every mutation. A future `werk doctor undo <run-id>` reads the
//! actions in reverse and restores from `backups/` byte-for-byte.
//!
//! ## Run-id
//!
//! ULID — time-ordered, lexically sortable, matches werk's existing gesture
//! identity scheme. Sortable by name is enough to find "the most recent run"
//! without parsing.
//!
//! ## Layout
//!
//! ```text
//! .werk/.doctor/
//! ├── runs/
//! │   └── <ULID>/
//! │       ├── report.json         ← findings + exit_code + schema_version
//! │       ├── actions.jsonl       ← one line per mutation
//! │       ├── backups/            ← verbatim per-file backups, layout preserved
//! │       └── stderr.log          ← (future) captured stderr
//! ├── latest -> runs/<ULID>/      ← updated atomically after finalize
//! └── scorecard_history.jsonl     ← (future) one line per run for trending
//! ```
//!
//! ## Crash recovery
//!
//! Each run directory is created early but not "promoted" until `finalize()`
//! atomically swings the `latest` symlink. A run that crashes before
//! `finalize()` leaves a half-populated directory which `werk doctor undo`
//! can still replay (the `actions.jsonl` is append-only and durable).
//!
//! Atomic symlink swap: write `latest.tmp.<pid>` → `std::fs::rename` over
//! `latest`. Stays a valid symlink at every observable moment.

use crate::tension::CoreError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use ulid::Ulid;

/// Schema version for the run artifact JSON. Bump on breaking changes.
pub const DOCTOR_CONTRACT_VERSION: u32 = 1;

/// Canonical action `op` strings recorded by doctor fixers. Capabilities JSON
/// (in `werk-cli`) advertises this set; tests assert no fixer emits an op
/// outside this list. Add new fixers' ops here when they ship, in lockstep
/// with the capabilities surface — that's the drift-prevention contract.
pub const ACTION_OPS: &[&str] = &["purge_noop_mutations"];

/// One entry in `actions.jsonl`. Represents a single mutation the doctor
/// performed, with cryptographic fingerprints of the before/after state so
/// `undo` can verify byte-identical restoration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub timestamp: String,
    /// Short, machine-grep-able operation tag, e.g. `"purge_noop_mutations"`,
    /// `"restore_backup"`, `"sql_delete"`.
    pub op: String,
    /// Path the action touched, relative to the workspace root when possible.
    pub target: String,
    /// Hex BLAKE3 of the target's contents before the action.
    pub before_hash: Option<String>,
    /// Hex BLAKE3 of the target's contents after the action.
    pub after_hash: Option<String>,
    /// Optional human-readable note (counts, row ids, etc.).
    pub note: Option<String>,
}

/// Top-level shape of `report.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReport {
    pub schema_version: u32,
    pub doctor_contract_version: u32,
    pub run_id: String,
    pub started_at: String,
    pub finished_at: String,
    /// Exit code the doctor surfaces to its caller (skill canonical codes).
    pub exit_code: i32,
    /// Free-form summary of what the run did.
    pub summary: String,
    /// Detector findings (read-only — populated even on a no-fix run).
    pub findings: Vec<serde_json::Value>,
    /// Count of mutations recorded in actions.jsonl (mirror for quick read).
    pub action_count: usize,
    /// `werk_version` of the binary that produced the run.
    pub werk_version: String,
}

/// Owned handle to an in-flight doctor run. Drop without `finalize()` leaves
/// the run directory present but un-promoted; `werk doctor ls` (future R-003)
/// can surface these as "aborted".
pub struct DoctorRun {
    workspace_root: PathBuf,
    run_id: String,
    run_dir: PathBuf,
    started_at: chrono::DateTime<Utc>,
    actions_path: PathBuf,
    backups_dir: PathBuf,
    action_count: usize,
}

impl DoctorRun {
    /// Create a fresh run directory. Returns a handle for recording actions.
    /// Failure to create the directory is fatal — without it, no run artifact
    /// can be produced and the caller should abort the doctor invocation
    /// rather than mutating without a record.
    pub fn start(workspace_root: &Path) -> Result<Self, CoreError> {
        let workspace_root = workspace_root.to_path_buf();
        let run_id = Ulid::new().to_string();
        let run_dir = workspace_root
            .join(".werk")
            .join(".doctor")
            .join("runs")
            .join(&run_id);
        let backups_dir = run_dir.join("backups");
        std::fs::create_dir_all(&backups_dir).map_err(|e| {
            CoreError::ValidationError(format!(
                "failed to create doctor run dir {}: {}",
                run_dir.display(),
                e
            ))
        })?;
        Ok(Self {
            workspace_root,
            actions_path: run_dir.join("actions.jsonl"),
            backups_dir,
            run_dir,
            run_id,
            started_at: Utc::now(),
            action_count: 0,
        })
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    /// Copy `source_abs` verbatim into the run's `backups/` tree, preserving
    /// the path layout relative to the workspace root. Returns the BLAKE3 hex
    /// hash of the backed-up content for inclusion in the action record.
    /// Verifies byte-identical copy with `std::fs::read` + `cmp`.
    pub fn record_backup(&self, source_abs: &Path) -> Result<String, CoreError> {
        // Refuse out-of-tree backup sources. Pre-empts a future fixer from
        // silently writing an absolute-path backup clone inside the run
        // directory — the doctor's documented blast radius is the workspace.
        let relative = source_abs.strip_prefix(&self.workspace_root).map_err(|_| {
            CoreError::ValidationError(format!(
                "refusing to back up out-of-tree source {}: not under workspace root {}",
                source_abs.display(),
                self.workspace_root.display()
            ))
        })?;
        let dest = self.backups_dir.join(relative);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoreError::ValidationError(format!(
                    "failed to create backup parent {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        std::fs::copy(source_abs, &dest).map_err(|e| {
            CoreError::ValidationError(format!(
                "failed to back up {} -> {}: {}",
                source_abs.display(),
                dest.display(),
                e
            ))
        })?;
        let bytes = std::fs::read(&dest).map_err(|e| {
            CoreError::ValidationError(format!("backup readback failed: {}", e))
        })?;
        let original = std::fs::read(source_abs).map_err(|e| {
            CoreError::ValidationError(format!("source readback failed: {}", e))
        })?;
        if bytes != original {
            return Err(CoreError::ValidationError(format!(
                "backup verification failed: {} differs from source",
                dest.display()
            )));
        }
        Ok(hex_blake3(&bytes))
    }

    /// Append a single action record to `actions.jsonl`. Append-only,
    /// fsync after each line — even a process kill mid-run leaves a
    /// recoverable trail.
    pub fn record_action(&mut self, record: ActionRecord) -> Result<(), CoreError> {
        let mut line = serde_json::to_string(&record).map_err(|e| {
            CoreError::ValidationError(format!("serialize action: {}", e))
        })?;
        line.push('\n');
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.actions_path)
            .map_err(|e| {
                CoreError::ValidationError(format!(
                    "open actions.jsonl: {}",
                    e
                ))
            })?;
        f.write_all(line.as_bytes()).map_err(|e| {
            CoreError::ValidationError(format!("write actions.jsonl: {}", e))
        })?;
        f.sync_all().map_err(|e| {
            CoreError::ValidationError(format!("fsync actions.jsonl: {}", e))
        })?;
        self.action_count += 1;
        Ok(())
    }

    /// Finalize the run: write `report.json` + `report.md`, atomically
    /// promote `latest` to point at this run, append a history line.
    pub fn finalize(
        self,
        exit_code: i32,
        summary: impl Into<String>,
        findings: Vec<serde_json::Value>,
    ) -> Result<RunReport, CoreError> {
        let finished_at = Utc::now();
        let report = RunReport {
            schema_version: DOCTOR_CONTRACT_VERSION,
            doctor_contract_version: DOCTOR_CONTRACT_VERSION,
            run_id: self.run_id.clone(),
            started_at: self.started_at.to_rfc3339(),
            finished_at: finished_at.to_rfc3339(),
            exit_code,
            summary: summary.into(),
            findings,
            action_count: self.action_count,
            werk_version: env!("CARGO_PKG_VERSION").to_string(),
        };
        let report_path = self.run_dir.join("report.json");
        let report_md_path = self.run_dir.join("report.md");
        let json = serde_json::to_string_pretty(&report).map_err(|e| {
            CoreError::ValidationError(format!("serialize report: {}", e))
        })?;
        atomic_write(&report_path, json.as_bytes())?;
        let md = render_report_md(&report);
        atomic_write(&report_md_path, md.as_bytes())?;

        // Atomic `latest` symlink swap.
        let doctor_dir = self
            .workspace_root
            .join(".werk")
            .join(".doctor");
        let _ = std::fs::create_dir_all(&doctor_dir);
        let latest_link = doctor_dir.join("latest");
        let latest_tmp = doctor_dir.join(format!("latest.tmp.{}", std::process::id()));
        let target = PathBuf::from("runs").join(&self.run_id);
        let _ = std::fs::remove_file(&latest_tmp);
        #[cfg(unix)]
        {
            if std::os::unix::fs::symlink(&target, &latest_tmp).is_ok() {
                let _ = std::fs::rename(&latest_tmp, &latest_link);
            }
        }
        #[cfg(not(unix))]
        {
            // On non-unix platforms, write a plain file naming the run.
            let _ = std::fs::write(&latest_tmp, self.run_id.as_bytes());
            let _ = std::fs::rename(&latest_tmp, &latest_link);
        }

        // History line.
        let history_path = doctor_dir.join("scorecard_history.jsonl");
        let history_line = serde_json::json!({
            "run_id": report.run_id,
            "finished_at": report.finished_at,
            "exit_code": report.exit_code,
            "action_count": report.action_count,
            "summary": report.summary,
        });
        let mut history_str = history_line.to_string();
        history_str.push('\n');
        let _ = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&history_path)
            .and_then(|mut f| f.write_all(history_str.as_bytes()));
        Ok(report)
    }
}

fn hex_blake3(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let tmp = path.with_extension(format!(
        "{}.tmp.{}",
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("partial"),
        std::process::id()
    ));
    let write_result = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        Ok(())
    })();
    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&tmp);
        return Err(CoreError::ValidationError(format!(
            "atomic write to {}: {}",
            tmp.display(),
            e
        )));
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        CoreError::ValidationError(format!(
            "rename {} -> {}: {}",
            tmp.display(),
            path.display(),
            e
        ))
    })?;
    Ok(())
}

fn render_report_md(r: &RunReport) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "# werk doctor run {}", r.run_id);
    let _ = writeln!(s);
    let _ = writeln!(s, "- **started:** {}", r.started_at);
    let _ = writeln!(s, "- **finished:** {}", r.finished_at);
    let _ = writeln!(s, "- **exit_code:** {}", r.exit_code);
    let _ = writeln!(s, "- **actions:** {}", r.action_count);
    let _ = writeln!(s, "- **werk_version:** {}", r.werk_version);
    let _ = writeln!(s, "- **contract_version:** {}", r.doctor_contract_version);
    let _ = writeln!(s);
    let _ = writeln!(s, "## summary");
    let _ = writeln!(s);
    let _ = writeln!(s, "{}", r.summary);
    if !r.findings.is_empty() {
        let _ = writeln!(s);
        let _ = writeln!(s, "## findings");
        let _ = writeln!(s);
        for f in &r.findings {
            let _ = writeln!(s, "- {}", f);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fresh_workspace() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join(".werk")).expect("mkdir .werk");
        (tmp, root)
    }

    #[test]
    fn doctor_run_creates_run_dir() {
        let (_tmp, root) = fresh_workspace();
        let run = DoctorRun::start(&root).expect("start");
        assert!(run.run_dir().exists());
        assert!(run.run_dir().join("backups").exists());
    }

    #[test]
    fn record_action_appends_jsonl() {
        let (_tmp, root) = fresh_workspace();
        let mut run = DoctorRun::start(&root).expect("start");
        let rec = ActionRecord {
            timestamp: Utc::now().to_rfc3339(),
            op: "test_op".into(),
            target: "phantom".into(),
            before_hash: None,
            after_hash: None,
            note: Some("unit test".into()),
        };
        run.record_action(rec).expect("record");
        let contents = std::fs::read_to_string(run.run_dir().join("actions.jsonl"))
            .expect("read");
        assert_eq!(contents.lines().count(), 1);
        assert!(contents.contains("test_op"));
    }

    #[test]
    fn record_backup_preserves_bytes_and_returns_hash() {
        let (_tmp, root) = fresh_workspace();
        let target = root.as_path().join("sample.txt");
        std::fs::write(&target, b"hello doctor\n").unwrap();
        let run = DoctorRun::start(&root).expect("start");
        let hash = run.record_backup(&target).expect("backup");
        assert_eq!(hash.len(), 64); // BLAKE3 hex == 64 chars
        let backup = run.run_dir().join("backups").join("sample.txt");
        assert!(backup.exists());
        assert_eq!(std::fs::read(&backup).unwrap(), b"hello doctor\n");
    }

    #[test]
    fn finalize_writes_report_and_promotes_latest() {
        let (_tmp, root) = fresh_workspace();
        let run = DoctorRun::start(&root).expect("start");
        let run_id = run.run_id().to_string();
        let report = run.finalize(0, "smoke", vec![]).expect("finalize");
        assert_eq!(report.exit_code, 0);
        let doctor_dir = root.as_path().join(".werk").join(".doctor");
        let report_json = doctor_dir.join("runs").join(&run_id).join("report.json");
        assert!(report_json.exists());
        let report_md = doctor_dir.join("runs").join(&run_id).join("report.md");
        assert!(report_md.exists());
        let latest = doctor_dir.join("latest");
        assert!(latest.exists() || latest.is_symlink());
        let history = doctor_dir.join("scorecard_history.jsonl");
        let history_contents = std::fs::read_to_string(&history).expect("history");
        assert!(history_contents.contains(&run_id));
    }

    #[test]
    fn two_finalized_runs_promote_latest_to_second() {
        let (_tmp, root) = fresh_workspace();
        let first = DoctorRun::start(&root).expect("first");
        let first_id = first.run_id().to_string();
        first.finalize(0, "first", vec![]).expect("first finalize");
        // ULID is time-ordered; sleep to guarantee monotonic ordering
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = DoctorRun::start(&root).expect("second");
        let second_id = second.run_id().to_string();
        second.finalize(0, "second", vec![]).expect("second finalize");
        assert_ne!(first_id, second_id);
        let latest_link = root.as_path().join(".werk").join(".doctor").join("latest");
        #[cfg(unix)]
        {
            let target = std::fs::read_link(&latest_link).expect("read_link");
            assert!(target.to_string_lossy().contains(&second_id));
        }
    }
}
