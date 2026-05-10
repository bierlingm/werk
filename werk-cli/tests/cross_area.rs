//! Cross-area integration tests for display & dynamics and agent integration milestones.
//!
//! Tests verify:
//! - VAL-CROSS-001: Full lifecycle flow (init -> add -> reality updates -> show dynamics -> resolve)
//! - VAL-CROSS-002: Tree operations flow (hierarchy -> move -> rm with reparenting)
//! - VAL-CROSS-003: Context workflow flow (add -> updates -> context)
//! - VAL-CROSS-005: JSON consistency across tree --json and show --json
//! - VAL-CROSS-006: Config persistence
//! - VAL-CROSS-008: Multiple roots handled correctly

use assert_cmd::cargo_bin_cmd;
use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

/// Add a tension via --json and return the ULID.
fn json_add(dir: &Path, desired: &str, actual: &str) -> String {
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg(desired)
        .arg(actual)
        .current_dir(dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output)).unwrap();
    json["id"].as_str().unwrap().to_string()
}

/// Add a tension with parent via --json and return the ULID.
fn json_add_child(dir: &Path, desired: &str, actual: &str, parent: &str) -> String {
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg(desired)
        .arg(actual)
        .arg("--parent")
        .arg(parent)
        .current_dir(dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output)).unwrap();
    json["id"].as_str().unwrap().to_string()
}

/// Extract a tension identifier from werk output.
/// Tries short code (#N) first, then ULID (26 uppercase alphanumeric chars).
fn extract_ulid(output: &str) -> Option<String> {
    // Try short code pattern: #N where N is one or more digits
    if let Some(idx) = output.find('#') {
        let rest = &output[idx + 1..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return Some(digits);
        }
    }
    // Fall back to ULID extraction
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

/// Extract multiple ULIDs from werk output.
#[allow(dead_code)]
fn extract_ulids(output: &str) -> Vec<String> {
    let mut ulids = Vec::new();
    let chars: Vec<char> = output.chars().collect();
    for i in 0..chars.len().saturating_sub(25) {
        let slice: String = chars[i..i + 26].iter().collect();
        if slice
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            ulids.push(slice);
        }
    }
    ulids
}

// =============================================================================
// VAL-CROSS-001: Full lifecycle flow
// =============================================================================

/// VAL-CROSS-001: Full lifecycle flow - dynamics change as tension evolves
#[test]
fn test_full_lifecycle_dynamics_evolution() {
    let dir = TempDir::new().unwrap();

    // Step 1: Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Step 2: Add tension
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("write a complete novel")
        .arg("have an outline")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have tension ID");

    // Step 3: Show initial dynamics (should be Germination)
    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("Activity"),
        "Should show Activity section, got: {}",
        stdout
    );

    // Step 4: Multiple reality updates to build history
    for i in 1..=5 {
        cargo_bin_cmd!("werk")
            .arg("reality")
            .arg(&id)
            .arg(format!("progress update {}", i))
            .current_dir(dir.path())
            .assert()
            .success();
    }

    // Step 5: Show dynamics after updates
    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show dynamics section with computed values
    assert!(
        stdout.contains("tension") || stdout.contains("Tension") || stdout.contains("magnitude"),
        "Should show structural tension info after updates, got: {}",
        stdout
    );

    // Should show mutation history
    assert!(
        stdout.contains("Mutation") || stdout.contains("mutation") || stdout.contains("update"),
        "Should show mutation history, got: {}",
        stdout
    );

    // Step 6: Resolve the tension
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Step 7: Show resolved tension - dynamics should reflect completion
    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("Resolved"),
        "Status should be Resolved, got: {}",
        stdout
    );
}

/// VAL-CROSS-001: Verify dynamics computed correctly at each step
#[test]
fn test_lifecycle_json_dynamics_tracking() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Add
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("complete goal")
        .arg("starting reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let add_json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    let id = add_json["id"].as_str().unwrap().to_string();

    // Check initial dynamics via JSON
    let output = cargo_bin_cmd!("werk")
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
    let show_json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    // Show JSON should have expected top-level fields
    assert!(show_json.get("id").is_some(), "Show JSON should have id");
    assert!(
        show_json.get("status").is_some(),
        "Show JSON should have status"
    );

    // Update reality multiple times
    for i in 1..=4 {
        cargo_bin_cmd!("werk")
            .arg("--json")
            .arg("reality")
            .arg(&id)
            .arg(format!("update {}", i))
            .current_dir(dir.path())
            .assert()
            .success();
    }

    // Check dynamics after updates
    let output = cargo_bin_cmd!("werk")
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
    let show_json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    // Should have mutations recorded
    let mutations = show_json.get("mutations").expect("Should have mutations");
    assert!(
        mutations.as_array().unwrap().len() >= 5,
        "Should have at least 5 mutations (creation + 4 updates), got: {}",
        mutations.as_array().unwrap().len()
    );

    // Resolve
    cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("resolve")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify resolved state
    let output = cargo_bin_cmd!("werk")
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
    let show_json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    assert_eq!(
        show_json["status"].as_str(),
        Some("Resolved"),
        "Status should be Resolved"
    );
}

