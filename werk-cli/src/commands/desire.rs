//! Desire command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_shared::{Config, HookEvent, HookRunner};

/// JSON output structure for desire command.
#[derive(Serialize)]
struct DesireResult {
    id: String,
    desired: String,
    old_desired: String,
}

pub fn cmd_desire(output: &Output, id: String, value: Option<String>) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get the new value - either from argument or editor
    let new_value = match value {
        Some(v) => v,
        None => {
            // Open editor with current desired
            let edited = crate::edit_content(&tension.desired)?;
            match edited {
                Some(v) => v,
                None => {
                    // Editor returned no change - nothing to do
                    if output.is_structured() {
                        let result = DesireResult {
                            id: tension.id.clone(),
                            desired: tension.desired.clone(),
                            old_desired: tension.desired.clone(),
                        };
                        output
                            .print_structured(&result)
                            .map_err(WerkError::IoError)?;
                    } else {
                        output
                            .info("No changes made (editor cancelled or content unchanged)")
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                    return Ok(());
                }
            }
        }
    };

    // Validate non-empty
    if new_value.is_empty() {
        return Err(WerkError::InvalidInput(
            "desired state cannot be empty".to_string(),
        ));
    }

    // Record old value for output
    let old_desired = tension.desired.clone();

    // Hook infrastructure
    let hooks = Config::load(&workspace)
        .map(|c| HookRunner::from_config(&c))
        .unwrap_or_else(|_| HookRunner::noop());
    let event = HookEvent::mutation(&tension.id, &tension.desired, "desired", Some(&old_desired), &new_value);
    if !hooks.pre_mutation(&event) {
        return Err(WerkError::InvalidInput("Blocked by pre_mutation hook".to_string()));
    }

    // Update via store (this handles status validation and mutation recording)
    let _ = store.begin_gesture(Some(&format!("update desire {}", &tension.id)));
    store
        .update_desired(&tension.id, &new_value)
        .map_err(WerkError::SdError)?;
    store.end_gesture();

    hooks.post_mutation(&event);

    let result = DesireResult {
        id: tension.id.clone(),
        desired: new_value,
        old_desired,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        output
            .success(&format!("Updated desired for tension {}", werk_shared::display_id(tension.short_code, &tension.id)))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Old:  {}", &result.old_desired);
        println!("  New:  {}", &result.desired);
    }

    Ok(())
}
