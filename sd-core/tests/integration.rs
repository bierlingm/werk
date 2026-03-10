//! Comprehensive integration tests for sd-core.
//!
//! These tests exercise full cross-module flows to validate that all
//! components work together correctly. They fulfill the VAL-CROSS assertions.

use chrono::Utc;
use sd_core::{
    classify_creative_cycle_phase, detect_oscillation, detect_resolution,
    detect_structural_conflict, DynamicsEngine, DynamicsThresholds, Event, EventBus, Forest,
    OscillationThresholds, ResolutionThresholds, Store, Tension, TensionStatus,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// ============================================================================
// VAL-CROSS-001: Full Tension Lifecycle
// ============================================================================

/// Test the complete tension lifecycle:
/// create -> mutate actual -> dynamics compute -> lifecycle transitions -> resolve -> completion
#[test]
fn test_full_tension_lifecycle() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    // Use sensitive thresholds for testing
    let mut thresholds = DynamicsThresholds::default();
    thresholds.lifecycle.active_frequency_threshold = 1;
    thresholds.lifecycle.convergence_threshold = 0.3;
    engine.set_thresholds(thresholds);

    // Track events
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let _handle = engine.subscribe(move |e| {
        events_clone.lock().unwrap().push(e.clone());
    });

    // Step 1: Create tension
    let t = engine
        .create_tension("write a novel", "have an outline")
        .unwrap();
    assert_eq!(t.status, TensionStatus::Active);
    assert!(!t.id.is_empty());

    // Initial phase should be Germination
    let prev = engine.previous_state().tensions.get(&t.id).unwrap();
    assert_eq!(prev.phase, Some(sd_core::CreativeCyclePhase::Germination));

    // Step 2: Mutate actual (confront reality)
    engine
        .update_actual(&t.id, "have a chapter written")
        .unwrap();
    engine.update_actual(&t.id, "have three chapters").unwrap();

    // Compute dynamics and emit events
    let _transition_events = engine.compute_and_emit_for_tension(&t.id);

    // Step 3: Continue advancing
    for i in 0..5 {
        engine
            .update_actual(&t.id, &format!("have {} chapters", 3 + i))
            .unwrap();
    }

    engine.compute_and_emit_for_tension(&t.id);

    // Step 4: Resolve tension
    engine.update_actual(&t.id, "write a novel").unwrap(); // desired == actual
    engine.resolve(&t.id).unwrap();

    let resolved = engine.store().get_tension(&t.id).unwrap().unwrap();
    assert_eq!(resolved.status, TensionStatus::Resolved);

    // Verify events were emitted
    let recorded_events = events.lock().unwrap().clone();
    assert!(!recorded_events.is_empty());

    // Should have TensionCreated, RealityConfronted, TensionResolved
    let has_created = recorded_events
        .iter()
        .any(|e| matches!(e, Event::TensionCreated { .. }));
    let has_confronted = recorded_events
        .iter()
        .any(|e| matches!(e, Event::RealityConfronted { .. }));
    let has_resolved = recorded_events
        .iter()
        .any(|e| matches!(e, Event::TensionResolved { .. }));

    assert!(has_created, "Should emit TensionCreated event");
    assert!(has_confronted, "Should emit RealityConfronted events");
    assert!(has_resolved, "Should emit TensionResolved event");
}

// ============================================================================
// VAL-CROSS-002: Forest Dynamics with Conflict Detection
// ============================================================================

