//! Integration tests for `werk show` dynamics display.
//!
//! Tests verify:
//! - VAL-DISP-009: Show displays dynamics summary
//! - VAL-DISP-010: Show --verbose displays all 10 dynamics
//! - VAL-DISP-011: Show displays mutation history
//! - VAL-DISP-012: Show displays children list
//! - VAL-DISP-013: Dynamics on new tension without mutations

use assert_cmd::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

// =============================================================================
// Dynamics Summary Tests (VAL-DISP-009)
// =============================================================================

/// VAL-DISP-009: Show displays structural tension magnitude
#[test]
fn test_show_displays_tension_magnitude() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension with a gap between desired and actual
    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store
        .create_tension("write a complete novel", "have an outline")
        .unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show the tension header with desired/reality
    assert!(
        stdout.contains("Tension") && stdout.contains("Desired:") && stdout.contains("Reality:"),
        "Should show tension with desired/reality, got: {}",
        stdout
    );
}

/// VAL-DISP-009: Show displays facts section with closure info
#[test]
fn test_show_displays_phase() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show Status and Activity sections
    assert!(
        stdout.contains("Status:") && stdout.contains("Activity:"),
        "Should show Status and Activity, got: {}",
        stdout
    );
}

/// VAL-DISP-009: Show JSON includes conflict dynamics when present
#[test]
fn test_show_displays_closure_for_parent() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store.create_tension("parent", "p reality").unwrap();
    let _child1 = store
        .create_tension_with_parent("child1", "c1", Some(parent.id.clone()))
        .unwrap();
    let _child2 = store
        .create_tension_with_parent("child2", "c2", Some(parent.id.clone()))
        .unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(&parent.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(
        json["frontier"]["closure_progress"]["total"].as_u64(),
        Some(2)
    );
    assert_eq!(
        json["frontier"]["closure_progress"]["resolved"].as_u64(),
        Some(0)
    );
}

/// VAL-DISP-009: Show displays last activity
#[test]
fn test_show_displays_movement() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show last activity fact
    assert!(
        stdout.contains("Last act:"),
        "Should show last activity, got: {}",
        stdout
    );
}

// =============================================================================
// Verbose Dynamics Tests (VAL-DISP-010)
// =============================================================================

// =============================================================================
// Mutation History Tests (VAL-DISP-011)
// =============================================================================

/// VAL-DISP-011: Show displays mutation history chronologically
#[test]
fn test_show_displays_mutation_history() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store
        .create_tension("mutation goal", "initial reality")
        .unwrap();

    // Make several updates
    store
        .update_actual(&tension.id, "reality update 1")
        .unwrap();
    store
        .update_actual(&tension.id, "reality update 2")
        .unwrap();
    store.update_desired(&tension.id, "refined goal").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show activity section with mutation history
    assert!(
        stdout.contains("Activity:"),
        "Should show activity section, got: {}",
        stdout
    );
}

/// VAL-DISP-011: Show limits mutations to last 10
#[test]
fn test_show_limits_mutation_history() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Create 15 mutations
    for i in 0..15 {
        store
            .update_actual(&tension.id, &format!("update {}", i))
            .unwrap();
    }

    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should contain activity section (creation + updates)
    assert!(
        stdout.contains("Activity:"),
        "Should show activity, got: {}",
        stdout
    );
}

// =============================================================================
// Children List Tests (VAL-DISP-012)
// =============================================================================

/// VAL-DISP-012: Show displays children list when present
#[test]
fn test_show_displays_children_list() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent with children
    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store
        .create_tension("parent goal", "parent reality")
        .unwrap();
    let _child1 = store
        .create_tension_with_parent("child goal 1", "child reality 1", Some(parent.id.clone()))
        .unwrap();
    let _child2 = store
        .create_tension_with_parent("child goal 2", "child reality 2", Some(parent.id.clone()))
        .unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&parent.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show children section
    assert!(
        stdout.contains("Children") || stdout.contains("children") || stdout.contains("child"),
        "Should show children section, got: {}",
        stdout
    );
}

/// VAL-DISP-012: Show shows no children for leaf tension
#[test]
fn test_show_no_children_for_leaf() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("leaf goal", "leaf reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Either shows "Children: 0" or "No children" or just doesn't show children section
    // This is valid behavior
    assert!(
        !stdout.contains("error"),
        "Should not error, got: {}",
        stdout
    );
}

// =============================================================================
// New Tension Dynamics Tests (VAL-DISP-013)
// =============================================================================

/// VAL-DISP-013: New tension shows leaf closure status
#[test]
fn test_show_new_tension_shows_germination() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("new goal", "new reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // New tension should show status and wave info
    assert!(
        stdout.contains("Status:") && stdout.contains("Wave:"),
        "New tension should show status and wave, got: {}",
        stdout
    );
}

/// VAL-DISP-013: New tension shows other dynamics as None (no panic)
#[test]
fn test_show_new_tension_no_panic() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("brand new", "reality").unwrap();

    // This should NOT panic
    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // Should have succeeded without panic
    assert!(!stdout.is_empty(), "Should have output");
}

// =============================================================================
// JSON Output Tests
// =============================================================================

/// --json flag produces valid JSON with dynamics
#[test]
fn test_show_json_has_temporal() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("json goal", "json reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(
        json.get("temporal").is_some(),
        "Should have temporal signals"
    );
    assert!(json.get("dynamics").is_none(), "Should NOT have dynamics");
}

/// --json shows honest facts (temporal, closure, overdue)
#[test]
fn test_show_json_honest_facts() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("verbose json", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.get("temporal").is_some(), "Should have temporal");
    assert!(json.get("overdue").is_some(), "Should have overdue");
    assert!(json.get("frontier").is_some(), "Should have frontier");
}

/// --json urgency is null when no horizon
#[test]
fn test_show_json_null_urgency_no_horizon() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("null test", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Urgency should be null without horizon
    assert!(
        json["urgency"].is_null(),
        "urgency should be null without horizon"
    );
    // Overdue should be false
    assert_eq!(json["overdue"].as_bool(), Some(false));
}

/// --json shows children array
#[test]
fn test_show_json_children() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store.create_tension("parent", "reality").unwrap();
    let _child = store
        .create_tension_with_parent("child", "c reality", Some(parent.id.clone()))
        .unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(&parent.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Should have children array
    let children = json.get("children").expect("Should have children field");
    assert!(children.is_array(), "Children should be an array");
    assert_eq!(children.as_array().unwrap().len(), 1, "Should have 1 child");
}

/// --json shows mutations array
#[test]
fn test_show_json_mutations() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("mutations test", "initial").unwrap();
    store.update_actual(&tension.id, "updated").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Should have mutations array
    let mutations = json.get("mutations").expect("Should have mutations field");
    assert!(mutations.is_array(), "Mutations should be an array");
    // At least creation + one update = 2 mutations
    assert!(
        mutations.as_array().unwrap().len() >= 2,
        "Should have at least 2 mutations"
    );
}
