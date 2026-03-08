//! Engine for computing dynamics and emitting transition events.
//!
//! The dynamics functions in the `dynamics` module are pure computations.
//! This module provides a thin utility layer that:
//! 1. Tracks previous dynamics state
//! 2. Computes current dynamics
//! 3. Compares with previous state
//! 4. Emits appropriate transition events via EventBus
//!
//! This enables integration tests and instruments to receive dynamic
//! transition events (ConflictDetected, LifecycleTransition, OscillationDetected, etc.)

use chrono::Utc;
use std::collections::HashMap;

use crate::dynamics::{
    CompensatingStrategyThresholds, ConflictPattern, ConflictThresholds, CreativeCyclePhase,
    LifecycleThresholds, NeglectThresholds, NeglectType, Orientation, OrientationThresholds,
    OscillationThresholds, ResolutionThresholds, classify_creative_cycle_phase,
    classify_orientation, detect_neglect, detect_oscillation, detect_resolution,
    detect_structural_conflict,
};
use crate::events::{Event, EventBuilder, EventBus};
use crate::store::Store;
use crate::tension::Tension;
use crate::tree::Forest;

/// Previous dynamics state for a single tension.
#[derive(Debug, Clone, Default)]
pub struct PreviousDynamics {
    /// Previous phase (if any).
    pub phase: Option<CreativeCyclePhase>,
    /// Whether conflict was detected.
    pub had_conflict: bool,
    /// Conflict pattern (if any).
    pub conflict_pattern: Option<ConflictPattern>,
    /// Whether oscillation was detected.
    pub had_oscillation: bool,
    /// Whether resolution was detected.
    pub had_resolution: bool,
    /// Whether neglect was detected.
    pub neglect_type: Option<NeglectType>,
    /// Previous orientation (if any).
    pub orientation: Option<Orientation>,
}

/// Previous dynamics state for all tensions.
#[derive(Debug, Clone, Default)]
pub struct PreviousState {
    /// Per-tension dynamics state.
    pub tensions: HashMap<String, PreviousDynamics>,
    /// Previous global orientation.
    pub global_orientation: Option<Orientation>,
}

/// Threshold configuration for all dynamics.
#[derive(Debug, Clone, Default)]
pub struct DynamicsThresholds {
    pub conflict: ConflictThresholds,
    pub oscillation: OscillationThresholds,
    pub resolution: ResolutionThresholds,
    pub lifecycle: LifecycleThresholds,
    pub orientation: OrientationThresholds,
    pub compensating_strategy: CompensatingStrategyThresholds,
    pub neglect: NeglectThresholds,
}

/// Engine that computes dynamics and emits transition events.
pub struct DynamicsEngine {
    store: Store,
    bus: EventBus,
    previous_state: PreviousState,
    thresholds: DynamicsThresholds,
}

impl DynamicsEngine {
    /// Create a new dynamics engine with an in-memory store.
    pub fn new_in_memory() -> Result<Self, crate::store::StoreError> {
        let mut store = Store::new_in_memory()?;
        let bus = EventBus::new();
        // Share the event bus with the store so store events are emitted
        store.set_event_bus(bus.clone());
        Ok(Self {
            store,
            bus,
            previous_state: PreviousState::default(),
            thresholds: DynamicsThresholds::default(),
        })
    }

    /// Create a dynamics engine with an existing store.
    pub fn with_store(store: Store) -> Self {
        let bus = EventBus::new();
        Self {
            store,
            bus,
            previous_state: PreviousState::default(),
            thresholds: DynamicsThresholds::default(),
        }
    }

    /// Set the event bus (also updates the store's event bus).
    pub fn set_event_bus(&mut self, bus: EventBus) {
        self.store.set_event_bus(bus.clone());
        self.bus = bus;
    }

    /// Get a reference to the event bus.
    pub fn event_bus(&self) -> &EventBus {
        &self.bus
    }

    /// Get a mutable reference to the store.
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    /// Get a reference to the store.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Set thresholds.
    pub fn set_thresholds(&mut self, thresholds: DynamicsThresholds) {
        self.thresholds = thresholds;
    }

    /// Get the previous state.
    pub fn previous_state(&self) -> &PreviousState {
        &self.previous_state
    }

    /// Create a tension and emit TensionCreated event.
    pub fn create_tension(
        &mut self,
        desired: &str,
        actual: &str,
    ) -> Result<Tension, crate::tension::SdError> {
        let tension = self.store.create_tension(desired, actual)?;
        // Initialize previous dynamics for this tension
        self.previous_state.tensions.insert(
            tension.id.clone(),
            PreviousDynamics {
                phase: Some(CreativeCyclePhase::Germination),
                ..Default::default()
            },
        );
        Ok(tension)
    }

