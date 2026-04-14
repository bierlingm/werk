#![forbid(unsafe_code)]

// werk: Operative instrument for structural dynamics
//
// The practitioner's workspace. Practice, presence, oracle.
// Built on werk-core. Maximally opinionated.

pub mod commands;
pub mod editor;
pub mod hints;
pub mod mutation_echo;
pub mod output;
pub mod palette;
pub mod serialize;

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
pub use output::Output;
pub use werk_shared::Config;
pub use werk_shared::PrefixResolver;
pub use werk_shared::Workspace;
pub use werk_shared::error::WerkError;