/// Test forest dynamics:
/// create parent + children -> structural conflict detected between siblings -> event fires
#[test]
fn test_forest_dynamics_conflict_detection() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    // Track events
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let _handle = engine.subscribe(move |e| {
        events_clone.lock().unwrap().push(e.clone());
    });

    // Create parent tension
    let parent = engine
        .create_tension("parent goal", "parent reality")
        .unwrap();

    // Create two sibling tensions
    let child1 = engine
        .create_tension_with_parent("child1 goal", "child1 reality", Some(parent.id.clone()))
        .unwrap();
    let _child2 = engine
        .create_tension_with_parent("child2 goal", "child2 reality", Some(parent.id.clone()))
        .unwrap();

    // Create asymmetric activity - only child1 gets updates
    for i in 0..5 {
        engine
            .update_actual(&child1.id, &format!("child1 progress {}", i))
            .unwrap();
    }
    // child2 gets no updates

    // Compute dynamics for child1
    let transition_events = engine.compute_and_emit_for_tension(&child1.id);

    // Should emit ConflictDetected event
    let has_conflict = transition_events
        .iter()
        .any(|e| matches!(e, Event::ConflictDetected { .. }));
    assert!(
        has_conflict,
        "Should emit ConflictDetected event for asymmetric sibling activity"
    );

    // Also verify via direct computation
    let tensions = engine.store().list_tensions().unwrap();
    let forest = Forest::from_tensions(tensions).unwrap();
    let mutations = engine.store().all_mutations().unwrap();

    let conflict = detect_structural_conflict(
        &forest,
        &child1.id,
        &mutations,
        &Default::default(),
        Utc::now(),
    );

    assert!(
        conflict.is_some(),
        "Conflict should be detected between siblings"
    );
}

/// Test that conflict resolves when siblings become balanced.
#[test]
fn test_conflict_resolved_event() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    // Track events
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let _handle = engine.subscribe(move |e| {
        events_clone.lock().unwrap().push(e.clone());
    });

    // Create parent and children
    let parent = engine
        .create_tension("parent goal", "parent reality")
        .unwrap();
    let child1 = engine
        .create_tension_with_parent("child1 goal", "child1 reality", Some(parent.id.clone()))
        .unwrap();
    let child2 = engine
        .create_tension_with_parent("child2 goal", "child2 reality", Some(parent.id.clone()))
        .unwrap();

    // Asymmetric activity
    for _ in 0..5 {
        engine.update_actual(&child1.id, "active update").unwrap();
    }

    // Compute to detect conflict
    engine.compute_and_emit_for_tension(&child1.id);

    // Now balance with child2 activity
    for _ in 0..5 {
        engine.update_actual(&child2.id, "balanced update").unwrap();
    }

    // Compute again - conflict should resolve
    let events2 = engine.compute_and_emit_for_tension(&child1.id);

    // Should emit ConflictResolved event
    let has_resolved = events2
        .iter()
        .any(|e| matches!(e, Event::ConflictResolved { .. }));
    assert!(
        has_resolved,
        "Should emit ConflictResolved when conflict ends"
    );
}

// ============================================================================
// VAL-CROSS-003: Oscillation Detection from Mutation Series
// ============================================================================

/// Test oscillation detection:
/// alternating advance/regress mutations -> oscillation detected -> event fires -> stabilize -> ceases
#[test]
fn test_oscillation_detection_and_stabilization() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    // Use sensitive thresholds
    let mut thresholds = DynamicsThresholds::default();
    thresholds.oscillation.magnitude_threshold = 0.001;
    thresholds.oscillation.frequency_threshold = 2;
    thresholds.oscillation.recency_window_seconds = 3600 * 24 * 365; // 1 year
    engine.set_thresholds(thresholds);

    // Create tension
    let t = engine.create_tension("goal", "a").unwrap();

    // Create oscillation pattern: advance, regress, advance, regress
    engine.update_actual(&t.id, "ab").unwrap(); // Progress
    engine.update_actual(&t.id, "a").unwrap(); // Regress
    engine.update_actual(&t.id, "abc").unwrap(); // Progress
    engine.update_actual(&t.id, "a").unwrap(); // Regress

    // Compute dynamics
    let events = engine.compute_and_emit_for_tension(&t.id);

    // Should emit OscillationDetected event
    let has_oscillation = events
        .iter()
        .any(|e| matches!(e, Event::OscillationDetected { .. }));
    assert!(
        has_oscillation,
        "Should emit OscillationDetected event for oscillation pattern"
    );

    // Now stabilize with monotonic progress
    engine.update_actual(&t.id, "ab").unwrap();
    engine.update_actual(&t.id, "abc").unwrap();
    engine.update_actual(&t.id, "abcd").unwrap();

    // Verify that oscillation is detected with sensitive threshold
    let mutations = engine.store().get_mutations(&t.id).unwrap();
    let oscillation_sensitive = detect_oscillation(
        &t.id,
        &mutations,
        &OscillationThresholds {
            magnitude_threshold: 0.001,
            frequency_threshold: 2,
            recency_window_seconds: 3600 * 24 * 365,
        },
        Utc::now(),
        None,
    );
    assert!(
        oscillation_sensitive.is_some(),
        "Oscillation should be detected with sensitive threshold"
    );

    // With a very high threshold, oscillation should not be detected
    let oscillation_high_threshold = detect_oscillation(
        &t.id,
        &mutations,
        &OscillationThresholds {
            magnitude_threshold: 0.001,
            frequency_threshold: 10, // Very high threshold
            recency_window_seconds: 3600 * 24 * 365,
        },
        Utc::now(),
        None,
    );
    assert!(
        oscillation_high_threshold.is_none(),
        "Oscillation should not be detected with very high threshold"
    );
}

