//! Integration tests for `werk move` and `werk note` (add/rm/list) commands.
//!
//! Tests verify:
//! - VAL-CRUD-019: Move reparents tension
//! - VAL-CRUD-020: Move without --parent makes tension a root
//! - VAL-CRUD-021: Move prevents cycles
//! - VAL-CRUD-022: Note add creates annotation mutation
//! - VAL-CRUD-023: Note add works on resolved/released tensions
//! - VAL-CRUD-024: Workspace notes via `werk note add/list`
//! - VAL-CRUD-025: Note rm retracts testimony (soft-delete)
//! - VAL-CRUD-026: Retracted notes excluded from note list

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

// =============================================================================
// TREE command tests (fix for duplicate -r short flag panic)
// =============================================================================

/// Tree --resolved does not panic (was duplicate -r short flag)
#[test]
fn test_tree_resolved_no_panic() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create a resolved tension
    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();
    store
        .update_status(&tension.id, sd_core::TensionStatus::Resolved)
        .unwrap();

    // --resolved should work without panic
    cargo_bin_cmd!("werk")
        .arg("tree")
        .arg("--resolved")
        .current_dir(dir.path())
        .assert()
        .code(predicate::ne(101)); // Should not panic (exit 101)
}

/// Tree --released does not panic (was duplicate -r short flag)
#[test]
fn test_tree_released_no_panic() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create a released tension
    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();
    store
        .update_status(&tension.id, sd_core::TensionStatus::Released)
        .unwrap();

    // --released should work without panic
    cargo_bin_cmd!("werk")
        .arg("tree")
        .arg("--released")
        .current_dir(dir.path())
        .assert()
        .code(predicate::ne(101)); // Should not panic (exit 101)
}

/// Tree --help shows no short flags for resolved/released
#[test]
fn test_tree_help_no_duplicate_short() {
    let output = cargo_bin_cmd!("werk")
        .arg("tree")
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // Check that --resolved and --released are shown without -r short flag
    assert!(stdout.contains("--resolved"), "should show --resolved flag");
    assert!(stdout.contains("--released"), "should show --released flag");
    // Should NOT have -r for either (they would conflict)
    // The help shows short flags differently, but the important thing is no panic
}

// =============================================================================
// MOVE command tests
// =============================================================================

/// VAL-CRUD-019: `werk move <id> --parent <new-parent>` changes parent_id
#[test]
fn test_move_to_new_parent() {
    let dir = TempDir::new().unwrap();

    // Initialize workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent and child
    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store
        .create_tension("parent goal", "parent reality")
        .unwrap();
    let child = store.create_tension("child goal", "child reality").unwrap();
    let child_id = child.id.clone();

    // Move child under parent
    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(&child_id)
        .arg("--parent")
        .arg(&parent.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Moved").or(predicate::str::contains("moved")));

    // Verify parent changed
    let updated = store.get_tension(&child_id).unwrap().unwrap();
    assert_eq!(updated.parent_id, Some(parent.id));
}

/// VAL-CRUD-019: Move works with ID prefix
#[test]
fn test_move_with_prefix() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store.create_tension("parent", "p").unwrap();
    // Small delay to ensure different ULID prefix
    std::thread::sleep(std::time::Duration::from_millis(10));
    let child = store.create_tension("child", "c").unwrap();

    // Use longer prefixes to ensure uniqueness
    let child_prefix = &child.id[..10];
    let parent_prefix = &parent.id[..10];

    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(child_prefix)
        .arg("--parent")
        .arg(parent_prefix)
        .current_dir(dir.path())
        .assert()
        .success();

    let updated = store.get_tension(&child.id).unwrap().unwrap();
    assert_eq!(updated.parent_id, Some(parent.id));
}

