//! Nuke command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use serde::Serialize;
use std::path::PathBuf;

/// JSON output structure for nuke command.
#[derive(Serialize)]
struct NukeResult {
    path: String,
    deleted: bool,
}

pub fn cmd_nuke(output: &Output, confirm: bool, global: bool) -> Result<(), WerkError> {
    // Determine target path
    let werk_dir: PathBuf = if global {
        let home = dirs::home_dir()
            .ok_or_else(|| WerkError::IoError("cannot determine home directory".to_string()))?;
        home.join(".werk")
    } else {
        // Try to discover workspace first
        match Workspace::discover() {
            Ok(ws) => ws.werk_dir().to_path_buf(),
            Err(_) => {
                // If no workspace found, use current directory's .werk
                let cwd = std::env::current_dir().map_err(|e| {
                    WerkError::IoError(format!("failed to get current directory: {}", e))
                })?;
                cwd.join(".werk")
            }
        }
    };

    // Check if the directory exists
    if !werk_dir.exists() {
        return Err(WerkError::InvalidInput(format!(
            "No .werk directory found at {}",
            werk_dir.display()
        )));
    }

    // Count tensions for the preview message
    let tension_count = if !global {
        Workspace::discover()
            .ok()
            .and_then(|ws| ws.open_store().ok())
            .and_then(|s| s.list_tensions().ok())
            .map(|t| t.len())
    } else {
        None
    };

    // If not confirmed, just show what would be deleted
    if !confirm {
        if output.is_structured() {
            let result = NukeResult {
                path: werk_dir.to_string_lossy().to_string(),
                deleted: false,
            };
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            output
                .info(&format!("Would delete: {}", werk_dir.display()))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
            if let Some(count) = tension_count {
                println!("  {} tension(s) will be permanently lost.", count);
            }
            // R-014: .werk/backups/ is destroyed by nuke. Point the user
            // at the read-only escape hatch before they --confirm.
            let backups_dir = werk_dir.join("backups");
            if backups_dir.exists() {
                println!(
                    "\n  {} also contains rotating DB backups that will be lost.",
                    backups_dir.display()
                );
                println!(
                    "  To copy them somewhere safe first, run:  werk doctor evacuate-backups"
                );
            }
            println!("\nPass --confirm to proceed with deletion.");
        }
        return Ok(());
    }

    // Delete the entire .werk directory
    std::fs::remove_dir_all(&werk_dir).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            WerkError::PermissionDenied(format!("{}", werk_dir.display()))
        } else {
            WerkError::IoError(format!("failed to delete {}: {}", werk_dir.display(), e))
        }
    })?;

    let result = NukeResult {
        path: werk_dir.to_string_lossy().to_string(),
        deleted: true,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!("Deleted workspace: {}", werk_dir.display()))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    Ok(())
}
