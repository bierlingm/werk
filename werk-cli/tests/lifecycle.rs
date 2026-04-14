//! Integration tests for `werk resolve`, `werk release`, and `werk rm` commands.
//!
//! Tests verify:
//! - VAL-CRUD-013: Resolve transitions Active to Resolved
//! - VAL-CRUD-014: Resolve reparents children to roots
//! - VAL-CRUD-015: Resolve fails on non-Active tension
//! - VAL-CRUD-016: Release requires --reason and transitions to Released
//! - VAL-CRUD-017: Release reparents children like resolve
//! - VAL-CRUD-018: Rm deletes tension and reparents children to grandparent

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

// =============================================================================
// RESOLVE command tests
// =============================================================================

/// VAL-CRUD-013: `werk resolve <id>` transitions Active to Resolved
#[test]
fn test_resolve_active_tension() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create a tension
    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();
    let tension_id = tension.id.clone();

    // Resolve via command
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&tension_id)
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Resolved").or(predicate::str::contains("resolved")));

    // Verify status changed
    let updated = store.get_tension(&tension_id).unwrap().unwrap();
    assert_eq!(updated.status, werk_core::TensionStatus::Resolved);
}

/// VAL-CRUD-013: Resolve works with ID prefix
#[test]
fn test_resolve_with_prefix() {
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
        .arg("resolve")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .success();

    let updated = store.get_tension(&tension.id).unwrap().unwrap();
    assert_eq!(updated.status, werk_core::TensionStatus::Resolved);
}

/// VAL-CRUD-014: Resolve reparents children to roots
#[test]
fn test_resolve_auto_resolves_children() {
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
    let child1 = store
        .create_tension_with_parent("child1", "c1", Some(parent.id.clone()))
        .unwrap();
    let child2 = store
        .create_tension_with_parent("child2", "c2", Some(parent.id.clone()))
        .unwrap();

    // Resolve parent
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&parent.id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Children should be auto-resolved, parent relationship preserved
    let child1_after = store.get_tension(&child1.id).unwrap().unwrap();
    let child2_after = store.get_tension(&child2.id).unwrap().unwrap();
    assert_eq!(child1_after.status, werk_core::TensionStatus::Resolved);
    assert_eq!(child2_after.status, werk_core::TensionStatus::Resolved);
    assert_eq!(child1_after.parent_id, Some(parent.id.clone()));
    assert_eq!(child2_after.parent_id, Some(parent.id.clone()));
}

/// VAL-CRUD-015: Resolve on non-Active tension fails
#[test]
fn test_resolve_non_active_fails() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Resolve it first
    store
        .update_status(&tension.id, werk_core::TensionStatus::Resolved)
        .unwrap();

    // Try to resolve again
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("transition")
                .or(predicate::str::contains("cannot"))
                .or(predicate::str::contains("invalid")),
        );
}

/// Resolve records mutation
#[test]
fn test_resolve_records_mutation() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify mutation was recorded
    let mutations = store.get_mutations(&tension.id).unwrap();
    let status_mutation = mutations.iter().find(|m| m.field() == "status");
    assert!(status_mutation.is_some());
    assert_eq!(status_mutation.unwrap().old_value(), Some("Active"));
    assert_eq!(status_mutation.unwrap().new_value(), "Resolved");
}

/// --json flag produces valid JSON for resolve
#[test]
fn test_resolve_json_output() {
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
        .arg("resolve")
        .arg(&tension.id)
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
        json.get("status").is_some(),
        "JSON should have 'status' field"
    );
}

// =============================================================================
// RELEASE command tests
// =============================================================================

/// VAL-CRUD-016: `werk release <id> --reason 'text'` transitions to Released
#[test]
fn test_release_with_reason() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();
    let tension_id = tension.id.clone();

    cargo_bin_cmd!("werk")
        .arg("release")
        .arg(&tension_id)
        .arg("--reason")
        .arg("no longer relevant")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Released").or(predicate::str::contains("released")));

    let updated = store.get_tension(&tension_id).unwrap().unwrap();
    assert_eq!(updated.status, werk_core::TensionStatus::Released);
}

/// VAL-CRUD-016: Release without --reason fails
#[test]
fn test_release_without_reason_fails() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Release without --reason should fail (clap required flag)
    cargo_bin_cmd!("werk")
        .arg("release")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("--reason").or(predicate::str::contains("required")));
}

/// VAL-CRUD-017: Release reparents children like resolve
#[test]
fn test_release_auto_releases_children() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store.create_tension("parent", "p reality").unwrap();
    let child = store
        .create_tension_with_parent("child", "c reality", Some(parent.id.clone()))
        .unwrap();

    cargo_bin_cmd!("werk")
        .arg("release")
        .arg(&parent.id)
        .arg("--reason")
        .arg("abandoned")
        .current_dir(dir.path())
        .assert()
        .success();

    // Child should be auto-released
    let child_after = store.get_tension(&child.id).unwrap().unwrap();
    assert_eq!(child_after.status, werk_core::TensionStatus::Released);
    assert_eq!(child_after.parent_id, Some(parent.id.clone()));
}