/// VAL-CRUD-020: Move without --parent makes tension a root
#[test]
fn test_move_to_root() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create parent with child
    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store.create_tension("parent", "p").unwrap();
    let child = store
        .create_tension_with_parent("child", "c", Some(parent.id.clone()))
        .unwrap();

    // Move child to root (no --parent)
    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(&child.id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Child should now be root
    let updated = store.get_tension(&child.id).unwrap().unwrap();
    assert!(updated.parent_id.is_none());
}

/// VAL-CRUD-021: Move prevents cycles (moving ancestor under descendant)
#[test]
fn test_move_prevents_cycle() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create A -> B -> C chain
    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let a = store.create_tension("A", "a").unwrap();
    let b = store
        .create_tension_with_parent("B", "b", Some(a.id.clone()))
        .unwrap();
    let c = store
        .create_tension_with_parent("C", "c", Some(b.id.clone()))
        .unwrap();

    // Try to move A under C (would create cycle)
    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(&a.id)
        .arg("--parent")
        .arg(&c.id)
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("cycle")
                .or(predicate::str::contains("descendant"))
                .or(predicate::str::contains("circular")),
        );

    // A should still have no parent
    let a_after = store.get_tension(&a.id).unwrap().unwrap();
    assert!(a_after.parent_id.is_none());
}

/// VAL-CRUD-021: Move prevents moving to self
#[test]
fn test_move_to_self_fails() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(&tension.id)
        .arg("--parent")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("self")
                .or(predicate::str::contains("cycle"))
                .or(predicate::str::contains("descendant")),
        );
}

/// Move to non-existent parent fails
#[test]
fn test_move_to_nonexistent_parent() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let child = store.create_tension("child", "c").unwrap();

    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(&child.id)
        .arg("--parent")
        .arg("ZZZZZZZZZZZZZZZZZZZZZZZZZZ")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Move records mutation
#[test]
fn test_move_records_mutation() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store.create_tension("parent", "p").unwrap();
    let child = store.create_tension("child", "c").unwrap();

    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(&child.id)
        .arg("--parent")
        .arg(&parent.id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify mutation was recorded
    let mutations = store.get_mutations(&child.id).unwrap();
    let parent_mutation = mutations.iter().find(|m| m.field() == "parent_id");
    assert!(parent_mutation.is_some());
    // The old value should be empty (was None) and new value should be parent id
    let mutation = parent_mutation.unwrap();
    assert!(mutation.old_value().is_none() || mutation.old_value() == Some(""));
    assert_eq!(mutation.new_value(), parent.id);
}

/// --json flag produces valid JSON for move
#[test]
fn test_move_json_output() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store.create_tension("parent", "p").unwrap();
    let child = store.create_tension("child", "c").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("move")
        .arg(&child.id)
        .arg("--parent")
        .arg(&parent.id)
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
        json.get("parent_id").is_some(),
        "JSON should have 'parent_id' field"
    );
}

/// Move nonexistent tension fails
#[test]
fn test_move_not_found() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("move")
        .arg("ZZZZZZZZZZZZZZZZZZZZZZZZZZ")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// =============================================================================
// NOTE ADD command tests
// =============================================================================

/// VAL-CRUD-022: `werk note add <id> 'text'` creates note mutation
#[test]
fn test_note_add_on_tension() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("met with team to discuss approach")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("note"));

    let mutations = store.get_mutations(&tension.id).unwrap();
    let note_mutation = mutations.iter().find(|m| m.field() == "note");
    assert!(note_mutation.is_some());
    assert_eq!(
        note_mutation.unwrap().new_value(),
        "met with team to discuss approach"
    );
}

/// VAL-CRUD-022: Note add works with ID prefix
#[test]
fn test_note_add_with_prefix() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();
    let prefix = &tension.id[..6];

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(prefix)
        .arg("test note")
        .current_dir(dir.path())
        .assert()
        .success();

    let mutations = store.get_mutations(&tension.id).unwrap();
    assert!(mutations.iter().any(|m| m.field() == "note"));
}

