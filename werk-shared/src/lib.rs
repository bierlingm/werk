#![forbid(unsafe_code)]

// werk-shared: Shared types and logic for werk
//
// Extracted from werk-cli to enable reuse by TUI and other frontends.

pub mod agent_response;
pub mod config;
pub mod error;
pub mod prefix;
pub mod util;
pub mod workspace;

pub use agent_response::{AgentMutation, StructuredResponse};
pub use config::Config;
pub use error::{ErrorCode, Result, WerkError};
pub use prefix::PrefixResolver;
pub use util::{relative_time, truncate};
pub use workspace::Workspace;