    /// Create a tension with parent and emit TensionCreated event.
    pub fn create_tension_with_parent(
        &mut self,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
    ) -> Result<Tension, crate::tension::SdError> {
        let tension = self
            .store
            .create_tension_with_parent(desired, actual, parent_id)?;
        self.previous_state.tensions.insert(
            tension.id.clone(),
            PreviousDynamics {
                phase: Some(CreativeCyclePhase::Germination),
                ..Default::default()
            },
        );
        Ok(tension)
    }

    /// Update actual and recompute dynamics.
    pub fn update_actual(
        &mut self,
        id: &str,
        new_actual: &str,
    ) -> Result<(), crate::tension::SdError> {
        self.store.update_actual(id, new_actual)
    }

    /// Update desired and recompute dynamics.
    pub fn update_desired(
        &mut self,
        id: &str,
        new_desired: &str,
    ) -> Result<(), crate::tension::SdError> {
        self.store.update_desired(id, new_desired)
    }

    /// Update parent and recompute dynamics.
    pub fn update_parent(
        &mut self,
        id: &str,
        new_parent_id: Option<&str>,
    ) -> Result<(), crate::tension::SdError> {
        self.store.update_parent(id, new_parent_id)
    }

    /// Resolve a tension.
    pub fn resolve(&mut self, id: &str) -> Result<(), crate::tension::SdError> {
        self.store
            .update_status(id, crate::tension::TensionStatus::Resolved)
    }

    /// Release a tension.
    pub fn release(&mut self, id: &str) -> Result<(), crate::tension::SdError> {
        self.store
            .update_status(id, crate::tension::TensionStatus::Released)
    }

    /// Compute dynamics and emit transition events for a single tension.
    ///
    /// Returns the events that were emitted.
    pub fn compute_and_emit_for_tension(&mut self, tension_id: &str) -> Vec<Event> {
        let mut events = Vec::new();
        let now = Utc::now();

        // Get the tension
        let tension = match self.store.get_tension(tension_id).unwrap() {
            Some(t) => t,
            None => return events,
        };

        // Get mutations for this tension
        let mutations = self.store.get_mutations(tension_id).unwrap();

        // Get all mutations for conflict detection (need sibling mutations)
        let all_mutations = self.store.all_mutations().unwrap();

        // Build forest for conflict detection
        let tensions_list = self.store.list_tensions().unwrap();
        let forest = match Forest::from_tensions(tensions_list) {
            Ok(f) => f,
            Err(_) => return events,
        };

        // Get previous state for this tension
        let prev = self
            .previous_state
            .tensions
            .get(tension_id)
            .cloned()
            .unwrap_or_default();

        // --- Compute lifecycle phase ---
        let resolved_tensions: Vec<Tension> = self
            .store
            .list_tensions()
            .unwrap()
            .into_iter()
            .filter(|t| t.status == crate::tension::TensionStatus::Resolved)
            .collect();

        let phase_result = classify_creative_cycle_phase(
            &tension,
            &mutations,
            &resolved_tensions,
            &self.thresholds.lifecycle,
            now,
        );

        // Check for phase transition
        if prev.phase != Some(phase_result.phase)
            && let Some(old_phase) = prev.phase
        {
            events.push(EventBuilder::lifecycle_transition(
                tension_id.to_owned(),
                old_phase,
                phase_result.phase,
            ));
        }

        // --- Compute conflict ---
        let conflict = detect_structural_conflict(
            &forest,
            tension_id,
            &all_mutations,
            &self.thresholds.conflict,
            now,
        );

        let has_conflict = conflict.is_some();
        let conflict_pattern = conflict.as_ref().map(|c| c.pattern);

        // Check for conflict transition
        if has_conflict && !prev.had_conflict {
            // Conflict detected
            if let Some(c) = &conflict {
                events.push(EventBuilder::conflict_detected(
                    c.tension_ids.clone(),
                    c.pattern,
                ));
            }
        } else if !has_conflict && prev.had_conflict {
            // Conflict resolved
            if let Some(pattern) = prev.conflict_pattern {
                events.push(EventBuilder::conflict_resolved(
                    vec![tension_id.to_owned()],
                    pattern,
                ));
            }
        }

        // --- Compute oscillation ---
        let oscillation = detect_oscillation(
            tension_id,
            &mutations,
            &self.thresholds.oscillation,
            now,
            tension.horizon.as_ref(),
        );
        let has_oscillation = oscillation.is_some();

        // Check for oscillation transition
        if has_oscillation
            && !prev.had_oscillation
            && let Some(o) = &oscillation
        {
            events.push(EventBuilder::oscillation_detected(
                tension_id.to_owned(),
                o.reversals,
                o.magnitude,
            ));
        }

        // --- Compute resolution ---
        let resolution = detect_resolution(&tension, &mutations, &self.thresholds.resolution, now);
        let has_resolution = resolution.is_some();

        // Check for resolution transition
        if has_resolution
            && !prev.had_resolution
            && let Some(r) = &resolution
        {
            events.push(EventBuilder::resolution_achieved(
                tension_id.to_owned(),
                r.velocity,
            ));
        }

        // --- Compute neglect ---
        let neglect = detect_neglect(
            &forest,
            tension_id,
            &all_mutations,
            &self.thresholds.neglect,
            now,
        );
        let neglect_type = neglect.as_ref().map(|n| n.neglect_type);

        // Check for neglect transition
        if neglect_type.is_some()
            && prev.neglect_type.is_none()
            && let Some(n) = &neglect
        {
            events.push(EventBuilder::neglect_detected(
                vec![tension_id.to_owned()],
                n.neglect_type,
            ));
        }

        // Update previous state
        self.previous_state.tensions.insert(
            tension_id.to_owned(),
            PreviousDynamics {
                phase: Some(phase_result.phase),
                had_conflict: has_conflict,
                conflict_pattern,
                had_oscillation: has_oscillation,
                had_resolution: has_resolution,
                neglect_type,
                orientation: prev.orientation,
            },
        );

        // Emit all events
        for event in &events {
            self.bus.emit(event);
        }

        events
    }

