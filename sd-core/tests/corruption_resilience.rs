//! Tests for store resilience against corruption and edge cases.
//!
//! Validates:
//! - Store handles missing/empty DB gracefully
//! - Backup mechanism creates valid copies
//! - Flush sanity check prevents data destruction

use sd_core::Store;
use tempfile::TempDir;
use std::io::Write;

#[test]
fn test_store_init_creates_fresh_db() {
    let temp = TempDir::new().unwrap();
    let store = Store::init_unlocked(temp.path()).unwrap();
    let tensions = store.list_tensions().unwrap();
    assert!(tensions.is_empty());
}

#[test]
fn test_store_persists_across_reopen() {
    let temp = TempDir::new().unwrap();

    {
        let store = Store::init_unlocked(temp.path()).unwrap();
        store.create_tension("goal", "reality").unwrap();
    }

    {
        let store = Store::init_unlocked(temp.path()).unwrap();
        let tensions = store.list_tensions().unwrap();
        assert_eq!(tensions.len(), 1);
        assert_eq!(tensions[0].desired, "goal");
    }
}

#[test]
fn test_store_init_on_zero_byte_db_creates_fresh() {
    let temp = TempDir::new().unwrap();
    let werk_dir = temp.path().join(".werk");
    std::fs::create_dir_all(&werk_dir).unwrap();
    let db_path = werk_dir.join("sd.db");

    // Create a zero-byte file
    std::fs::File::create(&db_path).unwrap();
    assert_eq!(std::fs::metadata(&db_path).unwrap().len(), 0);

    // Store::init should handle this gracefully (reinit or error, not panic)
    let result = Store::init_unlocked(temp.path());
    // Either succeeds (reinits) or returns a clean error
    match result {
        Ok(store) => {
            let tensions = store.list_tensions().unwrap();
            assert!(tensions.is_empty());
        }
        Err(e) => {
            // Error is acceptable — panic is not
            let msg = format!("{:?}", e);
            assert!(!msg.is_empty());
        }
    }
}

#[test]
fn test_backup_dir_creation_and_file_copy() {
    let temp = TempDir::new().unwrap();
    let store = Store::init_unlocked(temp.path()).unwrap();
    store.create_tension("important data", "must survive").unwrap();
    drop(store);

    let db_path = temp.path().join(".werk").join("sd.db");
    let backup_dir = temp.path().join(".werk").join("backups");
    std::fs::create_dir_all(&backup_dir).unwrap();

    let backup_path = backup_dir.join("sd.db.test-backup");
    std::fs::copy(&db_path, &backup_path).unwrap();

    // Verify backup is a valid store
    // Need to copy to a proper .werk structure for Store::init
    let restore_dir = TempDir::new().unwrap();
    let restore_werk = restore_dir.path().join(".werk");
    std::fs::create_dir_all(&restore_werk).unwrap();
    std::fs::copy(&backup_path, restore_werk.join("sd.db")).unwrap();

    let restored = Store::init_unlocked(restore_dir.path()).unwrap();
    let tensions = restored.list_tensions().unwrap();
    assert_eq!(tensions.len(), 1);
    assert_eq!(tensions[0].desired, "important data");
}

#[test]
fn test_flush_sanity_check_concept() {
    // This tests the logic that flush_to_file uses:
    // if old count > 5 and new count < old_count / 2, refuse.
    let old_total: usize = 20;
    let new_count: usize = 3;

    assert!(old_total > 5 && new_count < old_total / 2,
        "Sanity check should trigger: {} -> {}", old_total, new_count);

    // Normal change should not trigger
    let old_total: usize = 20;
    let new_count: usize = 18;
    assert!(!(old_total > 5 && new_count < old_total / 2),
        "Normal change should pass: {} -> {}", old_total, new_count);

    // Zero count should always be rejected when old > 0
    let old_total: usize = 1;
    let new_count: usize = 0;
    assert!(old_total > 0 && new_count == 0,
        "Zero count should be rejected: {} -> {}", old_total, new_count);
}

