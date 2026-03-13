//! Desire command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;

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
    let store = workspace.open_store()?;

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

    // Update via store (this handles status validation and mutation recording)
    store
        .update_desired(&tension.id, &new_value)
        .map_err(WerkError::SdError)?;

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
            .success(&format!("Updated desired for tension {}", &tension.id))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Old:  {}", &result.old_desired);
        println!("  New:  {}", &result.desired);
    }

    Ok(())
}