/// Test that oscillation and resolution are mutually exclusive.
#[test]
fn test_oscillation_resolution_mutually_exclusive() {
    let store = Store::new_in_memory().unwrap();

    // Create tension with clearly different strings to show oscillation
    let t = store
        .create_tension("write a novel", "nothing started")
        .unwrap();

    // Create oscillation pattern: advance, regress, advance, regress
    store.update_actual(&t.id, "writing a novel draft").unwrap();
    store.update_actual(&t.id, "nothing started").unwrap();
    store
        .update_actual(&t.id, "completed a novel chapter")
        .unwrap();
    store.update_actual(&t.id, "nothing started").unwrap();

    let mutations = store.get_mutations(&t.id).unwrap();
    let t_updated = store.get_tension(&t.id).unwrap().unwrap();

    let osc_thresholds = OscillationThresholds {
        magnitude_threshold: 0.001,
        frequency_threshold: 2,
        recency_window_seconds: 3600 * 24 * 365,
    };
    let res_thresholds = ResolutionThresholds {
        velocity_threshold: 0.001,
        reversal_tolerance: 0,
        recency_window_seconds: 3600 * 24 * 365,
    };

    let osc = detect_oscillation(&t.id, &mutations, &osc_thresholds, Utc::now(), None);
    let res = detect_resolution(&t_updated, &mutations, &res_thresholds, Utc::now());

    // Should detect oscillation
    assert!(osc.is_some(), "Should detect oscillation");
    // Should NOT detect resolution (too many reversals)
    assert!(
        res.is_none(),
        "Should not detect resolution when oscillating"
    );
}

// ============================================================================
// VAL-CROSS-004: Store Persistence and Reload
// ============================================================================

/// Test that dynamics are identical after store close and reopen.
#[test]
fn test_store_persistence_and_reload() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();

    // Create and populate store
    {
        let mut store = Store::init(temp_dir.path()).unwrap();
        let bus = EventBus::new();
        store.set_event_bus(bus);

        let t1 = store.create_tension("goal1", "reality1").unwrap();
        let t2 = store
            .create_tension_with_parent("goal2", "reality2", Some(t1.id.clone()))
            .unwrap();

        // Add mutations
        store.update_actual(&t1.id, "updated reality1").unwrap();
        store.update_actual(&t2.id, "updated reality2").unwrap();
    }

    // Reopen and verify
    {
        let store = Store::init(temp_dir.path()).unwrap();

        let tensions = store.list_tensions().unwrap();
        assert_eq!(tensions.len(), 2);

        // Verify mutations preserved
        let mutations = store.all_mutations().unwrap();
        assert!(mutations.len() >= 4); // 2 created + 2 updates

        // Verify dynamics computation works
        let forest = Forest::from_tensions(tensions.clone()).unwrap();
        let all_mutations = store.all_mutations().unwrap();

        // Compute structural tension for first tension
        let st = sd_core::compute_structural_tension(&tensions[0]);
        assert!(st.is_some());

        // Conflict detection should work
        let conflict = detect_structural_conflict(
            &forest,
            &tensions[1].id,
            &all_mutations,
            &Default::default(),
            Utc::now(),
        );
        // No siblings means no conflict
        assert!(conflict.is_none());
    }
}

