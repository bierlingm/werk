//! Move command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::palette;
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    signals: Vec<palette::Palette>,
}

pub fn cmd_move(output: &Output, id: String, parent: Option<String>, dry_run: bool) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let (mut store, _hook_handle) = workspace.open_store_with_hooks()?;

    // Get all tensions for prefix resolution and forest building
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions.clone());

    // Resolve the tension to move
    let tension = resolver.resolve(&id)?;
    let tension_id = tension.id.clone();
    let tension_display = werk_shared::display_id(tension.short_code, &tension.id);
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

    // Dry run: validate and preview without mutating
    if dry_run {
        let result = MoveResult {
            id: tension_id.clone(),
            parent_id: new_parent_id.clone(),
            old_parent_id,
            signals: Vec::new(),
        };
        if output.is_structured() {
            output.print_structured(&result).map_err(WerkError::IoError)?;
        } else {
            match &new_parent_id {
                Some(pid) => {
                    println!("Would move tension {} under {}", &tension_display, werk_shared::display_id(
                        tensions.iter().find(|t| &t.id == pid).and_then(|t| t.short_code), pid
                    ));
                }
                None => {
                    println!("Would move tension {} to root", &tension_display);
                }
            }
            println!("No changes made.");
        }
        return Ok(());
    }

    // Perform the move via store
    let _ = store.begin_gesture(Some(&format!("move {}", &tension_id)));
    store
        .update_parent(&tension_id, new_parent_id.as_deref())
        .map_err(WerkError::SdError)?;
    store.end_gesture();

    // Human-readable output before palette
    if !output.is_structured() {
        match &new_parent_id {
            Some(pid) => {
                output
                    .success(&format!("Moved tension {} under {}", &tension_display, werk_shared::display_id(
                        tensions.iter().find(|t| &t.id == pid).and_then(|t| t.short_code), pid
                    )))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            None => {
                output
                    .success(&format!("Moved tension {} to root", &tension_display))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
        }
    }

    // Pathway palettes: check containment and sequencing in new parent context
    let mut signals = Vec::new();

    // Containment: does the moved tension's deadline violate the new parent's?
    if new_parent_id.is_some() {
        let containment = palette::check_containment_after_horizon(output, &mut store, &tension_id)?;
        signals.extend(containment);
    }

    // Sequencing: does the moved tension's position conflict with new siblings?
    if new_parent_id.is_some() {
        // Re-read tension to get current state after move
        let updated_tensions = store.list_tensions().map_err(WerkError::StoreError)?;
        let moved = updated_tensions.iter().find(|t| t.id == tension_id);
        if let Some(t) = moved {
            if t.position.is_some() {
                let sequencing = palette::check_sequencing_after_position(output, &mut store, &tension_id)?;
                signals.extend(sequencing);
            }
        }
    }

    if output.is_structured() {
        let result = MoveResult {
            id: tension_id.clone(),
            parent_id: new_parent_id.clone(),
            old_parent_id,
            signals,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    }

    Ok(())
}
