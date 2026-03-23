//! Reality command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_shared::{Config, HookEvent, HookRunner};

/// JSON output structure for reality command.
#[derive(Serialize)]
struct RealityResult {
    id: String,
    actual: String,
    old_actual: String,
}

pub fn cmd_reality(output: &Output, id: String, value: Option<String>) -> Result<(), WerkError> {
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
            // Open editor with current actual
            let edited = crate::edit_content(&tension.actual)?;
            match edited {
                Some(v) => v,
                None => {
                    // Editor returned no change - nothing to do
                    if output.is_structured() {
                        let result = RealityResult {
                            id: tension.id.clone(),
                            actual: tension.actual.clone(),
                            old_actual: tension.actual.clone(),
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
            "actual state cannot be empty".to_string(),
        ));
    }

    // Record old value for output
    let old_actual = tension.actual.clone();

    // Hook infrastructure
    let hooks = Config::load(&workspace)
        .map(|c| HookRunner::from_config(&c))
        .unwrap_or_else(|_| HookRunner::noop());
    let event = HookEvent::mutation(&tension.id, &tension.desired, "actual", Some(&old_actual), &new_value);
    if !hooks.pre_mutation(&event) {
        return Err(WerkError::InvalidInput("Blocked by pre_mutation hook".to_string()));
    }

    // Update via store (this handles status validation and mutation recording)
    let _ = store.begin_gesture(Some(&format!("update reality {}", &tension.id)));
    store
        .update_actual(&tension.id, &new_value)
        .map_err(WerkError::SdError)?;
    store.end_gesture();

    hooks.post_mutation(&event);

    let result = RealityResult {
        id: tension.id.clone(),
        actual: new_value,
        old_actual,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        output
            .success(&format!("Updated reality for tension {}", werk_shared::display_id(tension.short_code, &tension.id)))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Old:  {}", &result.old_actual);
        println!("  New:  {}", &result.actual);
    }

    Ok(())
}
