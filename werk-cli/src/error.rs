//! Error types for werk-cli.
//!
//! Exit codes:
//! - 0: Success
//! - 1: User error (bad input, not found, invalid operation)
//! - 2: Internal error (unexpected failures)

use thiserror::Error;

/// Errors that can occur in werk-cli operations.
#[derive(Debug, Error)]
pub enum WerkError {
    /// No workspace found (no .werk/ in ancestor directories and no ~/.werk/)
    #[error("no workspace found: no .werk/ directory in current or ancestor directories, and no ~/.werk/ exists")]
    NoWorkspace,

    /// Tension not found by ID or prefix.
    #[error("tension not found: {0}")]
    TensionNotFound(String),

    /// Prefix is too short (minimum 4 characters required).
    #[error("prefix too short: '{prefix}' is {len} characters, need at least 4")]
    PrefixTooShort {
        /// The prefix that was provided.
        prefix: String,
        /// Length of the prefix.
        len: usize,
    },

    /// Prefix is ambiguous (matches multiple tensions).
    #[error("ambiguous prefix '{prefix}' matches multiple tensions:\n{matches}")]
    AmbiguousPrefix {
        /// The ambiguous prefix.
        prefix: String,
        /// Formatted list of matching tensions.
        matches: String,
    },

    /// Invalid input from user.
    #[error("{0}")]
    InvalidInput(String),

    /// Permission denied error.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(String),

    /// Configuration error.
    #[error("config error: {0}")]
    ConfigError(String),

    /// An error from sd-core.
    #[error("{0}")]
    SdError(#[from] sd_core::SdError),

    /// An error from the store.
    #[error("{0}")]
    StoreError(#[from] sd_core::StoreError),

    /// An error from the tree module.
    #[error("{0}")]
    TreeError(#[from] sd_core::TreeError),
}

impl WerkError {
    /// Returns the exit code for this error.
    ///
    /// - 0: Success (not an error)
    /// - 1: User error (recoverable, bad input)
    /// - 2: Internal error (unexpected failure)
    pub fn exit_code(&self) -> i32 {
        match self {
            // User errors: the user made a mistake or something expected didn't exist
            WerkError::NoWorkspace => 1,
            WerkError::TensionNotFound(_) => 1,
            WerkError::PrefixTooShort { .. } => 1,
            WerkError::AmbiguousPrefix { .. } => 1,
            WerkError::InvalidInput(_) => 1,
            WerkError::ConfigError(_) => 1,

            // Internal errors: unexpected failures
            WerkError::PermissionDenied(_) => 2,
            WerkError::IoError(_) => 2,
            WerkError::SdError(_) => 2,
            WerkError::StoreError(_) => 2,
            WerkError::TreeError(_) => 2,
        }
    }

    /// Create a no workspace error with path context.
    pub fn no_workspace_with_context(
        cwd: &std::path::Path,
        home: Option<&std::path::Path>,
    ) -> Self {
        // For now, just return the basic error. The message already explains the situation.
        let _ = (cwd, home);
        WerkError::NoWorkspace
    }
}

/// Result type alias for werk-cli operations.
pub type Result<T> = std::result::Result<T, WerkError>;
