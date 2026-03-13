//! Notes command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use serde::Serialize;

/// JSON output structure for notes command.
#[derive(Serialize)]
struct NotesResult {
    notes: Vec<NoteInfo>,
}

/// Note information for display.
#[derive(Serialize)]
struct NoteInfo {
    timestamp: String,
    text: String,
}

pub fn cmd_notes(output: &Output) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get workspace-level notes (mutations on the WORKSPACE_NOTES sentinel)
    const WORKSPACE_NOTE_TENSION_ID: &str = "WORKSPACE_NOTES";
    let mutations = store
        .get_mutations(WORKSPACE_NOTE_TENSION_ID)
        .map_err(WerkError::StoreError)?;

    // Filter for note mutations only
    let notes: Vec<NoteInfo> = mutations
        .into_iter()
        .filter(|m| m.field() == "note")
        .map(|m| NoteInfo {
            timestamp: m.timestamp().to_rfc3339(),
            text: m.new_value().to_owned(),
        })
        .collect();

    if output.is_structured() {
        let result = NotesResult { notes };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        if notes.is_empty() {
            output
                .info("No workspace notes")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        } else {
            output
                .success(&format!("Workspace notes ({})", notes.len()))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
            for (i, note) in notes.iter().enumerate() {
                println!(
                    "\n{}. {}",
                    i + 1,
                    &note.text
                );
                println!(
                    "   {}",
                    &note.timestamp[..19].replace('T', " ")
                );
            }
        }
    }

    Ok(())
}
