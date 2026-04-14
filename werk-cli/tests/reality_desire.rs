//! Integration tests for `werk reality` and `werk desire` commands.
//!
//! Tests verify:
//! - VAL-CRUD-008: Reality updates actual field with mutation
//! - VAL-CRUD-009: Reality opens $EDITOR when value omitted
//! - VAL-CRUD-010: Reality fails on resolved/released tension
//! - VAL-CRUD-011: Desire updates desired field with mutation
//! - VAL-CRUD-012: Desire opens $EDITOR when value omitted

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

// =============================================================================
// REALITY command tests
// =============================================================================

/// VAL-CRUD-008: `werk reality <id> "new actual"` updates actual and records mutation
#[test]
fn test_reality_updates_actual() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create a tension
    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "initial reality").unwrap();
    let tension_id = tension.id.clone();

    // Update actual via command
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&tension_id)
        .arg("updated reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated reality"));

    // Verify the actual was updated
    let updated = store.get_tension(&tension_id).unwrap().unwrap();
    assert_eq!(updated.actual, "updated reality");
    assert_eq!(updated.desired, "goal"); // unchanged

    // Verify a mutation was recorded
    let mutations = store.get_mutations(&tension_id).unwrap();
    assert!(
        mutations.len() >= 2,
        "should have creation + update mutations"
    );
    let actual_mutation = mutations.iter().find(|m| m.field() == "actual");
    assert!(
        actual_mutation.is_some(),
        "should have mutation for actual field"
    );
}

/// VAL-CRUD-008: Reality shows old and new values in output
#[test]
fn test_reality_shows_old_and_new() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "old value").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&tension.id)
        .arg("new value")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("old value") || stdout.contains("Old"),
        "Should show old value, got: {}",
        stdout
    );
    assert!(
        stdout.contains("new value"),
        "Should show new value, got: {}",
        stdout
    );
}

/// VAL-CRUD-008: Reality works with ID prefix
#[test]
fn test_reality_with_prefix() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();
    let prefix = &tension.id[..6];

    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(prefix)
        .arg("updated")
        .current_dir(dir.path())
        .assert()
        .success();

    let updated = store.get_tension(&tension.id).unwrap().unwrap();
    assert_eq!(updated.actual, "updated");
}

/// Reality rejects empty value
#[test]
fn test_reality_rejects_empty() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&tension.id)
        .arg("")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty").or(predicate::str::contains("cannot")));
}

/// VAL-CRUD-009: Reality opens $EDITOR when value omitted
#[test]
#[ignore] // Requires TTY — CLI correctly refuses editor in non-interactive mode
fn test_reality_opens_editor() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "initial").unwrap();

    // Use EDITOR=cat to verify the current value is passed to the editor
    // cat will just output the content, which should result in no change
    let output = cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&tension.id)
        .env("EDITOR", "cat")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // With EDITOR=cat, the content is passed through unchanged
    // So we should see the initial value in stdout (from cat)
    // And the command should report no changes
    assert!(
        stdout.contains("initial") || stdout.contains("No changes"),
        "Should see initial value or no changes message, got: {}",
        stdout
    );

    // Actual should remain unchanged
    let updated = store.get_tension(&tension.id).unwrap().unwrap();
    assert_eq!(updated.actual, "initial");
}

/// VAL-CRUD-010: Reality fails on resolved tension
#[test]
fn test_reality_fails_on_resolved() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Resolve the tension directly
    store
        .update_status(&tension.id, werk_core::TensionStatus::Resolved)
        .unwrap();

    // Try to update reality
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&tension.id)
        .arg("new reality")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Resolved")
                .or(predicate::str::contains("cannot update"))
                .or(predicate::str::contains("inactive")),
        );

    // Verify actual unchanged
    let check = store.get_tension(&tension.id).unwrap().unwrap();
    assert_eq!(check.actual, "reality");
}

/// VAL-CRUD-010: Reality fails on released tension
#[test]
fn test_reality_fails_on_released() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Release the tension directly
    store
        .update_status(&tension.id, werk_core::TensionStatus::Released)
        .unwrap();

    // Try to update reality
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&tension.id)
        .arg("new reality")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Released")
                .or(predicate::str::contains("cannot update"))
                .or(predicate::str::contains("inactive")),
        );
}

