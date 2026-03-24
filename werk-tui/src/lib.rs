#![forbid(unsafe_code)]

//! werk-tui: The Operative Instrument.

pub mod state;
pub mod glyphs;
pub mod vlist;
pub mod theme;
pub mod msg;
pub mod app;
pub mod render;
pub mod deck;
pub mod update;
pub mod helpers;
pub mod horizon;
pub mod search;

pub use app::InstrumentApp;

use std::collections::HashMap;
use sd_core::Store;
use werk_shared::Workspace;

use crate::state::FieldEntry;

/// Load all tensions from the workspace and compute activity.
pub fn load_field() -> Result<(Store, Vec<FieldEntry>), String> {
    let workspace = Workspace::discover().map_err(|e| e.to_string())?;

    // Pre-session backup: copy sd.db before opening so a crash can be recovered from
    let db_path = workspace.root().join(".werk").join("sd.db");
    if db_path.exists() {
        let backup_dir = workspace.root().join(".werk").join("backups");
        let _ = std::fs::create_dir_all(&backup_dir);
        let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let backup_path = backup_dir.join(format!("sd.db.{}", timestamp));
        // Only back up if no recent backup exists (avoid duplicate backups within same minute)
        if !backup_path.exists() {
            let _ = std::fs::copy(&db_path, &backup_path);
        }
        // Prune old backups: keep last 10
        if let Ok(entries) = std::fs::read_dir(&backup_dir) {
            let mut db_backups: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("sd.db."))
                .collect();
            db_backups.sort_by_key(|e| e.file_name());
            if db_backups.len() > 10 {
                for old in &db_backups[..db_backups.len() - 10] {
                    let _ = std::fs::remove_file(old.path());
                }
            }
        }
    }

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
