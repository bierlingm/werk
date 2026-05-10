//! Rm command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_shared::HookEvent;

/// JSON output structure for rm command.
#[derive(Serialize)]
struct RmResult {
    id: String,
    deleted: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children_reparented: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    dry_run: bool,
}

pub fn cmd_rm(output: &Output, id: String, dry_run: bool) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let (mut store, hook_handle) = workspace.open_store_with_hooks()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions.clone());

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record the tension ID before deletion
    let tension_id = tension.id.clone();
    let tension_display = werk_shared::display_id(tension.short_code, &tension.id);
    let tension_desired = tension.desired.clone();

    // Find children that would be reparented
    let children: Vec<_> = tensions
        .iter()
        .filter(|t| t.parent_id.as_deref() == Some(&tension_id))
        .collect();
    let children_ids: Vec<String> = children
        .iter()
        .map(|c| werk_shared::display_id(c.short_code, &c.id))
        .collect();

    // Dry run: show what would happen
    if dry_run {
        let result = RmResult {
            id: tension_id.clone(),
            deleted: false,
            children_reparented: children_ids.clone(),
            dry_run: true,
        };
        if output.is_structured() {
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            println!("Would delete tension {}", &tension_display);
            println!("  Desired: {}", &tension_desired);
            if !children_ids.is_empty() {
                println!(
                    "  Children reparented to grandparent: {}",
                    children_ids.join(", ")
                );
            }
            println!("No changes made.");
        }
        return Ok(());
    }

    // Pre-hook check
    let event = HookEvent::mutation(
        &tension_id,
        &tension_desired,
        Some(&tension.actual),
        tension.parent_id.as_deref(),
        "deleted",
        None,
        "true",
    );
    if !hook_handle.runner.pre_mutation(&event) {
        return Err(WerkError::InvalidInput(
            "Blocked by pre_mutation hook".to_string(),
        ));
    }

    // Delete via store (handles reparenting children to grandparent)
    let _ = store.begin_gesture(Some(&format!("delete {}", &tension_id)));
    store
        .delete_tension(&tension_id)
        .map_err(WerkError::CoreError)?;
    store.end_gesture();
    // Post-hooks fire automatically via the HookBridge

    let result = RmResult {
        id: tension_id.clone(),
        deleted: true,
        children_reparented: children_ids,
        dry_run: false,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        output
            .success(&format!("Deleted tension {}", &tension_display))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Desired: {}", &tension_desired);
    }

    Ok(())
}