/// Reality requires workspace
#[test]
fn test_reality_requires_workspace() {
    let dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg("SOMEID")
        .arg("value")
        .env("HOME", home.path())
        .current_dir(dir.path())
        .assert()
        .failure();
}

/// Reality shows not found for invalid ID
#[test]
fn test_reality_not_found() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg("ZZZZZZZZZZZZZZZZZZZZZZZZZZ")
        .arg("value")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// --json flag produces valid JSON for reality
#[test]
fn test_reality_json_output() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("reality")
        .arg(&tension.id)
        .arg("updated")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.get("id").is_some(), "JSON should have 'id' field");
    assert!(
        json.get("actual").is_some(),
        "JSON should have 'actual' field"
    );
    assert_eq!(json.get("actual").unwrap().as_str().unwrap(), "updated");
}

// =============================================================================
// DESIRE command tests
// =============================================================================

/// VAL-CRUD-011: `werk desire <id> "new desired"` updates desired and records mutation
#[test]
fn test_desire_updates_desired() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("initial goal", "reality").unwrap();
    let tension_id = tension.id.clone();

    cargo_bin_cmd!("werk")
        .arg("desire")
        .arg(&tension_id)
        .arg("refined goal")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated desired"));

    let updated = store.get_tension(&tension_id).unwrap().unwrap();
    assert_eq!(updated.desired, "refined goal");
    assert_eq!(updated.actual, "reality"); // unchanged

    // Verify mutation was recorded
    let mutations = store.get_mutations(&tension_id).unwrap();
    let desired_mutation = mutations.iter().find(|m| m.field() == "desired");
    assert!(desired_mutation.is_some());
}

/// VAL-CRUD-011: Desire shows old and new values
#[test]
fn test_desire_shows_old_and_new() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("old goal", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("desire")
        .arg(&tension.id)
        .arg("new goal")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("old goal") || stdout.contains("Old"));
    assert!(stdout.contains("new goal"));
}

/// Desire works with ID prefix
#[test]
fn test_desire_with_prefix() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();
    let prefix = &tension.id[..6];

    cargo_bin_cmd!("werk")
        .arg("desire")
        .arg(prefix)
        .arg("updated goal")
        .current_dir(dir.path())
        .assert()
        .success();

    let updated = store.get_tension(&tension.id).unwrap().unwrap();
    assert_eq!(updated.desired, "updated goal");
}

/// Desire rejects empty value
#[test]
fn test_desire_rejects_empty() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    cargo_bin_cmd!("werk")
        .arg("desire")
        .arg(&tension.id)
        .arg("")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty").or(predicate::str::contains("cannot")));
}

/// VAL-CRUD-012: Desire opens $EDITOR when value omitted
#[test]
#[ignore] // Requires TTY — CLI correctly refuses editor in non-interactive mode
fn test_desire_opens_editor() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("initial goal", "reality").unwrap();

    // Use EDITOR=cat to verify the current value is passed to the editor
    let output = cargo_bin_cmd!("werk")
        .arg("desire")
        .arg(&tension.id)
        .env("EDITOR", "cat")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // With EDITOR=cat, the content is passed through unchanged
    assert!(
        stdout.contains("initial goal") || stdout.contains("No changes"),
        "Should see initial goal or no changes message, got: {}",
        stdout
    );

    // Desired should remain unchanged
    let updated = store.get_tension(&tension.id).unwrap().unwrap();
    assert_eq!(updated.desired, "initial goal");
}

/// Desire fails on resolved tension (same as reality)
#[test]
fn test_desire_fails_on_resolved() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    store
        .update_status(&tension.id, werk_core::TensionStatus::Resolved)
        .unwrap();

    cargo_bin_cmd!("werk")
        .arg("desire")
        .arg(&tension.id)
        .arg("new goal")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Resolved")
                .or(predicate::str::contains("cannot update"))
                .or(predicate::str::contains("inactive")),
        );
}