/// Release on resolved tension fails
#[test]
fn test_release_on_resolved_fails() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Resolve first
    store
        .update_status(&tension.id, werk_core::TensionStatus::Resolved)
        .unwrap();

    // Try to release
    cargo_bin_cmd!("werk")
        .arg("release")
        .arg(&tension.id)
        .arg("--reason")
        .arg("test")
        .current_dir(dir.path())
        .assert()
        .failure();
}

/// --json flag produces valid JSON for release
#[test]
fn test_release_json_output() {
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
        .arg("release")
        .arg(&tension.id)
        .arg("--reason")
        .arg("test reason")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.get("id").is_some());
    assert!(json.get("status").is_some());
}

// =============================================================================
// RM command tests
// =============================================================================

/// VAL-CRUD-018: `werk rm <id>` deletes tension
#[test]
fn test_rm_deletes_tension() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();
    let tension_id = tension.id.clone();

    cargo_bin_cmd!("werk")
        .arg("rm")
        .arg(&tension_id)
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted").or(predicate::str::contains("removed")));

    // Tension should be gone
    let result = store.get_tension(&tension_id).unwrap();
    assert!(result.is_none());
}

/// VAL-CRUD-018: Rm reparents children to grandparent
#[test]
fn test_rm_reparents_to_grandparent() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();

    // Create A -> B -> C hierarchy
    let grandparent = store.create_tension("A", "a").unwrap();
    let parent = store
        .create_tension_with_parent("B", "b", Some(grandparent.id.clone()))
        .unwrap();
    let child = store
        .create_tension_with_parent("C", "c", Some(parent.id.clone()))
        .unwrap();

    // Delete B
    cargo_bin_cmd!("werk")
        .arg("rm")
        .arg(&parent.id)
        .current_dir(dir.path())
        .assert()
        .success();

    // B should be gone
    assert!(store.get_tension(&parent.id).unwrap().is_none());

    // C's parent should now be A
    let child_after = store.get_tension(&child.id).unwrap().unwrap();
    assert_eq!(child_after.parent_id, Some(grandparent.id));
}

/// VAL-CRUD-018: Rm on root with children makes children roots
#[test]
fn test_rm_root_makes_children_roots() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store.create_tension("parent", "p").unwrap();
    let child1 = store
        .create_tension_with_parent("child1", "c1", Some(parent.id.clone()))
        .unwrap();
    let child2 = store
        .create_tension_with_parent("child2", "c2", Some(parent.id.clone()))
        .unwrap();

    cargo_bin_cmd!("werk")
        .arg("rm")
        .arg(&parent.id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Children should now be roots
    let child1_after = store.get_tension(&child1.id).unwrap().unwrap();
    let child2_after = store.get_tension(&child2.id).unwrap().unwrap();
    assert!(child1_after.parent_id.is_none());
    assert!(child2_after.parent_id.is_none());
}

/// Rm on nonexistent tension fails
#[test]
fn test_rm_not_found() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("rm")
        .arg("ZZZZZZZZZZZZZZZZZZZZZZZZZZ")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Rm works with ID prefix
#[test]
fn test_rm_with_prefix() {
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
        .arg("rm")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(store.get_tension(&tension.id).unwrap().is_none());
}

/// --json flag produces valid JSON for rm
#[test]
fn test_rm_json_output() {
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
        .arg("rm")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.get("id").is_some());
}

// =============================================================================
// Cross-cutting tests
// =============================================================================

/// Commands require workspace
#[test]
fn test_resolve_requires_workspace() {
    let dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg("SOMEID")
        .env("HOME", home.path())
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn test_release_requires_workspace() {
    let dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("release")
        .arg("SOMEID")
        .arg("--reason")
        .arg("test")
        .env("HOME", home.path())
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn test_rm_requires_workspace() {
    let dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("rm")
        .arg("SOMEID")
        .env("HOME", home.path())
        .current_dir(dir.path())
        .assert()
        .failure();
}

/// Full lifecycle flow: init -> add -> resolve
#[test]
fn test_full_lifecycle_resolve() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Add via CLI (to get the actual ID from output)
    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Resolve via CLI using the actual tension ID
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify the tension is resolved
    let updated = store.get_tension(&tension.id).unwrap().unwrap();
    assert_eq!(updated.status, werk_core::TensionStatus::Resolved);
}

/// Release reason is recorded in mutation
#[test]
fn test_release_reason_recorded() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = werk_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    cargo_bin_cmd!("werk")
        .arg("release")
        .arg(&tension.id)
        .arg("--reason")
        .arg("changed priorities")
        .current_dir(dir.path())
        .assert()
        .success();

    // Check that a release_reason mutation was recorded
    let mutations = store.get_mutations(&tension.id).unwrap();
    let reason_mutation = mutations.iter().find(|m| m.field() == "release_reason");
    assert!(
        reason_mutation.is_some(),
        "should have release_reason mutation"
    );
    assert_eq!(reason_mutation.unwrap().new_value(), "changed priorities");
}