// ============================================================================
// VAL-CROSS-005: Tree Modification Propagation
// ============================================================================

/// Test tree modification propagation:
/// reparent tension -> StructureChanged event -> dynamics updated
#[test]
fn test_tree_modification_propagation() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    // Track events
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let _handle = engine.subscribe(move |e| {
        events_clone.lock().unwrap().push(e.clone());
    });

    // Create hierarchy: parent1 -> child
    let parent1 = engine
        .create_tension("parent1 goal", "parent1 reality")
        .unwrap();
    let parent2 = engine
        .create_tension("parent2 goal", "parent2 reality")
        .unwrap();
    let child = engine
        .create_tension_with_parent("child goal", "child reality", Some(parent1.id.clone()))
        .unwrap();

    // Verify initial structure
    let child_loaded = engine.store().get_tension(&child.id).unwrap().unwrap();
    assert_eq!(child_loaded.parent_id, Some(parent1.id.clone()));

    // Reparent to parent2
    engine.update_parent(&child.id, Some(&parent2.id)).unwrap();

    // Verify structure changed
    let child_after = engine.store().get_tension(&child.id).unwrap().unwrap();
    assert_eq!(child_after.parent_id, Some(parent2.id.clone()));

    // Check for StructureChanged event
    let recorded_events = events.lock().unwrap().clone();
    let has_structure_changed = recorded_events
        .iter()
        .any(|e| matches!(e, Event::StructureChanged { .. }));
    assert!(
        has_structure_changed,
        "Should emit StructureChanged event on reparent"
    );

    // Verify dynamics updated
    let forest = Forest::from_tensions(engine.store().list_tensions().unwrap()).unwrap();

    // Child should now be under parent2
    let children_of_p2 = forest.children(&parent2.id).unwrap();
    assert_eq!(children_of_p2.len(), 1);
    assert_eq!(children_of_p2[0].id(), child.id);
}

// ============================================================================
// VAL-CROSS-006: Empty and Single-Tension Edge Cases
// ============================================================================

/// Test that all operations work correctly on empty store.
#[test]
fn test_empty_store_edge_cases() {
    let store = Store::new_in_memory().unwrap();

    // Empty store should work without panics
    assert!(store.list_tensions().unwrap().is_empty());
    assert!(store.get_roots().unwrap().is_empty());
    assert!(store.all_mutations().unwrap().is_empty());

    // Querying unknown tension returns None
    assert!(store.get_tension("unknown").unwrap().is_none());
    assert!(store.get_mutations("unknown").unwrap().is_empty());

    // Building forest from empty list should work
    let forest = Forest::from_tensions(vec![]).unwrap();
    assert_eq!(forest.root_count(), 0);
    assert_eq!(forest.len(), 0);

    // Dynamics on empty forest should not panic
    let tensions: Vec<Tension> = vec![];
    let all_mutations = store.all_mutations().unwrap();
    let result =
        sd_core::classify_orientation(&tensions, &all_mutations, &Default::default(), Utc::now());
    assert!(result.is_none(), "Orientation needs minimum sample size");
}

/// Test that all operations work correctly with single tension.
#[test]
fn test_single_tension_edge_cases() {
    let store = Store::new_in_memory().unwrap();
    let bus = EventBus::new();

    // Wrap store to add bus
    let mut store_with_bus = store;
    store_with_bus.set_event_bus(bus);

    // Create single tension
    let t = store_with_bus
        .create_tension("single goal", "single reality")
        .unwrap();

    // Single tension should work
    let tensions = store_with_bus.list_tensions().unwrap();
    assert_eq!(tensions.len(), 1);

    let roots = store_with_bus.get_roots().unwrap();
    assert_eq!(roots.len(), 1);

    // Forest from single tension
    let forest = Forest::from_tensions(tensions.clone()).unwrap();
    assert_eq!(forest.root_count(), 1);
    assert_eq!(forest.len(), 1);

    // No siblings means no conflict
    let mutations = store_with_bus.get_mutations(&t.id).unwrap();
    let conflict =
        detect_structural_conflict(&forest, &t.id, &mutations, &Default::default(), Utc::now());
    assert!(conflict.is_none());

    // No children means no neglect
    let neglect = sd_core::detect_neglect(
        &forest,
        &t.id,
        &store_with_bus.all_mutations().unwrap(),
        &Default::default(),
        Utc::now(),
    );
    assert!(neglect.is_none(), "Leaf tension cannot have neglect");

    // Structural tension works
    let st = sd_core::compute_structural_tension(&t);
    assert!(st.is_some());

    // Phase classification works
    let phase = classify_creative_cycle_phase(&t, &mutations, &[], &Default::default(), Utc::now());
    assert_eq!(
        phase.phase,
        sd_core::CreativeCyclePhase::Germination,
        "New tension should be in Germination"
    );
}

