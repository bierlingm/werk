//! Integration tests for workspace discovery and cross-cutting scenarios.
//!
//! Tests verify:
//! - VAL-INIT-004: Store discovery walks up directory tree
//! - VAL-INIT-005: Global fallback when no local workspace
//! - VAL-CROSS-004: Workspace discovery consistency
//! - VAL-CROSS-007: Error recovery preserves state
//! - VAL-CROSS-009: ID prefix matching works across all commands
//! - VAL-CROSS-010: First-use experience is coherent

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

// =============================================================================
// VAL-INIT-004: Store discovery walks up directory tree
// =============================================================================

/// Running `werk add` from subdirectory uses parent's .werk/sd.db
#[test]
fn test_discovery_add_from_subdirectory() {
    let parent_dir = TempDir::new().unwrap();

    // Initialize workspace in parent
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(parent_dir.path())
        .assert()
        .success();

    // Create nested subdirectory
    let subdir = parent_dir.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&subdir).unwrap();

    // Add tension from subdirectory
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("subdir goal")
        .arg("subdir reality")
        .current_dir(&subdir)
        .assert()
        .success();

    // Verify tension is in parent's store
    let store = sd_core::Store::init_unlocked(parent_dir.path()).unwrap();
    let tensions = store.list_tensions().unwrap();
    assert_eq!(tensions.len(), 1);
    assert_eq!(tensions[0].desired, "subdir goal");
}