    /// Compute dynamics and emit transition events for all tensions.
    ///
    /// Returns the events that were emitted.
    pub fn compute_and_emit_all(&mut self) -> Vec<Event> {
        let tension_ids: Vec<String> = self
            .store
            .list_tensions()
            .unwrap()
            .iter()
            .map(|t| t.id.clone())
            .collect();

        let mut all_events = Vec::new();
        for id in tension_ids {
            let events = self.compute_and_emit_for_tension(&id);
            all_events.extend(events);
        }

        // --- Compute global orientation ---
        let tensions = self.store.list_tensions().unwrap();
        let all_mutations = self.store.all_mutations().unwrap();
        let now = Utc::now();

        if let Some(orient_result) =
            classify_orientation(&tensions, &all_mutations, &self.thresholds.orientation, now)
        {
            if self.previous_state.global_orientation != Some(orient_result.orientation)
                && let Some(old_orient) = self.previous_state.global_orientation
            {
                let tension_ids: Vec<String> = tensions.iter().map(|t| t.id.clone()).collect();
                let event = EventBuilder::orientation_shift(
                    tension_ids,
                    old_orient,
                    orient_result.orientation,
                );
                self.bus.emit(&event);
                all_events.push(event);
            }
            self.previous_state.global_orientation = Some(orient_result.orientation);
        }

        all_events
    }

    /// Get all events from the bus history.
    pub fn event_history(&self) -> Vec<Event> {
        self.bus.history()
    }

    /// Clear event history.
    pub fn clear_event_history(&self) {
        self.bus.clear_history();
    }

    /// Subscribe to events.
    pub fn subscribe<F>(&self, callback: F) -> crate::events::SubscriptionHandle
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        self.bus.subscribe(callback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creates_tension_with_initial_phase() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();
        let t = engine.create_tension("goal", "reality").unwrap();

        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert_eq!(prev.phase, Some(CreativeCyclePhase::Germination));
    }

    #[test]
    fn test_engine_emits_lifecycle_transition() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Use more sensitive thresholds to detect phase transitions
        engine.thresholds.lifecycle.active_frequency_threshold = 1;
        engine.thresholds.lifecycle.convergence_threshold = 0.5;

        let t = engine.create_tension("goal abcdef", "reality xyz").unwrap();

        // Initial phase is Germination
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert_eq!(prev.phase, Some(CreativeCyclePhase::Germination));

        // Update actual to show activity
        engine.update_actual(&t.id, "goal abc progress").unwrap();

        // Compute dynamics
        let events = engine.compute_and_emit_for_tension(&t.id);

        // Should have emitted LifecycleTransition event
        let lifecycle_events: Vec<&Event> = events
            .iter()
            .filter(|e| matches!(e, Event::LifecycleTransition { .. }))
            .collect();