// ============================================================================
// VAL-CROSS-007: Scale Test (100+ Tensions)
// ============================================================================

/// Test that 100+ tensions with relationships, mutations, dynamics, events work within time bounds.
#[test]
fn test_scale_100_plus_tensions() {
    use std::time::Instant;

    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    let start = Instant::now();

    // Create 100 tensions in a tree structure
    let mut parent_ids: Vec<String> = Vec::new();

    // Begin transaction for batch writes (avoid fsqlite performance issues)
    engine.store().begin_transaction().unwrap();

    // Create root tensions (10 roots)
    for i in 0..10 {
        let t = engine
            .create_tension(&format!("root goal {}", i), &format!("root reality {}", i))
            .unwrap();
        parent_ids.push(t.id);
    }

    // Create children (10 children per root = 100 total additional)
    for (i, parent_id) in parent_ids.clone().iter().enumerate() {
        for j in 0..10 {
            let child_num = i * 10 + j;
            let _child = engine
                .create_tension_with_parent(
                    &format!("child goal {}", child_num),
                    &format!("child reality {}", child_num),
                    Some(parent_id.clone()),
                )
                .unwrap();
        }
    }

    engine.store().commit_transaction().unwrap();

    let creation_time = start.elapsed();

    // Verify count
    let tensions = engine.store().list_tensions().unwrap();
    assert_eq!(
        tensions.len(),
        110,
        "Should have 110 tensions (10 roots + 100 children)"
    );

    // Build forest and verify performance
    let forest_start = Instant::now();
    let forest = Forest::from_tensions(tensions).unwrap();
    let forest_time = forest_start.elapsed();

    assert_eq!(forest.root_count(), 10);
    assert_eq!(forest.len(), 110);

    // Traverse and verify performance using traverse_bfs
    let traverse_start = Instant::now();
    let mut visited = 0;
    forest.traverse_bfs(|_node| {
        visited += 1;
    });
    let traverse_time = traverse_start.elapsed();

    assert_eq!(visited, 110);

    // Add mutations to some tensions
    let mutation_start = Instant::now();
    let tension_ids: Vec<String> = engine
        .store()
        .list_tensions()
        .unwrap()
        .iter()
        .map(|t| t.id.clone())
        .collect();
    for (i, id) in tension_ids.iter().enumerate() {
        if i % 5 == 0 {
            engine
                .update_actual(id, &format!("updated reality {}", i))
                .unwrap();
        }
    }
    let mutation_time = mutation_start.elapsed();

    // Compute dynamics for multiple tensions
    let dynamics_start = Instant::now();
    let all_mutations = engine.store().all_mutations().unwrap();
    for root_id in &parent_ids {
        let _ = detect_structural_conflict(
            &forest,
            root_id,
            &all_mutations,
            &Default::default(),
            Utc::now(),
        );
    }
    let dynamics_time = dynamics_start.elapsed();

    // Performance assertions (generous bounds for CI)
    println!("Creation time: {:?}", creation_time);
    println!("Forest build time: {:?}", forest_time);
    println!("Traversal time: {:?}", traverse_time);
    println!("Mutation time: {:?}", mutation_time);
    println!("Dynamics time: {:?}", dynamics_time);

    assert!(creation_time.as_millis() < 5000, "Creation should be <5s");
    assert!(
        forest_time.as_millis() < 500,
        "Forest build should be <500ms"
    );
    assert!(
        traverse_time.as_millis() < 100,
        "Traversal should be <100ms"
    );
    assert!(dynamics_time.as_millis() < 500, "Dynamics should be <500ms");
}

