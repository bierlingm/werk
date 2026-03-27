//! Note command handler — noun-verb subcommand group.
//!
//! `werk note add <id> <text>` — add note to tension
//! `werk note add <text>` — add workspace note
//! `werk note rm <id> <n>` — retract note #n from tension
//! `werk note rm <n>` — retract workspace note #n
//! `werk note list <id>` — list notes for tension
//! `werk note list` — list workspace notes

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::{DateTime, Utc};
use sd_core::Mutation;
use serde::Serialize;
use werk_shared::{format_timestamp, Config, HookEvent, HookRunner};

const WORKSPACE_NOTE_TENSION_ID: &str = "WORKSPACE_NOTES";

// ── JSON output structures ───────────────────────────────────────

#[derive(Serialize)]
struct NoteAddResult {
    id: Option<String>,
    #[serde(skip)]
    display_id: Option<String>,
    note: String,
}

#[derive(Serialize)]
struct NoteRmResult {
    id: Option<String>,
    retracted_note: String,
    note_number: usize,
}

#[derive(Serialize)]
struct NoteListResult {
    tension_id: Option<String>,
    notes: Vec<NoteInfo>,
}

#[derive(Serialize)]
struct NoteInfo {
    number: usize,
    timestamp: String,
    text: String,
}

// ── Subcommand dispatch ──────────────────────────────────────────

