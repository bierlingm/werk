//! Show command handler.

use crate::serialize::HorizonRangeJson;
use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::{DateTime, Utc};
use sd_core::{compute_frontier, compute_temporal_signals, compute_urgency, HorizonKind, TensionStatus};
use serde::Serialize;
use werk_shared::{display_id, relative_time, truncate};

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
    overdue: bool,
    frontier: sd_core::Frontier,
    temporal: sd_core::TemporalSignals,
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
    position: Option<i32>,
    /// When this child was resolved or released (for sort ordering).
    #[serde(skip_serializing_if = "Option::is_none")]
    completion_ts: Option<DateTime<Utc>>,
}

pub fn cmd_show(output: &Output, id: String) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    let all_tensions = store
        .list_tensions()
        .map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(all_tensions.clone());

    let tension = resolver.resolve(&id)?;

    let mutations = store
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    // Build forest for children
    let forest = sd_core::Forest::from_tensions(all_tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // Collect raw children and their mutations (needed for both sorting and frontier)
    let raw_children = forest.children(&tension.id).unwrap_or_default();
    let child_mutations: Vec<(String, Vec<sd_core::Mutation>)> = raw_children
        .iter()
        .filter_map(|c| {
            let muts = store.get_mutations(&c.id()).ok()?;
            Some((c.id().to_string(), muts))
        })
        .collect();

    // Build children sorted: positioned (by position) → held → resolved/released (by completion date)
    let mut children: Vec<ChildInfo> = raw_children
        .iter()
        .map(|child| {
            // Find completion timestamp from mutations (last status→Resolved or status→Released)
            let completion_ts = child_mutations
                .iter()
                .find(|(id, _)| id == child.id())
                .and_then(|(_, muts)| {
                    muts.iter()
                        .rev()
                        .find(|m| {
                            m.field() == "status"
                                && (m.new_value() == "Resolved" || m.new_value() == "Released")
                        })
                        .map(|m| m.timestamp())
                });
            ChildInfo {
                id: child.id().to_string(),
                short_code: child.tension.short_code,
                desired: truncate(&child.tension.desired, 40),
                status: child.tension.status.to_string(),
                position: child.tension.position,
                completion_ts,
            }
        })
        .collect();
    children.sort_by(|a, b| {
        fn sort_key(c: &ChildInfo) -> (u8, i64, i32) {
            match (c.status.as_str(), c.position) {
                // Positioned active children first, ordered by position
                (_, Some(pos)) if c.status == "Active" => (0, 0, pos),
                // Held active children second
                ("Active", None) => (1, 0, 0),
                // Resolved and released together, ordered by completion date
                ("Resolved" | "Released", _) => {
                    let ts = c.completion_ts.map(|t| t.timestamp()).unwrap_or(i64::MAX);
                    (2, ts, 0)
                }
                _ => (3, 0, 0),
            }
        }
        sort_key(a).cmp(&sort_key(b))
    });

    let now = Utc::now();

    // Frontier computation — structural projection of the operating envelope
    let epochs = store
        .get_epochs(&tension.id)
        .map_err(WerkError::StoreError)?;
    let frontier = compute_frontier(&forest, &tension.id, now, &epochs, &child_mutations);

    // Urgency (honest — computed from horizon)
    let urgency = compute_urgency(tension, now);

    // Overdue (honest — a fact)
    let overdue = tension.status == TensionStatus::Active
        && tension
            .horizon
            .as_ref()
            .map(|h| h.is_past(now))
            .unwrap_or(false);

    // Temporal signals (calculus of time)
    let temporal = compute_temporal_signals(&forest, &tension.id, now);

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
        overdue,
        frontier: frontier.clone(),
        temporal: temporal.clone(),
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
        println!("  Reality:    {}", &tension.actual);
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
                ", today is the deadline".to_string()
            } else {
                format!(", {} days past deadline", -days_remaining)
            };

            println!(
                "  Deadline:   {} ({}{})",
                &horizon_str, &interpretation, &days_str
            );
        }

        // === Facts ===
        println!();
        println!("Facts:");

        // Closure progress (resolved / active theory, with released parenthetical)
        let cp = &frontier.closure_progress;
        if cp.total > 0 {
            if cp.released > 0 {
                println!(
                    "  Closure:    [{}/{}] ({} released)",
                    cp.resolved, cp.active, cp.released
                );
            } else {
                println!("  Closure:    [{}/{}]", cp.resolved, cp.active);
            }
        } else {
            println!("  Closure:    no children (leaf tension)");
        }

        // Urgency (only if horizon exists)
        if let Some(urg) = &urgency {
            let pct = (urg.value * 100.0).min(999.0);
            if overdue {
                let days_past = (-urg.time_remaining as f64 / 86400.0).ceil() as i64;
                println!("  Urgency:    OVERDUE ({} days past deadline)", days_past);
            } else {
                let days_left = (urg.time_remaining as f64 / 86400.0).floor() as i64;
                println!(
                    "  Urgency:    {:.0}% of deadline elapsed ({} days remaining)",
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

        // Implied execution window
        if let Some(ref iw) = temporal.implied_window {
            let days = iw.duration_seconds as f64 / 86400.0;
            if days < 0.0 {
                println!("  Window:     negative ({:.0} days past)", -days);
            } else {
                println!("  Window:     {:.0} days", days);
            }
        }

        // Sequencing pressure (signal by exception)
        for sp in &temporal.sequencing_pressures {
            let pred_display = werk_shared::display_id(sp.predecessor_short_code, &sp.predecessor_id);
            let days = sp.gap_seconds as f64 / 86400.0;
            println!(
                "  PRESSURE:   deadline is {:.0} days before {} (ordered after)",
                days, pred_display
            );
        }

        // On critical path of parent
        if temporal.on_critical_path {
            println!("  CRITICAL:   on parent's critical path");
        }

        // Containment violation
        if temporal.has_containment_violation {
            println!("  VIOLATION:  deadline exceeds parent's deadline");
        }

        // Children on critical path
        for cp in &temporal.critical_path {
            let child_sc = all_tensions.iter().find(|t| t.id == cp.tension_id).and_then(|t| t.short_code);
            let child_display = werk_shared::display_id(child_sc, &cp.tension_id);
            let slack_days = cp.slack_seconds as f64 / 86400.0;
            if slack_days <= 0.0 {
                println!("  CRITICAL:   {} matches or exceeds deadline", child_display);
            } else {
                println!("  CRITICAL:   {} has only {:.0} days slack", child_display, slack_days);
            }
        }

        // Children with containment violations
        for cv in &temporal.containment_violations {
            let child_sc = all_tensions.iter().find(|t| t.id == cv.tension_id).and_then(|t| t.short_code);
            let child_display = werk_shared::display_id(child_sc, &cv.tension_id);
            let excess_days = cv.excess_seconds as f64 / 86400.0;
            println!(
                "  VIOLATION:  {} deadline exceeds by {:.0} days",
                child_display, excess_days
            );
        }

        // === Frontier (signal by exception) ===
        if frontier.closure_progress.total > 0 {
            let has_frontier_signals = frontier.next_step.is_some()
                || !frontier.overdue.is_empty()
                || !frontier.held.is_empty()
                || !frontier.recently_resolved.is_empty();

            if has_frontier_signals {
                println!();
                println!("Frontier:");

                // Next step — always show (this is the action vector)
                if let Some(ref ns) = frontier.next_step {
                    let ns_display = display_id(ns.short_code, &ns.tension_id);
                    let overdue_marker = if ns.is_overdue { " OVERDUE" } else { "" };
                    println!(
                        "  Next:       {}{} {}",
                        ns_display, overdue_marker, truncate(&ns.desired, 40)
                    );
                } else if !frontier.held.is_empty() {
                    // Theory exists but has no committed order — frontier cannot advance
                    let n = frontier.held.len();
                    let noun = if n == 1 { "step" } else { "steps" };
                    println!("  Sequence:   uncommitted ({} held {})", n, noun);
                }

                // Overdue (other than next step)
                if !frontier.overdue.is_empty() {
                    for step in &frontier.overdue {
                        let step_display = display_id(step.short_code, &step.tension_id);
                        println!(
                            "  Overdue:    {} {}",
                            step_display, truncate(&step.desired, 40)
                        );
                    }
                }

                // Held count (only if next_step exists — otherwise the Sequence line covers it)
                if !frontier.held.is_empty() && frontier.next_step.is_some() {
                    println!("  Held:       {} unpositioned", frontier.held.len());
                }

                // Recently resolved count — honest about epoch boundary
                if !frontier.recently_resolved.is_empty() {
                    if epochs.is_empty() {
                        println!(
                            "  Recent:     {} resolved",
                            frontier.recently_resolved.len()
                        );
                    } else {
                        println!(
                            "  Recent:     {} resolved since last epoch",
                            frontier.recently_resolved.len()
                        );
                    }
                }
            }
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
