//! Rm command handler.

use crate::error::WerkError;
use crate::output::{ColorStyle, Output};
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;

/// JSON output structure for rm command.
#[derive(Serialize)]
struct RmResult {
    id: String,
    deleted: bool,
}

pub fn cmd_rm(output: &Output, id: String) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve_interactive(&id)?;

    // Record the tension ID before deletion
    let tension_id = tension.id.clone();
    let tension_desired = tension.desired.clone();

    // Delete via store (handles reparenting children to grandparent)
    store
        .delete_tension(&tension_id)
        .map_err(WerkError::SdError)?;

    let result = RmResult {
        id: tension_id.clone(),
        deleted: true,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension_id, ColorStyle::Id);
        output
            .success(&format!("Deleted tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Desired: {}",
            output.styled(&tension_desired, ColorStyle::Muted)
        );
    }

    Ok(())
}