pub fn cmd_note_add(
    output: &Output,
    arg1: Option<String>,
    arg2: Option<String>,
) -> Result<(), WerkError> {
    let (id, text) = match (arg1, arg2) {
        (None, None) => {
            return Err(WerkError::InvalidInput(
                "note text is required: werk note add <text> or werk note add <id> <text>"
                    .to_string(),
            ));
        }
        (Some(text), None) => (None, text),
        (Some(id), Some(text)) => (Some(id), text),
        (None, Some(_)) => unreachable!("arg2 without arg1"), // ubs:ignore positional args guarantee this
    };

    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    let hooks = Config::load(&workspace)
        .map(|c| HookRunner::from_config(&c))
        .unwrap_or_else(|_| HookRunner::noop());

    let result = match id {
        Some(id_prefix) => {
            let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
            let resolver = PrefixResolver::new(tensions);
            let tension = resolver.resolve(&id_prefix)?;

            let event =
                HookEvent::mutation(&tension.id, &tension.desired, Some(&tension.actual), tension.parent_id.as_deref(), "note", None, &text);
            if !hooks.pre_mutation(&event) {
                return Err(WerkError::InvalidInput(
                    "Blocked by pre_mutation hook".to_string(),
                ));
            }

            let _ = store.begin_gesture(Some(&format!("note {}", &tension.id)));
            store
                .record_mutation(&Mutation::new(
                    tension.id.clone(),
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    text.clone(),
                ))
                .map_err(WerkError::SdError)?;
            store.end_gesture();

            hooks.post_mutation(&event);

            NoteAddResult {
                id: Some(tension.id.clone()),
                display_id: Some(werk_shared::display_id(tension.short_code, &tension.id)),
                note: text,
            }
        }
        None => {
            let event = HookEvent::mutation(
                WORKSPACE_NOTE_TENSION_ID,
                "workspace",
                None,
                None,
                "note",
                None,
                &text,
            );
            if !hooks.pre_mutation(&event) {
                return Err(WerkError::InvalidInput(
                    "Blocked by pre_mutation hook".to_string(),
                ));
            }

            let _ = store.begin_gesture(Some("note workspace"));
            store
                .record_mutation(&Mutation::new(
                    WORKSPACE_NOTE_TENSION_ID.to_owned(),
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    text.clone(),
                ))
                .map_err(WerkError::SdError)?;
            store.end_gesture();

            hooks.post_mutation(&event);

            NoteAddResult {
                id: None,
                display_id: None,
                note: text,
            }
        }
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        match &result.display_id {
            Some(did) => {
                output
                    .success(&format!("Added note to tension {}", did))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            None => {
                output
                    .success("Added workspace note")
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
        }
        println!("  Note: {}", &result.note);
    }

    Ok(())
}

pub fn cmd_note_rm(
    output: &Output,
    arg1: String,
    arg2: Option<String>,
) -> Result<(), WerkError> {
    // Parse: `note rm <n>` (workspace) or `note rm <id> <n>` (tension)
    let (tension_id_prefix, note_number) = match arg2 {
        Some(n_str) => {
            // Two args: first is tension ID, second is note number
            let n: usize = n_str
                .parse()
                .map_err(|_| WerkError::InvalidInput(format!("invalid note number: {}", n_str)))?;
            (Some(arg1), n)
        }
        None => {
            // Single arg: must be note number (workspace note)
            let n: usize = arg1
                .parse()
                .map_err(|_| WerkError::InvalidInput(format!("invalid note number: {}", arg1)))?;
            (None, n)
        }
    };

    if note_number == 0 {
        return Err(WerkError::InvalidInput(
            "note number must be 1 or greater".to_string(),
        ));
    }

    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    let hooks = Config::load(&workspace)
        .map(|c| HookRunner::from_config(&c))
        .unwrap_or_else(|_| HookRunner::noop());

    let (resolved_tension_id, display_label, tension_actual, tension_parent_id) = match tension_id_prefix {
        Some(id_prefix) => {
            let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
            let resolver = PrefixResolver::new(tensions);
            let tension = resolver.resolve(&id_prefix)?;
            let display = werk_shared::display_id(tension.short_code, &tension.id);
            (tension.id.clone(), format!("tension {}", display), Some(tension.actual.clone()), tension.parent_id.clone())
        }
        None => (WORKSPACE_NOTE_TENSION_ID.to_string(), "workspace".to_string(), None, None),
    };

    // Get all mutations, find active (non-retracted) notes
    let mutations = store
        .get_mutations(&resolved_tension_id)
        .map_err(WerkError::StoreError)?;

    // Collect retracted timestamps
    let retracted_timestamps: std::collections::HashSet<String> = mutations
        .iter()
        .filter(|m| m.field() == "note_retracted")
        .map(|m| m.new_value().to_owned())
        .collect();

    // Active notes: field == "note" and timestamp not in retracted set
    let active_notes: Vec<&Mutation> = mutations
        .iter()
        .filter(|m| {
            m.field() == "note" && !retracted_timestamps.contains(&m.timestamp().to_rfc3339())
        })
        .collect();

    if note_number > active_notes.len() {
        return Err(WerkError::InvalidInput(format!(
            "note #{} does not exist ({} has {} active note{})",
            note_number,
            display_label,
            active_notes.len(),
            if active_notes.len() == 1 { "" } else { "s" },
        )));
    }

    let target_note = active_notes[note_number - 1];
    let note_text = target_note.new_value().to_owned();
    let note_timestamp = target_note.timestamp().to_rfc3339();

    // Hook
    let event = HookEvent::mutation(
        &resolved_tension_id,
        &display_label,
        tension_actual.as_deref(),
        tension_parent_id.as_deref(),
        "note_retracted",
        Some(&note_text),
        &note_timestamp,
    );
    if !hooks.pre_mutation(&event) {
        return Err(WerkError::InvalidInput(
            "Blocked by pre_mutation hook".to_string(),
        ));
    }

    // Record retraction: old_value = note text (audit trail), new_value = note timestamp (identifier)
    let _ = store.begin_gesture(Some(&format!("retract note {}", &resolved_tension_id)));
    store
        .record_mutation(&Mutation::new(
            resolved_tension_id.clone(),
            Utc::now(),
            "note_retracted".to_owned(),
            Some(note_text.clone()),
            note_timestamp,
        ))
        .map_err(WerkError::SdError)?;
    store.end_gesture();

    hooks.post_mutation(&event);

    let result = NoteRmResult {
        id: if resolved_tension_id == WORKSPACE_NOTE_TENSION_ID {
            None
        } else {
            Some(resolved_tension_id)
        },
        retracted_note: note_text.clone(),
        note_number,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        output
            .success(&format!("Retracted note #{} from {}", note_number, display_label))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Was: {}", &note_text);
    }

    Ok(())
}

pub fn cmd_note_list(output: &Output, id: Option<String>) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    let (tension_id, label) = match id {
        Some(id_prefix) => {
            let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
            let resolver = PrefixResolver::new(tensions);
            let tension = resolver.resolve(&id_prefix)?;
            (
                tension.id.clone(),
                format!("Notes for \"{}\"", tension.desired),
            )
        }
        None => (WORKSPACE_NOTE_TENSION_ID.to_string(), "Workspace notes".to_string()),
    };

    let mutations = store
        .get_mutations(&tension_id)
        .map_err(WerkError::StoreError)?;

    // Collect retracted timestamps
    let retracted_timestamps: std::collections::HashSet<String> = mutations
        .iter()
        .filter(|m| m.field() == "note_retracted")
        .map(|m| m.new_value().to_owned())
        .collect();

    // Active notes only
    let notes: Vec<NoteInfo> = mutations
        .iter()
        .filter(|m| {
            m.field() == "note" && !retracted_timestamps.contains(&m.timestamp().to_rfc3339())
        })
        .enumerate()
        .map(|(i, m)| NoteInfo {
            number: i + 1,
            timestamp: m.timestamp().to_rfc3339(),
            text: m.new_value().to_owned(),
        })
        .collect();

    if output.is_structured() {
        let result = NoteListResult {
            tension_id: if tension_id == WORKSPACE_NOTE_TENSION_ID {
                None
            } else {
                Some(tension_id)
            },
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
            let now = Utc::now();
            for note in &notes {
                let ts = DateTime::parse_from_rfc3339(&note.timestamp)
                    .map(|dt| format_timestamp(dt.with_timezone(&Utc), now))
                    .unwrap_or_else(|_| note.timestamp[..19].replace('T', " "));
                println!("\n{}. {}", note.number, &note.text);
                println!("   {}", ts);
            }
        }
    }

    Ok(())
}