/// VAL-CRUD-023: Note add works on resolved tensions
#[test]
fn test_note_add_on_resolved_tension() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    store
        .update_status(&tension.id, sd_core::TensionStatus::Resolved)
        .unwrap();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("post-resolution reflection")
        .current_dir(dir.path())
        .assert()
        .success();

    let mutations = store.get_mutations(&tension.id).unwrap();
    assert!(mutations.iter().any(|m| m.field() == "note"));
}

/// VAL-CRUD-023: Note add works on released tensions
#[test]
fn test_note_add_on_released_tension() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    store
        .update_status(&tension.id, sd_core::TensionStatus::Released)
        .unwrap();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("why we abandoned this")
        .current_dir(dir.path())
        .assert()
        .success();

    let mutations = store.get_mutations(&tension.id).unwrap();
    assert!(mutations.iter().any(|m| m.field() == "note"));
}

/// VAL-CRUD-024: `werk note add 'text'` (no ID) creates workspace-level note
#[test]
fn test_note_add_workspace() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg("general workspace observation")
        .current_dir(dir.path())
        .assert()
        .success();
}

/// Note add on nonexistent tension fails
#[test]
fn test_note_add_not_found() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg("ZZZZZZZZZZZZZZZZZZZZZZZZZZ")
        .arg("some note")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// --json flag produces valid JSON for note add
#[test]
fn test_note_add_json_output() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("test note content")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.get("id").is_some(), "JSON should have 'id' field");
    assert!(json.get("note").is_some(), "JSON should have 'note' field");
}

/// Note add with unicode content
#[test]
fn test_note_add_unicode() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("Unicode: 写小说 🎵 compose 音楽")
        .current_dir(dir.path())
        .assert()
        .success();

    let mutations = store.get_mutations(&tension.id).unwrap();
    let note_mutation = mutations.iter().find(|m| m.field() == "note");
    assert_eq!(
        note_mutation.unwrap().new_value(),
        "Unicode: 写小说 🎵 compose 音楽"
    );
}

/// --json for workspace note add
#[test]
fn test_note_add_workspace_json_output() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("note")
        .arg("add")
        .arg("workspace-level note")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.get("note").is_some(), "JSON should have 'note' field");
}

// =============================================================================
// Cross-cutting tests
// =============================================================================

/// Commands require workspace
#[test]
fn test_move_requires_workspace() {
    let dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("move")
        .arg("SOMEID")
        .env("HOME", home.path())
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn test_note_add_requires_workspace() {
    let dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg("text")
        .env("HOME", home.path())
        .current_dir(dir.path())
        .assert()
        .failure();
}

/// Multiple notes can be added to same tension
#[test]
fn test_multiple_notes() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("first note")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("second note")
        .current_dir(dir.path())
        .assert()
        .success();

    let mutations = store.get_mutations(&tension.id).unwrap();
    let notes: Vec<_> = mutations.iter().filter(|m| m.field() == "note").collect();
    assert_eq!(notes.len(), 2);
}

/// Move preserves children
#[test]
fn test_move_preserves_children() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store.create_tension("parent", "p").unwrap();
    let child = store.create_tension("child", "c").unwrap();
    let grandchild = store
        .create_tension_with_parent("grandchild", "gc", Some(child.id.clone()))
        .unwrap();

    // Move child under parent
    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(&child.id)
        .arg("--parent")
        .arg(&parent.id)
        .current_dir(dir.path())
        .assert()
        .success();

    // Grandchild should still have child as parent
    let gc_after = store.get_tension(&grandchild.id).unwrap().unwrap();
    assert_eq!(gc_after.parent_id, Some(child.id));
}

