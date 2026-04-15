//! Epoch command handler.
//!
//! Three modes:
//! - `werk epoch <id>` — create a new epoch (manual snapshot)
//! - `werk epoch <id> --list` — list all epochs
//! - `werk epoch <id> --show <n>` — show what happened during epoch N
//!
//! Epochs are also created automatically by reality/desire updates.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_shared::format_datetime_compact;

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
    number: usize,
    timestamp: String,
    desire_snapshot: String,
    reality_snapshot: String,
    children_count: usize,
}

/// JSON output for --show (epoch span detail).
#[derive(Serialize)]
struct EpochShowResult {
    tension_id: String,
    epoch_number: usize,
    desire_snapshot: String,
    reality_snapshot: String,
    span_start: String,
    span_end: String,
    mutations: Vec<EpochMutationEntry>,
}

#[derive(Serialize)]
struct EpochMutationEntry {
    tension_id: String,
    timestamp: String,
    field: String,
    old_value: Option<String>,
    new_value: String,
}

pub fn cmd_epoch(
    output: &Output,
    id: String,
    list: bool,
    show: Option<usize>,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let mut store = workspace.open_store()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions.clone());
    let tension = resolver.resolve(&id)?;
    let tension_id = tension.id.clone();
    let tension_display = werk_shared::display_id(tension.short_code, &tension.id);

    if let Some(n) = show {
        // Show what happened during epoch N
        return cmd_epoch_show(output, &store, &tension_id, &tension_display, &tension, n);
    }

    if list {
        let epochs = store
            .get_epochs(&tension_id)
            .map_err(WerkError::StoreError)?;

        if output.is_structured() {
            let result = EpochListResult {
                tension_id: tension_id.clone(),
                epochs: epochs
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let children_count = count_children_snapshot(e);
                        EpochEntry {
                            id: e.id.clone(),
                            number: i + 1,
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
                let children_count = count_children_snapshot(epoch);

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
        let children = store
            .get_children(&tension_id)
            .map_err(WerkError::StoreError)?;

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
            let existing_epochs = store
                .get_epochs(&tension_id)
                .map_err(WerkError::StoreError)?;
            output
                .success(&format!(
                    "Epoch {} marked for {}",
                    existing_epochs.len(),
                    tension_display,
                ))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
            println!(
                "  Snapshotted: desire, reality, {} children",
                children.len()
            );
        }
    }

    Ok(())
}

fn cmd_epoch_show(
    output: &Output,
    store: &werk_core::Store,
    tension_id: &str,
    tension_display: &str,
    tension: &werk_core::Tension,
    n: usize,
) -> Result<(), WerkError> {
    if n == 0 {
        return Err(WerkError::InvalidInput(
            "epoch number must be 1 or greater".to_string(),
        ));
    }

    let epochs = store
        .get_epochs(tension_id)
        .map_err(WerkError::StoreError)?;

    if n > epochs.len() {
        return Err(WerkError::InvalidInput(format!(
            "epoch #{} does not exist ({} has {} epoch{})",
            n,
            tension_display,
            epochs.len(),
            if epochs.len() == 1 { "" } else { "s" },
        )));
    }

    let epoch = &epochs[n - 1];

    // Determine the span: from previous epoch (or creation) to this epoch
    let span_start = if n == 1 {
        tension.created_at
    } else {
        epochs[n - 2].timestamp
    };
    let span_end = epoch.timestamp;

    // Get all mutations for this tension + descendants in the span
    let mutations = store
        .get_epoch_mutations(tension_id, span_start, span_end)
        .map_err(WerkError::StoreError)?;

    if output.is_structured() {
        let result = EpochShowResult {
            tension_id: tension_id.to_owned(),
            epoch_number: n,
            desire_snapshot: epoch.desire_snapshot.clone(),
            reality_snapshot: epoch.reality_snapshot.clone(),
            span_start: span_start.to_rfc3339(),
            span_end: span_end.to_rfc3339(),
            mutations: mutations
                .iter()
                .map(|m| EpochMutationEntry {
                    tension_id: m.tension_id().to_owned(),
                    timestamp: m.timestamp().to_rfc3339(),
                    field: m.field().to_owned(),
                    old_value: m.old_value().map(|s| s.to_owned()),
                    new_value: m.new_value().to_owned(),
                })
                .collect(),
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        println!("Epoch {} for {}:", n, tension_display);
        println!();
        println!("  Delta at boundary:");
        println!("    Desire:  {}", truncate(&epoch.desire_snapshot, 72));
        println!("    Reality: {}", truncate(&epoch.reality_snapshot, 72));
        println!();

        let start_str = format_datetime_compact(span_start);
        let end_str = format_datetime_compact(span_end);
        println!("  Span: {} to {}", start_str, end_str);

        if mutations.is_empty() {
            println!("\n  No mutations in this epoch span.");
        } else {
            println!("\n  Mutations ({}):", mutations.len());
            // Group by tension_id for readability
            let mut current_tid = String::new();
            for m in &mutations {
                if m.tension_id() != current_tid {
                    current_tid = m.tension_id().to_owned();
                    // Find display name for this tension
                    let label = if current_tid == tension_id {
                        tension_display.to_string()
                    } else {
                        // Try to find short code
                        format!("({})", &current_tid[..8])
                    };
                    println!("\n    {}:", label);
                }

                let ts = format_datetime_compact(m.timestamp());
                match m.old_value() {
                    Some(old) => {
                        println!(
                            "      {} [{}] {} -> {}",
                            ts,
                            m.field(),
                            truncate(old, 40),
                            truncate(m.new_value(), 40),
                        );
                    }
                    None => {
                        println!(
                            "      {} [{}] -> {}",
                            ts,
                            m.field(),
                            truncate(m.new_value(), 60),
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn count_children_snapshot(epoch: &werk_core::EpochRecord) -> usize {
    epoch
        .children_snapshot_json
        .as_ref()
        .and_then(|j| serde_json::from_str::<Vec<serde_json::Value>>(j).ok())
        .map(|v| v.len())
        .unwrap_or(0)
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
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
