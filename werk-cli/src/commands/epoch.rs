//! Epoch command handler.
//!
//! Mark an epoch boundary — a user-initiated narrative beat when
//! desire or reality shifts significantly enough to warrant a new delta.
//! Snapshots the current desire, reality, and children state.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;

/// JSON output structure for epoch creation.
#[derive(Serialize)]
struct EpochCreatedResult {
    epoch_id: String,
    tension_id: String,
    desire_snapshot: String,
    reality_snapshot: String,
    children_count: usize,
}

/// JSON output structure for epoch listing.
#[derive(Serialize)]
struct EpochListResult {
    tension_id: String,
    epochs: Vec<EpochEntry>,
}

#[derive(Serialize)]
struct EpochEntry {
    id: String,
    timestamp: String,
    desire_snapshot: String,
    reality_snapshot: String,
    children_count: usize,
}

pub fn cmd_epoch(
    output: &Output,
    id: String,
    list: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions.clone());
    let tension = resolver.resolve(&id)?;
    let tension_id = tension.id.clone();
    let tension_display = werk_shared::display_id(tension.short_code, &tension.id);

    if list {
        // List epochs for this tension
        let epochs = store.get_epochs(&tension_id).map_err(WerkError::StoreError)?;

        if output.is_structured() {
            let result = EpochListResult {
                tension_id: tension_id.clone(),
                epochs: epochs
                    .iter()
                    .map(|e| {
                        let children_count = e
                            .children_snapshot_json
                            .as_ref()
                            .and_then(|j| serde_json::from_str::<Vec<serde_json::Value>>(j).ok())
                            .map(|v| v.len())
                            .unwrap_or(0);
                        EpochEntry {
                            id: e.id.clone(),
                            timestamp: e.timestamp.to_rfc3339(),
                            desire_snapshot: e.desire_snapshot.clone(),
                            reality_snapshot: e.reality_snapshot.clone(),
                            children_count,
                        }
                    })
                    .collect(),
            };
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            if epochs.is_empty() {
                println!("No epochs for {}", tension_display);
                return Ok(());
            }
            println!("Epochs for {}:", tension_display);
            for (i, epoch) in epochs.iter().enumerate() {
                let age = format_age(epoch.timestamp);
                let children_count = epoch
                    .children_snapshot_json
                    .as_ref()
                    .and_then(|j| serde_json::from_str::<Vec<serde_json::Value>>(j).ok())
                    .map(|v| v.len())
                    .unwrap_or(0);

                println!();
                println!("  Epoch {} ({})", i + 1, age);
                println!("    Desire:   {}", truncate(&epoch.desire_snapshot, 80));
                println!("    Reality:  {}", truncate(&epoch.reality_snapshot, 80));
                if children_count > 0 {
                    println!("    Children: {}", children_count);
                }
            }
        }
    } else {
        // Create a new epoch — snapshot current state
        let children = store.get_children(&tension_id).map_err(WerkError::StoreError)?;

        // Build children snapshot as JSON
        let children_snapshot: Vec<serde_json::Value> = children
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "desired": c.desired,
                    "actual": c.actual,
                    "status": c.status.to_string(),
                    "position": c.position,
                })
            })
            .collect();
        let children_json = serde_json::to_string(&children_snapshot)
            .map_err(|e| WerkError::IoError(e.to_string()))?;

        let _ = store.begin_gesture(Some(&format!("epoch {}", &tension_id)));
        let epoch_id = store
            .create_epoch(
                &tension_id,
                &tension.desired,
                &tension.actual,
                Some(&children_json),
                store.active_gesture().as_deref(),
            )
            .map_err(WerkError::StoreError)?;
        store.end_gesture();

        if output.is_structured() {
            let result = EpochCreatedResult {
                epoch_id: epoch_id.clone(),
                tension_id: tension_id.clone(),
                desire_snapshot: tension.desired.clone(),
                reality_snapshot: tension.actual.clone(),
                children_count: children.len(),
            };
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            let existing_epochs = store.get_epochs(&tension_id).map_err(WerkError::StoreError)?;
            output
                .success(&format!(
                    "Epoch {} marked for {}",
                    existing_epochs.len(),
                    tension_display,
                ))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
            println!("  Snapshotted: desire, reality, {} children", children.len());
        }
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

fn format_age(timestamp: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(timestamp);
    let seconds = duration.num_seconds();

    if seconds < 60 {
        "just now".to_string()
    } else if seconds < 3600 {
        format!("{} min ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{} hours ago", seconds / 3600)
    } else {
        format!("{} days ago", seconds / 86400)
    }
}