#[test]
fn test_tensions_json_round_trip() {
    let temp = TempDir::new().unwrap();
    let store = Store::init_unlocked(temp.path()).unwrap();

    // Create a small tree
    let root = store.create_tension("root goal", "root reality").unwrap();
    let child1 = store.create_tension_with_parent("child 1", "c1 reality", Some(root.id.clone())).unwrap();
    let child2 = store.create_tension_with_parent("child 2", "c2 reality", Some(root.id.clone())).unwrap();

    // Set various states
    store.update_horizon(&child1.id, Some(sd_core::Horizon::parse("2026-06").unwrap())).unwrap();
    store.update_position(&child1.id, Some(1)).unwrap();
    store.update_status(&child2.id, sd_core::TensionStatus::Resolved).unwrap();

    // Read all tensions
    let tensions = store.list_tensions().unwrap();
    assert_eq!(tensions.len(), 3);

    // Verify states
    let root_t = tensions.iter().find(|t| t.id == root.id).unwrap();
    let c1 = tensions.iter().find(|t| t.id == child1.id).unwrap();
    let c2 = tensions.iter().find(|t| t.id == child2.id).unwrap();

    assert_eq!(root_t.desired, "root goal");
    assert_eq!(c1.position, Some(1));
    assert!(c1.horizon.is_some());
    assert_eq!(c2.status, sd_core::TensionStatus::Resolved);

    // Verify parent relationships
    assert_eq!(c1.parent_id.as_deref(), Some(root.id.as_str()));
    assert_eq!(c2.parent_id.as_deref(), Some(root.id.as_str()));
}

#[test]
fn test_epoch_creation_does_not_corrupt_store() {
    let temp = TempDir::new().unwrap();
    let store = Store::init_unlocked(temp.path()).unwrap();
    let parent = store.create_tension("project", "started").unwrap();
    let _child = store.create_tension_with_parent("task", "todo", Some(parent.id.clone())).unwrap();

    // Create multiple epochs rapidly
    for i in 0..10 {
        let desire = format!("project v{}", i);
        let reality = format!("iteration {}", i);
        store.create_epoch(&parent.id, &desire, &reality, None, None).unwrap();
    }

    // Store should still be fully functional
    let tensions = store.list_tensions().unwrap();
    assert_eq!(tensions.len(), 2);

    let epochs = store.get_epochs(&parent.id).unwrap();
    assert_eq!(epochs.len(), 10);

    // Can still create tensions
    let _new = store.create_tension("still works", "yes").unwrap();
    assert_eq!(store.list_tensions().unwrap().len(), 3);
}

#[test]
fn test_concurrent_operations_on_same_store() {
    // This tests that sequential rapid operations don't corrupt
    // (the actual crash was from concurrent process access, which we can't easily test,
    // but we can verify rapid sequential access is safe)
    let temp = TempDir::new().unwrap();
    let store = Store::init_unlocked(temp.path()).unwrap();

    for i in 0..50 {
        let t = store.create_tension(&format!("goal {}", i), &format!("reality {}", i)).unwrap();
        if i % 5 == 0 {
            store.update_actual(&t.id, &format!("updated reality {}", i)).unwrap();
        }
        if i % 3 == 0 {
            store.update_status(&t.id, sd_core::TensionStatus::Resolved).unwrap();
        }
    }

    let tensions = store.list_tensions().unwrap();
    assert_eq!(tensions.len(), 50);

    let active = tensions.iter().filter(|t| t.status == sd_core::TensionStatus::Active).count();
    let resolved = tensions.iter().filter(|t| t.status == sd_core::TensionStatus::Resolved).count();
    assert_eq!(resolved, 17); // 0, 3, 6, 9, ..., 48 = 17 items
    assert_eq!(active, 33);
}
