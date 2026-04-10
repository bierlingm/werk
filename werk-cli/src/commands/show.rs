//! Show command handler.

use crate::serialize::{HorizonRangeJson, TensionInfo, node_to_tension_info};
use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::{DateTime, Utc};
use sd_core::{compute_frontier, compute_structural_signals, compute_temporal_signals, compute_urgency, detect_horizon_drift, extract_mutation_pattern, gap_magnitude, HorizonDriftType, HorizonKind, TensionStatus};
use serde::Serialize;
use werk_shared::cli_display::glyphs;
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
    structural: sd_core::StructuralSignals,
    #[serde(skip_serializing_if = "Option::is_none")]
    horizon_drift: Option<sd_core::HorizonDrift>,
    mutations: Vec<ShowMutationInfo>,
    children: Vec<ChildInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ancestors: Option<Vec<TensionInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    siblings: Option<Vec<TensionInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    engagement: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    epochs: Option<Vec<ShowEpochInfo>>,
}

/// Epoch information for show display.
#[derive(Serialize)]
struct ShowEpochInfo {
    number: usize,
    timestamp: String,
    desire_snapshot: String,
    reality_snapshot: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger_gesture_id: Option<String>,
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

pub fn cmd_show(output: &Output, id: String, full: bool) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let sig = crate::commands::signal_thresholds_from(&workspace);
    let analysis = crate::commands::analysis_thresholds_from(&workspace);

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

    // Structural signals (graph intelligence)
    let field_structural = compute_structural_signals(&forest);
    let structural = field_structural
        .signals
        .get(&tension.id)
        .cloned()
        .unwrap_or_default();

