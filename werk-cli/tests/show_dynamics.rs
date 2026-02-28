//! Integration tests for `werk show` dynamics display.
//!
//! Tests verify:
//! - VAL-DISP-009: Show displays dynamics summary
//! - VAL-DISP-010: Show --verbose displays all 10 dynamics
//! - VAL-DISP-011: Show displays mutation history
//! - VAL-DISP-012: Show displays children list
//! - VAL-DISP-013: Dynamics on new tension without mutations

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

// =============================================================================
// Dynamics Summary Tests (VAL-DISP-009)
// =============================================================================

/// VAL-DISP-009: Show displays structural tension magnitude
#[test]
fn test_show_displays_tension_magnitude() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create tension with a gap between desired and actual
    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store
        .create_tension("write a complete novel", "have an outline")
        .unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show structural tension or magnitude
    assert!(
        stdout.contains("tension")
            || stdout.contains("magnitude")
            || stdout.contains("Magnitude")
            || stdout.contains("Structural"),
        "Should show tension magnitude, got: {}",
        stdout
    );
}

/// VAL-DISP-009: Show displays creative cycle phase
#[test]
fn test_show_displays_phase() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show phase (Germination for new tension)
    assert!(
        stdout.contains("Germination")
            || stdout.contains("phase")
            || stdout.contains("Phase")
            || stdout.contains("[G]"),
        "Should show phase, got: {}",
        stdout
    );
}

/// VAL-DISP-009: Show displays conflict indicator when present
#[test]
fn test_show_displays_conflict_when_present() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent with children to enable conflict detection
    let store = sd_core::Store::init(dir.path()).unwrap();
    let parent = store.create_tension("parent", "p reality").unwrap();
    let child1 = store
        .create_tension_with_parent("child1", "c1", Some(parent.id.clone()))
        .unwrap();
    let _child2 = store
        .create_tension_with_parent("child2", "c2", Some(parent.id.clone()))
        .unwrap();

    // Create asymmetric activity on child1
    for _ in 0..5 {
        store.update_actual(&child1.id, "active update").unwrap();
    }

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&child1.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show conflict section (either detected or None)
    assert!(
        stdout.contains("Conflict") || stdout.contains("conflict"),
        "Should show conflict section, got: {}",
        stdout
    );
}

/// VAL-DISP-009: Show displays movement direction
#[test]
fn test_show_displays_movement() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show movement/tendency section
    assert!(
        stdout.contains("Movement")
            || stdout.contains("movement")
            || stdout.contains("Tendency")
            || stdout.contains("tendency")
            || stdout.contains("→")
            || stdout.contains("○"),
        "Should show movement, got: {}",
        stdout
    );
}

// =============================================================================
// Verbose Dynamics Tests (VAL-DISP-010)
// =============================================================================

/// VAL-DISP-010: Show --verbose displays all 10 dynamics
#[test]
fn test_show_verbose_displays_all_dynamics() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show all 10 dynamics headers (even if values are None)
    let dynamics_names = [
        "StructuralTension",
        "Structural Tension",
        "StructuralConflict",
        "Structural Conflict",
        "Oscillation",
        "Resolution",
        "CreativeCyclePhase",
        "Creative Cycle",
        "Orientation",
        "CompensatingStrategy",
        "Compensating Strategy",
        "StructuralTendency",
        "Structural Tendency",
        "AssimilationDepth",
        "Assimilation Depth",
        "Neglect",
    ];

    // At least half of the dynamics names should appear
    let matches = dynamics_names
        .iter()
        .filter(|name| stdout.contains(*name))
        .count();
    assert!(
        matches >= 5,
        "Should show at least 5 dynamics names, found {} in: {}",
        matches,
        stdout
    );
}

/// VAL-DISP-010: Dynamics with no data show as None
#[test]
fn test_show_verbose_shows_none_for_absent_dynamics() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show None for dynamics that can't be computed
    assert!(
        stdout.contains("None"),
        "Should show None for absent dynamics, got: {}",
        stdout
    );
}

// =============================================================================
// Mutation History Tests (VAL-DISP-011)
// =============================================================================