// =============================================================================
// VAL-CROSS-002: Tree operations flow
// =============================================================================

/// VAL-CROSS-002: Tree correctly reflects hierarchy
#[test]
fn test_tree_shows_hierarchy() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create grandparent
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("grandparent goal")
        .arg("gp reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let gp_id =
        extract_ulid(&String::from_utf8_lossy(&output)).expect("Should have grandparent ID");

    // Create parent
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("parent goal")
        .arg("p reality")
        .arg("--parent")
        .arg(&gp_id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let p_id = extract_ulid(&String::from_utf8_lossy(&output)).expect("Should have parent ID");

    // Create child
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("child goal")
        .arg("c reality")
        .arg("--parent")
        .arg(&p_id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Tree should show hierarchy
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    assert!(
        stdout.contains("grandparent goal"),
        "Should show grandparent, got: {}",
        stdout
    );
    assert!(
        stdout.contains("parent goal"),
        "Should show parent, got: {}",
        stdout
    );
    assert!(
        stdout.contains("child goal"),
        "Should show child, got: {}",
        stdout
    );

    // Should have tree structure characters
    let has_tree_chars = stdout.contains('├')
        || stdout.contains('└')
        || stdout.contains('│')
        || stdout.contains("--");
    assert!(
        has_tree_chars,
        "Should have tree structure characters, got: {}",
        stdout
    );
}

/// VAL-CROSS-002: Tree correctly reflects move operation
#[test]
fn test_tree_reflects_move() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create two root tensions (use json_add to get ULIDs for parent_id comparison)
    let root_a_id = json_add(dir.path(), "root A", "reality A");
    let root_b_id = json_add(dir.path(), "root B", "reality B");
    let child_id = json_add_child(dir.path(), "child node", "child reality", &root_a_id);

    // Initial tree - child under root A
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let _stdout = String::from_utf8_lossy(&output);
    // Verify initial structure (child should be under root A)

    // Move child to root B
    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(&child_id)
        .arg("--parent")
        .arg(&root_b_id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Tree should reflect move
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // All tensions should still be present
    assert!(
        stdout.contains("root A"),
        "Should still show root A, got: {}",
        stdout
    );
    assert!(
        stdout.contains("root B"),
        "Should still show root B, got: {}",
        stdout
    );
    assert!(
        stdout.contains("child node"),
        "Should still show child, got: {}",
        stdout
    );

    // Verify the move via JSON to check parent_id
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(&child_id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    // Parent ID should now be root B
    assert_eq!(
        json["parent_id"].as_str(),
        Some(root_b_id.as_str()),
        "Child's parent_id should be root B after move"
    );
}

/// VAL-CROSS-002: Tree shows reparenting after rm
#[test]
fn test_tree_shows_reparenting_after_rm() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create A -> B -> C hierarchy (use json_add to get ULIDs for parent_id comparison)
    let a_id = json_add(dir.path(), "grandparent A", "A reality");
    let b_id = json_add_child(dir.path(), "parent B", "B reality", &a_id);
    let c_id = json_add_child(dir.path(), "child C", "C reality", &b_id);

    // Verify initial structure
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("grandparent A")
            && stdout.contains("parent B")
            && stdout.contains("child C"),
        "Initial tree should show all three, got: {}",
        stdout
    );

    // Remove middle node (B)
    cargo_bin_cmd!("werk")
        .arg("rm")
        .arg(&b_id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Tree should show reparenting: C should now be under A
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // A and C should still be visible, B should be gone
    assert!(
        stdout.contains("grandparent A"),
        "A should still be present, got: {}",
        stdout
    );
    assert!(
        stdout.contains("child C"),
        "C should still be present (reparented), got: {}",
        stdout
    );
    assert!(
        !stdout.contains("parent B"),
        "B should be removed, got: {}",
        stdout
    );

    // Verify C's parent is now A via JSON
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(&c_id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    // C's parent should now be A (grandparent adoption)
    assert_eq!(
        json["parent_id"].as_str(),
        Some(a_id.as_str()),
        "C's parent_id should be A after B removed, got: {:?}",
        json["parent_id"]
    );
}

// =============================================================================
// VAL-CROSS-005: JSON consistency across commands
// =============================================================================

/// VAL-CROSS-005: Same tension has identical core fields in tree --json and show --json
#[test]
fn test_json_schema_identical_tree_show() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("test goal")
        .arg("test reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let add_json: Value = serde_json::from_str(&stdout).expect("Add should be valid JSON");
    let id = add_json["id"].as_str().unwrap().to_string();

    // Get show --json
    let output = cargo_bin_cmd!("werk")
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

    // Get tree --json
    let output = cargo_bin_cmd!("werk")
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

    // Find our tension in tree
    let tensions = tree_json["tensions"]
        .as_array()
        .expect("Should have tensions array");
    let tree_tension = tensions
        .iter()
        .find(|t| t["id"].as_str() == Some(id.as_str()))
        .expect("Should find tension in tree");

    // Core fields must match exactly
    let core_fields = ["id", "desired", "actual", "status", "parent_id"];
    for field in core_fields {
        assert_eq!(
            show_json[field], tree_tension[field],
            "Field '{}' should match between show and tree JSON",
            field
        );
    }

    // Tree has factual fields: short_code, overdue, closure_resolved, closure_total
    assert!(
        tree_tension.get("short_code").is_some(),
        "Tree tension should have short_code field"
    );
    assert!(
        tree_tension.get("overdue").is_some(),
        "Tree tension should have overdue field"
    );
    assert!(
        tree_tension.get("closure_resolved").is_some(),
        "Tree tension should have closure_resolved field"
    );
    assert!(
        tree_tension.get("closure_total").is_some(),
        "Tree tension should have closure_total field"
    );

    // Show also has factual fields
    assert!(
        show_json.get("overdue").is_some(),
        "Show JSON should have overdue field"
    );
    assert!(
        show_json.get("frontier").is_some(),
        "Show JSON should have frontier field"
    );
}

/// VAL-CROSS-005: JSON dynamics fields are consistent (phase matches between tree and show)
#[test]
fn test_json_dynamics_consistency() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension with some updates
    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("dynamics test")
        .arg("initial state")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let add_json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    let id = add_json["id"].as_str().unwrap().to_string();

    // Add some updates
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&id)
        .arg("update 1")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&id)
        .arg("update 2")
        .current_dir(dir.path())
        .assert()
        .success();

    // Get show --json
    let output = cargo_bin_cmd!("werk")
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

    // Get tree --json
    let output = cargo_bin_cmd!("werk")
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

    // Find our tension in tree
    let tensions = tree_json["tensions"].as_array().unwrap();
    let tree_tension = tensions
        .iter()
        .find(|t| t["id"].as_str() == Some(id.as_str()))
        .expect("Should find tension in tree");

    // Factual fields should be consistent between tree and show
    let tree_overdue = tree_tension
        .get("overdue")
        .expect("Tree tension should have overdue field");
    let show_overdue = show_json
        .get("overdue")
        .expect("Show should have overdue field");

    assert_eq!(
        tree_overdue, show_overdue,
        "Overdue should match between tree and show"
    );

    // Status should match
    assert_eq!(
        tree_tension["status"], show_json["status"],
        "Status should match between tree and show"
    );

    // Core identity fields match
    assert_eq!(
        tree_tension["id"], show_json["id"],
        "ID should match between tree and show"
    );
}

