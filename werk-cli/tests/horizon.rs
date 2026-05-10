//! Tests for horizon CLI commands (H10 - horizon-cli milestone).
//!
//! Covers:
//! - VAL-HCLI-001 through VAL-HCLI-022
//! - VAL-HCROSS-001 through VAL-HCROSS-006

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────────
// Add with --horizon flag
// ─────────────────────────────────────────────────────────────────────────────

// VAL-HCLI-001: Add with --horizon month
#[test]
fn test_add_with_horizon_month() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("desired state")
        .arg("actual state")
        .arg("--horizon")
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created tension"))
        .stdout(predicate::str::contains("Deadline: 2026-05"));

    // Verify with --json
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("another")
        .arg("test")
        .arg("--horizon")
        .arg("2026-06")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["horizon"], "2026-06");
}

// VAL-HCLI-002: Add without --horizon (backward compat)
#[test]
fn test_add_without_horizon() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(json["horizon"].is_null());
}

// VAL-HCLI-003: Add with --horizon year
#[test]
fn test_add_with_horizon_year() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .arg("--horizon")
        .arg("2026")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["horizon"], "2026");
}

// VAL-HCLI-004: Add with --horizon day
#[test]
fn test_add_with_horizon_day() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .arg("--horizon")
        .arg("2026-05-15")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["horizon"], "2026-05-15");
}

// VAL-HCLI-016: Invalid horizon format error
#[test]
fn test_add_with_invalid_horizon() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .arg("--horizon")
        .arg("abc")
        .current_dir(dir.path())
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Invalid horizon"))
        .stderr(predicate::str::contains("Examples:"));
}

// VAL-HCLI-017: Invalid month/day rejected
#[test]
fn test_add_with_invalid_month() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .arg("--horizon")
        .arg("2026-13")
        .current_dir(dir.path())
        .assert()
        .failure()
        .code(1);
}

// ─────────────────────────────────────────────────────────────────────────────
// Horizon command
// ─────────────────────────────────────────────────────────────────────────────

// VAL-HCLI-005: Set horizon on existing tension
#[test]
fn test_horizon_set() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let id = json["id"].as_str().unwrap();

    // Set horizon
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("horizon")
        .arg(id)
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["horizon"], "2026-05");
}

// VAL-HCLI-006: Clear horizon
#[test]
fn test_horizon_clear() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension with horizon
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .arg("--horizon")
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let id = json["id"].as_str().unwrap();

    // Clear horizon
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("horizon")
        .arg(id)
        .arg("none")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(json["horizon"].is_null());
}

// VAL-HCLI-007: Display horizon (no value)
#[test]
fn test_horizon_display() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension with horizon
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .arg("--horizon")
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let id = json["id"].as_str().unwrap();

    // Display horizon
    cargo_bin_cmd!("werk")
        .arg("horizon")
        .arg(id)
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Deadline: 2026-05"))
        .stdout(predicate::str::contains("Urgency:"));
}

// VAL-HCLI-018: Horizon on nonexistent tension
#[test]
fn test_horizon_nonexistent() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("horizon")
        .arg("BOGUS123")
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not found"));
}

// VAL-HCLI-019: Horizon on resolved tension rejected
#[test]
fn test_horizon_on_resolved_rejected() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let id = json["id"].as_str().unwrap();

    // Resolve tension
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Try to set horizon
    cargo_bin_cmd!("werk")
        .arg("horizon")
        .arg(id)
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("resolved").or(predicate::str::contains("Active")));
}

// VAL-HCLI-020: Horizon on released tension rejected
#[test]
fn test_horizon_on_released_rejected() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let id = json["id"].as_str().unwrap();

    // Release tension
    cargo_bin_cmd!("werk")
        .arg("release")
        .arg(id)
        .arg("--reason")
        .arg("test")
        .current_dir(dir.path())
        .assert()
        .success();

    // Try to set horizon
    cargo_bin_cmd!("werk")
        .arg("horizon")
        .arg(id)
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("released").or(predicate::str::contains("Active")));
}