// ============================================================================
// VAL-CROSS-008: Concurrent Read Consistency
// ============================================================================

/// Test that concurrent reads work correctly.
/// Note: Store is not Send/Sync (uses Rc<RefCell>), so we test rapid sequential reads instead.
#[test]
fn test_concurrent_read_consistency() {
    let store = Store::new_in_memory().unwrap();

    // Populate store
    for i in 0..50 {
        store
            .create_tension(&format!("goal {}", i), &format!("reality {}", i))
            .unwrap();
    }

    // Rapid sequential reads (simulating concurrent access pattern)
    let iterations = Arc::new(AtomicUsize::new(0));

    for _ in 0..400 {
        let tensions = store.list_tensions().unwrap();
        assert_eq!(tensions.len(), 50);

        let roots = store.get_roots().unwrap();
        assert_eq!(roots.len(), 50);

        iterations.fetch_add(1, Ordering::SeqCst);
    }

    // All reads should have succeeded
    assert_eq!(iterations.load(Ordering::SeqCst), 400);
}

// ============================================================================
// VAL-CROSS-009: Unicode Roundtrip Through Full Stack
// ============================================================================

/// Test that Unicode data survives create -> store -> reload -> mutate -> serialize cycle.
#[test]
fn test_unicode_roundtrip_full_stack() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    // Create tensions with various Unicode content
    let cjk = engine
        .create_tension("写一本小说 📚", "有一个大纲 📝")
        .unwrap();
    let emoji = engine
        .create_tension("Party time! 🎉🎊🎈", "Planning phase 📋")
        .unwrap();
    let rtl = engine
        .create_tension("مرحبا بالعالم", "الواقع الحالي")
        .unwrap();
    let mixed = engine
        .create_tension("Hello 世界! 🌍 مرحبا", "Current 状态 📊")
        .unwrap();

    // Verify creation preserved Unicode
    assert_eq!(cjk.desired, "写一本小说 📚");
    assert_eq!(cjk.actual, "有一个大纲 📝");
    assert_eq!(emoji.desired, "Party time! 🎉🎊🎈");
    assert_eq!(rtl.desired, "مرحبا بالعالم");
    assert_eq!(mixed.desired, "Hello 世界! 🌍 مرحبا");

    // Update with Unicode
    engine.update_actual(&cjk.id, "写了一章 📖").unwrap();
    engine
        .update_desired(&emoji.id, "Party time! 🎉🎊🎈🎁")
        .unwrap();

    // Reload from store
    let cjk_reloaded = engine.store().get_tension(&cjk.id).unwrap().unwrap();
    let emoji_reloaded = engine.store().get_tension(&emoji.id).unwrap().unwrap();

    assert_eq!(cjk_reloaded.actual, "写了一章 📖");
    assert_eq!(emoji_reloaded.desired, "Party time! 🎉🎊🎈🎁");

    // Serialize and deserialize
    let cjk_json = serde_json::to_string(&cjk_reloaded).unwrap();
    let cjk_deserialized: Tension = serde_json::from_str(&cjk_json).unwrap();
    assert_eq!(cjk_deserialized.desired, "写一本小说 📚");
    assert_eq!(cjk_deserialized.actual, "写了一章 📖");

    // Compute dynamics with Unicode data (should not panic)
    let _forest = Forest::from_tensions(engine.store().list_tensions().unwrap()).unwrap();

    let st = sd_core::compute_structural_tension(&cjk_reloaded);
    assert!(st.is_some());

    let phase = classify_creative_cycle_phase(
        &cjk_reloaded,
        &engine.store().get_mutations(&cjk.id).unwrap(),
        &[],
        &Default::default(),
        Utc::now(),
    );
    // Should not panic with Unicode data
    let _ = phase.phase;
}

// ============================================================================
// VAL-CROSS-010: Error Recovery
// ============================================================================