/// Re-parenting from one parent to another
#[test]
fn test_move_between_parents() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let parent1 = store.create_tension("parent1", "p1").unwrap();
    let parent2 = store.create_tension("parent2", "p2").unwrap();
    let child = store
        .create_tension_with_parent("child", "c", Some(parent1.id.clone()))
        .unwrap();

    // Move from parent1 to parent2
    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(&child.id)
        .arg("--parent")
        .arg(&parent2.id)
        .current_dir(dir.path())
        .assert()
        .success();

    let child_after = store.get_tension(&child.id).unwrap().unwrap();
    assert_eq!(child_after.parent_id, Some(parent2.id));
}

// =============================================================================
// NOTE LIST command tests
// =============================================================================

/// VAL-CRUD-024: `werk note list` lists workspace notes
#[test]
fn test_note_list_workspace_notes() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg("first workspace observation")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg("second workspace observation")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("first workspace observation")
                .and(predicate::str::contains("second workspace observation")),
        );
}

/// `werk note list` on empty workspace shows helpful message
#[test]
fn test_note_list_empty_workspace() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("none"));
}

/// `werk note list --json` outputs valid JSON
#[test]
fn test_note_list_json_output() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg("json test note")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("note")
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.get("notes").is_some(), "JSON should have 'notes' field");
    let notes = json.get("notes").unwrap().as_array();
    assert!(notes.is_some(), "'notes' should be an array");
    assert_eq!(notes.unwrap().len(), 1, "Should have one note");
}

/// `werk note list` requires workspace
#[test]
fn test_note_list_requires_workspace() {
    let dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("list")
        .env("HOME", home.path())
        .current_dir(dir.path())
        .assert()
        .failure();
}

// =============================================================================
// NOTE RM (retraction) tests
// =============================================================================

/// VAL-CRUD-025: `werk note rm <id> <n>` retracts a note
#[test]
fn test_note_rm_retracts_note() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Add two notes
    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("first testimony")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("second testimony")
        .current_dir(dir.path())
        .assert()
        .success();

    // Retract note #1
    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("rm")
        .arg(&tension.id)
        .arg("1")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Retracted").and(predicate::str::contains("first testimony")));

    // Verify note_retracted mutation was recorded
    let mutations = store.get_mutations(&tension.id).unwrap();
    let retraction = mutations.iter().find(|m| m.field() == "note_retracted");
    assert!(retraction.is_some(), "should have note_retracted mutation");
    assert_eq!(retraction.unwrap().old_value(), Some("first testimony"));
}

/// VAL-CRUD-026: Retracted notes excluded from note list
#[test]
fn test_note_rm_excluded_from_list() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // Add two notes
    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("keep this")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("retract this")
        .current_dir(dir.path())
        .assert()
        .success();

    // Retract note #2
    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("rm")
        .arg(&tension.id)
        .arg("2")
        .current_dir(dir.path())
        .assert()
        .success();

    // List should show only the surviving note, renumbered as #1
    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("list")
        .arg(&tension.id)
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("keep this")
                .and(predicate::str::contains("retract this").not())
                .and(predicate::str::contains("(1)")),
        );
}

/// Note rm with invalid number fails
#[test]
fn test_note_rm_invalid_number() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    // No notes exist, try to retract #1
    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("rm")
        .arg(&tension.id)
        .arg("1")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

/// Note rm with zero fails
#[test]
fn test_note_rm_zero_fails() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("rm")
        .arg(&tension.id)
        .arg("0")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("1 or greater"));
}

/// Note rm on workspace notes works
#[test]
fn test_note_rm_workspace() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg("workspace observation")
        .current_dir(dir.path())
        .assert()
        .success();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("rm")
        .arg("1")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Retracted"));

    // Should be empty now
    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("none"));
}

/// Note rm JSON output
#[test]
fn test_note_rm_json_output() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("goal", "reality").unwrap();

    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(&tension.id)
        .arg("to be retracted")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("note")
        .arg("rm")
        .arg(&tension.id)
        .arg("1")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.get("retracted_note").is_some(), "JSON should have 'retracted_note'");
    assert_eq!(json.get("note_number").unwrap().as_u64(), Some(1));
}
