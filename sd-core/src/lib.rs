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

pub mod mutation;
pub mod store;
pub mod tension;
pub mod tree;

// Re-export commonly used types for convenience
pub use mutation::{Mutation, ReconstructedTension, ReplayError, replay_mutations};
pub use store::{Store, StoreError};
pub use tension::{SdError, Tension, TensionStatus};
pub use tree::{Forest, Node, TreeError};

// Future modules (not yet implemented):
// pub mod dynamics;
// pub mod events;