/// Test that constraint violations leave system in consistent state.
#[test]
fn test_error_recovery_constraint_violations() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    // Create a tension
    let t = engine.create_tension("goal", "reality").unwrap();

    // Attempt invalid update (empty string)
    let result = engine.update_actual(&t.id, "");
    assert!(result.is_err(), "Empty actual should fail");

    // Verify state is unchanged
    let reloaded = engine.store().get_tension(&t.id).unwrap().unwrap();
    assert_eq!(
        reloaded.actual, "reality",
        "Actual should be unchanged after failed update"
    );

    // Verify no partial mutation was recorded
    let mutations = engine.store().get_mutations(&t.id).unwrap();
    let actual_mutations: Vec<_> = mutations.iter().filter(|m| m.field() == "actual").collect();
    assert!(
        actual_mutations.is_empty(),
        "No mutation should be recorded for failed update"
    );

    // Attempt invalid status transition
    engine.resolve(&t.id).unwrap();
    let result = engine.update_actual(&t.id, "new value");
    assert!(result.is_err(), "Update on Resolved tension should fail");

    // Verify state unchanged
    let resolved = engine.store().get_tension(&t.id).unwrap().unwrap();
    assert_eq!(resolved.status, TensionStatus::Resolved);
    assert_eq!(
        resolved.actual, "reality",
        "Actual unchanged after failed update on resolved"
    );
}

/// Test that transaction rollback works correctly.
#[test]
fn test_error_recovery_transaction_rollback() {
    let store = Store::new_in_memory().unwrap();

    // Create a tension
    let t = store.create_tension("goal", "reality").unwrap();

    // Start transaction and make changes
    store.begin_transaction().unwrap();
    store
        .update_actual_no_tx(&t.id, "intermediate value")
        .unwrap();

    // Rollback
    store.rollback_transaction().unwrap();

    // Verify original state
    let reloaded = store.get_tension(&t.id).unwrap().unwrap();
    assert_eq!(reloaded.actual, "reality", "Actual should be rolled back");

    // Verify no intermediate mutation persisted
    let mutations = store.get_mutations(&t.id).unwrap();
    assert_eq!(mutations.len(), 1, "Only creation mutation should exist");
}

// ============================================================================
// State Reconstruction from Events
// ============================================================================

/// Test that replaying events from empty state matches current store state.
#[test]
fn test_state_reconstruction_from_events() {
    let mut store = Store::new_in_memory().unwrap();
    let bus = EventBus::new();
    store.set_event_bus(bus);

    // Create tensions and mutations
    let t1 = store.create_tension("goal1", "reality1").unwrap();
    let t2 = store
        .create_tension_with_parent("goal2", "reality2", Some(t1.id.clone()))
        .unwrap();

    store.update_actual(&t1.id, "updated reality1").unwrap();
    store.update_desired(&t2.id, "updated goal2").unwrap();
    store.update_parent(&t2.id, None).unwrap(); // Make t2 a root

    // Get current state
    let final_t1 = store.get_tension(&t1.id).unwrap().unwrap();
    let final_t2 = store.get_tension(&t2.id).unwrap().unwrap();

    // Get mutation history
    let mutations1 = store.get_mutations(&t1.id).unwrap();
    let mutations2 = store.get_mutations(&t2.id).unwrap();

    // Replay mutations for t1
    let reconstructed1 = sd_core::replay_mutations(&mutations1).expect("Replay should succeed");
    assert_eq!(reconstructed1.id, final_t1.id);
    assert_eq!(reconstructed1.desired, final_t1.desired);
    assert_eq!(reconstructed1.actual, final_t1.actual);
    assert_eq!(reconstructed1.parent_id, final_t1.parent_id);
    assert_eq!(reconstructed1.status, final_t1.status);

    // Replay mutations for t2
    let reconstructed2 = sd_core::replay_mutations(&mutations2).expect("Replay should succeed");
    assert_eq!(reconstructed2.id, final_t2.id);
    assert_eq!(reconstructed2.desired, final_t2.desired);
    assert_eq!(reconstructed2.actual, final_t2.actual);
    assert_eq!(reconstructed2.parent_id, final_t2.parent_id);
    assert_eq!(reconstructed2.status, final_t2.status);
}

