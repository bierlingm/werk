#![forbid(unsafe_code)]

// werk-shared: Shared types and logic for werk
//
// Extracted from werk-cli to enable reuse by TUI and other frontends.

pub mod batch_mutation;
pub mod config;
pub mod error;
pub mod hooks;
pub mod prefix;
pub mod util;
pub mod workspace;

pub use batch_mutation::BatchMutation;
pub use config::Config;
pub use hooks::{HookEvent, HookRunner};
pub use error::{ErrorCode, Result, WerkError};
pub use prefix::PrefixResolver;
pub use util::{display_id, relative_time, truncate};
pub use workspace::Workspace;
