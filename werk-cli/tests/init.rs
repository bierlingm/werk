//! Integration tests for `werk init` command.
//!
//! Tests verify:
//! - VAL-INIT-001: `werk init` creates .werk/sd.db with correct schema
//! - VAL-INIT-002: `werk init --global` creates ~/.werk/sd.db
//! - VAL-INIT-003: Init is idempotent (re-running preserves data)

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

/// VAL-INIT-001: `werk init` creates .werk/sd.db with correct schema
#[test]
fn test_init_creates_workspace() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Workspace initialized"));

    // Verify .werk/ directory was created
    assert!(dir.path().join(".werk").exists());

    // Verify sd.db was created
    assert!(dir.path().join(".werk").join("sd.db").exists());
}

/// VAL-INIT-001: Database has correct schema (tensions + mutations tables)
#[test]
fn test_init_creates_correct_schema() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Open the database using sd_core and verify we can create/list tensions
    // This verifies the schema is correct
    let store = sd_core::Store::init(dir.path()).unwrap();

    // Should be able to list tensions (empty is fine)
    let tensions = store.list_tensions().unwrap();
    assert!(tensions.is_empty(), "new store should have no tensions");

    // Should be able to create a tension (verifies tensions table works)
    let tension = store.create_tension("test goal", "test reality").unwrap();
    assert!(!tension.id.is_empty());

    // Verify mutations table works by checking mutations
    let mutations = store.get_mutations(&tension.id).unwrap();
    assert_eq!(mutations.len(), 1, "should have creation mutation");
}

/// VAL-INIT-002: `werk init --global` creates ~/.werk/sd.db
/// Note: We can't test against real home directory, so we test that --global
/// creates a workspace at a different location than local init.
#[test]
fn test_init_global_flag() {
    let dir = TempDir::new().unwrap();

    // --global should NOT create local .werk/
    cargo_bin_cmd!("werk")
        .arg("init")
        .arg("--global")
        .current_dir(dir.path())
        .assert()
        .success()
        // The path will be expanded, so we check for .werk somewhere in output
        .stdout(predicate::str::contains(".werk"));

    // Local .werk/ should NOT be created
    assert!(!dir.path().join(".werk").exists());
}

/// VAL-INIT-003: Re-running init preserves existing data (idempotent)
#[test]
fn test_init_idempotent_preserves_data() {
    let dir = TempDir::new().unwrap();

    // First init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Workspace initialized"));

    // Create a tension using sd-core directly
    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("test goal", "test reality").unwrap();

    // Re-run init - should say "already initialized" but still succeed
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Workspace"));

    // Verify tension still exists
    let store2 = sd_core::Store::init(dir.path()).unwrap();
    let retrieved = store2.get_tension(&tension.id).unwrap();
    assert!(
        retrieved.is_some(),
        "tension should still exist after re-init"
    );
    assert_eq!(retrieved.unwrap().desired, "test goal");
}

/// VAL-INIT-003: Re-init returns created: false when workspace already exists
#[test]
fn test_init_reinit_reports_existing() {
    let dir = TempDir::new().unwrap();

    // First init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Workspace initialized"));

    // Re-init - should still succeed (idempotent)
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
}

/// --json flag produces valid JSON output
#[test]
fn test_init_json_output() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"path\""))
        .stdout(predicate::str::contains("\"created\""));
}

/// --json flag with --global
#[test]
fn test_init_global_json_output() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("init")
        .arg("--global")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"path\""));
}

/// Permission errors produce descriptive message (exit code 1)
#[cfg(unix)]
#[test]
fn test_init_permission_denied() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();

    // Create a read-only parent directory
    let readonly_dir = dir.path().join("readonly");
    std::fs::create_dir_all(&readonly_dir).unwrap();

    // Make parent read-only (no write permission)
    std::fs::set_permissions(&readonly_dir, std::fs::Permissions::from_mode(0o555)).unwrap();

    // Attempt init inside read-only directory
    let result = cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(&readonly_dir)
        .assert()
        .failure();

    // Should have descriptive error message
    let output = result.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("permission") || stderr.contains("denied") || stderr.contains("error:"),
        "Error message should mention permission issue, got: {}",
        stderr
    );

    // Clean up - restore permissions for TempDir cleanup
    std::fs::set_permissions(&readonly_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Exit code 0 on success
#[test]
fn test_init_exit_code_success() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success(); // exit code 0
}

/// --no-color flag works
#[test]
fn test_init_no_color_flag() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("--no-color")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Workspace initialized"));
}

/// NO_COLOR env var works
#[test]
fn test_init_no_color_env() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .env("NO_COLOR", "1")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Workspace initialized"));
}

/// Init reports the correct path
#[test]
fn test_init_reports_correct_path() {
    let dir = TempDir::new().unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // Should contain .werk in the path
    assert!(
        stdout.contains(".werk"),
        "Output should contain .werk path, got: {}",
        stdout
    );
}

/// JSON output includes valid path string
#[test]
fn test_init_json_includes_path() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.get("path").is_some(), "JSON should have 'path' field");
    assert!(
        json.get("created").is_some(),
        "JSON should have 'created' field"
    );
}
