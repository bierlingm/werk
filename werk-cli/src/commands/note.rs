//! Note command handler.

use crate::error::WerkError;
use crate::output::{ColorStyle, Output};
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::Mutation;
use serde::Serialize;

/// JSON output structure for note command.
#[derive(Serialize)]
struct NoteResult {
    id: Option<String>,
    note: String,
}

pub fn cmd_note(
    output: &Output,
    arg1: Option<String>,
    arg2: Option<String>,
) -> Result<(), WerkError> {
    // Parse arguments: determine ID and text
    let (id, text) = match (arg1, arg2) {
        (None, None) => {
            return Err(WerkError::InvalidInput(
                "note text is required: werk note <text> or werk note <id> <text>".to_string(),
            ));
        }
        (Some(text), None) => {
            // Single argument: treat as workspace note
            (None, text)
        }
        (Some(id), Some(text)) => {
            // Two arguments: first is ID, second is text
            (Some(id), text)
        }
        (None, Some(_)) => {
            // This shouldn't happen with clap, but handle it
            unreachable!("arg2 without arg1")
        }
    };

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    let result = match id {
        Some(id_prefix) => {
            // Note on specific tension
            let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
            let resolver = PrefixResolver::new(tensions);
            let tension = resolver.resolve_interactive(&id_prefix)?;

            // Record note mutation (notes work on any status, no validation needed)
            store
                .record_mutation(&Mutation::new(
                    tension.id.clone(),
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    text.clone(),
                ))
                .map_err(WerkError::SdError)?;

            NoteResult {
                id: Some(tension.id.clone()),
                note: text.clone(),
            }
        }
        None => {
            // General workspace note - store as mutation on a sentinel ID
            // The sentinel is not a real tension but serves as an anchor for workspace-level notes
            const WORKSPACE_NOTE_TENSION_ID: &str = "WORKSPACE_NOTES";

            // Record note mutation on the sentinel
            store
                .record_mutation(&Mutation::new(
                    WORKSPACE_NOTE_TENSION_ID.to_owned(),
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    text.clone(),
                ))
                .map_err(WerkError::SdError)?;

            NoteResult {
                id: None,
                note: text.clone(),
            }
        }
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        match &result.id {
            Some(tid) => {
                output
                    .success(&format!(
                        "Added note to tension {}",
                        output.styled(tid, ColorStyle::Id)
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            None => {
                output
                    .success("Added workspace note")
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
        }
        println!("  Note: {}", output.styled(&text, ColorStyle::Muted));
    }

    Ok(())
}
