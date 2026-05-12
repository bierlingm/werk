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
    // R-005 has shipped: the six Quint detectors must be `available: true`
    // with no `reserved_for` field. Drift would mean a future pass
    // silently re-reserved a Quint detector slot.
    let quint_ids = [
        "singleParent",
        "noSelfEdges",
        "edgesValid",
        "siblingPositionsUnique",
        "noContainmentViolations",
        "undoneSubsetOfCompleted",
    ];
    for id in quint_ids {
        let d = detectors
            .iter()
            .find(|d| d.get("id").and_then(|s| s.as_str()) == Some(id))
            .unwrap_or_else(|| panic!("capabilities missing detector `{}`", id));
        assert_eq!(
            d.get("available").and_then(|b| b.as_bool()),
            Some(true),
            "detector `{}` is not available",
            id
        );
        assert!(
            d.get("reserved_for").is_none()
                || d.get("reserved_for").map(|v| v.is_null()).unwrap_or(false),
            "detector `{}` still has reserved_for set",
            id
        );
    }
}

/// Pass-5 limit-#4 closure: undo writes a `restore_in_progress` marker
/// at start and removes it on success. Confirms crash-resumption shape.
#[test]
fn doctor_undo_clears_restore_in_progress_marker() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    let _ = seed_noop_position_mutation(dir.path());
    // Run fix → create run with actions
    let fix_out = cargo_bin_cmd!("werk")
        .args(["--json", "doctor", "--fix", "--yes"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(fix_out.status.success());
    // Latest run dir
    let latest = dir.path().join(".werk/.doctor/latest");
    let run_dir = std::fs::read_link(&latest).expect("latest symlink");
    let run_dir_abs = dir.path().join(".werk/.doctor").join(&run_dir);
    let marker = run_dir_abs.join("restore_in_progress");
    assert!(!marker.exists(), "marker should be absent before undo");
    let out = cargo_bin_cmd!("werk")
        .args(["--json", "doctor", "undo", "latest"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "undo must succeed");
    assert!(
        !marker.exists(),
        "marker must be cleared on successful undo (path: {})",
        marker.display()
    );
}

/// R-014: `werk doctor evacuate-backups` copies `.werk/backups/*` to
/// an external destination without mutating the source. Re-running is
/// idempotent (identical bytes are skipped).
#[test]
fn doctor_evacuate_backups_copies_and_is_idempotent() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Plant a couple of synthetic backup files under .werk/backups/.
    let backups = dir.path().join(".werk/backups");
    std::fs::create_dir_all(&backups).unwrap();
    std::fs::write(backups.join("werk.db.20260101T000000Z"), b"snapshot-1").unwrap();
    std::fs::write(backups.join("werk.db.20260102T000000Z"), b"snapshot-2").unwrap();

    let dest = TempDir::new().unwrap();
    let out = cargo_bin_cmd!("werk")
        .args([
            "--json",
            "doctor",
            "evacuate-backups",
            "--dest",
            dest.path().to_str().unwrap(),
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "first evacuate must succeed");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["verb"], "evacuate-backups");
    assert_eq!(v["data"]["files_copied"], 2);
    assert_eq!(v["data"]["files_skipped_identical"], 0);

    // Source must be untouched (W-3: copy-only, never move).
    assert!(backups.join("werk.db.20260101T000000Z").exists());
    assert!(backups.join("werk.db.20260102T000000Z").exists());

    // Dest must contain bytes matching source.
    let bytes = std::fs::read(dest.path().join("werk.db.20260101T000000Z")).unwrap();
    assert_eq!(bytes, b"snapshot-1");

    // Re-run: all files identical → 0 copied, 2 skipped.
    let out2 = cargo_bin_cmd!("werk")
        .args([
            "--json",
            "doctor",
            "evacuate-backups",
            "--dest",
            dest.path().to_str().unwrap(),
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out2.status.success());
    let v2: serde_json::Value = serde_json::from_slice(&out2.stdout).unwrap();
    assert_eq!(v2["data"]["files_copied"], 0);
    assert_eq!(v2["data"]["files_skipped_identical"], 2);
}

/// R-014 dry-run: enumerates plan without writing.
#[test]
fn doctor_evacuate_backups_dry_run_writes_nothing() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    let backups = dir.path().join(".werk/backups");
    std::fs::create_dir_all(&backups).unwrap();
    std::fs::write(backups.join("werk.db.x"), b"x").unwrap();

    let dest = TempDir::new().unwrap();
    let dest_subdir = dest.path().join("not-yet-created");
    let out = cargo_bin_cmd!("werk")
        .args([
            "--json",
            "doctor",
            "evacuate-backups",
            "--dest",
            dest_subdir.to_str().unwrap(),
            "--dry-run",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["data"]["dry_run"], true);
    assert_eq!(v["data"]["files_planned"], 1);
    assert!(!dest_subdir.exists(), "dry-run must not create dest dir");
}

/// R-013: golden-artifact drift detection.
///
/// `werk-cli/tests/golden/capabilities.json` is the committed snapshot
/// of `werk doctor capabilities --json`. Agents and downstream tools
/// depend on its shape being stable. ANY change to the capabilities
/// surface — adding a detector, renaming an op, bumping `schema_version`,
/// extending the exit-code dictionary — must be accompanied by running
/// `./scripts/snapshot-capabilities.sh`, which regenerates the file.
///
/// This test deliberately diffs the FULL JSON (not just a field subset)
/// so any silent drift fails CI. The previous
/// `doctor_capabilities_round_trip_pins_contract` test pins a narrow
/// schema contract; this one pins the entire artifact.
#[test]
fn doctor_capabilities_matches_golden_artifact() {
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
    let actual: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("capabilities --json must be valid JSON");

    let golden_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/capabilities.json");
    let golden_bytes = std::fs::read(&golden_path)
        .unwrap_or_else(|e| panic!("missing golden file {}: {}", golden_path.display(), e));
    let golden: serde_json::Value =
        serde_json::from_slice(&golden_bytes).expect("golden capabilities must be valid JSON");

    if actual != golden {
        // Print the diff in a CI-friendly way before panicking.
        let actual_pretty = serde_json::to_string_pretty(&actual).unwrap();
        let golden_pretty = serde_json::to_string_pretty(&golden).unwrap();
        eprintln!("--- golden ---\n{}\n--- actual ---\n{}", golden_pretty, actual_pretty);
        panic!(
            "capabilities surface drifted from golden. Run scripts/snapshot-capabilities.sh \
             to update {} after confirming the change is intentional.",
            golden_path.display()
        );
    }
}