    // Horizon drift (from mutation history)
    let horizon_drift = {
        let drift = detect_horizon_drift(&tension.id, &mutations);
        if drift.drift_type != HorizonDriftType::Stable {
            Some(drift)
        } else {
            None
        }
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

    // Context data (ancestors, siblings, engagement) — included when --full or --json
    let include_context = full || output.is_structured();
    let (ancestors, siblings, engagement) = if include_context {
        let ancestors: Vec<TensionInfo> = forest
            .ancestors(&tension.id)
            .unwrap_or_default()
            .into_iter()
            .map(|node| node_to_tension_info(node, now))
            .collect();

        let siblings: Vec<TensionInfo> = forest
            .siblings(&tension.id)
            .unwrap_or_default()
            .into_iter()
            .map(|node| node_to_tension_info(node, now))
            .collect();

        let proj_thresholds = crate::commands::to_projection_thresholds(&analysis);
        let pattern = extract_mutation_pattern(tension, &mutations, proj_thresholds.pattern_window_seconds, now);
        let gap = gap_magnitude(&tension.desired, &tension.actual);
        let engagement = serde_json::json!({
            "current_gap": gap,
            "mutation_count": pattern.mutation_count,
            "frequency_per_day": pattern.frequency_per_day,
            "frequency_trend": pattern.frequency_trend,
            "gap_trend": pattern.gap_trend,
            "gap_samples": pattern.gap_samples,
            "mean_interval_seconds": pattern.mean_interval_seconds,
        });

        (Some(ancestors), Some(siblings), Some(engagement))
    } else {
        (None, None, None)
    };

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
        structural: structural.clone(),
        horizon_drift: horizon_drift.clone(),
        mutations: mutation_infos,
        children,
        ancestors,
        siblings,
        engagement,
        epochs: if epochs.is_empty() {
            None
        } else {
            Some(
                epochs
                    .iter()
                    .enumerate()
                    .map(|(i, e)| ShowEpochInfo {
                        number: i + 1,
                        timestamp: e.timestamp.to_rfc3339(),
                        desire_snapshot: e.desire_snapshot.clone(),
                        reality_snapshot: e.reality_snapshot.clone(),
                        trigger_gesture_id: e.trigger_gesture_id.clone(),
                    })
                    .collect(),
            )
        },
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        let palette = output.palette();

        // === Identity: the tension IS the gap ===
        println!(
            "Tension {}",
            palette.bold(&werk_shared::display_id(tension.short_code, &tension.id))
        );
        println!("  {}  {}", palette.chrome("Desired:"), &tension.desired);
        println!("  {}  {}", palette.chrome("Reality:"), &tension.actual);

        // === Structural position ===
        println!();
        if let Some(pid) = &tension.parent_id {
            let parent = all_tensions.iter().find(|t| &t.id == pid);
            let parent_display = display_id(parent.and_then(|t| t.short_code), pid);
            let parent_desired = parent.map(|t| truncate(&t.desired, 50)).unwrap_or_default();
            println!("  Parent:   {} — {}", parent_display, parent_desired);
        }
        print!("  Status:   {}", &tension.status);
        println!("          Created: {}", relative_time(tension.created_at, now));

        // Horizon: own or inherited from parent (clearly distinguished)
        if let Some(h) = &tension.horizon {
            print_horizon(h, now, None);
        } else {
            // Check parent chain for inherited horizon
            let mut ancestor_id = tension.parent_id.clone();
            while let Some(aid) = &ancestor_id {
                if let Some(ancestor) = all_tensions.iter().find(|t| &t.id == aid) {
                    if let Some(h) = &ancestor.horizon {
                        let ancestor_display = display_id(ancestor.short_code, &ancestor.id);
                        print_horizon(h, now, Some(&ancestor_display));
                        break;
                    }
                    ancestor_id = ancestor.parent_id.clone();
                } else {
                    break;
                }
            }
        }

        // Position and last activity on one line
        let pos_str = if let Some(pos) = tension.position {
            format!("{} (positioned)", pos)
        } else if tension.parent_id.is_some() {
            "held".to_string()
        } else {
            String::new()
        };
        let last_act_str = if let Some(last) = mutations.last() {
            format!(
                "Last act: {} ({})",
                relative_time(last.timestamp(), now),
                last.field()
            )
        } else {
            String::new()
        };
        if !pos_str.is_empty() {
            print!("  Position: {}", pos_str);
            if !last_act_str.is_empty() {
                println!("    {}", last_act_str);
            } else {
                println!();
            }
        } else if !last_act_str.is_empty() {
            println!("  {}", last_act_str);
        }

        // Closure progress
        let cp = &frontier.closure_progress;
        if cp.total > 0 {
            if cp.released > 0 {
                println!(
                    "  Closure:  [{}/{}] ({} released)",
                    cp.resolved, cp.active, cp.released
                );
            } else {
                println!("  Closure:  [{}/{}]", cp.resolved, cp.active);
            }
        }

        // Wave (topological generation — always shown if computed)
        if let Some(wave) = structural.wave {
            println!("  Wave:     {} of {}", wave + 1, field_structural.wave_count);
        }

        // Urgency (only if horizon exists)
        if let Some(urg) = &urgency {
            let pct = (urg.value * 100.0).min(999.0);
            if overdue {
                let days_past = (-urg.time_remaining as f64 / 86400.0).ceil() as i64;
                println!(
                    "  Urgency:  {} ({} days past deadline)",
                    palette.bold(&palette.danger("OVERDUE")),
                    days_past,
                );
            } else {
                let days_left = (urg.time_remaining as f64 / 86400.0).floor() as i64;
                println!(
                    "  Urgency:  {:.0}% elapsed ({} days remaining)",
                    pct, days_left
                );
            }
        }

        // === Signals (by exception — only shown when something needs attention) ===
        let has_hub = structural.centrality.map(|c| c > sig.hub_centrality).unwrap_or(false);
        let has_spine = structural.on_longest_path;
        let has_reach = structural.descendant_count.map(|c| c > sig.reach_descendants as usize).unwrap_or(false);
        let has_signals = temporal.on_critical_path
            || temporal.has_containment_violation
            || !temporal.sequencing_pressures.is_empty()
            || !temporal.critical_path.is_empty()
            || !temporal.containment_violations.is_empty()
            || temporal.implied_window.as_ref().map(|w| w.duration_seconds < 0).unwrap_or(false)
            || horizon_drift.is_some()
            || has_hub || has_spine || has_reach;

        if has_signals {
            println!();
            println!("{}", palette.bold(&palette.structure("Signals:")));

            // Label colors mirror the palette semantics:
            //   CRITICAL / HUB / SPINE / REACH  → structure (cyan)
            //   VIOLATION                        → danger (red)
            //   PRESSURE / DRIFT / WINDOW        → warning (yellow)
            // Whitespace inside the strings is preserved exactly so that
            // non-TTY output is byte-identical to the pre-Phase-2 baseline.
            if temporal.on_critical_path {
                println!(
                    "  {} {}   on parent's critical path",
                    palette.structure(glyphs::SIGNAL_CRITICAL_PATH),
                    palette.structure("CRITICAL"),
                );
            }
            if temporal.has_containment_violation {
                println!(
                    "  {} {}  deadline exceeds parent's deadline",
                    palette.danger(glyphs::SIGNAL_CONTAINMENT),
                    palette.danger("VIOLATION"),
                );
            }
            for sp in &temporal.sequencing_pressures {
                let pred_display = display_id(sp.predecessor_short_code, &sp.predecessor_id);
                let days = sp.gap_seconds as f64 / 86400.0;
                println!(
                    "  {} {}   deadline is {:.0} days before {} (ordered after)",
                    palette.warning(glyphs::SIGNAL_SEQUENCING),
                    palette.warning("PRESSURE"),
                    days,
                    pred_display,
                );
            }
            for cpath in &temporal.critical_path {
                let child_sc = all_tensions.iter().find(|t| t.id == cpath.tension_id).and_then(|t| t.short_code);
                let child_display = display_id(child_sc, &cpath.tension_id);
                let slack_days = cpath.slack_seconds as f64 / 86400.0;
                if slack_days <= 0.0 {
                    println!(
                        "  {} {}   {} matches or exceeds deadline",
                        palette.structure(glyphs::SIGNAL_CRITICAL_PATH),
                        palette.structure("CRITICAL"),
                        child_display,
                    );
                } else {
                    println!(
                        "  {} {}   {} has only {:.0} days slack",
                        palette.structure(glyphs::SIGNAL_CRITICAL_PATH),
                        palette.structure("CRITICAL"),
                        child_display,
                        slack_days,
                    );
                }
            }
            for cv in &temporal.containment_violations {
                let child_sc = all_tensions.iter().find(|t| t.id == cv.tension_id).and_then(|t| t.short_code);
                let child_display = display_id(child_sc, &cv.tension_id);
                let excess_days = cv.excess_seconds as f64 / 86400.0;
                println!(
                    "  {} {}  {} deadline exceeds by {:.0} days",
                    palette.danger(glyphs::SIGNAL_CONTAINMENT),
                    palette.danger("VIOLATION"),
                    child_display,
                    excess_days,
                );
            }
            if let Some(ref iw) = temporal.implied_window {
                let days = iw.duration_seconds as f64 / 86400.0;
                if days < 0.0 {
                    println!(
                        "  {}      negative ({:.0} days past)",
                        palette.warning("WINDOW"),
                        -days,
                    );
                }
            }
            if let Some(ref drift) = horizon_drift {
                let net_days = drift.net_shift_seconds.abs() / 86400;
                let direction = if drift.net_shift_seconds > 0 { "+" } else { "-" };
                let since = drift.onset
                    .map(|ts| format!(" since {}", relative_time(ts, now)))
                    .unwrap_or_default();
                let desc = match drift.drift_type {
                    HorizonDriftType::Tightening => format!("tightened{} (net {}{}d)", since, direction, net_days),
                    HorizonDriftType::Postponement => format!("postponed{} (net +{}d)", since, net_days),
                    HorizonDriftType::RepeatedPostponement => format!("postponed {}\u{00d7}{} (net +{}d)", drift.change_count, since, net_days),
                    HorizonDriftType::Loosening => format!("loosening{} (net {}{}d)", since, direction, net_days),
                    HorizonDriftType::Oscillating => format!("oscillating{} ({} shifts, net {}{}d)", since, drift.change_count, direction, net_days),
                    HorizonDriftType::Stable => unreachable!(),
                };
                println!(
                    "  {} {}      {}",
                    palette.warning(glyphs::SIGNAL_DRIFT),
                    palette.warning("DRIFT"),
                    desc,
                );
            }
            if has_hub {
                println!(
                    "  {} {}        centrality {:.4} (structural routing point)",
                    palette.structure(glyphs::SIGNAL_HUB),
                    palette.structure("HUB"),
                    structural.centrality.unwrap_or(0.0),
                );
            }
            if has_spine {
                let depth = field_structural.longest_path.len();
                println!(
                    "  {} {}      on longest structural path (depth {})",
                    palette.structure(glyphs::SIGNAL_SPINE),
                    palette.structure("SPINE"),
                    depth,
                );
            }
            if has_reach {
                println!(
                    "  {} {}      {} transitive descendants",
                    palette.structure(glyphs::SIGNAL_REACH),
                    palette.structure("REACH"),
                    structural.descendant_count.unwrap_or(0),
                );
            }
        }

        // === Frontier (signal by exception) ===
        if frontier.closure_progress.total > 0 {
            let has_frontier_signals = frontier.next_step.is_some()
                || !frontier.overdue.is_empty()
                || !frontier.held.is_empty()
                || !frontier.recently_resolved.is_empty();

            if has_frontier_signals {
                println!();
                println!("{}", palette.bold(&palette.structure("Frontier:")));

                if let Some(ref ns) = frontier.next_step {
                    let ns_display = display_id(ns.short_code, &ns.tension_id);
                    let overdue_marker = if ns.is_overdue {
                        format!(" {}", palette.bold(&palette.danger("OVERDUE")))
                    } else {
                        String::new()
                    };
                    println!(
                        "  Next:     {}{} {}",
                        ns_display, overdue_marker, truncate(&ns.desired, 40)
                    );
                } else if !frontier.held.is_empty() {
                    let n = frontier.held.len();
                    let noun = if n == 1 { "step" } else { "steps" };
                    println!("  Sequence: uncommitted ({} held {})", n, noun);
                }

                if !frontier.overdue.is_empty() {
                    for step in &frontier.overdue {
                        let step_display = display_id(step.short_code, &step.tension_id);
                        println!(
                            "  Overdue:  {} {}",
                            step_display, truncate(&step.desired, 40)
                        );
                    }
                }

                if !frontier.held.is_empty() && frontier.next_step.is_some() {
                    println!("  Held:     {} unpositioned", frontier.held.len());
                }

                if !frontier.recently_resolved.is_empty() {
                    if epochs.is_empty() {
                        println!(
                            "  Recent:   {} resolved",
                            frontier.recently_resolved.len()
                        );
                    } else {
                        println!(
                            "  Recent:   {} resolved since last epoch",
                            frontier.recently_resolved.len()
                        );
                    }
                }
            }
        }

        // === Children ===
        if !result.children.is_empty() {
            println!();
            println!("{}", palette.bold(&palette.structure("Children:")));
            for child in &result.children {
                let child_id = display_id(child.short_code, &child.id);
                let status_marker = match child.status.as_str() {
                    "Resolved" => format!(" {}", palette.resolved(glyphs::STATUS_RESOLVED)),
                    "Released" => format!(" {}", palette.chrome(glyphs::STATUS_RELEASED)),
                    _ => String::new(),
                };
                println!(
                    "  {}{} {}",
                    child_id, status_marker, &child.desired
                );
            }
        }

        // === Activity (last 10, most recent first, concise summaries) ===
        if !result.mutations.is_empty() {
            println!();
            println!("{}", palette.bold(&palette.structure("Activity:")));
            // Reverse to show most recent first
            for m in result.mutations.iter().rev() {
                let ts = DateTime::parse_from_rfc3339(&m.timestamp)
                    .map(|dt| relative_time(dt.with_timezone(&Utc), now))
                    .unwrap_or_else(|_| m.timestamp[..19].replace('T', " "));

                let summary = format_mutation_summary(&m.field, m.old_value.as_deref(), &m.new_value);
                println!("  {}  {}", palette.chrome(&format!("{:>12}", ts)), summary);
            }
        }

        // === Epochs (compact summary, --full expands) ===
        if !epochs.is_empty() {
            println!();
            let latest = &epochs[epochs.len() - 1];
            let age = relative_time(latest.timestamp, now);
            if full {
                println!("{}", palette.bold(&palette.structure(&format!("Epochs ({}):", epochs.len()))));
                for (i, e) in epochs.iter().enumerate().rev() {
                    let e_age = relative_time(e.timestamp, now);
                    println!(
                        "  {:>3}.  {}  desire: {}",
                        i + 1,
                        e_age,
                        truncate(&e.desire_snapshot, 50),
                    );
                }
            } else {
                println!(
                    "Epochs:     {} (latest {})",
                    epochs.len(),
                    age,
                );
            }
        }

        // === Context (--full only) ===
        if full {
            if let Some(ref ancestors) = result.ancestors {
                if !ancestors.is_empty() {
                    println!();
                    println!("{}", palette.bold(&palette.structure("Ancestors:")));
                    for a in ancestors {
                        let sc = a.short_code.map(|c| format!("#{}", c)).unwrap_or_else(|| a.id[..8.min(a.id.len())].to_string());
                        println!("  {:<6} {}", sc, truncate(&a.desired, 55));
                    }
                }
            }
            if let Some(ref siblings) = result.siblings {
                if !siblings.is_empty() {
                    println!();
                    println!("{}", palette.bold(&palette.structure("Siblings:")));
                    for s in siblings {
                        let sc = s.short_code.map(|c| format!("#{}", c)).unwrap_or_else(|| s.id[..8.min(s.id.len())].to_string());
                        let status_marker = match s.status.as_str() {
                            "Resolved" => format!(" {}", palette.resolved(glyphs::STATUS_RESOLVED)),
                            "Released" => format!(" {}", palette.chrome(glyphs::STATUS_RELEASED)),
                            _ => String::new(),
                        };
                        println!("  {}{} {}", sc, status_marker, truncate(&s.desired, 50));
                    }
                }
            }
            if let Some(ref eng) = result.engagement {
                println!();
                println!("{}", palette.bold(&palette.structure("Engagement:")));
                if let Some(freq) = eng.get("frequency_per_day").and_then(|v| v.as_f64()) {
                    println!("  Frequency: {:.1}/day", freq);
                }
                if let Some(trend) = eng.get("frequency_trend").and_then(|v| v.as_f64()) {
                    let trend_word = if trend > 0.1 { "accelerating" } else if trend < -0.1 { "declining" } else { "steady" };
                    println!("  Trend: {}", trend_word);
                }
                if let Some(count) = eng.get("mutation_count").and_then(|v| v.as_u64()) {
                    println!("  Mutations: {} in window", count);
                }
            }
        }

        // Footer hint — actionable suggestions tuned to what's on screen.
        // We pass the bare addressable form (short code without `#`, or
        // ULID prefix) into the example commands, since `werk show`
        // accepts both forms.
        let hint_id: String = match tension.short_code {
            Some(c) => c.to_string(),
            None => tension.id[..8.min(tension.id.len())].to_string(),
        };
        let hint = if overdue {
            format!(
                "overdue — `werk reality {} ...` to update, `werk horizon {} <new>` to reschedule",
                hint_id, hint_id
            )
        } else if tension.status == TensionStatus::Active {
            if !result.children.is_empty() {
                format!(
                    "`werk reality {} ...` to log progress, `werk resolve {}` when done",
                    hint_id, hint_id
                )
            } else if !full {
                format!(
                    "`werk show {} --full` for ancestors, siblings, engagement",
                    hint_id
                )
            } else {
                format!(
                    "`werk reality {} ...` to log progress, `werk resolve {}` when done",
                    hint_id, hint_id
                )
            }
        } else {
            format!(
                "`werk reopen {}` to reactivate, `werk log {}` for full history",
                hint_id, hint_id
            )
        };
        crate::hints::print_hint(&palette, &hint);
    }