/// Desire fails on released tension
#[test]
fn test_desire_fails_on_released() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    store
        .update_status(&tension.id, werk_core::TensionStatus::Released)
        .unwrap();

    cargo_bin_cmd!("werk")
        .arg("desire")
        .arg(&tension.id)
        .arg("new goal")
        .current_dir(dir.path())
        .assert()
        .failure();
}

/// Desire requires workspace
#[test]
fn test_desire_requires_workspace() {
    let dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("desire")
        .arg("SOMEID")
        .arg("value")
        .env("HOME", home.path())
        .current_dir(dir.path())
        .assert()
        .failure();
}

/// Desire shows not found for invalid ID
#[test]
fn test_desire_not_found() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("desire")
        .arg("ZZZZZZZZZZZZZZZZZZZZZZZZZZ")
        .arg("value")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// --json flag produces valid JSON for desire
#[test]
fn test_desire_json_output() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("desire")
        .arg(&tension.id)
        .arg("updated goal")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.get("id").is_some(), "JSON should have 'id' field");
    assert!(
        json.get("desired").is_some(),
        "JSON should have 'desired' field"
    );
    assert_eq!(
        json.get("desired").unwrap().as_str().unwrap(),
        "updated goal"
    );
}

// =============================================================================
// Cross-cutting tests
// =============================================================================

/// Editor modifies the content
/// This test uses a script file that acts as an editor
#[test]
#[ignore] // Requires TTY — CLI correctly refuses editor in non-interactive mode
fn test_editor_modifies_content() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "initial reality").unwrap();

    // Create a script file that acts as an "editor" - it overwrites the file
    let script_path = dir.path().join("fake_editor.sh");
    std::fs::write(
        &script_path,
        "#!/bin/sh\necho \"modified reality\" > \"$1\"",
    )
    .unwrap();

    // Make script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&tension.id)
        .env("EDITOR", &script_path)
        .current_dir(dir.path())
        .assert()
        .success();

    // Check if reality was updated
    let updated = store.get_tension(&tension.id).unwrap().unwrap();
    assert_eq!(
        updated.actual, "modified reality",
        "actual should have been updated by fake editor"
    );
}

/// Verify mutations are recorded for reality updates
#[test]
fn test_reality_records_mutation() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Update reality
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&tension.id)
        .arg("new reality")
        .current_dir(dir.path())
        .assert()
        .success();

    // Check mutations
    let mutations = store.get_mutations(&tension.id).unwrap();

    // Should have at least 2 mutations: creation and actual update
    assert!(mutations.len() >= 2);

    // Find the actual mutation
    let actual_mut = mutations.iter().find(|m| m.field() == "actual");
    assert!(actual_mut.is_some());

    let actual_mut = actual_mut.unwrap();
    assert_eq!(actual_mut.old_value(), Some("reality"));
    assert_eq!(actual_mut.new_value(), "new reality");
}

/// Verify mutations are recorded for desire updates
#[test]
fn test_desire_records_mutation() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Update desire
    cargo_bin_cmd!("werk")
        .arg("desire")
        .arg(&tension.id)
        .arg("new goal")
        .current_dir(dir.path())
        .assert()
        .success();

    // Check mutations
    let mutations = store.get_mutations(&tension.id).unwrap();
    let desired_mut = mutations.iter().find(|m| m.field() == "desired");
    assert!(desired_mut.is_some());

    let desired_mut = desired_mut.unwrap();
    assert_eq!(desired_mut.old_value(), Some("goal"));
    assert_eq!(desired_mut.new_value(), "new goal");
}

/// Multiple updates create multiple mutations
#[test]
fn test_multiple_reality_updates() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "v1").unwrap();

    // Multiple updates
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&tension.id)
        .arg("v2")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(&tension.id)
        .arg("v3")
        .current_dir(dir.path())
        .assert()
        .success();

    // Check mutations
    let mutations = store.get_mutations(&tension.id).unwrap();
    let actual_mutations: Vec<_> = mutations.iter().filter(|m| m.field() == "actual").collect();
    assert_eq!(actual_mutations.len(), 2, "should have 2 actual mutations");
}
