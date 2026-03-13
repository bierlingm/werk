//! Move command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use sd_core::Forest;
use serde::Serialize;

/// JSON output structure for move command.
#[derive(Serialize)]
struct MoveResult {
    id: String,
    parent_id: Option<String>,
    old_parent_id: Option<String>,
}

pub fn cmd_move(output: &Output, id: String, parent: Option<String>) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution and forest building
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions.clone());

    // Resolve the tension to move
    let tension = resolver.resolve(&id)?;
    let tension_id = tension.id.clone();
    let old_parent_id = tension.parent_id.clone();

    // Resolve the new parent if provided
    let new_parent_id = if let Some(parent_prefix) = parent {
        // Prevent moving to self
        let parent_tension = resolver.resolve(&parent_prefix)?;
        if parent_tension.id == tension_id {
            return Err(WerkError::InvalidInput(
                "cannot move tension to itself".to_string(),
            ));
        }

        // Check for cycles: new parent cannot be a descendant of the tension being moved
        let forest = Forest::from_tensions(tensions.clone())
            .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

        if let Some(descendants) = forest.descendants(&tension_id) {
            let descendant_ids: std::collections::HashSet<_> =
                descendants.iter().map(|n| n.id()).collect();

            if descendant_ids.contains(parent_tension.id.as_str()) {
                return Err(WerkError::InvalidInput(
                    "cannot move tension under its descendant (would create cycle)".to_string(),
                ));
            }
        }

        Some(parent_tension.id.clone())
    } else {
        None
    };

    // Perform the move via store
    store
        .update_parent(&tension_id, new_parent_id.as_deref())
        .map_err(WerkError::SdError)?;

    let result = MoveResult {
        id: tension_id.clone(),
        parent_id: new_parent_id.clone(),
        old_parent_id,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        match &new_parent_id {
            Some(pid) => {
                output
                    .success(&format!("Moved tension {} under {}", &tension_id, pid))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            None => {
                output
                    .success(&format!("Moved tension {} to root", &tension_id))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
        }
    }

    Ok(())
}
