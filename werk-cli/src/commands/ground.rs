//! Ground command handler — the debrief and study surface.
//!
//! When you're not flying, you're on the ground. Shows epoch history,
//! engagement patterns, and structural telemetry across the field.

use chrono::Utc;
use serde::Serialize;

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use sd_core::TensionStatus;

#[derive(Serialize)]
struct EpochSummary {
    tension_id: String,
    tension_short_code: Option<i32>,
    tension_desired: String,
    epoch_count: usize,
}

#[derive(Serialize)]
struct EngagementStats {
    total_tensions: usize,
    active: usize,
    resolved: usize,
    released: usize,
    with_deadlines: usize,
    overdue: usize,
    held: usize,
    positioned: usize,
    total_mutations: usize,
    recent_mutations: usize,
}

#[derive(Serialize)]
struct RecentGesture {
    tension_id: String,
    tension_short_code: Option<i32>,
    field: String,
    timestamp: String,
    age: String,
}

#[derive(Serialize)]
struct GroundJson {
    stats: EngagementStats,
    epochs: Vec<EpochSummary>,
    recent_gestures: Vec<RecentGesture>,
}

pub fn cmd_ground(output: &Output, days: i64) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let now = Utc::now();

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;

    // Engagement statistics
    let total = tensions.len();
    let active = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .count();
    let resolved = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .count();
    let released = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Released)
        .count();
    let with_deadlines = tensions.iter().filter(|t| t.horizon.is_some()).count();
    let overdue_count = tensions
        .iter()
        .filter(|t| {
            t.status == TensionStatus::Active
                && t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false)
        })
        .count();
    let held_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active && t.position.is_none())
        .count();
    let positioned_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active && t.position.is_some())
        .count();

    // Gather all mutations for stats
    let mut total_mutations = 0usize;
    let mut recent_mutations = 0usize;
    let cutoff = now - chrono::Duration::days(days);
    let mut recent_gestures: Vec<RecentGesture> = Vec::new();

    // Build tension lookup
    let tension_lookup: std::collections::HashMap<String, (Option<i32>, String)> = tensions
        .iter()
        .map(|t| (t.id.clone(), (t.short_code, t.desired.clone())))
        .collect();

    for tension in &tensions {
        let mutations = store
            .get_mutations(&tension.id)
            .map_err(WerkError::StoreError)?;
        total_mutations += mutations.len();

        for m in &mutations {
            if m.timestamp() >= cutoff {
                recent_mutations += 1;

                // Only collect the most meaningful mutations for display
                let field = m.field();
                if field == "actual" || field == "desired" || field == "status" || field == "note" {
                    let age = format_age(now, m.timestamp());
                    recent_gestures.push(RecentGesture {
                        tension_id: tension.id.clone(),
                        tension_short_code: tension.short_code,
                        field: field.to_string(),
                        timestamp: m.timestamp().to_rfc3339(),
                        age,
                    });
                }
            }
        }
    }

    // Sort recent gestures by time (most recent first), limit to 15
    recent_gestures.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    recent_gestures.truncate(15);

    // Epoch summaries per tension
    let mut epochs: Vec<EpochSummary> = Vec::new();
    for tension in &tensions {
        let epoch_list = store
            .get_epochs(&tension.id)
            .map_err(WerkError::StoreError)?;
        if !epoch_list.is_empty() {
            epochs.push(EpochSummary {
                tension_id: tension.id.clone(),
                tension_short_code: tension.short_code,
                tension_desired: tension.desired.clone(),
                epoch_count: epoch_list.len(),
            });
        }
    }
    epochs.sort_by(|a, b| b.epoch_count.cmp(&a.epoch_count));

    let stats = EngagementStats {
        total_tensions: total,
        active,
        resolved,
        released,
        with_deadlines,
        overdue: overdue_count,
        held: held_count,
        positioned: positioned_count,
        total_mutations,
        recent_mutations,
    };

    if output.is_structured() {
        let result = GroundJson {
            stats,
            epochs,
            recent_gestures,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Field statistics
        println!("Field");
        println!(
            "  {} tensions  {} active  {} resolved  {} released",
            stats.total_tensions, stats.active, stats.resolved, stats.released
        );
        println!(
            "  {} with deadlines  {} overdue  {} positioned  {} held",
            stats.with_deadlines, stats.overdue, stats.positioned, stats.held
        );
        println!(
            "  {} mutations total  {} in last {} days",
            stats.total_mutations, stats.recent_mutations, days
        );

        // Epoch history
        if !epochs.is_empty() {
            println!();
            println!("Epochs");
            for e in &epochs {
                let id_display = match e.tension_short_code {
                    Some(c) => format!("#{}", c),
                    None => e.tension_id[..8.min(e.tension_id.len())].to_string(),
                };
                println!(
                    "  {:<6} {} epoch(s)  {}",
                    id_display,
                    e.epoch_count,
                    werk_shared::truncate(&e.tension_desired, 50)
                );
            }
        }

        // Recent gestures
        if !recent_gestures.is_empty() {
            println!();
            println!("Recent gestures (last {} days)", days);
            for g in &recent_gestures {
                let id_display = match g.tension_short_code {
                    Some(c) => format!("#{}", c),
                    None => g.tension_id[..8.min(g.tension_id.len())].to_string(),
                };

                let field_display = match g.field.as_str() {
                    "actual" => "reality",
                    "desired" => "desire",
                    "status" => "status",
                    "note" => "note",
                    other => other,
                };

                // Look up tension desired for context
                let context = tension_lookup
                    .get(&g.tension_id)
                    .map(|(_, d)| werk_shared::truncate(d, 30))
                    .unwrap_or_default();

                println!(
                    "  {:<6} {:<10} {:>10}  {}",
                    id_display, field_display, g.age, context
                );
            }
        }
    }

    Ok(())
}

fn format_age(now: chrono::DateTime<Utc>, then: chrono::DateTime<Utc>) -> String {
    let diff = now - then;
    let minutes = diff.num_minutes();
    if minutes < 60 {
        format!("{} min ago", minutes)
    } else if minutes < 1440 {
        format!("{} hr ago", minutes / 60)
    } else {
        format!("{} days ago", minutes / 1440)
    }
}
