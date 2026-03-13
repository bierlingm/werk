//! Show command handler.

use crate::dynamics::{compute_all_dynamics, HorizonRangeJson};
use crate::error::WerkError;
use crate::output::{ColorStyle, Output};
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::{
    compute_structural_tension, compute_urgency, DynamicsEngine, DynamicsThresholds, HorizonKind,
    TensionStatus,
};
use serde::Serialize;

/// JSON output structure for show command.
#[derive(Serialize)]
struct ShowResult {
    id: String,
    desired: String,
    actual: String,
    status: String,
    parent_id: Option<String>,
    created_at: String,
    horizon: Option<String>,
    horizon_range: Option<HorizonRangeJson>,
    urgency: Option<f64>,
    pressure: Option<f64>,
    staleness_ratio: Option<f64>,
    dynamics: crate::dynamics::DynamicsJson,
    mutations: Vec<ShowMutationInfo>,
    children: Vec<ChildInfo>,
}

/// Mutation information for show display (no tension_id field).
#[derive(Serialize)]
struct ShowMutationInfo {
    timestamp: String,
    field: String,
    old_value: Option<String>,
    new_value: String,
}

/// Child information for display.
#[derive(Serialize)]
struct ChildInfo {
    id: String,
    id_prefix: String,
    desired: String,
    status: String,
}

