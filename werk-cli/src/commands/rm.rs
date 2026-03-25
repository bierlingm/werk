//! Rm command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_shared::{Config, HookEvent, HookRunner};

/// JSON output structure for rm command.
#[derive(Serialize)]
struct RmResult {
    id: String,
    deleted: bool,
}

pub fn cmd_rm(output: &Output, id: String) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record the tension ID before deletion
    let tension_id = tension.id.clone();
    let tension_display = werk_shared::display_id(tension.short_code, &tension.id);
    let tension_desired = tension.desired.clone();

    // Hook infrastructure
    let hooks = Config::load(&workspace)
        .map(|c| HookRunner::from_config(&c))
        .unwrap_or_else(|_| HookRunner::noop());
    let event = HookEvent::mutation(&tension_id, &tension_desired, Some(&tension.actual), tension.parent_id.as_deref(), "deleted", None, "true");
    if !hooks.pre_mutation(&event) {
        return Err(WerkError::InvalidInput("Blocked by pre_mutation hook".to_string()));
    }

    // Delete via store (handles reparenting children to grandparent)
    let _ = store.begin_gesture(Some(&format!("delete {}", &tension_id)));
    store
        .delete_tension(&tension_id)
        .map_err(WerkError::SdError)?;
    store.end_gesture();

    hooks.post_mutation(&event);

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
        output
            .success(&format!("Deleted tension {}", &tension_display))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Desired: {}", &tension_desired);
    }

    Ok(())
}
