//! Reality command handler.

use crate::error::WerkError;
use crate::output::{ColorStyle, Output};
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;

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

    // Update via store (this handles status validation and mutation recording)
    store
        .update_actual(&tension.id, &new_value)
        .map_err(WerkError::SdError)?;

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
        let id_styled = output.styled(&tension.id, ColorStyle::Id);
        output
            .success(&format!("Updated actual for tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Old:  {}",
            output.styled(&result.old_actual, ColorStyle::Muted)
        );
        println!(
            "  New:  {}",
            output.styled(&result.actual, ColorStyle::Highlight)
        );
    }

    Ok(())
}
