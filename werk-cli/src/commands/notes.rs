//! Notes command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;

/// JSON output structure for notes command.
#[derive(Serialize)]
struct NotesResult {
    tension_id: Option<String>,
    notes: Vec<NoteInfo>,
}

/// Note information for display.
#[derive(Serialize)]
struct NoteInfo {
    timestamp: String,
    text: String,
}

pub fn cmd_notes(output: &Output, id: Option<String>) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    let (tension_id, label) = match id {
        Some(id_prefix) => {
            // Notes for a specific tension
            let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
            let resolver = PrefixResolver::new(tensions);
            let tension = resolver.resolve(&id_prefix)?;
            (tension.id.clone(), format!("Notes for \"{}\"", tension.desired))
        }
        None => {
            // Workspace-level notes
            ("WORKSPACE_NOTES".to_string(), "Workspace notes".to_string())
        }
    };

    let mutations = store
        .get_mutations(&tension_id)
        .map_err(WerkError::StoreError)?;

    let notes: Vec<NoteInfo> = mutations
        .into_iter()
        .filter(|m| m.field() == "note")
        .map(|m| NoteInfo {
            timestamp: m.timestamp().to_rfc3339(),
            text: m.new_value().to_owned(),
        })
        .collect();

    if output.is_structured() {
        let result = NotesResult {
            tension_id: if tension_id == "WORKSPACE_NOTES" { None } else { Some(tension_id) },
            notes,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        if notes.is_empty() {
            output
                .info(&format!("{} (none)", label))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        } else {
            output
                .success(&format!("{} ({})", label, notes.len()))
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