pub fn cmd_show(output: &Output, id: String, verbose: bool) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Create DynamicsEngine from store (all store access goes through engine.store())
    let mut engine = DynamicsEngine::with_store(store);

    // Get all tensions for prefix resolution
    let all_tensions = engine
        .store()
        .list_tensions()
        .map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(all_tensions.clone());

    // Resolve the ID/prefix
    let tension = resolver.resolve_interactive(&id)?;

    // Get mutations for this tension
    let mutations = engine
        .store()
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    // Build forest for children finding
    let forest = sd_core::Forest::from_tensions(all_tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // Get children
    let children: Vec<ChildInfo> = forest
        .children(&tension.id)
        .unwrap_or_default()
        .iter()
        .map(|child| ChildInfo {
            id: child.id().to_string(),
            id_prefix: child.id()[..8.min(child.id().len())].to_string(),
            desired: truncate(&child.tension.desired, 40),
            status: child.tension.status.to_string(),
        })
        .collect();

    // Compute dynamics via DynamicsEngine (shared module)
    let now = Utc::now();
    let dynamics_json = compute_all_dynamics(&mut engine, &tension.id);

    // Compute urgency and pressure for top-level fields
    let urgency = compute_urgency(tension, now);
    let structural_tension = compute_structural_tension(tension, now);

    // Staleness ratio
    let last_mutation_time = mutations.last().map(|m| m.timestamp());
    let staleness_ratio = match (&tension.horizon, last_mutation_time) {
        (Some(h), Some(last_time)) => Some(h.staleness(last_time, now)),
        _ => None,
    };

    // Build mutation info (last 10, chronological order - oldest first)
    let mutation_infos: Vec<ShowMutationInfo> = mutations
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|m| ShowMutationInfo {
            timestamp: m.timestamp().to_rfc3339(),
            field: m.field().to_owned(),
            old_value: m.old_value().map(|s| s.to_owned()),
            new_value: m.new_value().to_owned(),
        })
        .collect();

    let result = ShowResult {
        id: tension.id.clone(),
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        parent_id: tension.parent_id.clone(),
        created_at: tension.created_at.to_rfc3339(),
        horizon: tension.horizon.as_ref().map(|h| h.to_string()),
        horizon_range: tension.horizon.as_ref().map(|h| HorizonRangeJson {
            start: h.range_start().to_rfc3339(),
            end: h.range_end().to_rfc3339(),
        }),
        urgency: urgency.as_ref().map(|u| u.value),
        pressure: structural_tension.as_ref().and_then(|st| st.pressure),
        staleness_ratio,
        dynamics: dynamics_json,
        mutations: mutation_infos,
        children,
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, ColorStyle::Id);
        let status_style = match tension.status {
            TensionStatus::Active => ColorStyle::Active,
            TensionStatus::Resolved => ColorStyle::Resolved,
            TensionStatus::Released => ColorStyle::Released,
        };
        let status_styled = output.styled(&tension.status.to_string(), status_style);

        println!("Tension {}", id_styled);
        println!(
            "  Desired:    {}",
            output.styled(&tension.desired, ColorStyle::Highlight)
        );
        println!(
            "  Actual:     {}",
            output.styled(&tension.actual, ColorStyle::Muted)
        );
        println!("  Status:     {}", status_styled);
        println!(
            "  Created:    {}",
            output.styled(
                &tension
                    .created_at
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
                ColorStyle::Muted
            )
        );

        if let Some(pid) = &tension.parent_id {
            println!("  Parent:     {}", output.styled(pid, ColorStyle::Id));
        }

        // Horizon display
        if let Some(h) = &tension.horizon {
            let horizon_str = h.to_string();
            // Human interpretation
            let interpretation = match h.kind() {
                HorizonKind::Year(y) => format!("Year {}", y),
                HorizonKind::Month(y, m) => {
                    let month_names = [
                        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                        "Nov", "Dec",
                    ];
                    format!("{} {}", month_names[(m - 1) as usize], y)
                }
                HorizonKind::Day(d) => d.format("%B %d, %Y").to_string(),
                HorizonKind::DateTime(dt) => dt.format("%B %d, %Y %H:%M UTC").to_string(),
            };

            // Days remaining
            let days_remaining = h.range_end().signed_duration_since(now).num_days();
            let days_str = if days_remaining > 0 {
                format!(", {} days remaining", days_remaining)
            } else if days_remaining == 0 {
                ", today is the horizon".to_string()
            } else {
                format!(", {} days past horizon", -days_remaining)
            };

            println!(
                "  Horizon:    {} ({}{})",
                output.styled(&horizon_str, ColorStyle::Highlight),
                output.styled(&interpretation, ColorStyle::Muted),
                output.styled(&days_str, ColorStyle::Muted)
            );
        }

        // Mutation count
        println!(
            "  Mutations:  {}",
            output.styled(&format!("{}", mutations.len()), ColorStyle::Info)
        );

        // Children count
        if !result.children.is_empty() {
            println!(
                "  Children:   {}",
                output.styled(&format!("{}", result.children.len()), ColorStyle::Info)
            );
        }

        // === Dynamics Summary (always shown) ===
        println!();
        println!("Dynamics:");

        // Phase (always shown)
        let phase_display = output.styled(&result.dynamics.phase.phase, ColorStyle::Info);
        println!(
            "  Phase:      {} (mutations: {}, convergence: {:.0}%)",
            phase_display,
            result.dynamics.phase.evidence.mutation_count,
            (1.0 - result.dynamics.phase.evidence.convergence_ratio) * 100.0
        );

        // Structural Tension (show magnitude)
        match &result.dynamics.structural_tension {
            Some(st) => {
                println!(
                    "  Magnitude:  {}",
                    output.styled(&format!("{:.2}", st.magnitude), ColorStyle::Highlight)
                );
            }
            None => {
                println!(
                    "  Magnitude:  {}",
                    output.styled("None (no gap)", ColorStyle::Muted)
                );
            }
        }

        // Conflict (show if present, else None)
        match &result.dynamics.structural_conflict {
            Some(c) => {
                println!(
                    "  Conflict:   {} with {} tensions",
                    output.styled(&c.pattern, ColorStyle::Error),
                    c.tension_ids.len()
                );
            }
            None => {
                println!("  Conflict:   {}", output.styled("None", ColorStyle::Muted));
            }
        }

        // Movement/Tendency
        let movement_symbol = match result.dynamics.structural_tendency.tendency.as_str() {
            "Advancing" => "→",
            "Oscillating" => "↔",
            _ => "○",
        };
        println!(
            "  Movement:   {} {}",
            movement_symbol,
            output.styled(
                &result.dynamics.structural_tendency.tendency,
                ColorStyle::Info
            )
        );

        // === Verbose: Show all 10 dynamics ===
        if verbose {
            println!();
            println!("All Dynamics:");

            // Horizon dynamics (if present)
            if tension.horizon.is_some() {
                // Urgency
                let default_thresholds = DynamicsThresholds::default();
                match &urgency {
                    Some(urg) => {
                        let pct = (urg.value * 100.0).min(999.0);
                        println!(
                            "  Urgency:      {:.0}% ({}s remaining of {}s window)",
                            pct, urg.time_remaining, urg.total_window
                        );
                        // Urgency threshold status
                        let threshold = default_thresholds.urgency_threshold;
                        let status_str = if urg.value >= threshold {
                            format!("above threshold ({:.0}% >= {:.0}%)", pct, threshold * 100.0)
                        } else {
                            format!("below threshold ({:.0}% < {:.0}%)", pct, threshold * 100.0)
                        };
                        println!(
                            "  UrgencyThreshold: {}",
                            output.styled(&status_str, ColorStyle::Info)
                        );
                    }
                    None => {
                        println!(
                            "  Urgency:      {}",
                            output.styled("None", ColorStyle::Muted)
                        );
                    }
                }

                // Pressure
                match &structural_tension {
                    Some(st) if st.pressure.is_some() => {
                        println!(
                            "  Pressure:     {:.2} (magnitude * urgency)",
                            st.pressure.unwrap()
                        );
                    }
                    _ => {
                        println!(
                            "  Pressure:     {}",
                            output.styled("None", ColorStyle::Muted)
                        );
                    }
                }

                // Horizon drift
                println!(
                    "  HorizonDrift: {} ({} changes, net shift {}s)",
                    result.dynamics.horizon_drift.drift_type,
                    result.dynamics.horizon_drift.change_count,
                    result.dynamics.horizon_drift.net_shift_seconds
                );

                println!();
            }

            // 1. Structural Tension
            match &result.dynamics.structural_tension {
                Some(st) => {
                    println!(
                        "  StructuralTension: magnitude={:.2}, has_gap={}",
                        st.magnitude, st.has_gap
                    );
                }
                None => {
                    println!(
                        "  StructuralTension: {}",
                        output.styled("None", ColorStyle::Muted)
                    );
                }
            }

            // 2. Structural Conflict
            match &result.dynamics.structural_conflict {
                Some(c) => {
                    println!(
                        "  StructuralConflict: pattern={}, tensions={}",
                        c.pattern,
                        c.tension_ids.join(", ")
                    );
                }
                None => {
                    println!(
                        "  StructuralConflict: {}",
                        output.styled("None", ColorStyle::Muted)
                    );
                }
            }

            // 3. Oscillation
            match &result.dynamics.oscillation {
                Some(o) => {
                    println!(
                        "  Oscillation: reversals={}, magnitude={:.2}",
                        o.reversals, o.magnitude
                    );
                }
                None => {
                    println!(
                        "  Oscillation: {}",
                        output.styled("None", ColorStyle::Muted)
                    );
                }
            }

            // 4. Resolution
            match &result.dynamics.resolution {
                Some(r) => {
                    println!(
                        "  Resolution: velocity={:.2}, trend={}",
                        r.velocity, r.trend
                    );
                }
                None => {
                    println!("  Resolution: {}", output.styled("None", ColorStyle::Muted));
                }
            }

            // 5. Creative Cycle Phase (already in summary)
            println!(
                "  CreativeCyclePhase: phase={}, mutations={}, convergence={:.0}%",
                result.dynamics.phase.phase,
                result.dynamics.phase.evidence.mutation_count,
                (1.0 - result.dynamics.phase.evidence.convergence_ratio) * 100.0
            );

            // 6. Orientation
            match &result.dynamics.orientation {
                Some(o) => {
                    println!(
                        "  Orientation: {} (creative={:.0}%, problem={:.0}%, reactive={:.0}%)",
                        o.orientation,
                        o.creative_ratio * 100.0,
                        o.problem_solving_ratio * 100.0,
                        o.reactive_ratio * 100.0
                    );
                }
                None => {
                    println!(
                        "  Orientation: {}",
                        output.styled("None", ColorStyle::Muted)
                    );
                }
            }

            // 7. Compensating Strategy
            match &result.dynamics.compensating_strategy {
                Some(cs) => {
                    println!(
                        "  CompensatingStrategy: type={}, persistence={}s",
                        cs.strategy_type, cs.persistence_seconds
                    );
                }
                None => {
                    println!(
                        "  CompensatingStrategy: {}",
                        output.styled("None", ColorStyle::Muted)
                    );
                }
            }

            // 8. Structural Tendency (already in summary)
            println!(
                "  StructuralTendency: tendency={}, has_conflict={}",
                result.dynamics.structural_tendency.tendency,
                result.dynamics.structural_tendency.has_conflict
            );

            // 9. Assimilation Depth
            match &result.dynamics.assimilation_depth {
                Some(a) => {
                    println!(
                        "  AssimilationDepth: depth={}, frequency={:.2}, trend={:.2}",
                        a.depth, a.mutation_frequency, a.frequency_trend
                    );
                }
                None => {
                    println!(
                        "  AssimilationDepth: {}",
                        output.styled("None", ColorStyle::Muted)
                    );
                }
            }

            // 10. Neglect
            match &result.dynamics.neglect {
                Some(n) => {
                    println!(
                        "  Neglect: type={}, ratio={:.2}",
                        n.neglect_type, n.activity_ratio
                    );
                }
                None => {
                    println!("  Neglect: {}", output.styled("None", ColorStyle::Muted));
                }
            }
        }

        // === Mutation History (last 10) ===
        println!();
        println!("Mutation History:");
        for m in &result.mutations {
            let old = m.old_value.as_deref().unwrap_or("(none)");
            println!(
                "  {} [{}] {} -> {}",
                output.styled(&m.timestamp[..19].replace('T', " "), ColorStyle::Muted),
                output.styled(&m.field, ColorStyle::Info),
                output.styled(old, ColorStyle::Muted),
                output.styled(&m.new_value, ColorStyle::Highlight)
            );
        }

        // === Children List ===
        if !result.children.is_empty() {
            println!();
            println!("Children:");
            for child in &result.children {
                let status_style = match child.status.as_str() {
                    "Active" => ColorStyle::Active,
                    "Resolved" => ColorStyle::Resolved,
                    "Released" => ColorStyle::Released,
                    _ => ColorStyle::Muted,
                };
                println!(
                    "  {} {} [{}] {}",
                    output.styled(&child.id_prefix, ColorStyle::Id),
                    output.styled(&child.status, status_style),
                    output.styled(&child.status, status_style),
                    output.styled(&child.desired, ColorStyle::Muted)
                );
            }
        }
    }

    Ok(())
}

/// Truncate a string to max length, adding ellipsis if needed (Unicode-safe).
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
