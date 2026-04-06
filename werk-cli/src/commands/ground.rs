//! Ground command handler — the debrief and study surface.
//!
//! When you're not flying, you're on the ground. Shows epoch history,
//! engagement patterns, and structural telemetry across the field.

use chrono::Utc;
use serde::Serialize;

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use sd_core::{project_tension, TensionStatus};

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
struct TrajectoryEntry {
    tension_id: String,
    tension_short_code: Option<i32>,
    tension_desired: String,
    trajectory: String,
    current_gap: f64,
    projected_gap_1w: f64,
    projected_gap_1m: f64,
    projected_gap_3m: f64,
    time_to_resolution: Option<i64>,
    oscillation_risk: bool,
    neglect_risk: bool,
}

#[derive(Serialize)]
struct GroundJson {
    stats: EngagementStats,
    trajectories: Vec<TrajectoryEntry>,
    epochs: Vec<EpochSummary>,
    recent_gestures: Vec<RecentGesture>,
}

pub fn cmd_ground(output: &Output, days: i64) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let now = Utc::now();
    let analysis = crate::commands::analysis_thresholds_from(&workspace);
    let proj_thresholds = crate::commands::to_projection_thresholds(&analysis);

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

    // Trajectory projections — full analytical layer, ground-mode only (Stance B)
    let thresholds = proj_thresholds;
    let mut trajectories: Vec<TrajectoryEntry> = Vec::new();
    for tension in tensions.iter().filter(|t| t.status == TensionStatus::Active) {
        let mutations = store
            .get_mutations(&tension.id)
            .map_err(WerkError::StoreError)?;
        let projections = project_tension(tension, &mutations, &thresholds, now);
        if let Some(proj) = projections.first() {
            let find_gap = |idx: usize| -> f64 {
                projections.get(idx).map(|p| p.projected_gap).unwrap_or(0.0)
            };
            trajectories.push(TrajectoryEntry {
                tension_id: tension.id.clone(),
                tension_short_code: tension.short_code,
                tension_desired: tension.desired.clone(),
                trajectory: format!("{:?}", proj.trajectory),
                current_gap: proj.current_gap,
                projected_gap_1w: find_gap(0),
                projected_gap_1m: find_gap(1),
                projected_gap_3m: find_gap(2),
                time_to_resolution: proj.time_to_resolution,
                oscillation_risk: proj.oscillation_risk,
                neglect_risk: proj.neglect_risk,
            });
        }
    }
    // Sort: oscillating/drifting first (most attention-worthy), then stalling, then resolving
    trajectories.sort_by(|a, b| {
        fn trajectory_order(t: &str) -> u8 {
            match t {
                "Oscillating" => 0,
                "Drifting" => 1,
                "Stalling" => 2,
                _ => 3,
            }
        }
        trajectory_order(&a.trajectory).cmp(&trajectory_order(&b.trajectory))
    });

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
            trajectories,
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

        // Trajectory projections (observational analysis — ground-mode only)
        if !trajectories.is_empty() {
            println!();
            println!("Trajectories (observational analysis)");
            for t in &trajectories {
                let id_display = match t.tension_short_code {
                    Some(c) => format!("#{}", c),
                    None => t.tension_id[..8.min(t.tension_id.len())].to_string(),
                };
                let arrow = match t.trajectory.as_str() {
                    "Resolving" => "\u{2193}",
                    "Stalling" => "\u{2014}",
                    "Drifting" => "\u{2192}",
                    "Oscillating" => "\u{2194}",
                    _ => " ",
                };
                let mut flags = Vec::new();
                if t.oscillation_risk { flags.push("oscillation"); }
                if t.neglect_risk { flags.push("neglect"); }
                let ttr = t.time_to_resolution
                    .map(|s| format_ttr(s))
                    .unwrap_or_else(|| "unknown".to_string());
                let flag_str = if flags.is_empty() {
                    String::new()
                } else {
                    format!("  [{}]", flags.join(", "))
                };
                println!(
                    "  {:<6} {} {:<12} gap {:.2} \u{2192} {:.2} \u{2192} {:.2}  ttr {}{}",
                    id_display, arrow, t.trajectory,
                    t.projected_gap_1w, t.projected_gap_1m, t.projected_gap_3m,
                    ttr, flag_str,
                );
            }
        }

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

fn format_ttr(seconds: i64) -> String {
    let days = seconds / 86400;
    if days < 7 {
        format!("~{} days", days.max(1))
    } else if days < 30 {
        format!("~{} weeks", (days + 3) / 7)
    } else if days < 365 {
        format!("~{} months", (days + 15) / 30)
    } else {
        format!("~{} years", (days + 182) / 365)
    }
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
