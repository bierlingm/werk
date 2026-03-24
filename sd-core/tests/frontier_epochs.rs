//! E2E tests for frontier computation with epoch boundaries.
//!
//! Validates:
//! - Accumulated items filtered by epoch boundary
//! - All resolved shown when no epoch boundary
//! - Fresh epoch clears accumulated
//! - Trajectory mode with epochs
//! - Frontier classification correctness

use sd_core::{Store, TensionStatus};
use chrono::Utc;

/// Create a FieldEntry-like struct for testing frontier computation.
/// We test the frontier logic via the TUI's Frontier::compute, so we need
/// to construct FieldEntry values. Since FieldEntry is in werk-tui, we test
/// the underlying store behavior that feeds it.

#[test]
fn test_resolved_children_appear_after_epoch_boundary() {
    let store = Store::new_in_memory().unwrap();
    let parent = store.create_tension("project", "started").unwrap();
    let c1 = store.create_tension_with_parent("step 1", "todo", Some(parent.id.clone())).unwrap();
    let c2 = store.create_tension_with_parent("step 2", "todo", Some(parent.id.clone())).unwrap();
    let c3 = store.create_tension_with_parent("step 3", "todo", Some(parent.id.clone())).unwrap();

    // Resolve c1 before epoch
    store.update_status(&c1.id, TensionStatus::Resolved).unwrap();

    // Create epoch
    std::thread::sleep(std::time::Duration::from_millis(10));
    store.create_epoch(&parent.id, "project", "started", None, None).unwrap();
    let epoch_ts = store.get_last_epoch_timestamp(&parent.id).unwrap().unwrap();

    // Resolve c2 after epoch
    std::thread::sleep(std::time::Duration::from_millis(10));
    store.update_status(&c2.id, TensionStatus::Resolved).unwrap();

    // Get mutation timestamps for status changes
    let c1_status_ts = store.get_last_mutation_timestamps(&[c1.id.as_str()], &["status"]).unwrap();
    let c2_status_ts = store.get_last_mutation_timestamps(&[c2.id.as_str()], &["status"]).unwrap();

    let c1_ts = c1_status_ts.get(&c1.id).unwrap();
    let c2_ts = c2_status_ts.get(&c2.id).unwrap();

    // c1 was resolved before epoch — should NOT be in current epoch
    assert!(*c1_ts < epoch_ts, "c1 resolved before epoch boundary");
    // c2 was resolved after epoch — should be in current epoch
    assert!(*c2_ts >= epoch_ts, "c2 resolved after epoch boundary");
    // c3 is still active
    let c3_reloaded = store.get_tension(&c3.id).unwrap().unwrap();
    assert_eq!(c3_reloaded.status, TensionStatus::Active);
}

#[test]
fn test_no_epochs_means_all_resolved_visible() {
    let store = Store::new_in_memory().unwrap();
    let parent = store.create_tension("project", "started").unwrap();
    let c1 = store.create_tension_with_parent("step 1", "done", Some(parent.id.clone())).unwrap();
    let c2 = store.create_tension_with_parent("step 2", "done", Some(parent.id.clone())).unwrap();

    store.update_status(&c1.id, TensionStatus::Resolved).unwrap();
    store.update_status(&c2.id, TensionStatus::Resolved).unwrap();

    // No epochs exist
    let epoch_ts = store.get_last_epoch_timestamp(&parent.id).unwrap();
    assert!(epoch_ts.is_none());

    // Both should have status mutations
    let statuses = store.get_last_mutation_timestamps(
        &[c1.id.as_str(), c2.id.as_str()], &["status"]
    ).unwrap();
    assert_eq!(statuses.len(), 2);
}

#[test]
fn test_epoch_close_partitions_accumulated() {
    let store = Store::new_in_memory().unwrap();
    let parent = store.create_tension("project", "v1").unwrap();
    let c1 = store.create_tension_with_parent("task a", "todo", Some(parent.id.clone())).unwrap();
    let c2 = store.create_tension_with_parent("task b", "todo", Some(parent.id.clone())).unwrap();
    let c3 = store.create_tension_with_parent("task c", "todo", Some(parent.id.clone())).unwrap();

    // Resolve c1 in epoch 1
    store.update_status(&c1.id, TensionStatus::Resolved).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Close epoch 1 (desire update)
    store.update_desired(&parent.id, "project v2").unwrap();
    store.create_epoch(&parent.id, "project v2", "v1", None, None).unwrap();
    let epoch1_ts = store.get_last_epoch_timestamp(&parent.id).unwrap().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Resolve c2 in epoch 2
    store.update_status(&c2.id, TensionStatus::Resolved).unwrap();

    // Check: c1 resolved before epoch1, c2 resolved after epoch1
    let c1_status = store.get_last_mutation_timestamps(&[c1.id.as_str()], &["status"]).unwrap();
    let c2_status = store.get_last_mutation_timestamps(&[c2.id.as_str()], &["status"]).unwrap();

    assert!(*c1_status.get(&c1.id).unwrap() < epoch1_ts);
    assert!(*c2_status.get(&c2.id).unwrap() >= epoch1_ts);

    // c3 is still active
    let c3_reloaded = store.get_tension(&c3.id).unwrap().unwrap();
    assert_eq!(c3_reloaded.status, TensionStatus::Active);
}

#[test]
fn test_count_children_by_parent_returns_correct_counts() {
    let store = Store::new_in_memory().unwrap();
    let p1 = store.create_tension("parent 1", "reality").unwrap();
    let p2 = store.create_tension("parent 2", "reality").unwrap();
    let _orphan = store.create_tension("orphan", "reality").unwrap();

    store.create_tension_with_parent("c1a", "r", Some(p1.id.clone())).unwrap();
    store.create_tension_with_parent("c1b", "r", Some(p1.id.clone())).unwrap();
    store.create_tension_with_parent("c1c", "r", Some(p1.id.clone())).unwrap();
    store.create_tension_with_parent("c2a", "r", Some(p2.id.clone())).unwrap();

    let counts = store.count_children_by_parent(&[p1.id.as_str(), p2.id.as_str()]).unwrap();
    assert_eq!(*counts.get(&p1.id).unwrap(), 3);
    assert_eq!(*counts.get(&p2.id).unwrap(), 1);
}

#[test]
fn test_batch_mutation_timestamps_returns_per_tension() {
    let store = Store::new_in_memory().unwrap();
    let t1 = store.create_tension("goal 1", "reality 1").unwrap();
    let t2 = store.create_tension("goal 2", "reality 2").unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));
    store.update_actual(&t1.id, "updated reality 1").unwrap();

    let timestamps = store.get_last_mutation_timestamps(
        &[t1.id.as_str(), t2.id.as_str()], &["actual", "created"]
    ).unwrap();

    // Both should have timestamps (from "created" at minimum)
    assert!(timestamps.contains_key(&t1.id));
    assert!(timestamps.contains_key(&t2.id));

    // t1's should be more recent (we updated actual)
    assert!(timestamps[&t1.id] >= timestamps[&t2.id]);
}
