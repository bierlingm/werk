//! Insights command handler.
//!
//! Behavioral pattern analysis from mutation history.

use chrono::{Datelike, Utc};
use serde::Serialize;
use std::collections::HashMap;

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use sd_core::{
    DynamicsEngine, HorizonDriftType, StructuralTendency, TensionStatus,
};

/// JSON output for insights.
#[derive(Serialize)]
struct InsightsJson {
    days: i64,
    mutation_count: usize,
    attention: Vec<AttentionEntry>,
    oscillating_count: usize,
    postponed_count: usize,
    activity_by_day: HashMap<String, usize>,
}

#[derive(Serialize)]
struct AttentionEntry {
    tension_id: String,
    desired: String,
    mutation_count: usize,
}

fn bar_inline(count: usize, max: usize, width: usize) -> String {
    if max == 0 {
        return "\u{2591}".repeat(width);
    }
    let filled = (count as f64 / max as f64 * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
    )
}

pub fn cmd_insights(output: &Output, days: i64) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    let now = Utc::now();
    let since = now - chrono::Duration::days(days);

    // Get all mutations and filter by time window
    let mutations = store.all_mutations().map_err(WerkError::StoreError)?;
    let recent: Vec<_> = mutations
        .iter()
        .filter(|m| m.timestamp() >= since)
        .collect();
    let recent_count = recent.len();

    // Attention distribution: count mutations per tension
    let mut per_tension: HashMap<String, usize> = HashMap::new();
    for m in &recent {
        *per_tension.entry(m.tension_id().to_string()).or_insert(0) += 1;
    }

    // Day-of-week distribution
    let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut day_counts = [0usize; 7];
    for m in &recent {
        // weekday: Mon=0 .. Sun=6
        let wd = m.timestamp().weekday().num_days_from_monday() as usize;
        day_counts[wd] += 1;
    }

    // Load tensions to get desired state text and compute dynamics
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let tension_map: HashMap<&str, &sd_core::Tension> =
        tensions.iter().map(|t| (t.id.as_str(), t)).collect();

    // Build attention list sorted by mutation count descending
    let mut attention: Vec<(&str, usize)> = per_tension
        .iter()
        .map(|(id, &count)| (id.as_str(), count))
        .collect();
    attention.sort_by(|a, b| b.1.cmp(&a.1));

    // Compute oscillation and horizon drift counts across active tensions
    let mut engine = DynamicsEngine::with_store(store);
    let mut oscillating_count = 0usize;
    let mut postponed_count = 0usize;

    for t in &tensions {
        if t.status == TensionStatus::Resolved || t.status == TensionStatus::Released {
            continue;
        }
        if let Some(cd) = engine.compute_full_dynamics_for_tension(&t.id) {
            if cd.tendency.tendency == StructuralTendency::Oscillating {
                oscillating_count += 1;
            }
            match cd.horizon_drift.drift_type {
                HorizonDriftType::Postponement | HorizonDriftType::RepeatedPostponement => {
                    postponed_count += 1;
                }
                _ => {}
            }
        }
    }

    if output.is_structured() {
        let attention_entries: Vec<AttentionEntry> = attention
            .iter()
            .map(|(id, count)| {
                let desired = tension_map
                    .get(id)
                    .map(|t| t.desired.clone())
                    .unwrap_or_else(|| id.to_string());
                AttentionEntry {
                    tension_id: id.to_string(),
                    desired,
                    mutation_count: *count,
                }
            })
            .collect();

        let mut activity: HashMap<String, usize> = HashMap::new();
        for (i, &count) in day_counts.iter().enumerate() {
            activity.insert(day_names[i].to_string(), count);
        }

        let result = InsightsJson {
            days,
            mutation_count: recent_count,
            attention: attention_entries,
            oscillating_count,
            postponed_count,
            activity_by_day: activity,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        println!("Behavioral Insights (last {} days)", days);
        println!();

        if attention.is_empty() {
            println!("  No mutations recorded in this period.");
        } else {
            println!("Attention:");
            // Show most active
            if let Some((id, count)) = attention.first() {
                let desired = tension_map
                    .get(id)
                    .map(|t| t.desired.as_str())
                    .unwrap_or(id);
                let label = if desired.len() > 40 {
                    format!("{}...", &desired[..37])
                } else {
                    desired.to_string()
                };
                println!(
                    "  \"{}\" received {} update(s) (most active)",
                    label, count,
                );
            }
            // Show least active
            if attention.len() > 1 {
                if let Some((id, count)) = attention.last() {
                    let desired = tension_map
                        .get(id)
                        .map(|t| t.desired.as_str())
                        .unwrap_or(id);
                    let label = if desired.len() > 40 {
                        format!("{}...", &desired[..37])
                    } else {
                        desired.to_string()
                    };
                    println!(
                        "  \"{}\" received {} update(s) (least active)",
                        label, count,
                    );
                }
            }
            println!();

            println!("Patterns:");
            if oscillating_count > 0 {
                println!(
                    "  {} tension(s) show oscillation",
                    oscillating_count,
                );
            }
            if postponed_count > 0 {
                println!(
                    "  {} horizon(s) postponed repeatedly",
                    postponed_count,
                );
            }
            if oscillating_count == 0 && postponed_count == 0 {
                println!("  No concerning patterns detected.");
            }
            println!();

            // Activity by day
            let max_day = *day_counts.iter().max().unwrap_or(&0);
            println!("Activity by day:");
            let parts: Vec<String> = day_names
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    format!("{} {}", name, bar_inline(day_counts[i], max_day, 5))
                })
                .collect();
            println!("  {}", parts.join("  "));
        }
    }

    Ok(())
}
