//! Trajectory command handler — projection engine CLI.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::{
    project_field, project_tension, FieldProjection, ProjectionHorizon, ProjectionThresholds,
    TensionProjection, Trajectory,
};
use serde::Serialize;
use werk_shared::truncate;

/// Render a bar chart: filled blocks + empty blocks.
fn render_bar(value: f64, width: usize) -> String {
    let filled = ((value * width as f64).round() as usize).min(width);
    let empty = width - filled;
    format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty))
}

/// Human-friendly trajectory label with directional arrow.
fn trajectory_label(t: Trajectory) -> &'static str {
    match t {
        Trajectory::Resolving => "\u{2193} Resolving",
        Trajectory::Stalling => "\u{2014} Stalling",
        Trajectory::Drifting => "\u{2192} Drifting",
        Trajectory::Oscillating => "\u{2194} Oscillating",
    }
}

/// Human-friendly engagement label from frequency trend.
fn engagement_label(freq_trend: f64) -> &'static str {
    if freq_trend > 0.1 {
        "accelerating"
    } else if freq_trend < -0.1 {
        "declining"
    } else {
        "steady"
    }
}

/// Format seconds as a human-readable duration.
fn format_duration(seconds: i64) -> String {
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

// ── JSON serialization types ────────────────────────────────────────

#[derive(Serialize)]
struct TensionProjectionJson {
    tension_id: String,
    desired: String,
    trajectory: String,
    current_gap: f64,
    gap_1w: f64,
    gap_1m: f64,
    gap_3m: f64,
    time_to_resolution: Option<i64>,
    engagement: String,
    risks: Vec<String>,
}

#[derive(Serialize)]
struct FunnelRowJson {
    trajectory: String,
    week_1: usize,
    month_1: usize,
    month_3: usize,
}

#[derive(Serialize)]
struct CollisionJson {
    window_start: String,
    window_end: String,
    tension_ids: Vec<String>,
    peak_combined_urgency: f64,
}

#[derive(Serialize)]
struct FieldProjectionJson {
    computed_at: String,
    funnel: Vec<FunnelRowJson>,
    collisions: Vec<CollisionJson>,
    tensions: Vec<TensionProjectionJson>,
}

// ── Command implementation ──────────────────────────────────────────

pub fn cmd_trajectory(
    output: &Output,
    id: Option<String>,
    collisions: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let now = Utc::now();
    let analysis = crate::commands::analysis_thresholds_from(&workspace);
    let thresholds = crate::commands::to_projection_thresholds(&analysis);

    let all_tensions = store.list_tensions().map_err(WerkError::StoreError)?;

    match id {
        Some(id_str) => cmd_tension_trajectory(output, &store, &all_tensions, &id_str, &thresholds, now),
        None if collisions => cmd_collisions(output, &store, &all_tensions, &thresholds, now),
        None => cmd_field_trajectory(output, &store, &all_tensions, &thresholds, now),
    }
}

fn cmd_tension_trajectory(
    output: &Output,
    store: &sd_core::Store,
    all_tensions: &[sd_core::Tension],
    id: &str,
    thresholds: &ProjectionThresholds,
    now: chrono::DateTime<Utc>,
) -> Result<(), WerkError> {
    let resolver = PrefixResolver::new(all_tensions.to_vec());
    let tension = resolver.resolve(id)?;

    let mutations = store
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    let projections = project_tension(tension, &mutations, thresholds, now);

    // Extract per-horizon gaps.
    let find_gap = |h: ProjectionHorizon| -> f64 {
        projections
            .iter()
            .find(|p| p.horizon == h)
            .map(|p| p.projected_gap)
            .unwrap_or(0.0)
    };

    let current_gap = projections
        .first()
        .map(|p| p.current_gap)
        .unwrap_or(0.0);
    let gap_1w = find_gap(ProjectionHorizon::OneWeek);
    let gap_1m = find_gap(ProjectionHorizon::OneMonth);
    let gap_3m = find_gap(ProjectionHorizon::ThreeMonths);

    let trajectory = projections
        .first()
        .map(|p| p.trajectory)
        .unwrap_or(Trajectory::Stalling);
    let ttr = projections.first().and_then(|p| p.time_to_resolution);

    // Engagement label from mutation pattern.
    let pattern = sd_core::extract_mutation_pattern(
        tension,
        &mutations,
        thresholds.pattern_window_seconds,
        now,
    );
    let engagement = engagement_label(pattern.frequency_trend);

    // Risk flags.
    let mut risks: Vec<String> = Vec::new();
    for p in &projections {
        if p.oscillation_risk && !risks.contains(&"oscillation".to_string()) {
            risks.push("oscillation".to_string());
        }
        if p.neglect_risk && !risks.contains(&"neglect".to_string()) {
            risks.push("neglect".to_string());
        }
    }

    if output.is_structured() {
        let result = TensionProjectionJson {
            tension_id: tension.id.clone(),
            desired: tension.desired.clone(),
            trajectory: format!("{:?}", trajectory),
            current_gap,
            gap_1w,
            gap_1m,
            gap_3m,
            time_to_resolution: ttr,
            engagement: engagement.to_string(),
            risks,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        let id_prefix = &tension.id[..8.min(tension.id.len())];
        println!("{} ({})", truncate(&tension.desired, 50), id_prefix);
        println!("  Trajectory    {}", trajectory_label(trajectory));
        println!(
            "  Gap now       {}  {:.2}",
            render_bar(current_gap, 10),
            current_gap
        );
        println!(
            "  Gap +1w       {}  {:.2}",
            render_bar(gap_1w, 10),
            gap_1w
        );
        println!(
            "  Gap +1m       {}  {:.2}",
            render_bar(gap_1m, 10),
            gap_1m
        );
        println!(
            "  Gap +3m       {}  {:.2}",
            render_bar(gap_3m, 10),
            gap_3m
        );
        match ttr {
            Some(secs) => println!("  Resolution    {}", format_duration(secs)),
            None => println!("  Resolution    unknown"),
        }
        println!("  Engagement    {}", engagement);

        if !risks.is_empty() {
            println!("  Risks         {}", risks.join(", "));
        }
    }

    Ok(())
}

fn cmd_field_trajectory(
    output: &Output,
    store: &sd_core::Store,
    all_tensions: &[sd_core::Tension],
    thresholds: &ProjectionThresholds,
    now: chrono::DateTime<Utc>,
) -> Result<(), WerkError> {
    // Gather all mutations.
    let mut all_mutations = Vec::new();
    for t in all_tensions {
        let muts = store.get_mutations(&t.id).map_err(WerkError::StoreError)?;
        all_mutations.extend(muts);
    }

    let field = project_field(all_tensions, &all_mutations, thresholds, now);

    let get_buckets = |h: ProjectionHorizon| -> (usize, usize, usize, usize, usize) {
        field
            .funnel
            .get(&h)
            .map(|b| (b.resolving, b.stalling, b.drifting, b.oscillating, b.total))
            .unwrap_or((0, 0, 0, 0, 0))
    };

    let (r1w, s1w, d1w, o1w, t1w) = get_buckets(ProjectionHorizon::OneWeek);
    let (r1m, s1m, d1m, o1m, t1m) = get_buckets(ProjectionHorizon::OneMonth);
    let (r3m, s3m, d3m, o3m, t3m) = get_buckets(ProjectionHorizon::ThreeMonths);

    if output.is_structured() {
        let result = FieldProjectionJson {
            computed_at: now.to_rfc3339(),
            funnel: vec![
                FunnelRowJson { trajectory: "Resolving".into(), week_1: r1w, month_1: r1m, month_3: r3m },
                FunnelRowJson { trajectory: "Stalling".into(), week_1: s1w, month_1: s1m, month_3: s3m },
                FunnelRowJson { trajectory: "Drifting".into(), week_1: d1w, month_1: d1m, month_3: d3m },
                FunnelRowJson { trajectory: "Oscillating".into(), week_1: o1w, month_1: o1m, month_3: o3m },
            ],
            collisions: field
                .urgency_collisions
                .iter()
                .map(|c| CollisionJson {
                    window_start: c.window_start.to_rfc3339(),
                    window_end: c.window_end.to_rfc3339(),
                    tension_ids: c.tension_ids.clone(),
                    peak_combined_urgency: c.peak_combined_urgency,
                })
                .collect(),
            tensions: build_tension_json_list(&field, all_tensions, thresholds, now),
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        println!("Structural Funnel");
        println!("                    {:>5}   {:>5}   {:>5}", "+1w", "+1m", "+3m");
        println!("  Resolving       {:>5}   {:>5}   {:>5}", r1w, r1m, r3m);
        println!("  Stalling        {:>5}   {:>5}   {:>5}", s1w, s1m, s3m);
        println!("  Drifting        {:>5}   {:>5}   {:>5}", d1w, d1m, d3m);
        println!("  Oscillating     {:>5}   {:>5}   {:>5}", o1w, o1m, o3m);
        println!(
            "  {}",
            "\u{2500}".repeat(37)
        );
        println!("  Total           {:>5}   {:>5}   {:>5}", t1w, t1m, t3m);
    }

    Ok(())
}

fn cmd_collisions(
    output: &Output,
    store: &sd_core::Store,
    all_tensions: &[sd_core::Tension],
    thresholds: &ProjectionThresholds,
    now: chrono::DateTime<Utc>,
) -> Result<(), WerkError> {
    // Gather all mutations.
    let mut all_mutations = Vec::new();
    for t in all_tensions {
        let muts = store.get_mutations(&t.id).map_err(WerkError::StoreError)?;
        all_mutations.extend(muts);
    }

    let field = project_field(all_tensions, &all_mutations, thresholds, now);

    if output.is_structured() {
        let collisions: Vec<CollisionJson> = field
            .urgency_collisions
            .iter()
            .map(|c| CollisionJson {
                window_start: c.window_start.to_rfc3339(),
                window_end: c.window_end.to_rfc3339(),
                tension_ids: c.tension_ids.clone(),
                peak_combined_urgency: c.peak_combined_urgency,
            })
            .collect();
        output
            .print_structured(&collisions)
            .map_err(WerkError::IoError)?;
    } else if field.urgency_collisions.is_empty() {
        println!("No urgency collisions detected (next 90 days).");
    } else {
        println!("Urgency Collisions (next 90 days):");
        for collision in &field.urgency_collisions {
            let week_label = collision.window_start.format("Week of %b %d");
            // Resolve tension names.
            let labels: Vec<String> = collision
                .tension_ids
                .iter()
                .map(|tid| {
                    let urgency = all_tensions
                        .iter()
                        .find(|t| t.id == *tid)
                        .and_then(|t| {
                            sd_core::compute_urgency(t, collision.window_start)
                                .map(|u| (truncate(&t.desired, 20), u.value))
                        });
                    match urgency {
                        Some((name, u)) => format!("{} ({:.0}%)", name, u * 100.0),
                        None => tid[..8.min(tid.len())].to_string(),
                    }
                })
                .collect();
            println!(
                "  {}:  {}  combined: {:.0}%",
                week_label,
                labels.join(" + "),
                collision.peak_combined_urgency * 100.0
            );
        }
    }

    Ok(())
}

/// Build per-tension JSON entries from a FieldProjection.
fn build_tension_json_list(
    field: &FieldProjection,
    all_tensions: &[sd_core::Tension],
    _thresholds: &ProjectionThresholds,
    _now: chrono::DateTime<Utc>,
) -> Vec<TensionProjectionJson> {
    field
        .tension_projections
        .iter()
        .map(|(tid, projs)| {
            let tension = all_tensions.iter().find(|t| t.id == *tid);
            let desired = tension
                .map(|t| t.desired.clone())
                .unwrap_or_default();

            let find = |h: ProjectionHorizon| -> &TensionProjection {
                projs.iter().find(|p| p.horizon == h).unwrap() // ubs:ignore projection engine generates all three horizons
            };

            let p1w = find(ProjectionHorizon::OneWeek);
            let p1m = find(ProjectionHorizon::OneMonth);
            let p3m = find(ProjectionHorizon::ThreeMonths);

            let mut risks = Vec::new();
            for p in projs {
                if p.oscillation_risk && !risks.contains(&"oscillation".to_string()) {
                    risks.push("oscillation".to_string());
                }
                if p.neglect_risk && !risks.contains(&"neglect".to_string()) {
                    risks.push("neglect".to_string());
                }
            }

            TensionProjectionJson {
                tension_id: tid.clone(),
                desired,
                trajectory: format!("{:?}", p1w.trajectory),
                current_gap: p1w.current_gap,
                gap_1w: p1w.projected_gap,
                gap_1m: p1m.projected_gap,
                gap_3m: p3m.projected_gap,
                time_to_resolution: p1w.time_to_resolution,
                engagement: String::new(),
                risks,
            }
        })
        .collect()
}
