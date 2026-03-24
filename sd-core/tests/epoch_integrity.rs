//! E2E tests for epoch mechanics (V5).
//!
//! Validates:
//! - Epoch creation produces valid snapshots
//! - Epoch timestamps are monotonically increasing
//! - Children snapshot captures correct state at epoch close
//! - Multiple epochs partition correctly
//! - Last epoch timestamp query matches full query

use sd_core::{Store, TensionStatus};

#[test]
fn test_epoch_creates_valid_snapshot() {
    let store = Store::new_in_memory().unwrap();
    let parent = store.create_tension("build the thing", "nothing built yet").unwrap();
    let c1 = store.create_tension_with_parent("step one", "not started", Some(parent.id.clone())).unwrap();
    let _c2 = store.create_tension_with_parent("step two", "not started", Some(parent.id.clone())).unwrap();

    let children_json = serde_json::json!({"children": [
        {"id": c1.id, "desired": "step one", "status": "Active"}
    ]}).to_string();

    let epoch_id = store.create_epoch(
        &parent.id,
        "build the thing",
        "nothing built yet",
        Some(&children_json),
        None,
    ).unwrap();

    let epochs = store.get_epochs(&parent.id).unwrap();
    assert_eq!(epochs.len(), 1);
    assert_eq!(epochs[0].id, epoch_id);
    assert_eq!(epochs[0].desire_snapshot, "build the thing");
    assert_eq!(epochs[0].reality_snapshot, "nothing built yet");
    assert!(epochs[0].children_snapshot_json.is_some());
}

#[test]
fn test_multiple_epochs_are_chronological() {
    let store = Store::new_in_memory().unwrap();
    let t = store.create_tension("goal", "reality").unwrap();

    let e1 = store.create_epoch(&t.id, "goal v1", "reality v1", None, None).unwrap();
    // Small delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(10));
    let e2 = store.create_epoch(&t.id, "goal v2", "reality v2", None, None).unwrap();

    let epochs = store.get_epochs(&t.id).unwrap();
    assert_eq!(epochs.len(), 2);
    assert_eq!(epochs[0].id, e1);
    assert_eq!(epochs[1].id, e2);
    assert!(epochs[0].timestamp <= epochs[1].timestamp);
}

#[test]
fn test_last_epoch_timestamp_matches_full_query() {
    let store = Store::new_in_memory().unwrap();
    let t = store.create_tension("goal", "reality").unwrap();

    // No epochs yet
    let ts = store.get_last_epoch_timestamp(&t.id).unwrap();
    assert!(ts.is_none());

    // Create first epoch
    store.create_epoch(&t.id, "goal v1", "reality v1", None, None).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Create second epoch
    store.create_epoch(&t.id, "goal v2", "reality v2", None, None).unwrap();

    let last_ts = store.get_last_epoch_timestamp(&t.id).unwrap().unwrap();
    let all_epochs = store.get_epochs(&t.id).unwrap();
    let full_last_ts = all_epochs.last().unwrap().timestamp;

    assert_eq!(last_ts, full_last_ts);
}

#[test]
fn test_epoch_for_nonexistent_tension_returns_empty() {
    let store = Store::new_in_memory().unwrap();
    let epochs = store.get_epochs("nonexistent-id").unwrap();
    assert!(epochs.is_empty());

    let ts = store.get_last_epoch_timestamp("nonexistent-id").unwrap();
    assert!(ts.is_none());
}

#[test]
fn test_epoch_snapshot_after_child_resolve() {
    let store = Store::new_in_memory().unwrap();
    let parent = store.create_tension("project", "started").unwrap();
    let child = store.create_tension_with_parent("task", "todo", Some(parent.id.clone())).unwrap();

    // Resolve the child
    store.update_status(&child.id, TensionStatus::Resolved).unwrap();

    // Create epoch after resolution
    let children = store.get_children(&parent.id).unwrap();
    let children_json = serde_json::json!({"children": children.iter().map(|c| {
        serde_json::json!({"id": c.id, "desired": c.desired.clone(), "status": format!("{:?}", c.status)})
    }).collect::<Vec<_>>()}).to_string();

    store.create_epoch(&parent.id, "project", "started", Some(&children_json), None).unwrap();

    let epochs = store.get_epochs(&parent.id).unwrap();
    let snapshot = epochs[0].children_snapshot_json.as_ref().unwrap();
    assert!(snapshot.contains("Resolved"));
}