/// Running `werk show` from subdirectory finds tension created from parent
#[test]
fn test_discovery_show_from_subdirectory() {
    let parent_dir = TempDir::new().unwrap();

    // Initialize and create tension in parent
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(parent_dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(parent_dir.path()).unwrap();
    let tension = store
        .create_tension("parent goal", "parent reality")
        .unwrap();

    // Create subdirectory
    let subdir = parent_dir.path().join("child");
    std::fs::create_dir_all(&subdir).unwrap();

    // Show from subdirectory should find the tension
    let prefix = &tension.id[..6];
    cargo_bin_cmd!("werk")
        .arg("show")
        .arg(prefix)
        .current_dir(&subdir)
        .assert()
        .success()
        .stdout(predicate::str::contains("parent goal"));
}

/// Reality update from subdirectory modifies parent store
#[test]
fn test_discovery_reality_from_subdirectory() {
    let parent_dir = TempDir::new().unwrap();

    // Initialize and create tension
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(parent_dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(parent_dir.path()).unwrap();
    let tension = store
        .create_tension("update goal", "initial reality")
        .unwrap();

    // Create subdirectory and update from there
    let subdir = parent_dir.path().join("nested").join("dir");
    std::fs::create_dir_all(&subdir).unwrap();

    let prefix = &tension.id[..6];
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(prefix)
        .arg("updated reality")
        .current_dir(&subdir)
        .assert()
        .success();

    // Verify update in parent store
    let updated = sd_core::Store::init_unlocked(parent_dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();
    assert_eq!(updated.actual, "updated reality");
}

/// Desire update from subdirectory modifies parent store
#[test]
fn test_discovery_desire_from_subdirectory() {
    let parent_dir = TempDir::new().unwrap();

    // Initialize and create tension
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(parent_dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(parent_dir.path()).unwrap();
    let tension = store.create_tension("original desire", "reality").unwrap();

    // Create subdirectory and update from there
    let subdir = parent_dir.path().join("deep").join("path");
    std::fs::create_dir_all(&subdir).unwrap();

    let prefix = &tension.id[..6];
    cargo_bin_cmd!("werk")
        .arg("desire")
        .arg(prefix)
        .arg("refined desire")
        .current_dir(&subdir)
        .assert()
        .success();

    // Verify update in parent store
    let updated = sd_core::Store::init_unlocked(parent_dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();
    assert_eq!(updated.desired, "refined desire");
}

/// Resolve from subdirectory modifies parent store
#[test]
fn test_discovery_resolve_from_subdirectory() {
    let parent_dir = TempDir::new().unwrap();

    // Initialize and create tension
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(parent_dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(parent_dir.path()).unwrap();
    let tension = store
        .create_tension("resolve from subdir", "reality")
        .unwrap();

    // Create subdirectory and resolve from there
    let subdir = parent_dir.path().join("sub");
    std::fs::create_dir_all(&subdir).unwrap();

    let prefix = &tension.id[..6];
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(prefix)
        .current_dir(&subdir)
        .assert()
        .success();

    // Verify resolved in parent store
    let resolved = sd_core::Store::init_unlocked(parent_dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();
    assert_eq!(resolved.status, sd_core::TensionStatus::Resolved);
}

/// Release from subdirectory modifies parent store
#[test]
fn test_discovery_release_from_subdirectory() {
    let parent_dir = TempDir::new().unwrap();

    // Initialize and create tension
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(parent_dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(parent_dir.path()).unwrap();
    let tension = store
        .create_tension("release from subdir", "reality")
        .unwrap();

    // Create subdirectory and release from there
    let subdir = parent_dir.path().join("nested");
    std::fs::create_dir_all(&subdir).unwrap();

    let prefix = &tension.id[..6];
    cargo_bin_cmd!("werk")
        .arg("release")
        .arg(prefix)
        .arg("--reason")
        .arg("testing")
        .current_dir(&subdir)
        .assert()
        .success();

    // Verify released in parent store
    let released = sd_core::Store::init_unlocked(parent_dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();
    assert_eq!(released.status, sd_core::TensionStatus::Released);
}

/// Rm from subdirectory modifies parent store
#[test]
fn test_discovery_rm_from_subdirectory() {
    let parent_dir = TempDir::new().unwrap();

    // Initialize and create tension
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(parent_dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(parent_dir.path()).unwrap();
    let tension = store.create_tension("rm from subdir", "reality").unwrap();

    // Create subdirectory and rm from there
    let subdir = parent_dir.path().join("nested").join("deep");
    std::fs::create_dir_all(&subdir).unwrap();

    let prefix = &tension.id[..6];
    cargo_bin_cmd!("werk")
        .arg("rm")
        .arg(prefix)
        .current_dir(&subdir)
        .assert()
        .success();

    // Verify deleted in parent store
    let deleted = sd_core::Store::init_unlocked(parent_dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap();
    assert!(deleted.is_none());
}

/// Move from subdirectory modifies parent store
#[test]
fn test_discovery_move_from_subdirectory() {
    let parent_dir = TempDir::new().unwrap();

    // Initialize and create tensions
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(parent_dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(parent_dir.path()).unwrap();
    let parent_tension = store.create_tension("parent", "reality").unwrap();
    let child_tension = store.create_tension("child to move", "reality").unwrap();

    // Create subdirectory and move from there
    let subdir = parent_dir.path().join("sub");
    std::fs::create_dir_all(&subdir).unwrap();

    // Use 12-char prefix to avoid ambiguity
    let child_prefix = &child_tension.id[..12];
    let parent_prefix = &parent_tension.id[..12];
    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(child_prefix)
        .arg("--parent")
        .arg(parent_prefix)
        .current_dir(&subdir)
        .assert()
        .success();

    // Verify moved in parent store
    let moved = sd_core::Store::init_unlocked(parent_dir.path())
        .unwrap()
        .get_tension(&child_tension.id)
        .unwrap()
        .unwrap();
    assert_eq!(moved.parent_id, Some(parent_tension.id));
}

/// Note from subdirectory modifies parent store
#[test]
fn test_discovery_note_from_subdirectory() {
    let parent_dir = TempDir::new().unwrap();

    // Initialize and create tension
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(parent_dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(parent_dir.path()).unwrap();
    let tension = store.create_tension("note from subdir", "reality").unwrap();

    // Create subdirectory and add note from there
    let subdir = parent_dir.path().join("notes").join("here");
    std::fs::create_dir_all(&subdir).unwrap();

    let prefix = &tension.id[..8];
    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(prefix)
        .arg("note added from subdirectory")
        .current_dir(&subdir)
        .assert()
        .success();

    // Verify note in parent store
    let mutations = sd_core::Store::init_unlocked(parent_dir.path())
        .unwrap()
        .get_mutations(&tension.id)
        .unwrap();
    assert!(mutations.iter().any(|m| m.field() == "note"));
}

// =============================================================================
// VAL-INIT-005: Global fallback when no local workspace
// =============================================================================

/// Add uses global workspace when no local .werk/ exists
#[test]
fn test_global_fallback_add() {
    let work_dir = TempDir::new().unwrap();
    let home_dir = TempDir::new().unwrap();

    // Initialize global workspace in fake home
    let global_werk = home_dir.path().join(".werk");
    std::fs::create_dir_all(&global_werk).unwrap();
    let _store = sd_core::Store::init_unlocked(home_dir.path()).unwrap();

    // Add from work_dir (no local .werk/) should use global
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("global goal")
        .arg("global reality")
        .env("HOME", home_dir.path())
        .current_dir(work_dir.path())
        .assert()
        .success();

    // Verify in global store
    let store = sd_core::Store::init_unlocked(home_dir.path()).unwrap();
    let tensions = store.list_tensions().unwrap();
    assert_eq!(tensions.len(), 1);
    assert_eq!(tensions[0].desired, "global goal");
}

/// Show uses global workspace when no local .werk/ exists
#[test]
fn test_global_fallback_show() {
    let work_dir = TempDir::new().unwrap();
    let home_dir = TempDir::new().unwrap();

    // Initialize global workspace and create tension
    let global_werk = home_dir.path().join(".werk");
    std::fs::create_dir_all(&global_werk).unwrap();
    let store = sd_core::Store::init_unlocked(home_dir.path()).unwrap();
    let tension = store.create_tension("global show test", "reality").unwrap();

    // Show from work_dir should find it
    let prefix = &tension.id[..6];
    cargo_bin_cmd!("werk")
        .arg("show")
        .arg(prefix)
        .env("HOME", home_dir.path())
        .current_dir(work_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("global show test"));
}

/// Reality uses global workspace when no local .werk/ exists
#[test]
fn test_global_fallback_reality() {
    let work_dir = TempDir::new().unwrap();
    let home_dir = TempDir::new().unwrap();

    // Initialize global workspace and create tension
    let global_werk = home_dir.path().join(".werk");
    std::fs::create_dir_all(&global_werk).unwrap();
    let store = sd_core::Store::init_unlocked(home_dir.path()).unwrap();
    let tension = store
        .create_tension("global reality test", "initial")
        .unwrap();

    // Update reality from work_dir
    let prefix = &tension.id[..6];
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(prefix)
        .arg("updated")
        .env("HOME", home_dir.path())
        .current_dir(work_dir.path())
        .assert()
        .success();

    // Verify in global store
    let updated = sd_core::Store::init_unlocked(home_dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();
    assert_eq!(updated.actual, "updated");
}

/// Config uses global workspace when no local .werk/ exists
#[test]
fn test_global_fallback_config() {
    let work_dir = TempDir::new().unwrap();
    let home_dir = TempDir::new().unwrap();

    // Create global .werk/ directory
    let global_werk = home_dir.path().join(".werk");
    std::fs::create_dir_all(&global_werk).unwrap();

    // Set config from work_dir (should use global)
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("test.key")
        .arg("global-value")
        .env("HOME", home_dir.path())
        .current_dir(work_dir.path())
        .assert()
        .success();

    // Verify config is in global
    assert!(global_werk.join("config.toml").exists());
}

/// No workspace at all produces helpful error
#[test]
fn test_no_workspace_error() {
    let work_dir = TempDir::new().unwrap();
    let home_dir = TempDir::new().unwrap();

    // No .werk/ anywhere - add should fail gracefully
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("goal")
        .arg("reality")
        .env("HOME", home_dir.path())
        .current_dir(work_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("workspace").or(predicate::str::contains("init")));
}

// =============================================================================
// VAL-CROSS-004: Workspace discovery consistency
// =============================================================================

/// Init in project root, all commands from subdirectory use same store
#[test]
fn test_workspace_consistency_across_commands() {
    let project_root = TempDir::new().unwrap();

    // Initialize at root
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(project_root.path())
        .assert()
        .success();

    // Create deep subdirectory
    let subdir = project_root
        .path()
        .join("src")
        .join("components")
        .join("ui");
    std::fs::create_dir_all(&subdir).unwrap();

    // Add from subdirectory
    let output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("consistent goal")
        .arg("consistent reality")
        .current_dir(&subdir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let _stdout = String::from_utf8_lossy(&output);

    // Extract the tension ID from output
    let store = sd_core::Store::init_unlocked(project_root.path()).unwrap();
    let tensions = store.list_tensions().unwrap();
    let tension_id = &tensions[0].id;
    let prefix = &tension_id[..6];

    // Show from subdirectory
    cargo_bin_cmd!("werk")
        .arg("show")
        .arg(prefix)
        .current_dir(&subdir)
        .assert()
        .success()
        .stdout(predicate::str::contains("consistent goal"));

    // Reality from subdirectory
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(prefix)
        .arg("updated reality")
        .current_dir(&subdir)
        .assert()
        .success();

    // Verify from root
    cargo_bin_cmd!("werk")
        .arg("show")
        .arg(prefix)
        .current_dir(project_root.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("updated reality"));

    // Resolve from subdirectory
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(prefix)
        .current_dir(&subdir)
        .assert()
        .success();

    // Verify resolved from root
    cargo_bin_cmd!("werk")
        .arg("show")
        .arg(prefix)
        .current_dir(project_root.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Resolved"));
}

/// Local workspace takes precedence over global
#[test]
fn test_local_precedence_over_global() {
    let project_dir = TempDir::new().unwrap();
    let home_dir = TempDir::new().unwrap();

    // Initialize global workspace
    let global_werk = home_dir.path().join(".werk");
    std::fs::create_dir_all(&global_werk).unwrap();
    let global_store = sd_core::Store::init_unlocked(home_dir.path()).unwrap();
    global_store
        .create_tension("global tension", "reality")
        .unwrap();

    // Initialize local workspace
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(project_dir.path())
        .assert()
        .success();

    // Add tension from project dir
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("local tension")
        .arg("reality")
        .env("HOME", home_dir.path())
        .current_dir(project_dir.path())
        .assert()
        .success();

    // Verify local store has local tension, not global
    let local_store = sd_core::Store::init_unlocked(project_dir.path()).unwrap();
    let local_tensions = local_store.list_tensions().unwrap();
    assert_eq!(local_tensions.len(), 1);
    assert_eq!(local_tensions[0].desired, "local tension");

    // Verify global store still has only global tension
    let global_tensions = sd_core::Store::init_unlocked(home_dir.path())
        .unwrap()
        .list_tensions()
        .unwrap();
    assert_eq!(global_tensions.len(), 1);
    assert_eq!(global_tensions[0].desired, "global tension");
}

// =============================================================================
// VAL-CROSS-007: Error recovery preserves state
// =============================================================================

/// Failed resolve leaves tension unchanged
#[test]
fn test_error_recovery_resolve_twice() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store
        .create_tension("resolve twice test", "reality")
        .unwrap();
    let prefix = &tension.id[..6];

    // First resolve succeeds
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .success();

    // Get state after first resolve
    let store_after_first = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let after_first = store_after_first.get_tension(&tension.id).unwrap().unwrap();
    let mutations_first = store_after_first.get_mutations(&tension.id).unwrap().len();

    // Second resolve fails (already resolved)
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .failure();

    // State unchanged after failed resolve
    let store_after_second = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let after_second = store_after_second
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();
    let mutations_second = store_after_second.get_mutations(&tension.id).unwrap().len();
    assert_eq!(after_first.status, after_second.status);
    assert_eq!(mutations_first, mutations_second);
}

/// Failed release (without reason) leaves tension unchanged
#[test]
fn test_error_recovery_release_no_reason() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store
        .create_tension("release no reason test", "reality")
        .unwrap();
    let prefix = &tension.id[..6];

    // Get initial state
    let before = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();

    // Release without reason fails
    cargo_bin_cmd!("werk")
        .arg("release")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .failure();

    // State unchanged
    let after = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();
    assert_eq!(before.status, after.status);
}

/// Failed rm (not found) leaves other tensions unchanged
#[test]
fn test_error_recovery_rm_not_found() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store.create_tension("keep this one", "reality").unwrap();

    // Get initial state
    let before_count = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .list_tensions()
        .unwrap()
        .len();

    // Rm nonexistent fails
    cargo_bin_cmd!("werk")
        .arg("rm")
        .arg("ZZZZZZZZZZZZZZZZZZZZZZZZZZ")
        .current_dir(dir.path())
        .assert()
        .failure();

    // Tension count unchanged
    let after_count = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .list_tensions()
        .unwrap()
        .len();
    assert_eq!(before_count, after_count);

    // Original tension still exists
    let still_exists = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap();
    assert!(still_exists.is_some());
}

/// Failed move (cycle) leaves tensions unchanged
#[test]
fn test_error_recovery_move_cycle() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Create A -> B -> C hierarchy
    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let a = store.create_tension("A", "reality").unwrap();
    let b = store
        .create_tension_with_parent("B", "reality", Some(a.id.clone()))
        .unwrap();
    let c = store
        .create_tension_with_parent("C", "reality", Some(b.id.clone()))
        .unwrap();

    // Get state before
    let before_a = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&a.id)
        .unwrap()
        .unwrap();
    let before_c = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&c.id)
        .unwrap()
        .unwrap();

    // Try to move A under C (would create cycle) - should fail
    let a_prefix = &a.id[..6];
    let c_prefix = &c.id[..6];
    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(a_prefix)
        .arg("--parent")
        .arg(c_prefix)
        .current_dir(dir.path())
        .assert()
        .failure();

    // State unchanged
    let after_a = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&a.id)
        .unwrap()
        .unwrap();
    let after_c = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&c.id)
        .unwrap()
        .unwrap();

    assert_eq!(before_a.parent_id, after_a.parent_id);
    assert_eq!(before_c.parent_id, after_c.parent_id);
}

/// Failed reality on resolved tension leaves it unchanged
#[test]
fn test_error_recovery_reality_on_resolved() {
    let dir = TempDir::new().unwrap();

    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store
        .create_tension("resolved tension", "initial reality")
        .unwrap();
    let prefix = &tension.id[..6];

    // Resolve the tension
    store
        .update_status(&tension.id, sd_core::TensionStatus::Resolved)
        .unwrap();

    // Get state before
    let before = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();

    // Try to update reality - should fail
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(prefix)
        .arg("new reality")
        .current_dir(dir.path())
        .assert()
        .failure();

    // State unchanged
    let after = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();
    assert_eq!(before.actual, after.actual);
}

// =============================================================================
// VAL-CROSS-009: ID prefix matching works across all commands
// =============================================================================

/// Same prefix resolves same tension across show, reality, resolve, rm, note
#[test]
fn test_prefix_consistency_across_commands() {
    let dir = TempDir::new().unwrap();

    // Initialize and create tension
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tension = store
        .create_tension("prefix consistency test", "initial reality")
        .unwrap();

    // Use same 6-char prefix for all operations
    let prefix = &tension.id[..6];

    // Show with prefix
    let output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("prefix consistency test"));

    // Reality with prefix
    cargo_bin_cmd!("werk")
        .arg("reality")
        .arg(prefix)
        .arg("updated reality")
        .current_dir(dir.path())
        .assert()
        .success();

    // Desire with prefix
    cargo_bin_cmd!("werk")
        .arg("desire")
        .arg(prefix)
        .arg("updated desire")
        .current_dir(dir.path())
        .assert()
        .success();

    // Note with prefix
    cargo_bin_cmd!("werk")
        .arg("note")
        .arg("add")
        .arg(prefix)
        .arg("test note")
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify all changes applied to same tension
    let final_tension = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&tension.id)
        .unwrap()
        .unwrap();
    assert_eq!(final_tension.desired, "updated desire");
    assert_eq!(final_tension.actual, "updated reality");

    // Check mutations
    let mutations = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_mutations(&tension.id)
        .unwrap();
    assert!(mutations
        .iter()
        .any(|m| m.field() == "note" && m.new_value() == "test note"));
}

/// Prefix resolves across move and release commands
#[test]
fn test_prefix_move_and_release() {
    let dir = TempDir::new().unwrap();

    // Initialize
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let parent = store.create_tension("parent for move", "reality").unwrap();
    let child = store.create_tension("child to move", "reality").unwrap();

    // Use 12-char prefix to avoid ambiguity
    let child_prefix = &child.id[..12];
    let parent_prefix = &parent.id[..12];

    // Move child under parent using prefixes
    cargo_bin_cmd!("werk")
        .arg("move")
        .arg(child_prefix)
        .arg("--parent")
        .arg(parent_prefix)
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify move
    let moved = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&child.id)
        .unwrap()
        .unwrap();
    assert_eq!(moved.parent_id, Some(parent.id.clone()));

    // Release parent with prefix
    cargo_bin_cmd!("werk")
        .arg("release")
        .arg(parent_prefix)
        .arg("--reason")
        .arg("done")
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify release
    let released = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&parent.id)
        .unwrap()
        .unwrap();
    assert_eq!(released.status, sd_core::TensionStatus::Released);
}

/// Prefix resolves across resolve and rm commands
#[test]
fn test_prefix_resolve_and_rm() {
    let dir = TempDir::new().unwrap();

    // Initialize
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let t1 = store.create_tension("to resolve", "reality").unwrap();
    let t2 = store.create_tension("to delete", "reality").unwrap();

    // Use 12-char prefix to avoid ambiguity
    let t1_prefix = &t1.id[..12];
    let t2_prefix = &t2.id[..12];

    // Resolve t1 with prefix
    cargo_bin_cmd!("werk")
        .arg("resolve")
        .arg(t1_prefix)
        .current_dir(dir.path())
        .assert()
        .success();

    // Rm t2 with prefix
    cargo_bin_cmd!("werk")
        .arg("rm")
        .arg(t2_prefix)
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify resolve
    let resolved = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&t1.id)
        .unwrap()
        .unwrap();
    assert_eq!(resolved.status, sd_core::TensionStatus::Resolved);

    // Verify rm
    let deleted = sd_core::Store::init_unlocked(dir.path())
        .unwrap()
        .get_tension(&t2.id)
        .unwrap();
    assert!(deleted.is_none());
}

// =============================================================================
// VAL-CROSS-010: First-use experience is coherent
// =============================================================================

/// Fresh init -> add -> show produces coherent output
#[test]
fn test_first_use_experience() {
    let dir = TempDir::new().unwrap();

    // Step 1: Init
    let init_output = cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let init_stdout = String::from_utf8_lossy(&init_output);

    // Init should say something helpful
    assert!(
        init_stdout.contains("Workspace") || init_stdout.contains("initialized"),
        "Init should report success, got: {}",
        init_stdout
    );

    // Step 2: Add
    let add_output = cargo_bin_cmd!("werk")
        .arg("add")
        .arg("my first goal")
        .arg("current reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let add_stdout = String::from_utf8_lossy(&add_output);

    // Add should confirm creation and show ID
    assert!(
        add_stdout.contains("Created")
            || add_stdout.contains("Tension")
            || add_stdout.contains("my first goal"),
        "Add should confirm creation, got: {}",
        add_stdout
    );

    // Step 3: Show
    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();
    let tensions = store.list_tensions().unwrap();
    let tension_id = &tensions[0].id;
    let prefix = &tension_id[..6];

    let show_output = cargo_bin_cmd!("werk")
        .arg("show")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let show_stdout = String::from_utf8_lossy(&show_output);

    // Show should display all key fields
    assert!(
        show_stdout.contains("my first goal"),
        "Show should display desired, got: {}",
        show_stdout
    );
    assert!(
        show_stdout.contains("current reality"),
        "Show should display actual, got: {}",
        show_stdout
    );
    assert!(
        show_stdout.contains("Active"),
        "Show should display status, got: {}",
        show_stdout
    );
}

/// First use with --json produces valid JSON throughout
#[test]
fn test_first_use_json_experience() {
    use serde_json::Value;

    let dir = TempDir::new().unwrap();

    // Init with --json
    let init_output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let init_stdout = String::from_utf8_lossy(&init_output);
    let init_json: Value =
        serde_json::from_str(&init_stdout).expect("Init should output valid JSON");
    assert!(init_json.get("path").is_some());

    // Add with --json
    let add_output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("add")
        .arg("json goal")
        .arg("json reality")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let add_stdout = String::from_utf8_lossy(&add_output);
    let add_json: Value = serde_json::from_str(&add_stdout).expect("Add should output valid JSON");
    assert!(add_json.get("id").is_some());
    assert_eq!(
        add_json.get("desired").unwrap().as_str().unwrap(),
        "json goal"
    );

    // Show with --json
    let id = add_json.get("id").unwrap().as_str().unwrap();
    let prefix = &id[..6];

    let show_output = cargo_bin_cmd!("werk")
        .arg("--json")
        .arg("show")
        .arg(prefix)
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let show_stdout = String::from_utf8_lossy(&show_output);
    let show_json: Value =
        serde_json::from_str(&show_stdout).expect("Show should output valid JSON");
    assert_eq!(show_json.get("id").unwrap().as_str().unwrap(), id);
    assert!(show_json.get("mutations").is_some());
}

/// Config flow works as part of first-use
#[test]
fn test_first_use_config() {
    let dir = TempDir::new().unwrap();

    // Init
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Config set
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("set")
        .arg("agent.command")
        .arg("echo hello")
        .current_dir(dir.path())
        .assert()
        .success();

    // Config get
    cargo_bin_cmd!("werk")
        .arg("config")
        .arg("get")
        .arg("agent.command")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("echo hello"));
}

// =============================================================================
// Additional cross-cutting tests
// =============================================================================

/// Deep nesting (10+ levels) still finds workspace
#[test]
fn test_deep_nesting_discovery() {
    let root = TempDir::new().unwrap();

    // Initialize at root
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(root.path())
        .assert()
        .success();

    // Create 12 levels of nesting
    let mut deep_path = root.path().to_path_buf();
    for i in 0..12 {
        deep_path = deep_path.join(format!("level{}", i));
    }
    std::fs::create_dir_all(&deep_path).unwrap();

    // Add from deepest level
    cargo_bin_cmd!("werk")
        .arg("add")
        .arg("deep goal")
        .arg("deep reality")
        .current_dir(&deep_path)
        .assert()
        .success();

    // Verify in root store
    let store = sd_core::Store::init_unlocked(root.path()).unwrap();
    let tensions = store.list_tensions().unwrap();
    assert_eq!(tensions.len(), 1);
    assert_eq!(tensions[0].desired, "deep goal");
}

/// Multiple tensions with similar prefixes work correctly
#[test]
fn test_multiple_tensions_prefix_handling() {
    let dir = TempDir::new().unwrap();

    // Initialize
    cargo_bin_cmd!("werk")
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let store = sd_core::Store::init_unlocked(dir.path()).unwrap();

    // Create multiple tensions rapidly (similar timestamps = similar prefixes)
    let _t1 = store.create_tension("first", "reality").unwrap();
    let _t2 = store.create_tension("second", "reality").unwrap();
    let _t3 = store.create_tension("third", "reality").unwrap();

    // Find unique prefixes
    // ULIDs are time-sorted, so tensions created close together share prefixes
    let tensions = store.list_tensions().unwrap();
    assert_eq!(tensions.len(), 3);

    // Use 12-char prefixes which should be unique
    for tension in &tensions {
        let prefix = &tension.id[..12];
        cargo_bin_cmd!("werk")
            .arg("show")
            .arg(prefix)
            .current_dir(dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains(&tension.desired));
    }
}
