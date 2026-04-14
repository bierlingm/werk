//! Reopen command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use serde::Serialize;
use werk_core::Mutation;
use werk_shared::HookEvent;

/// JSON output structure for reopen command.
#[derive(Serialize)]
struct ReopenResult {
    id: String,
    status: String,
    old_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

pub fn cmd_reopen(output: &Output, id: String, reason: Option<String>) -> Result<(), WerkError> {
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

    // Check if already active
    if old_status == werk_core::TensionStatus::Active {
        return Err(WerkError::InvalidInput(
            "tension is already active".to_string(),
        ));
    }

    // Pre-hook check
    let event = HookEvent::status_change(
        &tension.id,
        &tension.desired,
        Some(&tension.actual),
        tension.parent_id.as_deref(),
        "Active",
    );
    if !hook_handle.runner.pre_mutation(&event) {
        return Err(WerkError::InvalidInput(
            "Blocked by pre_mutation hook".to_string(),
        ));
    }

    // Update status via store
    let _ = store.begin_gesture(Some(&format!("reopen {}", &tension.id)));
    store
        .update_status(&tension.id, werk_core::TensionStatus::Active)
        .map_err(WerkError::CoreError)?;

    // Record the reopen reason as a mutation
    if let Some(ref reason) = reason {
        store
            .record_mutation(&Mutation::new(
                tension.id.clone(),
                Utc::now(),
                "reopen_reason".to_owned(),
                None,
                reason.clone(),
            ))
            .map_err(WerkError::CoreError)?;
    }
    store.end_gesture();
    // Post-hooks fire automatically via the HookBridge

    let result = ReopenResult {
        id: tension.id.clone(),
        status: "Active".to_string(),
        old_status: old_status.to_string(),
        reason: reason.clone(),
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!(
                "Reopened tension {}",
                werk_shared::display_id(tension.short_code, &tension.id)
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Status: {} -> Active", old_status);
        if let Some(ref reason) = reason {
            println!("  Reason: {}", reason);
        }
    }

    Ok(())
}
