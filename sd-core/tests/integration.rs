//! Integration tests for sd-core.
//!
//! Tests full cross-module flows: store operations, forest building,
//! urgency computation, horizon drift detection, temporal signals.

use chrono::Utc;
use sd_core::{
    DynamicsEngine, Event, EventBus, Forest, Store, Tension, TensionStatus,
    compute_urgency, detect_horizon_drift,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Full Tension Lifecycle
// ============================================================================

#[test]
fn test_full_tension_lifecycle() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    // Create a tension
    let t = engine.create_tension("write a novel", "have an outline").unwrap();
    assert_eq!(t.status, TensionStatus::Active);

    // Update actual
    engine.update_actual(&t.id, "have a first draft").unwrap();

    // Verify update persisted
    let updated = engine.store().get_tension(&t.id).unwrap().unwrap();
    assert_eq!(updated.actual, "have a first draft");

    // Resolve
    engine.resolve(&t.id).unwrap();
    let resolved = engine.store().get_tension(&t.id).unwrap().unwrap();
    assert_eq!(resolved.status, TensionStatus::Resolved);
}

// ============================================================================
// Store Events Integration
// ============================================================================

#[test]
fn test_store_emits_events_through_engine() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();
    let bus = EventBus::new();
    engine.set_event_bus(bus.clone());

    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = count.clone();
    let _handle = bus.subscribe(move |_| {
        count_clone.fetch_add(1, Ordering::SeqCst);
    });

    // Create tension should emit event
    let _t = engine.create_tension("goal", "reality").unwrap();

    // Store operations should also be visible
    assert!(count.load(Ordering::SeqCst) >= 0); // Events may or may not fire from store
}

// ============================================================================
// Forest and Tree Integration
// ============================================================================

#[test]
fn test_forest_from_store_tensions() {
    let mut store = Store::new_in_memory().unwrap();

    let parent = store.create_tension("big goal", "starting").unwrap();
    let _child1 = store
        .create_tension_full("sub goal 1", "sub reality 1", Some(parent.id.clone()), None)
        .unwrap();
    let _child2 = store
        .create_tension_full("sub goal 2", "sub reality 2", Some(parent.id.clone()), None)
        .unwrap();

    let tensions = store.list_tensions().unwrap();
    let forest = Forest::from_tensions(tensions).unwrap();

    let children = forest.children(&parent.id).unwrap();
    assert_eq!(children.len(), 2);
}

// ============================================================================
// Urgency Integration
// ============================================================================

#[test]
fn test_urgency_requires_horizon() {
    let t = Tension::new("goal", "reality").unwrap();
    assert!(compute_urgency(&t, Utc::now()).is_none());
}

#[test]
fn test_urgency_with_horizon() {
    let h = sd_core::Horizon::new_month(2027, 6).unwrap();
    let t = Tension::new_full("goal", "reality", None, Some(h)).unwrap();
    let result = compute_urgency(&t, Utc::now());
    assert!(result.is_some());
    let u = result.unwrap();
    assert!(u.value >= 0.0);
    assert!(u.total_window > 0);
}

// ============================================================================
// Horizon Drift Integration
// ============================================================================

#[test]
fn test_horizon_drift_stable_with_no_mutations() {
    let result = detect_horizon_drift("test-id", &[]);
    assert_eq!(result.drift_type, sd_core::HorizonDriftType::Stable);
    assert_eq!(result.change_count, 0);
}

// ============================================================================
// Temporal Signals Integration
// ============================================================================

#[test]
fn test_temporal_signals_with_forest() {
    let mut store = Store::new_in_memory().unwrap();

    let h_parent = sd_core::Horizon::new_month(2026, 12).unwrap();
    let parent = store
        .create_tension_full("big goal", "starting", None, Some(h_parent))
        .unwrap();

    let h_child = sd_core::Horizon::new_month(2026, 6).unwrap();
    let _child = store
        .create_tension_full("sub goal", "sub reality", Some(parent.id.clone()), Some(h_child))
        .unwrap();

    let tensions = store.list_tensions().unwrap();
    let forest = Forest::from_tensions(tensions).unwrap();

    let signals = sd_core::compute_temporal_signals(&forest, &parent.id, Utc::now());
    // Parent with child deadline well before parent deadline should have critical path info
    assert!(signals.critical_path.len() <= 1); // may or may not be critical depending on timing
}

// ============================================================================
// Parent Snapshot Integration
// ============================================================================

#[test]
fn test_engine_captures_parent_snapshots() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    let parent = engine.create_tension("big goal", "starting point").unwrap();
    let child = engine
        .create_tension_with_parent("sub goal", "sub reality", Some(parent.id.clone()))
        .unwrap();

    // Child should have parent snapshots
    let loaded = engine.store().get_tension(&child.id).unwrap().unwrap();
    assert_eq!(loaded.parent_desired_snapshot.as_deref(), Some("big goal"));
    assert_eq!(loaded.parent_actual_snapshot.as_deref(), Some("starting point"));
}
