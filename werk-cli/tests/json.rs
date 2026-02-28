//! Integration tests for JSON output across all commands.
//!
//! Tests verify:
//! - VAL-JSON-001: --json on tree produces valid JSON
//! - VAL-JSON-002: --json on show produces valid JSON with dynamics
//! - VAL-JSON-003: --json on CRUD commands produces valid JSON
//! - VAL-JSON-004: --json respects filter flags
//! - VAL-JSON-005: --json absent dynamics are null not omitted
//! - VAL-JSON-006: --json error responses are structured JSON
//! - VAL-JSON-007: --json on empty tree returns empty array

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// Extract a ULID from werk output.
fn extract_ulid(output: &str) -> Option<String> {
    let chars: Vec<char> = output.chars().collect();
    for i in 0..chars.len().saturating_sub(25) {
        let slice: String = chars[i..i + 26].iter().collect();
        if slice
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return Some(slice);
        }
    }
    None
}

// =============================================================================
// VAL-JSON-001: --json on tree produces valid JSON
// =============================================================================

#[test]
fn test_tree_json_valid() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Tree output should be valid JSON");
    assert!(json.is_object(), "Tree JSON should be an object");
    assert!(json.get("tensions").is_some(), "Should have tensions array");
    assert!(json.get("summary").is_some(), "Should have summary object");
}

// =============================================================================
// VAL-JSON-002: --json on show produces valid JSON with dynamics
// =============================================================================

#[test]
fn test_show_json_valid_with_dynamics() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("show")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Show output should be valid JSON");
    assert!(json.get("id").is_some(), "Should have id field");
    assert!(json.get("dynamics").is_some(), "Should have dynamics field");
    assert!(
        json.get("dynamics").unwrap().get("phase").is_some(),
        "Should have phase in dynamics"
    );
}

// =============================================================================
// VAL-JSON-003: --json on CRUD commands produces valid JSON
// =============================================================================

#[test]
fn test_add_json_valid() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("desired state")
        .arg("actual state")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Add output should be valid JSON");
    assert!(json.get("id").is_some(), "Should have id field");
    assert!(json.get("desired").is_some(), "Should have desired field");
    assert!(json.get("actual").is_some(), "Should have actual field");
    assert!(json.get("status").is_some(), "Should have status field");
}

#[test]
fn test_reality_json_valid() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("goal")
        .arg("initial")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("reality")
        .arg(&id)
        .arg("updated reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Reality output should be valid JSON");
    assert!(json.get("id").is_some(), "Should have id field");
    assert!(json.get("actual").is_some(), "Should have actual field");
}

#[test]
fn test_desire_json_valid() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("initial goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("desire")
        .arg(&id)
        .arg("updated goal")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Desire output should be valid JSON");
    assert!(json.get("id").is_some(), "Should have id field");
    assert!(json.get("desired").is_some(), "Should have desired field");
}

#[test]
fn test_resolve_json_valid() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("resolve")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Resolve output should be valid JSON");
    assert!(json.get("id").is_some(), "Should have id field");
    assert!(json.get("status").is_some(), "Should have status field");
    assert_eq!(
        json["status"].as_str(),
        Some("Resolved"),
        "Status should be Resolved"
    );
}

#[test]
fn test_release_json_valid() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("release")
        .arg(&id)
        .arg("--reason")
        .arg("test release")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Release output should be valid JSON");
    assert!(json.get("id").is_some(), "Should have id field");
    assert!(json.get("status").is_some(), "Should have status field");
    assert!(json.get("reason").is_some(), "Should have reason field");
    assert_eq!(
        json["status"].as_str(),
        Some("Released"),
        "Status should be Released"
    );
}

#[test]
fn test_rm_json_valid() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("goal to delete")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("rm")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Rm output should be valid JSON");
    assert!(json.get("id").is_some(), "Should have id field");
    assert!(json.get("deleted").is_some(), "Should have deleted field");
    assert_eq!(
        json["deleted"].as_bool(),
        Some(true),
        "deleted should be true"
    );
}

