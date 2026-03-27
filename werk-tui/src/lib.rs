#![forbid(unsafe_code)]

//! werk-tui: The Operative Instrument.

pub mod state;
pub mod glyphs;
pub mod theme;
pub mod msg;
pub mod app;
pub mod render;
pub mod deck;
pub mod update;
pub mod helpers;
pub mod horizon;
pub mod search;
pub mod session_log;
pub mod survey;

pub use app::InstrumentApp;

use std::collections::HashMap;
use sd_core::Store;
use werk_shared::Workspace;

use crate::state::FieldEntry;

/// Load all tensions from the workspace and compute activity.
pub fn load_field() -> Result<(Store, Vec<FieldEntry>), String> {
    let workspace = Workspace::discover().map_err(|e| e.to_string())?;

    // Backup and locking now handled by Store::init()
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
            let child_count = child_counts.get(&t.id).copied().unwrap_or(0);
            let mutations = store.get_mutations(&t.id).unwrap_or_default();
            let last_reality_update = mutations.iter().rev()
                .find(|m| m.field() == "actual" || m.field() == "created")
                .map(|m| m.timestamp().to_owned())
                .unwrap_or(t.created_at);
            let last_status_change = mutations.iter().rev()
                .find(|m| m.field() == "status")
                .map(|m| m.timestamp().to_owned())
                .unwrap_or(t.created_at);
            FieldEntry::from_tension(t, last_reality_update, child_count, last_status_change, now)
        })
        .collect();

    Ok((store, entries))
}

/// Launch the Operative Instrument TUI.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    use ftui::{Program, ProgramConfig, RuntimeDiffConfig};

    // Install panic handler that restores terminal before printing the panic.
    // Without this, a panic leaves the terminal in raw mode and may corrupt the DB WAL.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Attempt to restore terminal to normal mode
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stderr(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show,
        );
        // Print panic info to stderr so the user sees it
        default_hook(info);
        eprintln!("\nwerk TUI crashed. Your data is backed up in .werk/backups/");
    }));

    // Disable Bayesian diff strategy to prevent the "all white" rendering glitch.
    //
    // Root cause: reset_for_frame() marks ALL cells dirty every frame, which
    // inflates the Bayesian change-rate estimate. This causes the strategy to
    // oscillate between FullRedraw and diff modes. During diff-only frames,
    // un-re-emitted cells rely on the terminal preserving its display state.
    // Some terminals (especially under DEC 2026 sync output) can lose that
    // state during focus changes, compositor redraws, or rapid keypresses,
    // causing persistent "all white" or "all black" corruption that the diff
    // engine never repairs because it thinks those cells are unchanged.
    //
    // Fix: disable Bayesian selection and dirty-row optimization so every frame
    // uses a full cell-by-cell diff (DiffStrategy::Full). This still skips
    // unchanged cells (not a FullRedraw) but compares every cell against
    // prev_buffer, preventing strategy oscillation.
    let diff_config = RuntimeDiffConfig::default()
        .with_bayesian_enabled(false)
        .with_dirty_rows_enabled(false);
    let config = ProgramConfig::fullscreen()
        .with_diff_config(diff_config);

    let result = match load_field() {
        Ok((store, entries)) => {
            let app = InstrumentApp::new(store, entries);
            let mut program = Program::with_config(app, config)?;
            program.run()
        }
        Err(_) => {
            let app = InstrumentApp::new_empty();
            let fallback_config = ProgramConfig::fullscreen()
                .with_diff_config(
                    RuntimeDiffConfig::default()
                        .with_bayesian_enabled(false)
                        .with_dirty_rows_enabled(false),
                );
            let mut program = Program::with_config(app, fallback_config)?;
            program.run()
        }
    };

    // Restore default panic hook
    let _ = std::panic::take_hook();

    result.map_err(|e| e.into())
}
