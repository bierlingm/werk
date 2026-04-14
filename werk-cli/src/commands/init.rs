//! Init command handler.

use crate::error::WerkError;
use crate::output::Output;
use serde::Serialize;
use std::path::PathBuf;

/// JSON output structure for init command.
#[derive(Serialize)]
struct InitResult {
    path: String,
    created: bool,
}

pub fn cmd_init(output: &Output, global: bool) -> Result<(), WerkError> {
    let cwd = std::env::current_dir()
        .map_err(|e| WerkError::IoError(format!("failed to get current directory: {}", e)))?;

    // Determine target path
    let target_path: PathBuf = if global {
        dirs::home_dir()
            .ok_or_else(|| WerkError::IoError("cannot determine home directory".to_string()))?
    } else {
        cwd.clone()
    };

    // Check if workspace already exists
    let werk_dir = target_path.join(".werk");
    let db_path = werk_dir.join("werk.db");
    let already_exists = db_path.exists();

    // Initialize the store (this creates .werk/ and werk.db)
    // Store::init is idempotent - it won't overwrite existing data
    let _store = werk_core::Store::init(&target_path)?;

    let result = InitResult {
        path: werk_dir.to_string_lossy().to_string(),
        created: !already_exists,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        let message = if already_exists {
            format!("Workspace already initialized at {}", werk_dir.display())
        } else {
            format!("Workspace initialized at {}", werk_dir.display())
        };
        output
            .success(&message)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    Ok(())
}
