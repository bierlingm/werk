//! Integration tests for `werk doctor` (R-003).
//!
//! Coverage matches the obligations in
//! `werk__doctor_workspace/analysis/repair_specs/r-003-doctor-cli.md` §8.
//!
//! - `doctor_no_flags_is_read_only`
//! - `doctor_fix_then_undo_round_trip`
//! - `doctor_fix_twice_idempotent`
//! - `capabilities_pinned_against_emitter` (already in the unit-test module)
//! - `stats_health_envelope_pinned`
//! - `doctor_robot_triage_self_contained`

use assert_cmd::cargo_bin_cmd;
use std::path::Path;
use tempfile::TempDir;

/// Seed a noop position mutation directly via the Store API so the doctor
/// has something to detect/fix. Returns the post-seed DB byte snapshot.
fn seed_noop_position_mutation(workspace_root: &Path) -> Vec<u8> {
    // Use the same `Store::init` path the doctor uses so persistence
    // semantics (fsqlite page flush on Drop, backup rotation, etc.)
    // are identical to the doctor's view of the world.
    let store = werk_core::Store::init(workspace_root).unwrap();
    let tension = store.create_tension("test desire", "test reality").unwrap();
    let mutation = werk_core::Mutation::new(
        tension.id.clone(),
        chrono::Utc::now(),
        "position".to_string(),
        Some("5".to_string()),
        "5".to_string(),
    );
    store.record_mutation(&mutation).unwrap();
    drop(store);
    std::fs::read(workspace_root.join(".werk").join("werk.db")).unwrap()
}

/// Logical state probe. fsqlite mutates DB file bytes on open even when
/// no app-level writes happen (WAL/housekeeping), so byte-equality is not
/// a valid invariant for read-only / idempotence claims. Instead the
/// doctor's read-only and idempotence claims are asserted over logical
/// state: `(noop_position_mutation_count, total_mutation_count,
/// tension_count)`.
fn logical_state(workspace_root: &Path) -> (usize, usize, usize) {
    let store = werk_core::Store::init_unlocked(workspace_root).unwrap();
    let noop = store.count_noop_mutations().unwrap();
    let total_mut = store.all_mutations().unwrap().len();
    let tensions = store.list_tensions().unwrap().len();
    (noop, total_mut, tensions)
}

#[test]
fn doctor_no_flags_is_read_only() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let _ = seed_noop_position_mutation(dir.path());
    let pre_state = logical_state(dir.path());

    cargo_bin_cmd!("werk")
        .arg("doctor")
        .arg("--json")
        .current_dir(dir.path())
        .assert()
        .code(1); // findings present

    // Logical state must be unchanged: read-only verb cannot mutate user state.
    // (fsqlite touches bytes on open for housekeeping, so byte-equality is
    // not a valid invariant. See `logical_state` doc comment.)
    assert_eq!(
        pre_state,
        logical_state(dir.path()),
        "doctor (no flags) mutated logical state"
    );
    // The run artifact directory must exist (observability output, not user state).
    assert!(
        dir.path().join(".werk").join(".doctor").join("runs").exists(),
        "expected runs/ directory"
    );
}

#[test]
fn doctor_fix_then_undo_round_trip() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    let _ = seed_noop_position_mutation(dir.path());
    let pre_fix_state = logical_state(dir.path());
    assert!(pre_fix_state.0 > 0, "seed should produce ≥1 noop");

    // Apply the fix.
    cargo_bin_cmd!("werk")
        .args(["doctor", "--fix", "--yes", "--json"])
        .current_dir(dir.path())
        .assert()
        .code(0);
    let post_fix_state = logical_state(dir.path());
    assert_eq!(post_fix_state.0, 0, "fix should leave 0 noop mutations");

    // Locate the run id (only one run exists at this point).
    let runs_dir = dir.path().join(".werk").join(".doctor").join("runs");
    let run_id = std::fs::read_dir(&runs_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .next()
        .expect("at least one run dir");
    let backup_db = runs_dir
        .join(&run_id)
        .join("backups")
        .join(".werk")
        .join("werk.db");
    assert!(backup_db.exists(), "backup file missing");
    let backup_bytes = std::fs::read(&backup_db).unwrap();

    // Undo the latest run.
    cargo_bin_cmd!("werk")
        .args(["doctor", "undo", "latest"])
        .current_dir(dir.path())
        .assert()
        .code(0);

    // Byte-equality assertion: undo restores byte-for-byte from the stored
    // backup. The backup is the state captured at the moment `record_backup`
    // ran (immediately before mutation), so the inverse-pair invariant is
    // `post_undo == backup`.
    let post_undo_bytes =
        std::fs::read(dir.path().join(".werk").join("werk.db")).unwrap();
    assert_eq!(
        backup_bytes, post_undo_bytes,
        "undo did not restore byte-identical backup"
    );

    // And logical state matches pre-fix.
    assert_eq!(
        pre_fix_state,
        logical_state(dir.path()),
        "undo did not restore logical pre-fix state"
    );
}

