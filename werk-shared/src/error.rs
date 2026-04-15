//! Error types for werk.
//!
//! Exit codes:
//! - 0: Success
//! - 1: User error (bad input, not found, invalid operation)
//! - 2: Internal error (unexpected failures)

use serde::Serialize;
use thiserror::Error;

/// Errors that can occur in werk operations.
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

    /// An error from werk-core.
    #[error("{0}")]
    CoreError(#[from] werk_core::CoreError),

    /// An error from the store.
    #[error("{0}")]
    StoreError(#[from] werk_core::StoreError),

    /// An error from the tree module.
    #[error("{0}")]
    TreeError(#[from] werk_core::TreeError),
}

/// Error codes for JSON error output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(non_camel_case_types)]
pub enum ErrorCode {
    /// Resource not found.
    NOT_FOUND,
    /// Invalid input from user.
    INVALID_INPUT,
    /// Ambiguous identifier.
    AMBIGUOUS,
    /// No workspace found.
    NO_WORKSPACE,
    /// Permission denied.
    PERMISSION_DENIED,
    /// I/O error.
    IO_ERROR,
    /// Configuration error.
    CONFIG_ERROR,
    /// Internal error.
    INTERNAL_ERROR,
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
            WerkError::CoreError(_) => 2,
            WerkError::StoreError(_) => 2,
            WerkError::TreeError(_) => 2,
        }
    }

    /// Returns the error code for JSON error output.
    pub fn error_code(&self) -> ErrorCode {
        match self {
            WerkError::NoWorkspace => ErrorCode::NO_WORKSPACE,
            WerkError::TensionNotFound(_) => ErrorCode::NOT_FOUND,
            WerkError::PrefixTooShort { .. } => ErrorCode::INVALID_INPUT,
            WerkError::AmbiguousPrefix { .. } => ErrorCode::AMBIGUOUS,
            WerkError::InvalidInput(_) => ErrorCode::INVALID_INPUT,
            WerkError::ConfigError(_) => ErrorCode::CONFIG_ERROR,
            WerkError::PermissionDenied(_) => ErrorCode::PERMISSION_DENIED,
            WerkError::IoError(_) => ErrorCode::IO_ERROR,
            WerkError::CoreError(_) => ErrorCode::INTERNAL_ERROR,
            WerkError::StoreError(_) => ErrorCode::INTERNAL_ERROR,
            WerkError::TreeError(_) => ErrorCode::INTERNAL_ERROR,
        }
    }

    /// Create a no workspace error with path context.
    ///
    /// Path arguments are accepted for caller ergonomics but not yet threaded
    /// into the message — `NoWorkspace`'s Display already describes the situation.
    pub fn no_workspace_with_context(
        cwd: &std::path::Path,
        home: Option<&std::path::Path>,
    ) -> Self {
        let _ = (cwd, home);
        WerkError::NoWorkspace
    }
}

/// Result type alias for werk operations.
pub type Result<T> = std::result::Result<T, WerkError>;
