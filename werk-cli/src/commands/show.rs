//! Show command handler.

use crate::dynamics::{compute_all_dynamics, HorizonRangeJson};
use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use werk_shared::{relative_time, truncate};
use chrono::{DateTime, Utc};
use sd_core::{
    compute_structural_tension, compute_urgency, DynamicsEngine, HorizonKind,
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

pub fn cmd_show(output: &Output, id: String) -> Result<(), WerkError> {
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
    let tension = resolver.resolve(&id)?;

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
        println!("Tension {}", &tension.id);
        println!("  Desired:    {}", &tension.desired);
        println!("  Actual:     {}", &tension.actual);
        println!("  Status:     {}", &tension.status);
        println!(
            "  Created:    {}",
            relative_time(tension.created_at, now)
        );

        if let Some(pid) = &tension.parent_id {
            println!("  Parent:     {}", pid);
        }

        // Horizon display
        if let Some(h) = &tension.horizon {
            let horizon_str = h.to_string();
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
                &horizon_str, &interpretation, &days_str
            );
        }

        // === Key Dynamics (5 only) ===
        println!();
        println!("Dynamics:");

        // 1. Phase
        println!(
            "  Phase:      {} (mutations: {}, convergence: {:.0}%)",
            &result.dynamics.phase.phase,
            result.dynamics.phase.evidence.mutation_count,
            (1.0 - result.dynamics.phase.evidence.convergence_ratio) * 100.0
        );

        // 2. Magnitude (skip if not computed)
        if let Some(st) = &result.dynamics.structural_tension {
            println!("  Magnitude:  {:.2}", st.magnitude);
        }

        // 3. Urgency (skip if not computed)
        if let Some(urg) = &urgency {
            let pct = (urg.value * 100.0).min(999.0);
            println!("  Urgency:    {:.0}%", pct);
        }

        // 4. Neglect (skip if not detected)
        if let Some(n) = &result.dynamics.neglect {
            println!("  Neglect:    {} (ratio: {:.2})", n.neglect_type, n.activity_ratio);
        }

        // 5. Movement/Tendency
        let movement_symbol = match result.dynamics.structural_tendency.tendency.as_str() {
            "Advancing" => "->",
            "Oscillating" => "<>",
            _ => "--",
        };
        println!(
            "  Movement:   {} {}",
            movement_symbol, &result.dynamics.structural_tendency.tendency
        );

        // === Children List ===
        if !result.children.is_empty() {
            println!();
            println!("Children:");
            for child in &result.children {
                println!(
                    "  {} [{}] {}",
                    &child.id_prefix,
                    &child.status,
                    &child.desired
                );
            }
        }

        // === Mutation History (last 10) ===
        if !result.mutations.is_empty() {
            println!();
            println!("Recent mutations:");
            for m in &result.mutations {
                let ts = DateTime::parse_from_rfc3339(&m.timestamp)
                    .map(|dt| relative_time(dt.with_timezone(&Utc), now))
                    .unwrap_or_else(|_| m.timestamp[..19].replace('T', " "));
                let old = m.old_value.as_deref().unwrap_or("(none)");
                println!(
                    "  {} [{}] {} -> {}",
                    ts,
                    &m.field,
                    old,
                    &m.new_value
                );
            }
        }
    }

    Ok(())
}
