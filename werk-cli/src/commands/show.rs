//! Show command handler.

use crate::dynamics::{compute_all_dynamics, HorizonRangeJson};
use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::{DateTime, Utc};
use sd_core::{compute_structural_tension, compute_urgency, DynamicsEngine, HorizonKind, TensionStatus};
use serde::Serialize;
use werk_shared::{relative_time, truncate};

/// JSON output structure for show command.
#[derive(Serialize)]
struct ShowResult {
    id: String,
    short_code: Option<i32>,
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
    overdue: bool,
    closure_resolved: usize,
    closure_total: usize,
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
    short_code: Option<i32>,
    desired: String,
    status: String,
}

pub fn cmd_show(output: &Output, id: String) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    let mut engine = DynamicsEngine::with_store(store);

    let all_tensions = engine
        .store()
        .list_tensions()
        .map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(all_tensions.clone());

    let tension = resolver.resolve(&id)?;

    let mutations = engine
        .store()
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    // Build forest for children
    let forest = sd_core::Forest::from_tensions(all_tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // Get children with short codes
    let children: Vec<ChildInfo> = forest
        .children(&tension.id)
        .unwrap_or_default()
        .iter()
        .map(|child| ChildInfo {
            id: child.id().to_string(),
            short_code: child.tension.short_code,
            desired: truncate(&child.tension.desired, 40),
            status: child.tension.status.to_string(),
        })
        .collect();

    // Theory of closure progress
    let closure_total = children.len();
    let closure_resolved = children
        .iter()
        .filter(|c| c.status == "Resolved")
        .count();

    // Compute dynamics (kept for JSON output / agent consumers)
    let now = Utc::now();
    let dynamics_json = compute_all_dynamics(&mut engine, &tension.id);

    // Urgency (honest — computed from horizon)
    let urgency = compute_urgency(tension, now);

    // Structural tension and pressure (kept for JSON backward compat)
    let structural_tension = compute_structural_tension(tension, now);

    // Staleness ratio (kept for JSON backward compat)
    let last_mutation_time = mutations.last().map(|m| m.timestamp());
    let staleness_ratio = match (&tension.horizon, last_mutation_time) {
        (Some(h), Some(last_time)) => Some(h.staleness(last_time, now)),
        _ => None,
    };

    // Overdue (honest — a fact)
    let overdue = tension.status == TensionStatus::Active
        && tension
            .horizon
            .as_ref()
            .map(|h| h.is_past(now))
            .unwrap_or(false);

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
        short_code: tension.short_code,
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
        overdue,
        closure_resolved,
        closure_total,
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
        println!("Tension {}", werk_shared::display_id(tension.short_code, &tension.id));
        println!("  Desired:    {}", &tension.desired);
        println!("  Actual:     {}", &tension.actual);
        println!("  Status:     {}", &tension.status);
        println!(
            "  Created:    {}",
            relative_time(tension.created_at, now)
        );

        // Parent
        if let Some(pid) = &tension.parent_id {
            let parent_sc = all_tensions.iter().find(|t| &t.id == pid).and_then(|t| t.short_code);
            println!("  Parent:     {}", werk_shared::display_id(parent_sc, pid));
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

        // === Facts (replacing old Dynamics section) ===
        println!();
        println!("Facts:");

        // Theory of closure progress
        if closure_total > 0 {
            println!(
                "  Closure:    {}/{} children resolved",
                closure_resolved, closure_total
            );
        } else {
            println!("  Closure:    no children (leaf tension)");
        }

        // Urgency (only if horizon exists)
        if let Some(urg) = &urgency {
            let pct = (urg.value * 100.0).min(999.0);
            if overdue {
                let days_past = (-urg.time_remaining as f64 / 86400.0).ceil() as i64;
                println!("  Urgency:    OVERDUE ({} days past horizon)", days_past);
            } else {
                let days_left = (urg.time_remaining as f64 / 86400.0).floor() as i64;
                println!(
                    "  Urgency:    {:.0}% of horizon elapsed ({} days remaining)",
                    pct, days_left
                );
            }
        }

        // Last activity
        if let Some(last) = mutations.last() {
            println!(
                "  Last act:   {} ({})",
                relative_time(last.timestamp(), now),
                last.field()
            );
        } else {
            println!("  Last act:   no mutations recorded");
        }

        // Position
        if let Some(pos) = tension.position {
            println!("  Position:   {} (positioned)", pos);
        } else if tension.parent_id.is_some() {
            println!("  Position:   held (unpositioned)");
        }

        // === Children List ===
        if !result.children.is_empty() {
            println!();
            println!("Children:");
            for child in &result.children {
                let child_id = werk_shared::display_id(child.short_code, &child.id);
                let status_marker = match child.status.as_str() {
                    "Resolved" => " ✓",
                    "Released" => " ~",
                    _ => "",
                };
                println!(
                    "  {}{} {}",
                    child_id, status_marker, &child.desired
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
                    ts, &m.field, old, &m.new_value
                );
            }
        }
    }

    Ok(())
}
