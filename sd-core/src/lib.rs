// sd-core: Structural Dynamics Grammar
//
// A faithful computational model of Robert Fritz's structural dynamics.
// The primitive is structural tension — the gap between a desired state
// and current reality. Everything else is computed from tensions and
// their mutation histories.
//
// This crate has zero instrument dependencies. It computes dynamics
// and emits events. Instruments subscribe and react.

pub mod tension;
pub mod mutation;
pub mod dynamics;
pub mod events;
pub mod store;
pub mod tree;
