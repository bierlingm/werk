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
use sd_core::DynamicsEngine;
use werk_shared::Workspace;

use crate::state::FieldEntry;

/// Load all tensions from the workspace and compute dynamics + activity.
pub fn load_field() -> Result<(DynamicsEngine, Vec<FieldEntry>), String> {
    let workspace = Workspace::discover().map_err(|e| e.to_string())?;
    let store = workspace.open_store().map_err(|e| e.to_string())?;
    let mut engine = DynamicsEngine::with_store(store);

    let tensions = engine
        .store()
        .list_tensions()
        .map_err(|e| e.to_string())?;

    let now = chrono::Utc::now();
    let window = chrono::Duration::days(7);

    // Compute per-tension activity from mutations (7-day window, 7 buckets)
    let mut activity_map: HashMap<String, Vec<f64>> = HashMap::new();
    for t in &tensions {
        for m in engine.store().get_mutations(&t.id).unwrap_or_default() {
            if m.timestamp() >= now - window {
                let bucket = (now - m.timestamp()).num_days().min(6) as usize;
                activity_map
                    .entry(m.tension_id().to_string())
                    .or_insert_with(|| vec![0.0; 7])[6 - bucket] += 1.0;
            }
        }
    }

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
            let computed = engine.compute_full_dynamics_for_tension(&t.id);
            let activity = activity_map.remove(&t.id).unwrap_or_default();
            let has_children = child_counts.get(&t.id).copied().unwrap_or(0) > 0;
            FieldEntry::from_tension(t, &computed, activity, has_children)
        })
        .collect();

    Ok((engine, entries))
}

/// Launch the Operative Instrument TUI.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    use ftui::App;

    match load_field() {
        Ok((engine, entries)) => {
            let app = InstrumentApp::new(engine, entries);
            App::fullscreen(app).run()?;
        }
        Err(_) => {
            let app = InstrumentApp::new_empty();
            App::fullscreen(app).run()?;
        }
    }
    Ok(())
}