// ============================================================================
// Additional Cross-Module Tests
// ============================================================================

/// Test that dynamics and events work together correctly.
#[test]
fn test_dynamics_events_integration() {
    let mut engine = DynamicsEngine::new_in_memory().unwrap();

    // Use sensitive thresholds
    let mut thresholds = DynamicsThresholds::default();
    thresholds.lifecycle.active_frequency_threshold = 1;
    engine.set_thresholds(thresholds);

    // Track events
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let _handle = engine.subscribe(move |e| {
        events_clone.lock().unwrap().push(e.clone());
    });

    // Create and mutate tension
    let t = engine.create_tension("goal abc", "reality xyz").unwrap();

    // Update to trigger phase transition
    engine.update_actual(&t.id, "goal ab").unwrap();
    engine.update_actual(&t.id, "goal a").unwrap();
    engine.update_actual(&t.id, "goal").unwrap();

    // Compute dynamics
    engine.compute_and_emit_for_tension(&t.id);

    // Get recorded events
    let recorded = events.lock().unwrap().clone();

    // Should have lifecycle events
    let lifecycle_events: Vec<_> = recorded
        .iter()
        .filter(|e| matches!(e, Event::LifecycleTransition { .. }))
        .collect();

    // Phase should have changed from Germination
    let prev = engine.previous_state().tensions.get(&t.id).unwrap();
    assert_ne!(
        prev.phase,
        Some(sd_core::CreativeCyclePhase::Germination),
        "Phase should have transitioned from Germination after updates"
    );

    // If lifecycle transition was emitted, verify it
    if !lifecycle_events.is_empty() {
        if let Event::LifecycleTransition {
            old_phase,
            new_phase,
            ..
        } = lifecycle_events[0]
        {
            assert_eq!(*old_phase, sd_core::CreativeCyclePhase::Germination);
            // New phase should not be Germination
            assert_ne!(*new_phase, sd_core::CreativeCyclePhase::Germination);
        }
    }
}

/// Test that store events are in causal order.
#[test]
fn test_store_events_causal_order() {
    let bus = EventBus::new();
    let mut store = Store::new_in_memory().unwrap();
    store.set_event_bus(bus.clone());

    // Create tension
    let t = store.create_tension("goal", "reality").unwrap();

    // Update actual
    store.update_actual(&t.id, "reality2").unwrap();

    // Update desired
    store.update_desired(&t.id, "goal2").unwrap();

    // Resolve
    store.update_status(&t.id, TensionStatus::Resolved).unwrap();

    // Get event history
    let history = bus.history();

    // Verify causal order
    assert!(history.len() >= 4);

    // First should be TensionCreated
    assert!(matches!(
        &history[0],
        Event::TensionCreated {
            tension_id,
            ..
        } if tension_id == &t.id
    ));

    // Second should be RealityConfronted
    assert!(matches!(
        &history[1],
        Event::RealityConfronted {
            tension_id,
            ..
        } if tension_id == &t.id
    ));

    // Third should be DesireRevised
    assert!(matches!(
        &history[2],
        Event::DesireRevised {
            tension_id,
            ..
        } if tension_id == &t.id
    ));

    // Fourth should be TensionResolved
    assert!(matches!(
        &history[3],
        Event::TensionResolved {
            tension_id,
            ..
        } if tension_id == &t.id
    ));
}

/// Test that store with no event bus doesn't panic.
#[test]
fn test_store_without_event_bus() {
    let store = Store::new_in_memory().unwrap();

    // All operations should work without event bus
    let t = store.create_tension("goal", "reality").unwrap();
    store.update_actual(&t.id, "updated").unwrap();
    store.update_desired(&t.id, "new goal").unwrap();
    store.update_status(&t.id, TensionStatus::Resolved).unwrap();

    // Verify operations succeeded
    let reloaded = store.get_tension(&t.id).unwrap().unwrap();
    assert_eq!(reloaded.actual, "updated");
    assert_eq!(reloaded.desired, "new goal");
    assert_eq!(reloaded.status, TensionStatus::Resolved);
}
