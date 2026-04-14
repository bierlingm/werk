//! Release command handler.

use crate::error::WerkError;
use crate::mutation_echo;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use serde::Serialize;
use werk_core::Mutation;
use werk_shared::HookEvent;

/// JSON output structure for release command.
#[derive(Serialize)]
struct ReleaseResult {
    id: String,
    status: String,
    reason: String,
}

pub fn cmd_release(
    output: &Output,
    id: String,
    reason: String,
    show_after: bool,
) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let (mut store, hook_handle) = workspace.open_store_with_hooks()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record old status for output
    let old_status = tension.status;

    // Check if already resolved/released
    if old_status != werk_core::TensionStatus::Active {
        return Err(WerkError::InvalidInput(format!(
            "cannot release tension with status {} (must be Active)",
            old_status
        )));
    }

    // Pre-hook check
    let event = HookEvent::status_change(
        &tension.id,
        &tension.desired,
        Some(&tension.actual),
        tension.parent_id.as_deref(),
        "Released",
    );
    if !hook_handle.runner.pre_mutation(&event) {
        return Err(WerkError::InvalidInput(
            "Blocked by pre_mutation hook".to_string(),
        ));
    }

    // Update status via store (handles validation and mutation recording)
    let _ = store.begin_gesture(Some(&format!("release {}", &tension.id)));
    store
        .update_status(&tension.id, werk_core::TensionStatus::Released)
        .map_err(WerkError::CoreError)?;
    store.end_gesture();
    // Post-hooks fire automatically via the HookBridge

    // Record the release reason as a mutation
    store
        .record_mutation(&Mutation::new(
            tension.id.clone(),
            Utc::now(),
            "release_reason".to_owned(),
            None,
            reason.clone(),
        ))
        .map_err(WerkError::CoreError)?;

    let result = ReleaseResult {
        id: tension.id.clone(),
        status: "Released".to_string(),
        reason: reason.clone(),
    };

    // Count active children for context
    let all_tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let active_children = all_tensions
        .iter()
        .filter(|t| {
            t.parent_id.as_deref() == Some(&tension.id)
                && t.status == werk_core::TensionStatus::Active
        })
        .count();

    if output.is_structured() {
        let mut val =
            serde_json::to_value(&result).map_err(|e| WerkError::IoError(e.to_string()))?;
        if show_after {
            val["show"] = mutation_echo::build_json_echo(&store, &tension.id)?;
        }
        let json =
            serde_json::to_string_pretty(&val).map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("{}", json);
    } else {
        output
            .success(&format!(
                "Released tension {}",
                werk_shared::display_id(tension.short_code, &tension.id)
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Status: {} -> Released", old_status);
        println!("  Reason: {}", &reason);
        if active_children > 0 {
            let noun = if active_children == 1 {
                "child"
            } else {
                "children"
            };
            println!("  ({} active {} still open)", active_children, noun);
        }
        mutation_echo::print_human_echo(&store, &output.palette(), &tension.id)?;
    }

    Ok(())
}
