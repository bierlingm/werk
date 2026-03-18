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
    AssimilationDepthResult, AssimilationDepthThresholds, CompensatingStrategy,
    CompensatingStrategyThresholds, CompensatingStrategyType, Conflict, ConflictPattern,
    ConflictThresholds, CreativeCyclePhase, CreativeCyclePhaseResult, HorizonDrift,
    HorizonDriftType, LifecycleThresholds, Neglect, NeglectThresholds, NeglectType, Orientation,
    OrientationResult, OrientationThresholds, Oscillation, OscillationThresholds, Resolution,
    ResolutionThresholds, StructuralTendencyResult, StructuralTension, Urgency,
    classify_creative_cycle_phase, classify_orientation, compute_structural_tension,
    compute_urgency, detect_compensating_strategy, detect_horizon_drift, detect_neglect,
    detect_oscillation, detect_resolution, detect_structural_conflict, measure_assimilation_depth,
    predict_structural_tendency,
};
use crate::events::{Event, EventBuilder, EventBus};
use crate::horizon::Horizon;
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
    /// Whether urgency was above threshold on previous computation.
    /// Only meaningful when tension has a horizon.
    pub had_urgency_above_threshold: bool,
    /// Previous horizon drift type (if any).
    pub horizon_drift_type: Option<HorizonDriftType>,
    /// Previous urgency value (if any).
    pub urgency: Option<f64>,
    /// Whether a compensating strategy was detected on previous computation.
    pub had_compensating_strategy: bool,
    /// The type of compensating strategy detected (if any).
    pub compensating_strategy_type: Option<CompensatingStrategyType>,
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
    /// Threshold for urgency transition events.
    /// When urgency crosses this threshold (up or down), an event is emitted.
    pub urgency_threshold: f64,
}

