#![forbid(unsafe_code)]

// sd-core: Structural Dynamics Grammar
//
// A faithful computational model of Robert Fritz's structural dynamics.
// The primitive is structural tension — the gap between a desired state
// and current reality. Everything else is computed from tensions and
// their mutation histories.
//
// This crate has zero instrument dependencies. It computes dynamics
// and emits events. Instruments subscribe and react.

pub mod dynamics;
pub mod engine;
pub mod events;
pub mod horizon;
pub mod mutation;
pub mod store;
pub mod tension;
pub mod tree;

// Re-export commonly used types for convenience
pub use dynamics::{
    // Secondary dynamics
    AssimilationDepth,
    AssimilationDepthResult,
    AssimilationDepthThresholds,
    AssimilationEvidence,
    CompensatingStrategy,
    CompensatingStrategyThresholds,
    CompensatingStrategyType,
    // Core dynamics
    Conflict,
    ConflictPattern,
    ConflictThresholds,
    // Lifecycle and orientation
    CreativeCyclePhase,
    CreativeCyclePhaseResult,
    // Horizon dynamics
    HorizonDrift,
    HorizonDriftType,
    LifecycleThresholds,
    Neglect,
    NeglectThresholds,
    NeglectType,
    Orientation,
    OrientationEvidence,
    OrientationResult,
    OrientationThresholds,
    Oscillation,
    OscillationThresholds,
    PhaseEvidence,
    Resolution,
    ResolutionThresholds,
    ResolutionTrend,
    StructuralTendency,
    StructuralTendencyResult,
    StructuralTension,
    Urgency,
    // Functions
    classify_creative_cycle_phase,
    classify_orientation,
    compute_structural_tension,
    compute_temporal_pressure,
    compute_urgency,
    detect_compensating_strategy,
    detect_horizon_drift,
    detect_neglect,
    detect_oscillation,
    detect_resolution,
    detect_structural_conflict,
    measure_assimilation_depth,
    predict_structural_tendency,
};
pub use engine::{DynamicsEngine, DynamicsThresholds, PreviousDynamics, PreviousState};
pub use events::{Event, EventBuilder, EventBus, SubscriptionHandle};
pub use horizon::{Horizon, HorizonKind, HorizonParseError};
pub use mutation::{Mutation, ReconstructedTension, ReplayError, replay_mutations};
pub use store::{Store, StoreError};
pub use tension::{SdError, Tension, TensionStatus};
pub use tree::{Forest, Node, TreeError};
