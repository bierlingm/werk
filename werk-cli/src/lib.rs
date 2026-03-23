#![forbid(unsafe_code)]

// werk: Operative instrument for structural dynamics
//
// The practitioner's workspace. Practice, presence, oracle.
// Built on sd-core. Maximally opinionated.

pub mod agent_response;
pub mod commands;
pub mod serialize;
pub mod editor;
pub mod output;

// Re-export shared types from werk-shared for backward compatibility
pub mod error {
    pub use werk_shared::error::*;
}

pub mod workspace {
    pub use werk_shared::workspace::*;
}

pub mod prefix {
    pub use werk_shared::prefix::*;
}

pub use editor::edit_content;
pub use werk_shared::error::WerkError;
pub use werk_shared::Config;
pub use output::Output;
pub use werk_shared::PrefixResolver;
pub use werk_shared::Workspace;