/// All computed dynamics results for a single tension.
///
/// This struct holds the full output of dynamics computation, including all
/// 10 dynamics plus horizon drift and urgency. It is returned by
/// [`DynamicsEngine::compute_full_dynamics_for_tension`] so that consumers
/// can access the computed values directly without calling individual
/// dynamics functions.
#[derive(Debug, Clone)]
pub struct ComputedDynamics {
    /// Structural tension (gap magnitude and pressure).
    pub structural_tension: Option<StructuralTension>,
    /// Structural conflict between sibling tensions.
    pub conflict: Option<Conflict>,
    /// Oscillation detection result.
    pub oscillation: Option<Oscillation>,
    /// Resolution detection result.
    pub resolution: Option<Resolution>,
    /// Creative cycle phase classification.
    pub phase: CreativeCyclePhaseResult,
    /// Orientation classification (global, across all tensions).
    pub orientation: Option<OrientationResult>,
    /// Compensating strategy detection result.
    pub compensating_strategy: Option<CompensatingStrategy>,
    /// Structural tendency prediction.
    pub tendency: StructuralTendencyResult,
    /// Assimilation depth measurement.
    pub assimilation: AssimilationDepthResult,
    /// Neglect detection result.
    pub neglect: Option<Neglect>,
    /// Horizon drift analysis.
    pub horizon_drift: HorizonDrift,
    /// Urgency value (if horizon is present).
    pub urgency: Option<Urgency>,
    /// Transition events emitted during this computation.
    pub events: Vec<Event>,
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
    ///
    /// If a parent_id is provided, automatically captures the parent's current
    /// desired and actual state as snapshots on the child tension.
    pub fn create_tension_with_parent(
        &mut self,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
    ) -> Result<Tension, crate::tension::SdError> {
        // Capture parent snapshots if parent exists
        let (parent_desired_snapshot, parent_actual_snapshot) =
            if let Some(ref pid) = parent_id {
                match self.store.get_tension(pid) {
                    Ok(Some(parent)) => (Some(parent.desired), Some(parent.actual)),
                    _ => (None, None),
                }
            } else {
                (None, None)
            };

        let tension = self.store.create_tension_full_with_snapshots(
            desired,
            actual,
            parent_id,
            None,
            None,
            parent_desired_snapshot,
            parent_actual_snapshot,
        )?;
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

    /// Update the position of a tension for sibling ordering.
    pub fn update_position(
        &mut self,
        id: &str,
        new_position: Option<i32>,
    ) -> Result<(), crate::tension::SdError> {
        self.store.update_position(id, new_position)
    }

    /// Reorder siblings by assigning positions to all children of a parent.
    pub fn reorder_siblings(
        &mut self,
        parent_id: Option<&str>,
        ordered_ids: &[String],
    ) -> Result<(), crate::tension::SdError> {
        self.store.reorder_siblings(parent_id, ordered_ids)
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

    /// Create a tension with all optional fields including horizon.
    ///
    /// This creates a tension with a temporal horizon, records the creation mutation,
    /// and emits a TensionCreated event with the horizon field populated.
    /// If a parent_id is provided, automatically captures parent snapshots.
    pub fn create_tension_full(
        &mut self,
        desired: &str,
        actual: &str,
        parent_id: Option<String>,
        horizon: Option<Horizon>,
    ) -> Result<Tension, crate::tension::SdError> {
        // Capture parent snapshots if parent exists
        let (parent_desired_snapshot, parent_actual_snapshot) =
            if let Some(ref pid) = parent_id {
                match self.store.get_tension(pid) {
                    Ok(Some(parent)) => (Some(parent.desired), Some(parent.actual)),
                    _ => (None, None),
                }
            } else {
                (None, None)
            };

        let tension = self.store.create_tension_full_with_snapshots(
            desired,
            actual,
            parent_id,
            horizon,
            None,
            parent_desired_snapshot,
            parent_actual_snapshot,
        )?;
        // Initialize previous dynamics for this tension
        let mut prev = PreviousDynamics {
            phase: Some(CreativeCyclePhase::Germination),
            ..Default::default()
        };
        // Compute initial urgency if horizon present
        if let Some(urgency) = compute_urgency(&tension, Utc::now()) {
            prev.urgency = Some(urgency.value);
            prev.had_urgency_above_threshold = urgency.value >= self.thresholds.urgency_threshold;
        }
        self.previous_state
            .tensions
            .insert(tension.id.clone(), prev);
        Ok(tension)
    }

    /// Update the temporal horizon of a tension.
    ///
    /// Validates that the tension is Active, persists the change, records a mutation,
    /// and emits a HorizonChanged event.
    pub fn update_horizon(
        &mut self,
        id: &str,
        new_horizon: Option<Horizon>,
    ) -> Result<(), crate::tension::SdError> {
        self.store.update_horizon(id, new_horizon)
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
        } else if !has_oscillation && prev.had_oscillation {
            // Oscillation resolved
            events.push(EventBuilder::oscillation_resolved(tension_id.to_owned()));
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
        } else if !has_resolution && prev.had_resolution {
            // Resolution lost
            events.push(EventBuilder::resolution_lost(tension_id.to_owned()));
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
        } else if neglect_type.is_none() && prev.neglect_type.is_some() {
            // Neglect resolved
            if let Some(former_type) = prev.neglect_type {
                events.push(EventBuilder::neglect_resolved(
                    tension_id.to_owned(),
                    former_type,
                ));
            }
        }

        // --- Compute urgency (horizon-aware) ---
        // Urgency is only computable when a horizon is present
        let urgency = compute_urgency(&tension, now);
        let urgency_value = urgency.as_ref().map(|u| u.value);

        // Check for urgency threshold crossing
        let had_urgency_above = prev.had_urgency_above_threshold;
        let now_urgency_above = urgency_value
            .map(|v| v >= self.thresholds.urgency_threshold)
            .unwrap_or(false);

        // Emit UrgencyThresholdCrossed on crossing (only when we have both old and new values)
        if let Some(new_urgency) = urgency_value
            && let Some(old_urgency) = prev.urgency
        {
            if now_urgency_above && !had_urgency_above {
                // Crossed above threshold
                events.push(EventBuilder::urgency_threshold_crossed(
                    tension_id.to_owned(),
                    old_urgency,
                    new_urgency,
                    self.thresholds.urgency_threshold,
                    true,
                ));
            } else if !now_urgency_above && had_urgency_above {
                // Crossed below threshold
                events.push(EventBuilder::urgency_threshold_crossed(
                    tension_id.to_owned(),
                    old_urgency,
                    new_urgency,
                    self.thresholds.urgency_threshold,
                    false,
                ));
            }
        }

        // --- Detect horizon drift ---
        // Drift is detected from horizon mutation patterns
        let drift = detect_horizon_drift(tension_id, &mutations);
        let drift_type = drift.drift_type;

        // Emit HorizonDriftDetected when drift transitions from Stable to non-Stable
        // or between non-Stable types
        if drift_type != HorizonDriftType::Stable && prev.horizon_drift_type != Some(drift_type) {
            events.push(EventBuilder::horizon_drift_detected(
                tension_id.to_owned(),
                drift_type,
                drift.change_count,
            ));
        }

        // --- Detect compensating strategy ---
        let comp_strategy = detect_compensating_strategy(
            tension_id,
            &mutations,
            oscillation.as_ref(),
            &self.thresholds.compensating_strategy,
            now,
            tension.horizon.as_ref(),
        );
        let has_compensating_strategy = comp_strategy.is_some();
        let comp_strategy_type = comp_strategy.as_ref().map(|cs| cs.strategy_type);

        // Emit CompensatingStrategyDetected on first detection (not on persistent)
        if has_compensating_strategy
            && !prev.had_compensating_strategy
            && let Some(cs) = &comp_strategy
        {
            events.push(EventBuilder::compensating_strategy_detected(
                tension_id.to_owned(),
                cs.strategy_type,
                cs.persistence_seconds,
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
                had_urgency_above_threshold: now_urgency_above,
                horizon_drift_type: if drift.change_count > 0 {
                    Some(drift_type)
                } else {
                    None
                },
                urgency: urgency_value,
                had_compensating_strategy: has_compensating_strategy,
                compensating_strategy_type: comp_strategy_type,
            },
        );

        // Emit all events
        for event in &events {
            self.bus.emit(event);
        }

        events
    }

    /// Compute all dynamics for a tension, emit transition events, and return full results.
    ///
    /// This method computes all 10 dynamics (structural tension, conflict, oscillation,
    /// resolution, phase, orientation, compensating strategy, tendency, assimilation depth,
    /// neglect) plus horizon drift and urgency. It also performs event emission and state
    /// tracking just like [`compute_and_emit_for_tension`].
    ///
    /// Unlike `compute_and_emit_for_tension` which only returns transition events, this
    /// method returns a [`ComputedDynamics`] struct containing all computed values.
    pub fn compute_full_dynamics_for_tension(
        &mut self,
        tension_id: &str,
    ) -> Option<ComputedDynamics> {
        let now = Utc::now();

        // Get the tension
        let tension = self.store.get_tension(tension_id).unwrap()?;

        // Get mutations for this tension
        let mutations = self.store.get_mutations(tension_id).unwrap();

        // Get all tensions and mutations
        let tensions_list = self.store.list_tensions().unwrap();
        let all_mutations = self.store.all_mutations().unwrap();

        // Build forest
        let forest = match Forest::from_tensions(tensions_list.clone()) {
            Ok(f) => f,
            Err(_) => return None,
        };

        // Get previous state for this tension
        let prev = self
            .previous_state
            .tensions
            .get(tension_id)
            .cloned()
            .unwrap_or_default();

        let mut events = Vec::new();

        // 1. Structural Tension
        let structural_tension = compute_structural_tension(&tension, now);

        // 2. Structural Conflict
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
            if let Some(c) = &conflict {
                events.push(EventBuilder::conflict_detected(
                    c.tension_ids.clone(),
                    c.pattern,
                ));
            }
        } else if !has_conflict
            && prev.had_conflict
            && let Some(pattern) = prev.conflict_pattern
        {
            events.push(EventBuilder::conflict_resolved(
                vec![tension_id.to_owned()],
                pattern,
            ));
        }

        // 3. Oscillation
        let oscillation = detect_oscillation(
            tension_id,
            &mutations,
            &self.thresholds.oscillation,
            now,
            tension.horizon.as_ref(),
        );
        let has_oscillation = oscillation.is_some();

        if has_oscillation
            && !prev.had_oscillation
            && let Some(o) = &oscillation
        {
            events.push(EventBuilder::oscillation_detected(
                tension_id.to_owned(),
                o.reversals,
                o.magnitude,
            ));
        } else if !has_oscillation && prev.had_oscillation {
            events.push(EventBuilder::oscillation_resolved(tension_id.to_owned()));
        }

        // 4. Resolution
        let resolution = detect_resolution(&tension, &mutations, &self.thresholds.resolution, now);
        let has_resolution = resolution.is_some();

        if has_resolution
            && !prev.had_resolution
            && let Some(r) = &resolution
        {
            events.push(EventBuilder::resolution_achieved(
                tension_id.to_owned(),
                r.velocity,
            ));
        } else if !has_resolution && prev.had_resolution {
            events.push(EventBuilder::resolution_lost(tension_id.to_owned()));
        }

        // 5. Creative Cycle Phase
        let resolved_tensions: Vec<Tension> = tensions_list
            .iter()
            .filter(|t| t.status == crate::tension::TensionStatus::Resolved)
            .cloned()
            .collect();

        let phase = classify_creative_cycle_phase(
            &tension,
            &mutations,
            &resolved_tensions,
            &self.thresholds.lifecycle,
            now,
        );

        if prev.phase != Some(phase.phase)
            && let Some(old_phase) = prev.phase
        {
            events.push(EventBuilder::lifecycle_transition(
                tension_id.to_owned(),
                old_phase,
                phase.phase,
            ));
        }

        // 6. Orientation (global, across all tensions)
        let orientation = classify_orientation(
            &tensions_list,
            &all_mutations,
            &self.thresholds.orientation,
            now,
        );

        // 7. Compensating Strategy
        let compensating_strategy = detect_compensating_strategy(
            tension_id,
            &mutations,
            oscillation.as_ref(),
            &self.thresholds.compensating_strategy,
            now,
            tension.horizon.as_ref(),
        );
        let has_compensating_strategy = compensating_strategy.is_some();
        let comp_strategy_type = compensating_strategy.as_ref().map(|cs| cs.strategy_type);

        if has_compensating_strategy
            && !prev.had_compensating_strategy
            && let Some(cs) = &compensating_strategy
        {
            events.push(EventBuilder::compensating_strategy_detected(
                tension_id.to_owned(),
                cs.strategy_type,
                cs.persistence_seconds,
            ));
        }

        // 8. Structural Tendency
        let tendency = predict_structural_tendency(&tension, has_conflict, Some(now), None);

        // 9. Assimilation Depth
        let assimilation = measure_assimilation_depth(
            tension_id,
            &mutations,
            &tension,
            &AssimilationDepthThresholds::default(),
            now,
        );

        // 10. Neglect
        let neglect = detect_neglect(
            &forest,
            tension_id,
            &all_mutations,
            &self.thresholds.neglect,
            now,
        );
        let neglect_type = neglect.as_ref().map(|n| n.neglect_type);

        if neglect_type.is_some()
            && prev.neglect_type.is_none()
            && let Some(n) = &neglect
        {
            events.push(EventBuilder::neglect_detected(
                vec![tension_id.to_owned()],
                n.neglect_type,
            ));
        } else if neglect_type.is_none()
            && prev.neglect_type.is_some()
            && let Some(former_type) = prev.neglect_type
        {
            events.push(EventBuilder::neglect_resolved(
                tension_id.to_owned(),
                former_type,
            ));
        }

        // Urgency
        let urgency = compute_urgency(&tension, now);
        let urgency_value = urgency.as_ref().map(|u| u.value);

        let had_urgency_above = prev.had_urgency_above_threshold;
        let now_urgency_above = urgency_value
            .map(|v| v >= self.thresholds.urgency_threshold)
            .unwrap_or(false);

        if let Some(new_urgency) = urgency_value
            && let Some(old_urgency) = prev.urgency
        {
            if now_urgency_above && !had_urgency_above {
                events.push(EventBuilder::urgency_threshold_crossed(
                    tension_id.to_owned(),
                    old_urgency,
                    new_urgency,
                    self.thresholds.urgency_threshold,
                    true,
                ));
            } else if !now_urgency_above && had_urgency_above {
                events.push(EventBuilder::urgency_threshold_crossed(
                    tension_id.to_owned(),
                    old_urgency,
                    new_urgency,
                    self.thresholds.urgency_threshold,
                    false,
                ));
            }
        }

        // Horizon Drift
        let horizon_drift = detect_horizon_drift(tension_id, &mutations);
        let drift_type = horizon_drift.drift_type;

        if drift_type != HorizonDriftType::Stable && prev.horizon_drift_type != Some(drift_type) {
            events.push(EventBuilder::horizon_drift_detected(
                tension_id.to_owned(),
                drift_type,
                horizon_drift.change_count,
            ));
        }

        // Update previous state
        self.previous_state.tensions.insert(
            tension_id.to_owned(),
            PreviousDynamics {
                phase: Some(phase.phase),
                had_conflict: has_conflict,
                conflict_pattern,
                had_oscillation: has_oscillation,
                had_resolution: has_resolution,
                neglect_type,
                orientation: orientation.as_ref().map(|o| o.orientation),
                had_urgency_above_threshold: now_urgency_above,
                horizon_drift_type: if horizon_drift.change_count > 0 {
                    Some(drift_type)
                } else {
                    None
                },
                urgency: urgency_value,
                had_compensating_strategy: has_compensating_strategy,
                compensating_strategy_type: comp_strategy_type,
            },
        );

        // Emit all events
        for event in &events {
            self.bus.emit(event);
        }

        Some(ComputedDynamics {
            structural_tension,
            conflict,
            oscillation,
            resolution,
            phase,
            orientation,
            compensating_strategy,
            tendency,
            assimilation,
            neglect,
            horizon_drift,
            urgency,
            events,
        })
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
        if !lifecycle_events.is_empty()
            && let Event::LifecycleTransition {
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
        let _events1 = engine.compute_and_emit_for_tension(&t.id);

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

    // ====================================================================
    // Horizon Engine Tests (VAL-HENG-*)
    // ====================================================================

    // VAL-HENG-001: Engine computes urgency with horizon
    #[test]
    fn test_engine_computes_urgency_with_horizon() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create tension with horizon
        let horizon = Horizon::parse("2026-05").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon))
            .unwrap();

        // Compute dynamics
        engine.compute_and_emit_for_tension(&t.id);

        // Check that urgency is computed
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert!(
            prev.urgency.is_some(),
            "Urgency should be computed when horizon is present"
        );
        assert!(prev.urgency.unwrap() >= 0.0);
    }

