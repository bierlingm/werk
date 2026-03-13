//! Release command handler.

use crate::error::WerkError;
use crate::output::{ColorStyle, Output};
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::Mutation;
use serde::Serialize;

/// JSON output structure for release command.
#[derive(Serialize)]
struct ReleaseResult {
    id: String,
    status: String,
    reason: String,
}

pub fn cmd_release(output: &Output, id: String, reason: String) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve_interactive(&id)?;

    // Record old status for output
    let old_status = tension.status;

    // Check if already resolved/released
    if old_status != sd_core::TensionStatus::Active {
        return Err(WerkError::InvalidInput(format!(
            "cannot release tension with status {} (must be Active)",
            old_status
        )));
    }

    // Update status via store (handles validation and mutation recording)
    store
        .update_status(&tension.id, sd_core::TensionStatus::Released)
        .map_err(WerkError::SdError)?;

    // Record the release reason as a mutation
    store
        .record_mutation(&Mutation::new(
            tension.id.clone(),
            Utc::now(),
            "release_reason".to_owned(),
            None,
            reason.clone(),
        ))
        .map_err(WerkError::SdError)?;

    let result = ReleaseResult {
        id: tension.id.clone(),
        status: "Released".to_string(),
        reason: reason.clone(),
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, ColorStyle::Id);
        output
            .success(&format!("Released tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Status: {} -> {}",
            output.styled(&old_status.to_string(), ColorStyle::Muted),
            output.styled("Released", ColorStyle::Released)
        );
        println!("  Reason: {}", output.styled(&reason, ColorStyle::Muted));
    }

    Ok(())
}