#[test]
fn test_move_json_valid() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("parent goal")
        .arg("parent reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let parent_id = extract_ulid(&stdout).expect("Should have extracted parent ID");

    // Create child
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("child goal")
        .arg("child reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let child_id = extract_ulid(&stdout).expect("Should have extracted child ID");

    // Move child under parent
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("move")
        .arg(&child_id)
        .arg("--parent")
        .arg(&parent_id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Move output should be valid JSON");
    assert!(json.get("id").is_some(), "Should have id field");
    assert!(
        json.get("parent_id").is_some(),
        "Should have parent_id field"
    );
}

#[test]
fn test_note_json_valid() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("note")
        .arg(&id)
        .arg("test note")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Note output should be valid JSON");
    assert!(json.get("id").is_some(), "Should have id field");
    assert!(json.get("note").is_some(), "Should have note field");
}

// =============================================================================
// VAL-JSON-004: --json respects filter flags
// =============================================================================

#[test]
fn test_tree_json_filter_open() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create active tension
    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("active goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create and resolve another
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("resolved goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    Command::cargo_bin("werk")
        .unwrap()
        .arg("resolve")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success();

    // --open --json should only show Active
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("tree")
        .arg("--open")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    let tensions = json.get("tensions").unwrap().as_array().unwrap();
    assert_eq!(tensions.len(), 1, "Should have exactly 1 active tension");
    assert_eq!(
        tensions[0]["status"].as_str(),
        Some("Active"),
        "Only tension should be Active"
    );
}

#[test]
fn test_tree_json_filter_resolved() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create active tension
    Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("active goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create and resolve another
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("add")
        .arg("resolved goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    Command::cargo_bin("werk")
        .unwrap()
        .arg("resolve")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success();

    // --resolved --json should only show Resolved
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("tree")
        .arg("--resolved")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    let tensions = json.get("tensions").unwrap().as_array().unwrap();
    assert_eq!(tensions.len(), 1, "Should have exactly 1 resolved tension");
    assert_eq!(
        tensions[0]["status"].as_str(),
        Some("Resolved"),
        "Only tension should be Resolved"
    );
}

// =============================================================================
// VAL-JSON-005: --json absent dynamics are null not omitted
// =============================================================================

#[test]
fn test_show_json_dynamics_null_not_omitted() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("brand new goal")
        .arg("brand new reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("show")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    let dynamics = json.get("dynamics").expect("Should have dynamics");

    // Check that certain dynamics are null (not omitted)
    // For a new tension, these should be null:
    let null_fields = [
        "oscillation",
        "resolution",
        "structural_conflict",
        "neglect",
    ];
    for field in null_fields {
        assert!(
            dynamics.get(field).is_some(),
            "Field '{}' should exist in dynamics (not omitted)",
            field
        );
        assert!(
            dynamics.get(field).unwrap().is_null(),
            "Field '{}' should be null for new tension, got: {:?}",
            field,
            dynamics.get(field)
        );
    }
}

// =============================================================================
// VAL-JSON-006: --json error responses are structured JSON
// =============================================================================

#[test]
fn test_json_error_not_found() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("show")
        .arg("NONEXISTENTID12345678")
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .clone();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Error should be valid JSON");

    assert!(
        json.get("error").is_some(),
        "Should have 'error' object, got: {}",
        stdout
    );

    let error = json.get("error").unwrap();
    assert!(
        error.get("code").is_some(),
        "Error should have 'code' field"
    );
    assert!(
        error.get("message").is_some(),
        "Error should have 'message' field"
    );
    assert_eq!(
        error["code"].as_str(),
        Some("NOT_FOUND"),
        "Error code should be NOT_FOUND"
    );
}

#[test]
fn test_json_error_prefix_too_short() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("show")
        .arg("abc")
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .clone();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Error should be valid JSON");

    let error = json.get("error").expect("Should have 'error' object");
    assert_eq!(
        error["code"].as_str(),
        Some("INVALID_INPUT"),
        "Error code should be INVALID_INPUT"
    );
}

#[test]
fn test_json_error_on_resolved_tension() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("goal")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have extracted ID");

    // Resolve it
    Command::cargo_bin("werk")
        .unwrap()
        .arg("resolve")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Try to update reality on resolved tension (should fail)
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("reality")
        .arg(&id)
        .arg("new value")
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .clone();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Error should be valid JSON");

    let error = json.get("error").expect("Should have 'error' object");
    assert!(
        error.get("code").is_some(),
        "Error should have 'code' field"
    );
}

// =============================================================================
// VAL-JSON-007: --json on empty tree returns empty array
// =============================================================================

#[test]
fn test_tree_json_empty() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    let tensions = json.get("tensions").expect("Should have tensions field");
    assert!(tensions.is_array(), "tensions should be an array");
    assert_eq!(
        tensions.as_array().unwrap().len(),
        0,
        "tensions should be empty array"
    );
}

// =============================================================================
// Consistent schema across commands (VAL-CROSS-005)
// =============================================================================

#[test]
fn test_json_schema_consistency() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("add")
        .arg("consistent goal")
        .arg("consistent reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let add_json: Value = serde_json::from_str(&stdout).expect("Add should be valid JSON");
    let id = add_json.get("id").unwrap().as_str().unwrap().to_string();

    // Get show JSON
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("show")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let show_json: Value = serde_json::from_str(&stdout).expect("Show should be valid JSON");

    // Get tree JSON
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let tree_json: Value = serde_json::from_str(&stdout).expect("Tree should be valid JSON");

    // Check that the tension appears in tree with consistent fields
    let tree_tensions = tree_json["tensions"].as_array().unwrap();
    let tree_tension = tree_tensions
        .iter()
        .find(|t| t["id"].as_str() == Some(id.as_str()))
        .expect("Should find tension in tree");

    // Core fields should match
    assert_eq!(
        show_json["id"].as_str(),
        tree_tension["id"].as_str(),
        "ID should match between show and tree"
    );
    assert_eq!(
        show_json["desired"].as_str(),
        tree_tension["desired"].as_str(),
        "Desired should match between show and tree"
    );
    assert_eq!(
        show_json["actual"].as_str(),
        tree_tension["actual"].as_str(),
        "Actual should match between show and tree"
    );
    assert_eq!(
        show_json["status"].as_str(),
        tree_tension["status"].as_str(),
        "Status should match between show and tree"
    );
}