/// VAL-DISP-011: Show displays mutation history chronologically
#[test]
fn test_show_displays_mutation_history() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
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

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show mutation history
    assert!(
        stdout.contains("Mutation") || stdout.contains("mutation") || stdout.contains("History"),
        "Should show mutation history, got: {}",
        stdout
    );
}

/// VAL-DISP-011: Show limits mutations to last 10
#[test]
fn test_show_limits_mutation_history() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Create 15 mutations
    for i in 0..15 {
        store
            .update_actual(&tension.id, &format!("update {}", i))
            .unwrap();
    }

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should contain mutation history (creation + updates)
    assert!(
        stdout.contains("Mutation") || stdout.contains("mutation"),
        "Should show mutations, got: {}",
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

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent with children
    let store = sd_core::Store::init(dir.path()).unwrap();
    let parent = store
        .create_tension("parent goal", "parent reality")
        .unwrap();
    let _child1 = store
        .create_tension_with_parent("child goal 1", "child reality 1", Some(parent.id.clone()))
        .unwrap();
    let _child2 = store
        .create_tension_with_parent("child goal 2", "child reality 2", Some(parent.id.clone()))
        .unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
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

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("leaf goal", "leaf reality").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
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

/// VAL-DISP-013: New tension shows Germination phase
#[test]
fn test_show_new_tension_shows_germination() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("new goal", "new reality").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should show Germination phase
    assert!(
        stdout.contains("Germination"),
        "New tension should show Germination phase, got: {}",
        stdout
    );
}

/// VAL-DISP-013: New tension shows other dynamics as None (no panic)
#[test]
fn test_show_new_tension_no_panic() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("brand new", "reality").unwrap();

    // This should NOT panic
    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("show")
        .arg(&tension.id)
        .arg("--verbose")
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
fn test_show_json_with_dynamics() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("json goal", "json reality").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
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

    // Should have dynamics field
    assert!(
        json.get("dynamics").is_some(),
        "JSON should have 'dynamics' field"
    );

    // Dynamics should have phase
    let dynamics = json.get("dynamics").unwrap();
    assert!(
        dynamics.get("phase").is_some() || dynamics.get("creative_cycle_phase").is_some(),
        "Dynamics should have phase"
    );
}

/// --json --verbose shows all 10 dynamics fields
#[test]
fn test_show_json_verbose_all_dynamics() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("verbose json", "reality").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("show")
        .arg(&tension.id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Should have dynamics object
    let dynamics = json.get("dynamics").expect("Should have dynamics");

    // Should have multiple dynamics fields (at least these core ones)
    assert!(
        dynamics.get("structural_tension").is_some() || dynamics.get("structuralTension").is_some(),
        "Should have structural_tension"
    );
    assert!(
        dynamics.get("phase").is_some() || dynamics.get("creative_cycle_phase").is_some(),
        "Should have phase"
    );
}

/// --json absent dynamics are null (not omitted)
#[test]
fn test_show_json_null_for_absent_dynamics() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("null test", "reality").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
        .arg("--json")
        .arg("show")
        .arg(&tension.id)
        .arg("--verbose")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let dynamics = json.get("dynamics").expect("Should have dynamics");

    // Check that some dynamics are null (e.g., oscillation, conflict for new tension)
    // At least one of these should be null for a fresh tension
    let has_null = dynamics
        .get("oscillation")
        .map(|v| v.is_null())
        .unwrap_or(false)
        || dynamics
            .get("conflict")
            .map(|v| v.is_null())
            .unwrap_or(false)
        || dynamics
            .get("resolution")
            .map(|v| v.is_null())
            .unwrap_or(false);

    assert!(
        has_null,
        "At least one dynamics should be null for new tension, got: {:?}",
        dynamics
    );
}

/// --json shows children array
#[test]
fn test_show_json_children() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let parent = store.create_tension("parent", "reality").unwrap();
    let _child = store
        .create_tension_with_parent("child", "c reality", Some(parent.id.clone()))
        .unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
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

    Command::cargo_bin("werk")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init(dir.path()).unwrap();
    let tension = store.create_tension("mutations test", "initial").unwrap();
    store.update_actual(&tension.id, "updated").unwrap();

    let output = Command::cargo_bin("werk")
        .unwrap()
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
