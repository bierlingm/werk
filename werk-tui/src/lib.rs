#![forbid(unsafe_code)]

//! werk-tui: The Operative Instrument.

pub mod state;
pub mod glyphs;
pub mod vlist;
pub mod theme;
pub mod msg;
pub mod app;
pub mod render;
pub mod update;
pub mod helpers;
pub mod horizon;
pub mod search;
pub mod agent;

pub use app::InstrumentApp;

use std::collections::HashMap;
use sd_core::Store;
use werk_shared::Workspace;

use crate::state::FieldEntry;

/// Load all tensions from the workspace and compute activity.
pub fn load_field() -> Result<(Store, Vec<FieldEntry>), String> {
    let workspace = Workspace::discover().map_err(|e| e.to_string())?;
    let store = workspace.open_store().map_err(|e| e.to_string())?;

    let tensions = store
        .list_tensions()
        .map_err(|e| e.to_string())?;

    let now = chrono::Utc::now();

    // Check which tensions have children
    let child_counts: HashMap<String, usize> = {
        let mut counts = HashMap::new();
        for t in &tensions {
            if let Some(ref pid) = t.parent_id {
                *counts.entry(pid.clone()).or_insert(0) += 1;
            }
        }
        counts
    };

    let entries: Vec<FieldEntry> = tensions
        .iter()
        .map(|t| {
            let has_children = child_counts.get(&t.id).copied().unwrap_or(0) > 0;
            let last_reality_update = store
                .get_mutations(&t.id)
                .unwrap_or_default()
                .iter()
                .rev()
                .find(|m| m.field() == "actual" || m.field() == "created")
                .map(|m| m.timestamp().to_owned())
                .unwrap_or(t.created_at);
            FieldEntry::from_tension(t, last_reality_update, has_children, now)
        })
        .collect();

    Ok((store, entries))
}

/// Launch the Operative Instrument TUI.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    use ftui::App;

    match load_field() {
        Ok((store, entries)) => {
            let app = InstrumentApp::new(store, entries);
            App::fullscreen(app).run()?;
        }
        Err(_) => {
            let app = InstrumentApp::new_empty();
            App::fullscreen(app).run()?;
        }
    }
    Ok(())
}