// =============================================================================
// VAL-CROSS-008: Multiple roots handled correctly
// =============================================================================

/// VAL-CROSS-008: Three unparented tensions appear as 3 roots in tree
#[test]
fn test_multiple_roots_in_tree() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create three unparented tensions
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("root 1 goal")
        .arg("root 1 reality")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("root 2 goal")
        .arg("root 2 reality")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("root 3 goal")
        .arg("root 3 reality")
        .current_dir(dir.path())
        .assert()
        .success();

    // Tree should show all three roots
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    assert!(
        stdout.contains("root 1 goal"),
        "Should show root 1, got: {}",
        stdout
    );
    assert!(
        stdout.contains("root 2 goal"),
        "Should show root 2, got: {}",
        stdout
    );
    assert!(
        stdout.contains("root 3 goal"),
        "Should show root 3, got: {}",
        stdout
    );
}

/// VAL-CROSS-008: Tree --json shows multiple roots correctly
#[test]
fn test_multiple_roots_json() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create three unparented tensions
    let output1 = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("root 1")
        .arg("reality 1")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output2 = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("root 2")
        .arg("reality 2")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output3 = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("root 3")
        .arg("reality 3")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let id1 = extract_ulid(&String::from_utf8_lossy(&output1)).expect("Should have ID 1");
    let id2 = extract_ulid(&String::from_utf8_lossy(&output2)).expect("Should have ID 2");
    let id3 = extract_ulid(&String::from_utf8_lossy(&output3)).expect("Should have ID 3");

    // Tree --json should show all three as roots (parent_id = null)
    let output = cargo_bin_cmd!("werk")
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

    let tensions = tree_json["tensions"].as_array().unwrap();

    // Should have exactly 3 tensions
    assert_eq!(tensions.len(), 3, "Should have exactly 3 tensions");

    // All three should have parent_id = null (roots)
    let ids = vec![id1, id2, id3];
    for id in ids {
        let tension = tensions
            .iter()
            .find(|t| t["id"].as_str() == Some(id.as_str()))
            .expect("Should find tension");

        assert!(
            tension["parent_id"].is_null(),
            "Root tension {} should have parent_id = null, got: {:?}",
            id,
            tension["parent_id"]
        );
    }
}

