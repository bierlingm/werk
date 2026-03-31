#![forbid(unsafe_code)]

// sd-core: Core computation and storage layer for werk.
//
// Provides: tension model, tree/forest, store, temporal computations,
// urgency, horizon drift, mutations, events, projections.
//
// This crate has zero instrument dependencies. Instruments subscribe
// and react to events.

pub mod engine;
pub mod events;
pub mod frontier;
pub mod graph;
pub mod horizon;
pub mod mutation;
pub mod projection;
pub mod search;
pub mod store;
pub mod temporal;
pub mod tension;
pub mod tree;

// Re-export commonly used types
pub use frontier::{ClosureProgress, Frontier, FrontierStep, compute_frontier};
pub use engine::Engine;
pub use events::{Event, EventBuilder, EventBus, SubscriptionHandle};
pub use horizon::{Horizon, HorizonKind, HorizonParseError};
pub use mutation::{Mutation, ReconstructedTension, ReplayError, replay_mutations};
pub use projection::{
    FieldProjection, MutationPattern, ProjectionHorizon, ProjectionThresholds, TensionProjection,
    Trajectory, TrajectoryBuckets, UrgencyCollision, estimate_time_to_resolution,
    extract_mutation_pattern, project_field, project_frequency_at, project_gap_at, project_tension,
};
pub use store::{EpochRecord, Store, StoreError};
pub use temporal::{
    ContainmentViolation, CriticalPath, HorizonDrift, HorizonDriftType, ImpliedWindow,
    SequencingPressure, TemporalSignals, Urgency,
    compute_implied_windows, compute_temporal_signals, compute_urgency,
    detect_containment_violations, detect_critical_path, detect_critical_path_recursive,
    detect_horizon_drift, detect_sequencing_pressure, gap_magnitude,
};
pub use tension::{SdError, Tension, TensionStatus};
pub use graph::{
    ComputationWitness, FieldStructuralSignals, StructuralSignals, compute_structural_signals,
};
pub use search::{SearchHit, SearchIndex};
pub use tree::{Forest, Node, TreeError};
