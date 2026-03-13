#![forbid(unsafe_code)]

//! werk-tui: FrankenTUI dashboard for structural dynamics.

pub mod theme;
pub mod types;
pub mod input;
pub mod msg;
pub mod app;
pub mod helpers;
pub mod agent;
pub mod lever;
pub mod update;
pub mod views;
pub mod overlays;
pub mod horizon;

// Re-exports for the public API
pub use app::WerkApp;
pub use types::{TensionRow, UrgencyTier};

use std::collections::HashMap;
use sd_core::DynamicsEngine;
use werk_shared::Workspace;

use crate::helpers::build_tension_row_from_computed;

/// Load all tensions from the workspace and compute dynamics.
/// Returns (engine, rows) so the engine persists in WerkApp.
pub fn load_tensions() -> Result<(DynamicsEngine, Vec<TensionRow>), String> {
    let workspace = Workspace::discover().map_err(|e| e.to_string())?;
    let store = workspace.open_store().map_err(|e| e.to_string())?;
    let mut engine = DynamicsEngine::with_store(store);

    let tensions = engine
        .store()
        .list_tensions()
        .map_err(|e| e.to_string())?;

    let now = chrono::Utc::now();

    // Compute per-tension activity from mutations (7-day window)
    let window = chrono::Duration::days(7);
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

    let mut rows: Vec<TensionRow> = Vec::with_capacity(tensions.len());

    for tension in &tensions {
        let computed = engine.compute_full_dynamics_for_tension(&tension.id);
        let activity = activity_map.remove(&tension.id).unwrap_or_default();
        rows.push(build_tension_row_from_computed(&computed, tension, now, activity));
    }

    rows.sort_by(|a, b| {
        a.tier.cmp(&b.tier).then_with(|| {
            let ua = a.urgency.unwrap_or(-1.0);
            let ub = b.urgency.unwrap_or(-1.0);
            ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    Ok((engine, rows))
}

/// Launch the TUI dashboard.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    use ftui::App;

    match load_tensions() {
        Ok((mut engine, tensions)) => {
            let lever_result = lever::compute_lever(&mut engine);
            let mut app = WerkApp::new(engine, tensions);
            app.lever = lever_result;
            App::fullscreen(app).run()?;
        }
        Err(_) => {
            let app = WerkApp::new_welcome();
            App::fullscreen(app).run()?;
        }
    }
    Ok(())
}