    Ok(())
}

/// Format a horizon line for display, distinguishing own vs inherited.
fn print_horizon(h: &sd_core::Horizon, now: DateTime<Utc>, inherited_from: Option<&str>) {
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

    match inherited_from {
        None => println!(
            "  Deadline: {} ({}{})",
            &horizon_str, &interpretation, &days_str
        ),
        Some(ancestor) => println!(
            "  Deadline: none (parent {} due {}{})",
            ancestor, &horizon_str, &days_str
        ),
    }
}

/// Format a mutation into a concise human-readable summary.
///
/// Used by `show` (Activity section) and `diff` for consistent mutation display.
///
/// Instead of dumping full old→new text, produce a short description
/// of what changed. The desired/actual text is already shown at the top
/// of the display — no need to repeat it in the activity log.
pub fn format_mutation_summary(field: &str, old_value: Option<&str>, new_value: &str) -> String {
    match field {
        "created" => {
            // Creation mutation — don't repeat the desired/actual
            "created".to_string()
        }
        "status" => {
            match new_value {
                "Resolved" => "resolved".to_string(),
                "Released" => "released".to_string(),
                "Active" => match old_value {
                    Some(old) => format!("reopened (was {})", old.to_lowercase()),
                    None => "reopened".to_string(),
                },
                _ => format!("status -> {}", new_value),
            }
        }
        "desired" => "desired updated".to_string(),
        "actual" => "reality updated".to_string(),
        "position" => {
            if new_value == "(none)" || new_value == "null" {
                "held (removed from sequence)".to_string()
            } else {
                match old_value {
                    None | Some("(none)") | Some("null") => format!("positioned at {}", new_value),
                    Some(_) => format!("repositioned to {}", new_value),
                }
            }
        }
        "parent" => {
            if new_value == "(none)" {
                "moved to root".to_string()
            } else {
                format!("moved under {}", truncate(new_value, 30))
            }
        }
        "horizon" => {
            if new_value == "(none)" {
                "deadline cleared".to_string()
            } else {
                format!("deadline set to {}", new_value)
            }
        }
        "note" => {
            if old_value.is_some() {
                format!("note retracted: {}", truncate(new_value, 50))
            } else {
                format!("note: {}", truncate(new_value, 50))
            }
        }
        "deleted" => "deleted".to_string(),
        "snoozed_until" => {
            if new_value == "(none)" {
                "snooze cleared".to_string()
            } else {
                format!("snoozed until {}", new_value)
            }
        }
        "recurrence" => {
            if new_value == "(none)" {
                "recurrence cleared".to_string()
            } else {
                format!("recurrence set to {}", new_value)
            }
        }
        _ => {
            // Fallback for unknown fields
            format!("{} updated", field)
        }
    }
}