// VAL-HCLI-021: Overwrite existing horizon
#[test]
fn test_horizon_overwrite() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension with horizon
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .arg("--horizon")
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let id = json["id"].as_str().unwrap();

    // Overwrite horizon
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("horizon")
        .arg(id)
        .arg("2026-08")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["horizon"], "2026-08");
}

// VAL-HCLI-022: Add with --horizon and --parent
#[test]
fn test_add_with_horizon_and_parent() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("parent")
        .arg("actual")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let parent_id = json["id"].as_str().unwrap();

    // Create child with horizon and parent
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("child")
        .arg("actual")
        .arg("--parent")
        .arg(parent_id)
        .arg("--horizon")
        .arg("2026-06")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["parent_id"], parent_id);
    assert_eq!(json["horizon"], "2026-06");
}

// ─────────────────────────────────────────────────────────────────────────────
// Show with horizon
// ─────────────────────────────────────────────────────────────────────────────

// VAL-HCLI-008: Show with horizon
#[test]
fn test_show_with_horizon() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension with horizon
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired state")
        .arg("actual state")
        .arg("--horizon")
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let id = json["id"].as_str().unwrap();

    // Show
    cargo_bin_cmd!("werk")
        .arg("show")
        .arg(id)
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("2026-05"));
}

// VAL-HCLI-009: Show without horizon
#[test]
fn test_show_without_horizon() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension without horizon
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let id = json["id"].as_str().unwrap();

    // Show - should not show Horizon line or show as None
    let _output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output();
    // Should not have urgency/pressure for no-horizon tension
}

// ─────────────────────────────────────────────────────────────────────────────
// Tree with horizon
// ─────────────────────────────────────────────────────────────────────────────

// VAL-HCLI-012: Tree sorts by horizon
#[test]
fn test_tree_sorts_by_horizon() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("parent")
        .arg("actual")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let parent_id = json["id"].as_str().unwrap();

    // Create children with different horizons
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("child august")
        .arg("actual")
        .arg("--parent")
        .arg(parent_id)
        .arg("--horizon")
        .arg("2026-08")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("child march")
        .arg("actual")
        .arg("--parent")
        .arg(parent_id)
        .arg("--horizon")
        .arg("2026-03")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("child no horizon")
        .arg("actual")
        .arg("--parent")
        .arg(parent_id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Tree should show march before august before no horizon
    let assert = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Find positions - march should appear before august
    let march_pos = stdout.find("child march").unwrap();
    let august_pos = stdout.find("child august").unwrap();
    assert!(
        march_pos < august_pos,
        "March should appear before August in tree output"
    );
}

// VAL-HCLI-013: Tree horizon annotations
#[test]
fn test_tree_horizon_annotations() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension with horizon
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("with horizon")
        .arg("actual")
        .arg("--horizon")
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension without horizon
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("no horizon")
        .arg("actual")
        .current_dir(dir.path())
        .assert()
        .success();

    // Tree should show [2026-05] and [—] annotations
    cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("[2026-05]").or(predicate::str::contains("2026-05")))
        .stdout(predicate::str::contains("no horizon"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Show with horizon — show --json is the agent surface for horizon data
// ─────────────────────────────────────────────────────────────────────────────

// VAL-HCLI-014: Show JSON with horizon
#[test]
fn test_show_json_with_horizon() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .arg("--horizon")
        .arg("2026-05")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let id = json["id"].as_str().unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["horizon"], "2026-05");
    assert!(json["horizon_range"].is_object());
    assert!(json["horizon_range"]["start"].is_string());
    assert!(json["horizon_range"]["end"].is_string());
    assert!(json["urgency"].is_number());
}

// VAL-HCLI-015: Show JSON without horizon
#[test]
fn test_show_json_without_horizon() {
    let dir = TempDir::new().unwrap();
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("desired")
        .arg("actual")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let id = json["id"].as_str().unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(json["horizon"].is_null());
    assert!(json["urgency"].is_null());
}
