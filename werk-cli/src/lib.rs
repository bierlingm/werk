#![forbid(unsafe_code)]

// werk: Operative instrument for structural dynamics
//
// The practitioner's workspace. Practice, presence, oracle.
// Built on sd-core. Maximally opinionated.

pub mod agent_response;
pub mod commands;
pub mod dynamics;
pub mod editor;
pub mod error;
pub mod output;
pub mod prefix;
pub mod workspace;

pub use editor::edit_content;
pub use error::WerkError;
pub use output::Output;
pub use prefix::PrefixResolver;
pub use workspace::Workspace;