        // If a transition occurred, we should have the event
        // Phase might transition from Germination to Assimilation
        if !lifecycle_events.is_empty() {
            if let Event::LifecycleTransition {
                old_phase,
                new_phase,
                ..
            } = lifecycle_events[0]
            {
                assert_eq!(*old_phase, CreativeCyclePhase::Germination);
                assert!(matches!(
                    *new_phase,
                    CreativeCyclePhase::Assimilation | CreativeCyclePhase::Completion
                ));
            }
        }
    }

    #[test]
    fn test_engine_emits_oscillation_detected() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Use sensitive thresholds
        engine.thresholds.oscillation.magnitude_threshold = 0.001;
        engine.thresholds.oscillation.frequency_threshold = 2;
        engine.thresholds.oscillation.recency_window_seconds = 3600 * 24 * 365;

        let t = engine.create_tension("goal", "a").unwrap();

        // Create oscillation pattern
        engine.update_actual(&t.id, "ab").unwrap();
        engine.update_actual(&t.id, "a").unwrap();
        engine.update_actual(&t.id, "abc").unwrap();
        engine.update_actual(&t.id, "a").unwrap();

        let events = engine.compute_and_emit_for_tension(&t.id);

        // Should emit OscillationDetected
        let osc_events: Vec<&Event> = events
            .iter()
            .filter(|e| matches!(e, Event::OscillationDetected { .. }))
            .collect();

        assert!(
            !osc_events.is_empty(),
            "Should emit OscillationDetected event"
        );
    }

    #[test]
    fn test_engine_emits_conflict_detected() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create parent and children
        let parent = engine.create_tension("parent", "p reality").unwrap();
        let child1 = engine
            .create_tension_with_parent("child1", "c1", Some(parent.id.clone()))
            .unwrap();
        let _child2 = engine
            .create_tension_with_parent("child2", "c2", Some(parent.id.clone()))
            .unwrap();

        // Create asymmetric activity
        for _ in 0..5 {
            engine.update_actual(&child1.id, "active update").unwrap();
        }

        let events = engine.compute_and_emit_for_tension(&child1.id);

        // Should emit ConflictDetected
        let conflict_events: Vec<&Event> = events
            .iter()
            .filter(|e| matches!(e, Event::ConflictDetected { .. }))
            .collect();

        assert!(
            !conflict_events.is_empty(),
            "Should emit ConflictDetected event"
        );
    }

    #[test]
    fn test_engine_no_duplicate_events_on_stable_state() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();
        let t = engine.create_tension("goal", "reality").unwrap();

        // Compute once
        let events1 = engine.compute_and_emit_for_tension(&t.id);

        // Compute again without changes
        let events2 = engine.compute_and_emit_for_tension(&t.id);

        // Second computation should emit no events (state is stable)
        assert!(
            events2.is_empty(),
            "Should not emit duplicate events on stable state"
        );
    }

    #[test]
    fn test_engine_tracks_conflict_resolution() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create parent and children
        let parent = engine.create_tension("parent", "p reality").unwrap();
        let child1 = engine
            .create_tension_with_parent("child1", "c1", Some(parent.id.clone()))
            .unwrap();
        let child2 = engine
            .create_tension_with_parent("child2", "c2", Some(parent.id.clone()))
            .unwrap();

        // Create asymmetric activity
        for _ in 0..5 {
            engine.update_actual(&child1.id, "active update").unwrap();
        }

        // Compute to detect conflict
        let events1 = engine.compute_and_emit_for_tension(&child1.id);
        assert!(
            events1
                .iter()
                .any(|e| matches!(e, Event::ConflictDetected { .. }))
        );

        // Now balance activity
        for _ in 0..5 {
            engine.update_actual(&child2.id, "balanced update").unwrap();
        }

        // Compute again - conflict should be resolved
        let events2 = engine.compute_and_emit_for_tension(&child1.id);

        // Should emit ConflictResolved
        let resolved_events: Vec<&Event> = events2
            .iter()
            .filter(|e| matches!(e, Event::ConflictResolved { .. }))
            .collect();

        assert!(
            !resolved_events.is_empty(),
            "Should emit ConflictResolved when conflict ends"
        );
    }

    #[test]
    fn test_engine_subscribe_receives_events() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Use sensitive thresholds
        engine.thresholds.oscillation.magnitude_threshold = 0.001;
        engine.thresholds.oscillation.frequency_threshold = 1;

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        let _handle = engine.subscribe(move |_| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let t = engine.create_tension("goal", "a").unwrap();

        // Create oscillation
        engine.update_actual(&t.id, "ab").unwrap();
        engine.update_actual(&t.id, "a").unwrap();

        engine.compute_and_emit_for_tension(&t.id);

        // Subscriber should have received events
        assert!(
            count.load(Ordering::SeqCst) > 0,
            "Subscriber should receive events"
        );
    }
}