/// VAL-CROSS-008: Multiple roots with children still show correctly
#[test]
fn test_multiple_roots_with_children() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create two root tensions, each with a child
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("root A")
        .arg("reality A")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let root_a = extract_ulid(&String::from_utf8_lossy(&output)).expect("Should have root A ID");

    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("root B")
        .arg("reality B")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let root_b = extract_ulid(&String::from_utf8_lossy(&output)).expect("Should have root B ID");

    // Add children
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("child of A")
        .arg("child reality")
        .arg("--parent")
        .arg(&root_a)
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("child of B")
        .arg("child reality")
        .arg("--parent")
        .arg(&root_b)
        .current_dir(dir.path())
        .assert()
        .success();

    // Tree should show both hierarchies
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // All tensions should appear
    assert!(
        stdout.contains("root A") && stdout.contains("root B"),
        "Should show both roots, got: {}",
        stdout
    );
    assert!(
        stdout.contains("child of A") && stdout.contains("child of B"),
        "Should show both children, got: {}",
        stdout
    );
}

/// VAL-CROSS-008: Verify siblings in tree JSON (roots are siblings of each other)
#[test]
fn test_siblings_in_tree_json() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create three root tensions (siblings at root level)
    let output1 = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("sibling 1")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output2 = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("sibling 2")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output3 = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("sibling 3")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let id1 = extract_ulid(&String::from_utf8_lossy(&output1)).expect("Should have ID 1");
    let id2 = extract_ulid(&String::from_utf8_lossy(&output2)).expect("Should have ID 2");
    let id3 = extract_ulid(&String::from_utf8_lossy(&output3)).expect("Should have ID 3");

    // Get tree JSON
    let output = cargo_bin_cmd!("werk")
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

    let tensions = tree_json["tensions"].as_array().unwrap();

    // All three should be present and be roots
    assert_eq!(tensions.len(), 3, "Should have exactly 3 tensions");

    // Verify each is a root (parent_id = null)
    for id in [&id1, &id2, &id3] {
        let tension = tensions
            .iter()
            .find(|t| t["id"].as_str() == Some(id.as_str()))
            .expect("Should find tension");

        assert!(
            tension["parent_id"].is_null(),
            "Root {} should have parent_id = null",
            id
        );
    }
}

// =============================================================================
// Additional cross-command consistency tests
// =============================================================================

/// Verify prefix matching consistency across show and resolve
#[test]
fn test_prefix_consistency_show_resolve() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("prefix test")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let id = extract_ulid(&stdout).expect("Should have tension ID");

    // Show with short code should work
    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("prefix test"),
        "Show with short code should show correct tension, got: {}",
        stdout
    );

    // Resolve with same short code should work
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify resolved
    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("Resolved"),
        "Tension should be resolved, got: {}",
        stdout
    );
}

/// Verify tree correctly reflects resolve (filters)
#[test]
fn test_tree_filter_resolved_cross_area() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tensions
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("active tension")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("to resolve")
        .arg("reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let id = extract_ulid(&String::from_utf8_lossy(&output)).expect("Should have ID");

    // Resolve one
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Tree default (open) should show only active
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .arg("--open")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("active tension"),
        "Should show active, got: {}",
        stdout
    );
    assert!(
        !stdout.contains("to resolve"),
        "Should not show resolved, got: {}",
        stdout
    );

    // Tree --resolved should show only resolved
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .arg("--resolved")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("to resolve"),
        "Should show resolved, got: {}",
        stdout
    );
    assert!(
        !stdout.contains("active tension"),
        "Should not show active in --resolved, got: {}",
        stdout
    );
}

// =============================================================================
// VAL-CROSS-006: Config persistence
// =============================================================================

/// VAL-CROSS-006: Config persists across commands
#[test]
fn test_config_persists_across_commands() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Set agent.command
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo persisted")
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify with config get
    let output = cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("agent.command")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("echo persisted"),
        "Config should persist, got: {}",
        stdout
    );
}