#[test]
fn doctor_fix_twice_idempotent() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    // No noops to fix — clean workspace.
    cargo_bin_cmd!("werk")
        .args(["doctor", "--fix", "--yes", "--json"])
        .current_dir(dir.path())
        .assert()
        .code(0);
    let state_after_first = logical_state(dir.path());
    cargo_bin_cmd!("werk")
        .args(["doctor", "--fix", "--yes", "--json"])
        .current_dir(dir.path())
        .assert()
        .code(0);
    let state_after_second = logical_state(dir.path());
    assert_eq!(
        state_after_first, state_after_second,
        "idempotence broken: --fix twice changed logical state"
    );
}

#[test]
fn stats_health_envelope_pinned() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    let _ = seed_noop_position_mutation(dir.path());

    // `werk stats --health --json` must still emit the legacy envelope.
    let out = cargo_bin_cmd!("werk")
        .args(["stats", "--health", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stats --health failed: {:?}", out);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // The top-level envelope wraps vitals + sections; health lives under "health".
    let health = v.get("health").expect("missing `health` field");
    assert!(
        health.get("noop_mutations").is_some(),
        "missing legacy field noop_mutations"
    );
    // The keys must be a subset of the pinned legacy schema. Tolerate
    // optional `purged` / `doctor_run_id` (R-004) being absent.
    let allowed = ["noop_mutations", "purged", "doctor_run_id"];
    if let Some(obj) = health.as_object() {
        for k in obj.keys() {
            assert!(
                allowed.contains(&k.as_str()),
                "unexpected field `{}` in stats --health envelope; legacy schema is pinned",
                k
            );
        }
    } else {
        panic!("health field is not a JSON object");
    }
}

#[test]
fn doctor_robot_triage_self_contained() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    let _ = seed_noop_position_mutation(dir.path());

    let out = cargo_bin_cmd!("werk")
        .args(["doctor", "--robot-triage"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "robot-triage should report findings_present"
    );
    // Stdout must be a SINGLE JSON object (parseable cold).
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout not JSON");
    for key in [
        "schema_version",
        "verb",
        "run_id",
        "exit_code",
        "summary",
        "findings",
        "actions_planned",
        "recommended_command",
        "capabilities_command",
    ] {
        assert!(v.get(key).is_some(), "missing key `{}` in robot-triage", key);
    }
    assert_eq!(v.get("verb").and_then(|s| s.as_str()), Some("robot-triage"));
    assert!(
        v.get("findings").and_then(|a| a.as_array()).is_some(),
        "findings must be an array"
    );
    assert!(
        v.get("actions_planned")
            .and_then(|a| a.as_array())
            .is_some(),
        "actions_planned must be an array"
    );
}

#[test]
fn doctor_capabilities_round_trip_pins_contract() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    let out = cargo_bin_cmd!("werk")
        .args(["doctor", "capabilities", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v.get("schema_version").and_then(|x| x.as_u64()), Some(1));
    assert_eq!(
        v.get("doctor_contract_version").and_then(|x| x.as_u64()),
        Some(1)
    );
    let detectors = v
        .get("detectors")
        .and_then(|a| a.as_array())
        .expect("detectors array");
    // R-005 reservations must remain visible.
    let mut reserved_count = 0;
    for d in detectors {
        if d.get("available").and_then(|b| b.as_bool()) == Some(false)
            && d.get("reserved_for").and_then(|s| s.as_str()) == Some("R-005")
        {
            reserved_count += 1;
        }
    }
    assert_eq!(
        reserved_count, 6,
        "expected 6 R-005-reserved detector slots"
    );
}