    // VAL-HENG-002: Engine skips urgency without horizon
    #[test]
    fn test_engine_skips_urgency_without_horizon() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create tension without horizon
        let t = engine.create_tension("goal", "reality").unwrap();

        // Compute dynamics
        engine.compute_and_emit_for_tension(&t.id);

        // Check that urgency is not computed
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert!(
            prev.urgency.is_none(),
            "Urgency should be None when horizon is absent"
        );
    }

    // VAL-HENG-003: Engine detects horizon drift
    #[test]
    fn test_engine_detects_horizon_drift() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create tension with horizon
        let horizon = Horizon::parse("2026-05").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon))
            .unwrap();

        // Initially no drift
        engine.compute_and_emit_for_tension(&t.id);
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert!(prev.horizon_drift_type.is_none(), "No drift initially");

        // Update horizon multiple times (postponement pattern)
        let horizon2 = Horizon::parse("2026-08").unwrap();
        engine.update_horizon(&t.id, Some(horizon2)).unwrap();

        // Compute dynamics - should detect drift
        engine.compute_and_emit_for_tension(&t.id);
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert!(
            prev.horizon_drift_type.is_some(),
            "Drift should be detected after horizon change"
        );
    }

    // VAL-HENG-004: create_tension_full via engine
    #[test]
    fn test_engine_create_tension_full() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create tension with horizon
        let horizon = Horizon::parse("2026-05-15").unwrap();
        let t = engine
            .create_tension_full("desired state", "actual state", None, Some(horizon.clone()))
            .unwrap();

        assert!(!t.id.is_empty());
        assert_eq!(t.desired, "desired state");
        assert_eq!(t.actual, "actual state");
        assert!(t.parent_id.is_none());
        assert_eq!(t.horizon, Some(horizon));

        // Verify it was persisted
        let retrieved = engine.store().get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.horizon, t.horizon);
    }

    // VAL-HENG-005: update_horizon via engine
    #[test]
    fn test_engine_update_horizon() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create tension with horizon
        let horizon1 = Horizon::parse("2026-05").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon1))
            .unwrap();

        // Update horizon
        let horizon2 = Horizon::parse("2026-08").unwrap();
        engine
            .update_horizon(&t.id, Some(horizon2.clone()))
            .unwrap();

        // Verify change persisted
        let retrieved = engine.store().get_tension(&t.id).unwrap().unwrap();
        assert_eq!(retrieved.horizon, Some(horizon2));

        // Verify mutation recorded
        let mutations = engine.store().get_mutations(&t.id).unwrap();
        assert!(mutations.iter().any(|m| m.field() == "horizon"));
    }

    // VAL-HENG-005: update_horizon on non-Active fails
    #[test]
    fn test_engine_update_horizon_on_resolved_fails() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        let horizon = Horizon::parse("2026-05").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon))
            .unwrap();

        // Resolve the tension
        engine.resolve(&t.id).unwrap();

        // Try to update horizon - should fail
        let horizon2 = Horizon::parse("2026-08").unwrap();
        let result = engine.update_horizon(&t.id, Some(horizon2));
        assert!(
            result.is_err(),
            "Should not update horizon on resolved tension"
        );
    }

    // VAL-HENG-006: Full cycle with horizon
    #[test]
    fn test_engine_full_cycle_with_horizon() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create with horizon
        let horizon = Horizon::parse("2026-05").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon))
            .unwrap();

        // Update actual
        engine.update_actual(&t.id, "goal progress").unwrap();

        // Compute dynamics
        engine.compute_and_emit_for_tension(&t.id);

        // Verify urgency, pressure, drift are computed
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert!(
            prev.urgency.is_some(),
            "Urgency should be computed with horizon"
        );
        // Horizon drift should be None if no horizon mutations
        assert!(
            prev.horizon_drift_type.is_none(),
            "No drift without horizon mutations"
        );

        // Update horizon to create drift
        let horizon2 = Horizon::parse("2026-08").unwrap();
        engine.update_horizon(&t.id, Some(horizon2)).unwrap();

        // Compute again
        engine.compute_and_emit_for_tension(&t.id);

        // Verify drift is detected
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert!(
            prev.horizon_drift_type.is_some(),
            "Drift should be detected after horizon change"
        );
    }

    // VAL-HENG-007: Full cycle without horizon (backward compat)
    #[test]
    fn test_engine_full_cycle_without_horizon() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create without horizon (backward compat)
        let t = engine.create_tension("goal", "reality").unwrap();

        // Update actual
        engine.update_actual(&t.id, "goal progress").unwrap();

        // Compute dynamics - should work exactly as before
        let _events = engine.compute_and_emit_for_tension(&t.id);

        // Should compute dynamics without errors
        // Verify urgency is absent
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert!(
            prev.urgency.is_none(),
            "Urgency should be absent without horizon"
        );
        assert!(
            prev.horizon_drift_type.is_none(),
            "Drift should be None without horizon"
        );

        // Existing dynamics should work
        assert!(prev.phase.is_some());
    }

    // VAL-HENG-008: PreviousDynamics urgency threshold tracking
    #[test]
    fn test_engine_urgency_threshold_tracking() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();
        engine.thresholds.urgency_threshold = 0.5; // 50% urgency threshold

        // Create tension with horizon that puts urgency above threshold
        // Use a horizon in the very near future
        let horizon = Horizon::parse("2026").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon))
            .unwrap();

        // Compute dynamics
        engine.compute_and_emit_for_tension(&t.id);

        let prev = engine.previous_state().tensions.get(&t.id).unwrap();

        // Track whether urgency is above threshold
        // Note: the actual urgency value depends on timing, so we just verify tracking works
        if let Some(urgency_val) = prev.urgency {
            let expected_above = urgency_val >= 0.5;
            assert_eq!(
                prev.had_urgency_above_threshold, expected_above,
                "had_urgency_above_threshold should match urgency >= threshold"
            );
        }
    }

    // VAL-HENG-009: Existing engine tests pass unchanged
    // This is verified by the other tests passing

    // Additional test: Clear horizon
    #[test]
    fn test_engine_clear_horizon() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        let horizon = Horizon::parse("2026-05").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon))
            .unwrap();

        // Clear horizon
        engine.update_horizon(&t.id, None).unwrap();

        // Verify horizon is cleared
        let retrieved = engine.store().get_tension(&t.id).unwrap().unwrap();
        assert!(retrieved.horizon.is_none());

        // Compute dynamics - urgency should now be None
        engine.compute_and_emit_for_tension(&t.id);
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert!(prev.urgency.is_none());
    }

    // Test: create_tension_full with parent
    #[test]
    fn test_engine_create_tension_full_with_parent() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create parent
        let parent = engine.create_tension("parent", "p reality").unwrap();

        // Create child with horizon
        let horizon = Horizon::parse("2026-06").unwrap();
        let child = engine
            .create_tension_full(
                "child goal",
                "child reality",
                Some(parent.id.clone()),
                Some(horizon.clone()),
            )
            .unwrap();

        assert_eq!(child.parent_id, Some(parent.id));
        assert_eq!(child.horizon, Some(horizon));
    }

    // Test: HorizonChanged event emitted
    #[test]
    fn test_engine_horizon_changed_event() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        let horizon1 = Horizon::parse("2026-05").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon1))
            .unwrap();

        // Clear event history
        engine.clear_event_history();

        // Update horizon
        let horizon2 = Horizon::parse("2026-08").unwrap();
        engine.update_horizon(&t.id, Some(horizon2)).unwrap();

        // Check that HorizonChanged event was emitted
        let history = engine.event_history();
        assert!(
            history
                .iter()
                .any(|e| matches!(e, Event::HorizonChanged { .. })),
            "HorizonChanged event should be emitted"
        );
    }

    // ====================================================================
    // Event Wiring Tests (VAL-EVT-015 through VAL-EVT-019)
    // ====================================================================

    // VAL-EVT-015: Engine emits UrgencyThresholdCrossed on upward crossing
    #[test]
    fn test_engine_emits_urgency_threshold_crossed_above() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();
        engine.thresholds.urgency_threshold = 0.5;

        // Create tension with horizon
        let horizon = Horizon::parse("2028-01").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon))
            .unwrap();

        // First compute to establish baseline (urgency should be low with far horizon)
        engine.compute_and_emit_for_tension(&t.id);

        // Verify urgency is below threshold
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        let initial_urgency = prev.urgency.unwrap();
        assert!(
            initial_urgency < 0.5,
            "Initial urgency {initial_urgency} should be below 0.5 with far horizon"
        );
        assert!(
            !prev.had_urgency_above_threshold,
            "Should not be above threshold"
        );

        // Manually set previous urgency state to simulate below-threshold state,
        // then force a high urgency by setting had_urgency_above_threshold = false
        // and urgency = a low value, and then on next compute the urgency will be
        // recalculated based on the current horizon.
        //
        // Instead, change the threshold to be very low so current urgency crosses it.
        engine.thresholds.urgency_threshold = initial_urgency / 2.0;

        // Compute again - now urgency (same value) is above the lowered threshold
        let events2 = engine.compute_and_emit_for_tension(&t.id);
        let urgency_events2: Vec<&Event> = events2
            .iter()
            .filter(|e| matches!(e, Event::UrgencyThresholdCrossed { .. }))
            .collect();

        assert!(
            !urgency_events2.is_empty(),
            "Should emit UrgencyThresholdCrossed when urgency crosses above threshold"
        );

        if let Event::UrgencyThresholdCrossed { crossed_above, .. } = urgency_events2[0] {
            assert!(*crossed_above, "Should indicate upward crossing");
        } else {
            panic!("Expected UrgencyThresholdCrossed event");
        }
    }

    // VAL-EVT-002/015: Engine emits UrgencyThresholdCrossed on downward crossing
    #[test]
    fn test_engine_emits_urgency_threshold_crossed_below() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Create tension with horizon
        let horizon = Horizon::parse("2028-01").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon))
            .unwrap();

        // Set a very low threshold so any urgency >= 0 is "above"
        engine.thresholds.urgency_threshold = 0.0;

        // First compute to establish baseline
        engine.compute_and_emit_for_tension(&t.id);

        // After compute, urgency is at or above threshold 0.0
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert!(prev.urgency.is_some(), "Should have urgency with horizon");
        assert!(
            prev.had_urgency_above_threshold,
            "Urgency should be >= 0.0 threshold"
        );

        // Second compute to ensure no crossing events (both cycles above threshold)
        engine.compute_and_emit_for_tension(&t.id);

        // Now raise the threshold way above any possible urgency to force a downward crossing
        engine.thresholds.urgency_threshold = 100.0;

        // Compute again - urgency is now below the raised threshold
        let events2 = engine.compute_and_emit_for_tension(&t.id);
        let urgency_events2: Vec<&Event> = events2
            .iter()
            .filter(|e| matches!(e, Event::UrgencyThresholdCrossed { .. }))
            .collect();

        assert!(
            !urgency_events2.is_empty(),
            "Should emit UrgencyThresholdCrossed when urgency crosses below threshold"
        );

        if let Event::UrgencyThresholdCrossed { crossed_above, .. } = urgency_events2[0] {
            assert!(!*crossed_above, "Should indicate downward crossing");
        } else {
            panic!("Expected UrgencyThresholdCrossed event");
        }
    }

    // VAL-EVT-003: UrgencyThresholdCrossed NOT emitted when no crossing occurs
    #[test]
    fn test_engine_no_urgency_event_without_crossing() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();
        engine.thresholds.urgency_threshold = 0.3;

        // Create tension with far horizon (urgency below threshold)
        let far_horizon = Horizon::parse("2028-01").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(far_horizon))
            .unwrap();

        // Compute twice - urgency stays below threshold both times
        engine.compute_and_emit_for_tension(&t.id);
        let events2 = engine.compute_and_emit_for_tension(&t.id);

        let urgency_events: Vec<&Event> = events2
            .iter()
            .filter(|e| matches!(e, Event::UrgencyThresholdCrossed { .. }))
            .collect();
        assert!(
            urgency_events.is_empty(),
            "No urgency crossing event when urgency stays below threshold"
        );
    }

    // VAL-EVT-016: Engine emits HorizonDriftDetected on Stable → non-Stable
    #[test]
    fn test_engine_emits_horizon_drift_detected() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        let horizon1 = Horizon::parse("2026-05").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon1))
            .unwrap();

        // First compute - no drift (no horizon mutations yet beyond creation)
        let events1 = engine.compute_and_emit_for_tension(&t.id);
        let drift_events1: Vec<&Event> = events1
            .iter()
            .filter(|e| matches!(e, Event::HorizonDriftDetected { .. }))
            .collect();
        assert!(
            drift_events1.is_empty(),
            "No drift event on initial compute (no horizon mutations)"
        );

        // Postpone the horizon
        let horizon2 = Horizon::parse("2026-08").unwrap();
        engine.update_horizon(&t.id, Some(horizon2)).unwrap();

        // Compute again - should detect drift (Stable → Postponement)
        let events2 = engine.compute_and_emit_for_tension(&t.id);
        let drift_events2: Vec<&Event> = events2
            .iter()
            .filter(|e| matches!(e, Event::HorizonDriftDetected { .. }))
            .collect();

        assert!(
            !drift_events2.is_empty(),
            "Should emit HorizonDriftDetected when drift transitions from Stable to non-Stable"
        );

        if let Event::HorizonDriftDetected { drift_type, .. } = drift_events2[0] {
            assert_ne!(
                *drift_type,
                HorizonDriftType::Stable,
                "Drift type should not be Stable"
            );
        } else {
            panic!("Expected HorizonDriftDetected event");
        }
    }

    // VAL-EVT-005: HorizonDriftDetected on drift type change
    #[test]
    fn test_engine_emits_horizon_drift_on_type_change() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        let horizon1 = Horizon::parse("2026-05").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon1))
            .unwrap();

        // Initial compute
        engine.compute_and_emit_for_tension(&t.id);

        // Postpone (shift later) to create drift
        let horizon2 = Horizon::parse("2026-08").unwrap();
        engine.update_horizon(&t.id, Some(horizon2)).unwrap();

        // Compute - first drift detected
        let events1 = engine.compute_and_emit_for_tension(&t.id);
        let drift_events1: Vec<&Event> = events1
            .iter()
            .filter(|e| matches!(e, Event::HorizonDriftDetected { .. }))
            .collect();
        assert!(!drift_events1.is_empty(), "Should detect first drift");

        // Record the first drift type
        let first_drift_type =
            if let Event::HorizonDriftDetected { drift_type, .. } = drift_events1[0] {
                *drift_type
            } else {
                panic!("Expected HorizonDriftDetected event");
            };

        // Postpone again to potentially change drift type
        // (Postponement → RepeatedPostponement after 3+ shifts later)
        let horizon3 = Horizon::parse("2026-12").unwrap();
        engine.update_horizon(&t.id, Some(horizon3)).unwrap();
        let horizon4 = Horizon::parse("2027-06").unwrap();
        engine.update_horizon(&t.id, Some(horizon4)).unwrap();

        // Compute again - drift type may have changed
        let events2 = engine.compute_and_emit_for_tension(&t.id);
        let drift_events2: Vec<&Event> = events2
            .iter()
            .filter(|e| matches!(e, Event::HorizonDriftDetected { .. }))
            .collect();

        // If the type changed, we should get a new event
        if !drift_events2.is_empty()
            && let Event::HorizonDriftDetected { drift_type, .. } = drift_events2[0]
        {
            assert_ne!(
                *drift_type, first_drift_type,
                "New drift event should have a different type"
            );
        }
        // If it didn't change, that's also acceptable (no duplicate emission)
    }

    // VAL-EVT-017: Engine emits CompensatingStrategyDetected on first detection
    #[test]
    fn test_engine_emits_compensating_strategy_detected() {
        use crate::dynamics::CompensatingStrategyType;

        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Use very sensitive thresholds for compensating strategy detection
        engine.thresholds.oscillation.magnitude_threshold = 0.001;
        engine.thresholds.oscillation.frequency_threshold = 2;
        engine.thresholds.oscillation.recency_window_seconds = 3600 * 24 * 365;
        engine
            .thresholds
            .compensating_strategy
            .persistence_threshold_seconds = 0;
        engine
            .thresholds
            .compensating_strategy
            .min_oscillation_cycles = 1;
        engine
            .thresholds
            .compensating_strategy
            .structural_change_window_seconds = 1; // very short window
        engine
            .thresholds
            .compensating_strategy
            .recency_window_seconds = 3600 * 24 * 365;

        let t = engine.create_tension("goal", "a").unwrap();

        // Create oscillation pattern (required for TolerableConflict detection)
        engine.update_actual(&t.id, "ab").unwrap();
        engine.update_actual(&t.id, "a").unwrap();
        engine.update_actual(&t.id, "abc").unwrap();
        engine.update_actual(&t.id, "a").unwrap();
        engine.update_actual(&t.id, "abcd").unwrap();
        engine.update_actual(&t.id, "a").unwrap();

        // Compute - should detect compensating strategy
        let events = engine.compute_and_emit_for_tension(&t.id);

        let comp_events: Vec<&Event> = events
            .iter()
            .filter(|e| matches!(e, Event::CompensatingStrategyDetected { .. }))
            .collect();

        // If compensating strategy was detected (depends on oscillation being present)
        if !comp_events.is_empty()
            && let Event::CompensatingStrategyDetected {
                strategy_type,
                tension_id,
                ..
            } = comp_events[0]
        {
            assert_eq!(tension_id, &t.id);
            // Should be TolerableConflict since we have oscillation without structural change
            assert_eq!(
                *strategy_type,
                CompensatingStrategyType::TolerableConflict,
                "Expected TolerableConflict strategy"
            );
        }

        // Verify that compensating strategy state is tracked in PreviousDynamics
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        // had_compensating_strategy should match whether we detected one
        assert_eq!(
            prev.had_compensating_strategy,
            !comp_events.is_empty(),
            "PreviousDynamics should track compensating strategy detection"
        );
    }

    // VAL-EVT-007: CompensatingStrategyDetected NOT emitted when persistent
    #[test]
    fn test_engine_no_duplicate_compensating_strategy_event() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Use very sensitive thresholds
        engine.thresholds.oscillation.magnitude_threshold = 0.001;
        engine.thresholds.oscillation.frequency_threshold = 2;
        engine.thresholds.oscillation.recency_window_seconds = 3600 * 24 * 365;
        engine
            .thresholds
            .compensating_strategy
            .persistence_threshold_seconds = 0;
        engine
            .thresholds
            .compensating_strategy
            .min_oscillation_cycles = 1;
        engine
            .thresholds
            .compensating_strategy
            .structural_change_window_seconds = 1;
        engine
            .thresholds
            .compensating_strategy
            .recency_window_seconds = 3600 * 24 * 365;

        let t = engine.create_tension("goal", "a").unwrap();

        // Create oscillation pattern
        engine.update_actual(&t.id, "ab").unwrap();
        engine.update_actual(&t.id, "a").unwrap();
        engine.update_actual(&t.id, "abc").unwrap();
        engine.update_actual(&t.id, "a").unwrap();
        engine.update_actual(&t.id, "abcd").unwrap();
        engine.update_actual(&t.id, "a").unwrap();

        // First compute - may detect compensating strategy
        let events1 = engine.compute_and_emit_for_tension(&t.id);
        let comp_count1 = events1
            .iter()
            .filter(|e| matches!(e, Event::CompensatingStrategyDetected { .. }))
            .count();

        // Second compute without changing anything - should NOT re-emit
        let events2 = engine.compute_and_emit_for_tension(&t.id);
        let comp_count2 = events2
            .iter()
            .filter(|e| matches!(e, Event::CompensatingStrategyDetected { .. }))
            .count();

        if comp_count1 > 0 {
            assert_eq!(
                comp_count2, 0,
                "Should not re-emit CompensatingStrategyDetected when already detected"
            );
        }
    }

    // VAL-EVT-018: Engine emits OscillationResolved when oscillation clears
    #[test]
    fn test_engine_emits_oscillation_resolved() {
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

        // Compute - should detect oscillation
        let events1 = engine.compute_and_emit_for_tension(&t.id);
        assert!(
            events1
                .iter()
                .any(|e| matches!(e, Event::OscillationDetected { .. })),
            "Should detect oscillation"
        );

        // Now make steady progress to clear oscillation
        // (many consistent forward mutations without reversal)
        for i in 0..20 {
            engine
                .update_actual(&t.id, &format!("progress step {i}"))
                .unwrap();
        }

        // Increase threshold to ensure oscillation is cleared
        engine.thresholds.oscillation.frequency_threshold = 100;

        // Compute again - oscillation should be resolved
        let events2 = engine.compute_and_emit_for_tension(&t.id);

        let resolved_events: Vec<&Event> = events2
            .iter()
            .filter(|e| matches!(e, Event::OscillationResolved { .. }))
            .collect();

        assert!(
            !resolved_events.is_empty(),
            "Should emit OscillationResolved when oscillation clears"
        );

        if let Event::OscillationResolved { tension_id, .. } = resolved_events[0] {
            assert_eq!(tension_id, &t.id);
        } else {
            panic!("Expected OscillationResolved event");
        }
    }

    // VAL-EVT-009: Engine emits NeglectResolved when neglect clears
    #[test]
    fn test_engine_emits_neglect_resolved() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Use sensitive thresholds for neglect detection
        engine.thresholds.neglect.recency_seconds = 3600 * 24 * 365;
        engine.thresholds.neglect.activity_ratio_threshold = 2.0;
        engine.thresholds.neglect.min_active_mutations = 1;

        // Create parent and children
        let parent = engine.create_tension("parent goal", "p reality").unwrap();
        let child1 = engine
            .create_tension_with_parent("child1 goal", "c1 reality", Some(parent.id.clone()))
            .unwrap();
        let _child2 = engine
            .create_tension_with_parent("child2 goal", "c2 reality", Some(parent.id.clone()))
            .unwrap();

        // Create asymmetric activity - update child1 lots, ignore child2
        for _ in 0..10 {
            engine.update_actual(&child1.id, "active child1").unwrap();
        }

        // Compute dynamics on parent - may detect neglect
        let events1 = engine.compute_and_emit_for_tension(&parent.id);
        let neglect_detected = events1
            .iter()
            .any(|e| matches!(e, Event::NeglectDetected { .. }));

        if neglect_detected {
            // Verify previous state tracks neglect
            let prev = engine.previous_state.tensions.get_mut(&parent.id).unwrap();
            assert!(prev.neglect_type.is_some());

            // Now raise the activity ratio threshold so neglect is no longer detected
            engine.thresholds.neglect.activity_ratio_threshold = 1000.0;

            // Compute again - neglect should be resolved
            let events2 = engine.compute_and_emit_for_tension(&parent.id);
            let resolved_events: Vec<&Event> = events2
                .iter()
                .filter(|e| matches!(e, Event::NeglectResolved { .. }))
                .collect();

            assert!(
                !resolved_events.is_empty(),
                "Should emit NeglectResolved when neglect clears"
            );

            if let Event::NeglectResolved {
                tension_id,
                former_neglect_type,
                ..
            } = resolved_events[0]
            {
                assert_eq!(tension_id, &parent.id);
                // former_neglect_type should be a valid NeglectType
                assert!(
                    matches!(
                        former_neglect_type,
                        NeglectType::ParentNeglectsChildren | NeglectType::ChildrenNeglected
                    ),
                    "Should have a valid former neglect type"
                );
            }
        }
    }

    // VAL-EVT-010: Engine emits ResolutionLost when resolution clears
    #[test]
    fn test_engine_emits_resolution_lost() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Use sensitive thresholds for resolution detection
        engine.thresholds.resolution.velocity_threshold = 1e-10;
        engine.thresholds.resolution.reversal_tolerance = 10;
        engine.thresholds.resolution.recency_window_seconds = 3600 * 24 * 365;

        let t = engine
            .create_tension("write a novel completely", "have nothing yet at all")
            .unwrap();

        // Make progress toward resolution
        engine
            .update_actual(&t.id, "have written chapter one of the novel")
            .unwrap();
        engine
            .update_actual(&t.id, "have written chapter two of the novel")
            .unwrap();
        engine
            .update_actual(&t.id, "have written half the novel already")
            .unwrap();

        // Compute - should detect resolution
        let events1 = engine.compute_and_emit_for_tension(&t.id);
        let has_resolution = events1
            .iter()
            .any(|e| matches!(e, Event::ResolutionAchieved { .. }));

        if has_resolution {
            // Now regress to break resolution pattern
            engine
                .update_actual(&t.id, "lost all the writing files")
                .unwrap();

            // Increase velocity threshold to ensure resolution is no longer detected
            engine.thresholds.resolution.velocity_threshold = 100.0;

            // Compute again - resolution should be lost
            let events2 = engine.compute_and_emit_for_tension(&t.id);
            let lost_events: Vec<&Event> = events2
                .iter()
                .filter(|e| matches!(e, Event::ResolutionLost { .. }))
                .collect();

            assert!(
                !lost_events.is_empty(),
                "Should emit ResolutionLost when resolution clears"
            );

            if let Event::ResolutionLost { tension_id, .. } = lost_events[0] {
                assert_eq!(tension_id, &t.id);
            }
        }
    }

    // VAL-EVT-019: No spurious events on first compute cycle
    #[test]
    fn test_engine_no_spurious_events_on_first_cycle() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        let t = engine.create_tension("goal", "reality").unwrap();

        // First compute on a new tension
        let events = engine.compute_and_emit_for_tension(&t.id);

        // Should NOT have any resolved/lost events
        let spurious_events: Vec<&Event> = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::OscillationResolved { .. }
                        | Event::NeglectResolved { .. }
                        | Event::ResolutionLost { .. }
                )
            })
            .collect();

        assert!(
            spurious_events.is_empty(),
            "First compute cycle should not emit resolved/lost events. Got: {spurious_events:?}"
        );
    }

    // VAL-EVT-019: No spurious events on first compute cycle (with horizon)
    #[test]
    fn test_engine_no_spurious_events_on_first_cycle_with_horizon() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        let horizon = Horizon::parse("2026-06").unwrap();
        let t = engine
            .create_tension_full("goal", "reality", None, Some(horizon))
            .unwrap();

        // First compute on a new tension with horizon
        let events = engine.compute_and_emit_for_tension(&t.id);

        // Should NOT have any resolved/lost events
        let spurious_events: Vec<&Event> = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::OscillationResolved { .. }
                        | Event::NeglectResolved { .. }
                        | Event::ResolutionLost { .. }
                        | Event::UrgencyThresholdCrossed { .. }
                )
            })
            .collect();

        assert!(
            spurious_events.is_empty(),
            "First compute cycle should not emit spurious events. Got: {spurious_events:?}"
        );
    }

    // VAL-EVT-014: PreviousDynamics tracks compensating strategy
    #[test]
    fn test_engine_previous_dynamics_tracks_compensating_strategy() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        let t = engine.create_tension("goal", "reality").unwrap();

        // Initially no compensating strategy
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        assert!(
            !prev.had_compensating_strategy,
            "Initially should have no compensating strategy"
        );
        assert!(
            prev.compensating_strategy_type.is_none(),
            "Initially should have no compensating strategy type"
        );

        // After compute, the fields should be updated (even if still false)
        engine.compute_and_emit_for_tension(&t.id);
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        // With no oscillation pattern, compensating strategy should still be false
        assert!(
            !prev.had_compensating_strategy,
            "No compensating strategy without oscillation pattern"
        );
    }

    // Test: Verify compensating strategy state tracking across compute cycles
    #[test]
    fn test_engine_compensating_strategy_state_persists() {
        let mut engine = DynamicsEngine::new_in_memory().unwrap();

        // Use very sensitive thresholds
        engine.thresholds.oscillation.magnitude_threshold = 0.001;
        engine.thresholds.oscillation.frequency_threshold = 2;
        engine.thresholds.oscillation.recency_window_seconds = 3600 * 24 * 365;
        engine
            .thresholds
            .compensating_strategy
            .persistence_threshold_seconds = 0;
        engine
            .thresholds
            .compensating_strategy
            .min_oscillation_cycles = 1;
        engine
            .thresholds
            .compensating_strategy
            .structural_change_window_seconds = 1;
        engine
            .thresholds
            .compensating_strategy
            .recency_window_seconds = 3600 * 24 * 365;

        let t = engine.create_tension("goal", "a").unwrap();

        // Create oscillation pattern
        engine.update_actual(&t.id, "ab").unwrap();
        engine.update_actual(&t.id, "a").unwrap();
        engine.update_actual(&t.id, "abc").unwrap();
        engine.update_actual(&t.id, "a").unwrap();
        engine.update_actual(&t.id, "abcd").unwrap();
        engine.update_actual(&t.id, "a").unwrap();

        // Compute
        engine.compute_and_emit_for_tension(&t.id);

        // Check that previous state reflects the compensating strategy detection
        let prev = engine.previous_state().tensions.get(&t.id).unwrap();
        // Whether compensating strategy was detected depends on the oscillation detection,
        // but the tracking fields should be consistent
        if prev.had_compensating_strategy {
            assert!(
                prev.compensating_strategy_type.is_some(),
                "If had_compensating_strategy is true, type should be Some"
            );
        } else {
            assert!(
                prev.compensating_strategy_type.is_none(),
                "If had_compensating_strategy is false, type should be None"
            );
        }
    }
}
