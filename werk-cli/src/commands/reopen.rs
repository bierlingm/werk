//! Reopen command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_shared::{Config, HookEvent, HookRunner};

/// JSON output structure for reopen command.
#[derive(Serialize)]
struct ReopenResult {
    id: String,
    status: String,
    old_status: String,
}

pub fn cmd_reopen(output: &Output, id: String) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record old status for output
    let old_status = tension.status;

    // Check if already active
    if old_status == sd_core::TensionStatus::Active {
        return Err(WerkError::InvalidInput(
            "tension is already active".to_string(),
        ));
    }

    // Hook infrastructure
    let hooks = Config::load(&workspace)
        .map(|c| HookRunner::from_config(&c))
        .unwrap_or_else(|_| HookRunner::noop());
    let event = HookEvent::status_change(&tension.id, &tension.desired, "Active");
    if !hooks.pre_mutation(&event) {
        return Err(WerkError::InvalidInput(
            "Blocked by pre_mutation hook".to_string(),
        ));
    }

    // Update status via store
    store
        .update_status(&tension.id, sd_core::TensionStatus::Active)
        .map_err(WerkError::SdError)?;

    hooks.post_mutation(&event);

    let result = ReopenResult {
        id: tension.id.clone(),
        status: "Active".to_string(),
        old_status: old_status.to_string(),
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!("Reopened tension {}", &tension.id))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Status: {} -> Active", old_status);
    }

    Ok(())
}
