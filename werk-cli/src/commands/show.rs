//! Show command handler — the CLI briefing surface for a single tension.
//!
//! ## Design Language
//!
//! The visual vocabulary is shared with `tree.rs` and the canonical glyph
//! registry in `werk-shared/src/cli_display/glyphs.rs`:
//!
//! - **Zone boundaries**: `╭─`/`╰─` frame the identity zone (desire above,
//!   reality below — the spatial law).
//! - **Section rules**: `─── Name ─────────── suffix` for every section header.
//!   Continuous `─` characters (TREE_HORIZONTAL). No dashed variants.
//! - **Content width**: capped at 72 columns regardless of terminal width.
//!   Prevents two-column metadata from spreading across wide terminals.
//! - **Column grid**: children align to a fixed grid — `#ID` in col 2-7,
//!   status glyph in col 8-11, text from col 12.
//! - **Typography**: Bold for IDs + section names. Chrome (dim) for metadata,
//!   timestamps, chrome punctuation. Structure (cyan) for section rules and
//!   signal glyphs. The seven palette roles carry the same meaning as in tree.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::serialize::{HorizonRangeJson, TensionInfo, node_to_tension_info};
use chrono::{DateTime, Utc};
use serde::Serialize;
use werk_core::{
    Address, HorizonDriftType, TensionStatus, compute_frontier, compute_structural_signals,
    compute_temporal_signals, compute_urgency, detect_horizon_drift, extract_mutation_pattern,
    gap_magnitude,
};
use werk_shared::cli_display::glyphs;
use werk_shared::cross_space;
use werk_shared::{display_id, relative_time, truncate};

/// Resolve a tension ID within the local (CWD-discovered) workspace.
fn resolve_local(
    id: &str,
) -> Result<
    (
        crate::workspace::Workspace,
        werk_core::Store,
        Vec<werk_core::Tension>,
        werk_core::Tension,
    ),
    WerkError,
> {
    let workspace = crate::workspace::Workspace::discover()?;
    let store = workspace.open_store()?;
    let all_tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(all_tensions.clone());
    let tension = resolver.resolve(id)?.clone();
    Ok((workspace, store, all_tensions, tension))
}

/// Content width cap. All text, rules, and alignment targets stay within
/// this many columns. Wide terminals get a left-aligned block, not a
/// spread-out mess.
const CONTENT_WIDTH: usize = 72;

/// Flags controlling which sections are expanded.
pub struct ShowFlags {
    pub brief: bool,
    pub notes: bool,
    pub route: bool,
    pub activity: bool,
    pub epochs: bool,
    pub context: bool,
    pub history: bool,
}

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
    frontier: werk_core::Frontier,
    temporal: werk_core::TemporalSignals,
    structural: werk_core::StructuralSignals,
    #[serde(skip_serializing_if = "Option::is_none")]
    horizon_drift: Option<werk_core::HorizonDrift>,
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
    /// "present" (filtered to the latest epoch boundary) or "all" (full history).
    epoch_scope: &'static str,
    /// Timestamp of the present epoch boundary, when scoped to "present".
    #[serde(skip_serializing_if = "Option::is_none")]
    epoch_boundary: Option<String>,
}

/// JSON output structure for a sigil record.
#[derive(Serialize)]
struct SigilShowJson {
    short_code: i32,
    scope: String,
    logic: String,
    logic_version: String,
    seed: i64,
    rendered_at: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    completion_ts: Option<DateTime<Utc>>,
    /// Full desired text (for --route mode), not truncated.
    #[serde(skip_serializing)]
    desired_full: String,
    /// Reality text (for --route mode).
    #[serde(skip_serializing)]
    actual: String,
    /// Horizon string (for --route mode).
    #[serde(skip_serializing)]
    horizon: Option<String>,
}

pub fn cmd_show(output: &Output, id: String, flags: ShowFlags) -> Result<(), WerkError> {
    if let Ok(addr) = werk_core::parse_address(&id) {
        if let Address::Sigil(code) = addr {
            return cmd_show_sigil(output, code);
        }
        if let Address::CrossSpace {
            ref space,
            ref inner,
        } = addr
        {
            if matches!(**inner, Address::Sigil(_)) {
                return Err(WerkError::InvalidInput(
                    "cross-space sigil addresses are not supported".to_string(),
                ));
            }
            let result = cross_space::resolve_cross_space(space, inner)?;
            return cmd_show_tension(
                output,
                flags,
                result.workspace,
                result.store,
                result.all_tensions,
                result.tension,
            );
        }
    }

    let (workspace, store, all_tensions, tension) = resolve_local(&id)?;
    cmd_show_tension(output, flags, workspace, store, all_tensions, tension)
}

fn cmd_show_tension(
    output: &Output,
    flags: ShowFlags,
    workspace: crate::workspace::Workspace,
    store: werk_core::Store,
    all_tensions: Vec<werk_core::Tension>,
    tension: werk_core::Tension,
) -> Result<(), WerkError> {
    let sig = crate::commands::signal_thresholds_from(&workspace);
    let analysis = crate::commands::analysis_thresholds_from(&workspace);

    let mutations = store
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    let forest = werk_core::Forest::from_tensions(all_tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    let raw_children = forest.children(&tension.id).unwrap_or_default();
    let child_mutations: Vec<(String, Vec<werk_core::Mutation>)> = raw_children
        .iter()
        .filter_map(|c| {
            let muts = store.get_mutations(&c.id()).ok()?;
            Some((c.id().to_string(), muts))
        })
        .collect();

    let mut children: Vec<ChildInfo> = raw_children
        .iter()
        .map(|child| {
            let completion_ts = child_mutations
                .iter()
                .find(|(cid, _)| cid == child.id())
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
                desired: truncate(&child.tension.desired, CONTENT_WIDTH.saturating_sub(13)),
                status: child.tension.status.to_string(),
                position: child.tension.position,
                completion_ts,
                desired_full: child.tension.desired.clone(),
                actual: child.tension.actual.clone(),
                horizon: child.tension.horizon.as_ref().map(|h| h.to_string()),
            }
        })
        .collect();
    children.sort_by(|a, b| {
        fn sort_key(c: &ChildInfo) -> (u8, i64, i32) {
            match (c.status.as_str(), c.position) {
                (_, Some(pos)) if c.status == "Active" => (0, 0, pos),
                ("Active", None) => (1, 0, 0),
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

    let epochs = store
        .get_epochs(&tension.id)
        .map_err(WerkError::StoreError)?;
    let frontier = compute_frontier(&forest, &tension.id, now, &epochs, &child_mutations);

    // Present-epoch boundary: timestamp of the most recent epoch. When --history
    // is set (or no epochs exist), no filtering is applied.
    let epoch_boundary: Option<DateTime<Utc>> = if flags.history {
        None
    } else {
        epochs.last().map(|e| e.timestamp)
    };

    if let Some(boundary) = epoch_boundary {
        children.retain(|c| match c.status.as_str() {
            "Resolved" | "Released" => c.completion_ts.map(|ts| ts >= boundary).unwrap_or(true),
            _ => true,
        });
    }
    let urgency = compute_urgency(&tension, now);
    let overdue = tension.status == TensionStatus::Active
        && tension
            .horizon
            .as_ref()
            .map(|h| h.is_past(now))
            .unwrap_or(false);
    let temporal = compute_temporal_signals(&forest, &tension.id, now);
    let field_structural = compute_structural_signals(&forest);
    let structural = field_structural
        .signals
        .get(&tension.id)
        .cloned()
        .unwrap_or_default();
    let horizon_drift = {
        let drift = detect_horizon_drift(&tension.id, &mutations);
        if drift.drift_type != HorizonDriftType::Stable {
            Some(drift)
        } else {
            None
        }
    };

    let scoped_mutations: Vec<&werk_core::Mutation> = if let Some(boundary) = epoch_boundary {
        mutations
            .iter()
            .filter(|m| m.timestamp() >= boundary)
            .collect()
    } else {
        mutations.iter().collect()
    };
    let mutation_limit = if flags.activity {
        scoped_mutations.len()
    } else {
        10
    };
    let mutation_infos: Vec<ShowMutationInfo> = scoped_mutations
        .iter()
        .rev()
        .take(mutation_limit)
        .rev()
        .map(|m| ShowMutationInfo {
            timestamp: m.timestamp().to_rfc3339(),
            field: m.field().to_owned(),
            old_value: m.old_value().map(|s| s.to_owned()),
            new_value: m.new_value().to_owned(),
        })
        .collect();

    let include_context = flags.context || output.is_structured();
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
        let pattern = extract_mutation_pattern(
            &tension,
            &mutations,
            proj_thresholds.pattern_window_seconds,
            now,
        );
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
        epoch_scope: if flags.history { "all" } else { "present" },
        epoch_boundary: epoch_boundary.map(|ts| ts.to_rfc3339()),
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        let palette = output.palette();
        let scoped_owned: Vec<werk_core::Mutation> =
            scoped_mutations.iter().map(|m| (*m).clone()).collect();
        let render_mutations: &[werk_core::Mutation] = if epoch_boundary.is_some() {
            &scoped_owned
        } else {
            &mutations
        };
        render_human(
            &result,
            &tension,
            &all_tensions,
            render_mutations,
            &epochs,
            &frontier,
            &temporal,
            &structural,
            &field_structural,
            &horizon_drift,
            &urgency,
            overdue,
            now,
            &palette,
            &flags,
            &sig,
        );
    }

    Ok(())
}

fn cmd_show_sigil(output: &Output, code: i32) -> Result<(), WerkError> {
    let workspace = crate::workspace::Workspace::discover()?;
    let store = workspace.open_store()?;
    let record = store
        .get_sigil_by_short_code(code)
        .map_err(WerkError::StoreError)?
        .ok_or_else(|| WerkError::TensionNotFound(format!("sigil *{code}")))?;

    if output.is_structured() {
        let result = SigilShowJson {
            short_code: record.short_code,
            scope: record.scope_canonical,
            logic: record.logic_id,
            logic_version: record.logic_version,
            seed: record.seed,
            rendered_at: record.rendered_at.to_rfc3339(),
            path: record.file_path,
            label: record.label,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        println!("Sigil *{}", record.short_code);
        println!("  Scope:   {}", record.scope_canonical);
        println!("  Logic:   {}@{}", record.logic_id, record.logic_version);
        println!("  Seed:    {}", record.seed);
        println!("  Rendered {}", record.rendered_at.to_rfc3339());
        println!("  Path:    {}", record.file_path);
        if let Some(label) = record.label {
            println!("  Label:   {}", label);
        }
    }

    Ok(())
}

// ============================================================================
// Human-readable rendering
// ============================================================================

/// Content-width horizontal rule using TREE_HORIZONTAL.
fn rule(w: usize) -> String {
    glyphs::TREE_HORIZONTAL.repeat(w)
}

/// Section header: `─── Name ──────────────────── suffix`
fn section_header(name: &str, suffix: &str, palette: &werk_shared::cli_display::Palette) -> String {
    let prefix_rule = format!("{} ", rule(3));
    let name_part = format!("{} ", name);
    let used = 4 + name_part.len(); // "─── Name "
    let suffix_part = if suffix.is_empty() {
        String::new()
    } else {
        format!(" {}", suffix)
    };
    let fill = CONTENT_WIDTH
        .saturating_sub(used + suffix_part.len())
        .max(3);
    format!(
        "{}{}{}{}",
        palette.structure(&prefix_rule),
        palette.bold(&palette.structure(name)),
        palette.structure(&format!(" {}", rule(fill))),
        palette.chrome(&suffix_part),
    )
}

/// Sub-section divider: `  ─── label ──────────────── count`
fn sub_divider(label: &str, count: &str, palette: &werk_shared::cli_display::Palette) -> String {
    let prefix = "  ";
    let label_part = format!("{} {} ", rule(3), label);
    let used = prefix.len() + label_part.len() + count.len() + 1;
    let fill = CONTENT_WIDTH.saturating_sub(used).max(3);
    format!(
        "{}{}{}{}",
        prefix,
        palette.chrome(&label_part),
        palette.chrome(&rule(fill)),
        palette.chrome(&format!(" {}", count)),
    )
}

/// Print left and right text aligned within CONTENT_WIDTH.
fn two_col(left: &str, right: &str) {
    if right.is_empty() {
        println!("{}", left);
        return;
    }
    let left_vis = strip_ansi(left).len();
    let right_vis = strip_ansi(right).len();
    let gap = CONTENT_WIDTH.saturating_sub(left_vis + right_vis).max(2);
    print!("{}", left);
    print!("{:width$}", "", width = gap);
    println!("{}", right);
}

#[allow(clippy::too_many_arguments)]
fn render_human(
    result: &ShowResult,
    tension: &werk_core::Tension,
    all_tensions: &[werk_core::Tension],
    mutations: &[werk_core::Mutation],
    epochs: &[werk_core::EpochRecord],
    frontier: &werk_core::Frontier,
    temporal: &werk_core::TemporalSignals,
    structural: &werk_core::StructuralSignals,
    field_structural: &werk_core::FieldStructuralSignals,
    horizon_drift: &Option<werk_core::HorizonDrift>,
    urgency: &Option<werk_core::Urgency>,
    overdue: bool,
    now: DateTime<Utc>,
    palette: &werk_shared::cli_display::Palette,
    flags: &ShowFlags,
    sig: &crate::commands::SignalThresholds,
) {
    let _text_width = CONTENT_WIDTH.saturating_sub(4); // indent allowance

    // ── Identity zone ──────────────────────────────────────────────
    // Uses tree's ╭─/╰─ zone boundaries. Desire is top anchor, reality
    // is bottom anchor. The gap between them IS the tension.
    let id_str = display_id(tension.short_code, &tension.id);
    let id_prefix_len = 3 + id_str.len() + 2; // "╭─ #N  "
    let desire_width = CONTENT_WIDTH.saturating_sub(id_prefix_len).max(40);
    let desire_lines = wrap_text(&tension.desired, desire_width);
    for (i, line) in desire_lines.iter().enumerate() {
        if i == 0 {
            println!(
                "{} {}  {}",
                palette.structure(glyphs::TREE_ZONE_OPEN),
                palette.bold(&id_str),
                line,
            );
        } else {
            println!("{:indent$}{}", "", line, indent = id_prefix_len);
        }
    }

    let reality_lines = wrap_text(&tension.actual, CONTENT_WIDTH.saturating_sub(3));
    for (i, line) in reality_lines.iter().enumerate() {
        if i == 0 {
            println!(
                "{} {}",
                palette.structure(glyphs::TREE_ZONE_CLOSE),
                palette.chrome(line),
            );
        } else {
            println!("   {}", palette.chrome(line));
        }
    }

    // ── Metadata ───────────────────────────────────────────────────
    // Three compact lines. Right column aligns within CONTENT_WIDTH.
    println!();
    render_metadata(
        tension,
        all_tensions,
        mutations,
        frontier,
        structural,
        field_structural,
        urgency,
        overdue,
        now,
        palette,
    );

    // ── Signals ────────────────────────────────────────────────────
    render_signals(
        all_tensions,
        temporal,
        structural,
        field_structural,
        horizon_drift,
        now,
        palette,
        sig,
    );

    if flags.brief {
        render_hint(tension, overdue, &result.children, palette);
        return;
    }

    // ── Theory of Closure ──────────────────────────────────────────
    if !result.children.is_empty() {
        render_theory_of_closure(&result.children, frontier, palette, flags);
    }

    // ── Notes ──────────────────────────────────────────────────────
    render_notes(mutations, now, palette, flags);

    // ── Activity ───────────────────────────────────────────────────
    render_activity(&result.mutations, now, palette, flags, mutations.len());

    // ── Epochs ─────────────────────────────────────────────────────
    render_epochs(epochs, now, palette, flags);

    // ── Context ────────────────────────────────────────────────────
    if flags.context {
        render_context(result, palette);
    }

    render_hint(tension, overdue, &result.children, palette);
}

// ============================================================================
// Section renderers
// ============================================================================

fn render_metadata(
    tension: &werk_core::Tension,
    all_tensions: &[werk_core::Tension],
    mutations: &[werk_core::Mutation],
    frontier: &werk_core::Frontier,
    structural: &werk_core::StructuralSignals,
    field_structural: &werk_core::FieldStructuralSignals,
    urgency: &Option<werk_core::Urgency>,
    overdue: bool,
    now: DateTime<Utc>,
    palette: &werk_shared::cli_display::Palette,
) {
    // Line 1: state + structural position | age
    let mut left_parts: Vec<String> = vec![tension.status.to_string()];
    if let Some(pos) = tension.position {
        left_parts.push(format!("position {}", pos));
    } else if tension.parent_id.is_some() {
        left_parts.push("held".to_string());
    }
    if let Some(wave) = structural.wave {
        left_parts.push(format!("wave {}/{}", wave + 1, field_structural.wave_count));
    }
    let left1 = format!(
        "  {}",
        left_parts.join(&format!(" {} ", palette.chrome("\u{00b7}")))
    );
    let age_str = relative_time(tension.created_at, now);
    let right1 = palette.chrome(&age_str.replace(" ago", " old"));
    two_col(&left1, &right1);

    // Line 2: temporal situation | last act
    let mut left2_parts: Vec<String> = Vec::new();
    if let Some(h) = &tension.horizon {
        let days_remaining = h.range_end().signed_duration_since(now).num_days();
        if overdue {
            left2_parts.push(format!(
                "{} ({} days past)",
                palette.bold(&palette.danger(&h.to_string())),
                palette.danger(&(-days_remaining).to_string())
            ));
        } else {
            left2_parts.push(format!("{} ({} days)", h, days_remaining));
            if let Some(urg) = urgency {
                let pct = (urg.value * 100.0).min(999.0);
                left2_parts.push(format!(
                    "{} \u{2014} {:.0}%",
                    werk_shared::value_labels::urgency_label(urg.value),
                    pct,
                ));
            }
        }
    } else {
        // Inherited horizon
        let mut ancestor_id = tension.parent_id.clone();
        while let Some(aid) = &ancestor_id {
            if let Some(ancestor) = all_tensions.iter().find(|t| &t.id == aid) {
                if let Some(h) = &ancestor.horizon {
                    let ad = display_id(ancestor.short_code, &ancestor.id);
                    let days = h.range_end().signed_duration_since(now).num_days();
                    left2_parts.push(
                        palette.chrome(&format!("no deadline ({} due {}, {}d)", ad, h, days)),
                    );
                    break;
                }
                ancestor_id = ancestor.parent_id.clone();
            } else {
                break;
            }
        }
    }
    let right2 = if let Some(last) = mutations.last() {
        palette.chrome(&format!(
            "last act {} ({})",
            relative_time(last.timestamp(), now),
            last.field()
        ))
    } else {
        String::new()
    };
    if !left2_parts.is_empty() || !right2.is_empty() {
        let left2 = if left2_parts.is_empty() {
            "  ".to_string()
        } else {
            format!(
                "  {}",
                left2_parts.join(&format!(" {} ", palette.chrome("\u{00b7}")))
            )
        };
        two_col(&left2, &right2);
    }

    // Line 3: closure progress | parent context
    let cp = &frontier.closure_progress;
    let left3 = if cp.total > 0 {
        if cp.released > 0 {
            format!(
                "  {}/{} done ({} released)",
                cp.resolved, cp.total, cp.released
            )
        } else {
            format!("  {}/{} done", cp.resolved, cp.total)
        }
    } else {
        "  ".to_string()
    };
    let right3 = if let Some(pid) = &tension.parent_id {
        let parent = all_tensions.iter().find(|t| &t.id == pid);
        let pd = display_id(parent.and_then(|t| t.short_code), pid);
        let pn = parent.map(|t| truncate(&t.desired, 30)).unwrap_or_default();
        palette.chrome(&format!("under {} {}", pd, pn))
    } else {
        palette.chrome("root tension")
    };
    two_col(&left3, &right3);
}

fn render_signals(
    all_tensions: &[werk_core::Tension],
    temporal: &werk_core::TemporalSignals,
    structural: &werk_core::StructuralSignals,
    field_structural: &werk_core::FieldStructuralSignals,
    horizon_drift: &Option<werk_core::HorizonDrift>,
    now: DateTime<Utc>,
    palette: &werk_shared::cli_display::Palette,
    sig: &crate::commands::SignalThresholds,
) {
    let has_hub = structural
        .centrality
        .map(|c| c > sig.hub_centrality)
        .unwrap_or(false);
    let has_spine = structural.on_longest_path;
    let has_reach = structural
        .descendant_count
        .map(|c| c > sig.reach_descendants as usize)
        .unwrap_or(false);
    let has_signals = temporal.on_critical_path
        || temporal.has_containment_violation
        || !temporal.sequencing_pressures.is_empty()
        || !temporal.critical_path.is_empty()
        || !temporal.containment_violations.is_empty()
        || temporal
            .implied_window
            .as_ref()
            .map(|w| w.duration_seconds < 0)
            .unwrap_or(false)
        || temporal.position_gaps.is_some()
        || horizon_drift.is_some()
        || has_hub
        || has_spine
        || has_reach;

    if !has_signals {
        return;
    }

    println!();
    println!("{}", section_header("Signals", "", palette));

    use werk_shared::cli_display::Palette;
    let s = Palette::structure;
    let d = Palette::danger;
    let w = Palette::warning;

    if temporal.on_critical_path {
        signal_line(
            glyphs::SIGNAL_CRITICAL_PATH,
            "CRITICAL",
            "on parent's critical path",
            s,
            palette,
        );
    }
    if temporal.has_containment_violation {
        signal_line(
            glyphs::SIGNAL_CONTAINMENT,
            "VIOLATION",
            "deadline exceeds parent's deadline",
            d,
            palette,
        );
    }
    for sp in &temporal.sequencing_pressures {
        let pred_display = display_id(sp.predecessor_short_code, &sp.predecessor_id);
        let days = sp.gap_seconds as f64 / 86400.0;
        signal_line(
            glyphs::SIGNAL_SEQUENCING,
            "PRESSURE",
            &format!(
                "deadline is {:.0} days before {} (ordered after)",
                days, pred_display
            ),
            w,
            palette,
        );
    }

    // Collapse homogeneous critical path signals
    if !temporal.critical_path.is_empty() {
        let all_zero = temporal
            .critical_path
            .iter()
            .all(|cp| cp.slack_seconds <= 0);
        if temporal.critical_path.len() > 3 && all_zero {
            let all_unpos = temporal.critical_path.iter().all(|cp| {
                all_tensions
                    .iter()
                    .find(|t| t.id == cp.tension_id)
                    .map(|t| t.position.is_none())
                    .unwrap_or(false)
            });
            let qualifier = if all_unpos { " (none positioned)" } else { "" };
            signal_line(
                glyphs::SIGNAL_CRITICAL_PATH,
                "CRITICAL",
                &format!(
                    "{} children match or exceed deadline{}",
                    temporal.critical_path.len(),
                    qualifier
                ),
                s,
                palette,
            );
        } else {
            for cpath in &temporal.critical_path {
                let child_sc = all_tensions
                    .iter()
                    .find(|t| t.id == cpath.tension_id)
                    .and_then(|t| t.short_code);
                let cd = display_id(child_sc, &cpath.tension_id);
                let slack_days = cpath.slack_seconds as f64 / 86400.0;
                if slack_days <= 0.0 {
                    signal_line(
                        glyphs::SIGNAL_CRITICAL_PATH,
                        "CRITICAL",
                        &format!("{} matches or exceeds deadline", cd),
                        s,
                        palette,
                    );
                } else {
                    signal_line(
                        glyphs::SIGNAL_CRITICAL_PATH,
                        "CRITICAL",
                        &format!("{} has only {:.0} days slack", cd, slack_days),
                        s,
                        palette,
                    );
                }
            }
        }
    }

    if let Some(pg) = &temporal.position_gaps {
        let missing_str = format_missing_positions(&pg.missing);
        signal_line(
            glyphs::SIGNAL_POSITION_GAPS,
            "GAPS",
            &format!(
                "{} positioned children span {}..{} (gaps at {})",
                pg.positioned_count, pg.min_position, pg.max_position, missing_str,
            ),
            w,
            palette,
        );
    }

    // Collapse homogeneous containment violations
    if !temporal.containment_violations.is_empty() {
        if temporal.containment_violations.len() > 3 {
            signal_line(
                glyphs::SIGNAL_CONTAINMENT,
                "VIOLATION",
                &format!(
                    "{} children exceed parent deadline",
                    temporal.containment_violations.len()
                ),
                d,
                palette,
            );
        } else {
            for cv in &temporal.containment_violations {
                let child_sc = all_tensions
                    .iter()
                    .find(|t| t.id == cv.tension_id)
                    .and_then(|t| t.short_code);
                let cd = display_id(child_sc, &cv.tension_id);
                let excess_days = cv.excess_seconds as f64 / 86400.0;
                signal_line(
                    glyphs::SIGNAL_CONTAINMENT,
                    "VIOLATION",
                    &format!("{} deadline exceeds by {:.0} days", cd, excess_days),
                    d,
                    palette,
                );
            }
        }
    }

    if let Some(iw) = &temporal.implied_window {
        let days = iw.duration_seconds as f64 / 86400.0;
        if days < 0.0 {
            signal_line(
                " ",
                "WINDOW",
                &format!("negative ({:.0} days past)", -days),
                w,
                palette,
            );
        }
    }
    if let Some(drift) = horizon_drift {
        let net_days = drift.net_shift_seconds.abs() / 86400;
        let direction = if drift.net_shift_seconds > 0 {
            "+"
        } else {
            "-"
        };
        let since = drift
            .onset
            .map(|ts| format!(" since {}", relative_time(ts, now)))
            .unwrap_or_default();
        let desc = match drift.drift_type {
            HorizonDriftType::Tightening => {
                format!("tightened{} (net {}{}d)", since, direction, net_days)
            }
            HorizonDriftType::Postponement => format!("postponed{} (net +{}d)", since, net_days),
            HorizonDriftType::RepeatedPostponement => format!(
                "postponed {}\u{00d7}{} (net +{}d)",
                drift.change_count, since, net_days
            ),
            HorizonDriftType::Loosening => {
                format!("loosening{} (net {}{}d)", since, direction, net_days)
            }
            HorizonDriftType::Oscillating => format!(
                "oscillating{} ({} shifts, net {}{}d)",
                since, drift.change_count, direction, net_days
            ),
            HorizonDriftType::Stable => {
                unreachable!("horizon_drift is only Some when drift_type != Stable")
            }
        };
        signal_line(glyphs::SIGNAL_DRIFT, "DRIFT", &desc, w, palette);
    }
    if has_hub {
        signal_line(
            glyphs::SIGNAL_HUB,
            "HUB",
            &format!(
                "centrality {:.4} (structural routing point)",
                structural.centrality.unwrap_or(0.0)
            ),
            s,
            palette,
        );
    }
    if has_spine {
        signal_line(
            glyphs::SIGNAL_SPINE,
            "SPINE",
            &format!(
                "on longest structural path (depth {})",
                field_structural.longest_path.len()
            ),
            s,
            palette,
        );
    }
    if has_reach {
        signal_line(
            glyphs::SIGNAL_REACH,
            "REACH",
            &format!(
                "{} transitive descendants",
                structural.descendant_count.unwrap_or(0)
            ),
            s,
            palette,
        );
    }
}

fn render_theory_of_closure(
    children: &[ChildInfo],
    frontier: &werk_core::Frontier,
    palette: &werk_shared::cli_display::Palette,
    flags: &ShowFlags,
) {
    let cp = &frontier.closure_progress;

    // Build suffix: "17/39 · 12 held"
    let mut suffix_parts: Vec<String> = Vec::new();
    if cp.total > 0 {
        suffix_parts.push(format!("{}/{}", cp.resolved, cp.total));
    }
    let held_count = children
        .iter()
        .filter(|c| c.status == "Active" && c.position.is_none())
        .count();
    if held_count > 0 {
        suffix_parts.push(format!("{} held", held_count));
    }

    println!();
    println!(
        "{}",
        section_header(
            "Theory of Closure",
            &suffix_parts.join(" \u{00b7} "),
            palette,
        )
    );

    // Partition into zones
    let positioned: Vec<&ChildInfo> = children
        .iter()
        .filter(|c| c.status == "Active" && c.position.is_some())
        .collect();
    let held: Vec<&ChildInfo> = children
        .iter()
        .filter(|c| c.status == "Active" && c.position.is_none())
        .collect();
    let done: Vec<&ChildInfo> = children
        .iter()
        .filter(|c| c.status == "Resolved" || c.status == "Released")
        .collect();

    // ── Positioned children ──
    // Format: "  #ID  ▸N  desire text..."
    // Column grid: col 2 = #ID (left-padded to 5), col 8 = glyph, col 12 = text
    if positioned.is_empty() && !held.is_empty() {
        println!("  {}", palette.chrome("No committed sequence"));
    }
    for child in &positioned {
        let child_id = display_id(child.short_code, &child.id);
        let pos = child.position.unwrap_or(0);
        let pos_glyph = format!("{}{}", glyphs::STATUS_POSITION, pos);
        let is_next = frontier
            .next_step
            .as_ref()
            .map(|ns| ns.tension_id == child.id)
            .unwrap_or(false);
        let is_overdue_child = frontier.overdue.iter().any(|o| o.tension_id == child.id);

        let id_col = format!("{:<5}", child_id);
        let glyph_col = format!("{:<4}", pos_glyph);

        // In --route mode, wrap full desire text; otherwise use truncated version
        let text_lines = if flags.route {
            let text_width = CONTENT_WIDTH.saturating_sub(13); // 2 + 5 + 1 + 4 + 1
            wrap_text(&child.desired_full, text_width)
        } else {
            vec![child.desired.clone()]
        };

        for (line_idx, desired_text) in text_lines.iter().enumerate() {
            if line_idx == 0 {
                // First line: include ID and glyph
                if is_next && is_overdue_child {
                    println!(
                        "  {} {} {} {}",
                        palette.resolved(&id_col),
                        palette.resolved(&glyph_col),
                        palette.bold(&palette.danger("OVERDUE")),
                        desired_text,
                    );
                } else if is_next {
                    println!(
                        "  {} {} {}",
                        palette.resolved(&id_col),
                        palette.resolved(&glyph_col),
                        desired_text
                    );
                } else if is_overdue_child {
                    println!(
                        "  {} {} {} {}",
                        palette.bold(&id_col),
                        palette.chrome(&glyph_col),
                        palette.bold(&palette.danger("OVERDUE")),
                        desired_text,
                    );
                } else {
                    println!(
                        "  {} {} {}",
                        palette.bold(&id_col),
                        palette.chrome(&glyph_col),
                        desired_text
                    );
                }
            } else {
                // Continuation lines: align to text column (12 spaces)
                println!("{:<13}{}", "", desired_text);
            }
        }

        if flags.route {
            render_child_detail(child, palette);
        }
    }

    // ── Held children ──
    if !held.is_empty() {
        println!("{}", sub_divider("held", &held.len().to_string(), palette));
        for child in &held {
            let child_id = display_id(child.short_code, &child.id);

            // In --route mode, wrap full desire text; otherwise use truncated version
            let text_lines = if flags.route {
                let text_width = CONTENT_WIDTH.saturating_sub(13); // 2 + 5 + 4 + 2
                wrap_text(&child.desired_full, text_width)
            } else {
                vec![child.desired.clone()]
            };

            for (line_idx, desired_text) in text_lines.iter().enumerate() {
                if line_idx == 0 {
                    println!(
                        "  {} {:<4} {}",
                        palette.chrome(&format!("{:<5}", child_id)),
                        "",
                        palette.chrome(desired_text)
                    );
                } else {
                    println!("{:<13}{}", "", palette.chrome(desired_text));
                }
            }

            if flags.route {
                render_child_detail(child, palette);
            }
        }
    }

    // ── Done children ──
    if !done.is_empty() {
        println!("{}", sub_divider("done", &done.len().to_string(), palette));
        for child in &done {
            let child_id = display_id(child.short_code, &child.id);
            let glyph = match child.status.as_str() {
                "Resolved" => palette.resolved(glyphs::STATUS_RESOLVED),
                "Released" => palette.chrome(glyphs::STATUS_RELEASED),
                _ => " ".to_string(),
            };

            // In --route mode, wrap full desire text; otherwise use truncated version
            let text_lines = if flags.route {
                let text_width = CONTENT_WIDTH.saturating_sub(13); // 2 + 5 + 3 + 3
                wrap_text(&child.desired_full, text_width)
            } else {
                vec![child.desired.clone()]
            };

            for (line_idx, desired_text) in text_lines.iter().enumerate() {
                if line_idx == 0 {
                    println!(
                        "  {} {:<3}  {}",
                        palette.chrome(&format!("{:<5}", child_id)),
                        glyph,
                        palette.chrome(desired_text),
                    );
                } else {
                    println!("{:<13}{}", "", palette.chrome(desired_text));
                }
            }
        }
    }
}

fn render_child_detail(child: &ChildInfo, palette: &werk_shared::cli_display::Palette) {
    let mut parts: Vec<String> = Vec::new();
    if let Some(ref h) = child.horizon {
        parts.push(h.clone());
    } else {
        parts.push("no deadline".to_string());
    }
    if !child.actual.is_empty() {
        // Keep line within CONTENT_WIDTH (72). Line format:
        // "  " (2) + ID (5) + " " (1) + glyph (4) + " " (1) + "no deadline · reality: " (23) + actual
        // Total before actual: 2+5+1+4+1+23 = 36. Available: 72-36 = 36 chars.
        parts.push(format!("reality: {}", truncate(&child.actual, 36)));
    } else {
        parts.push("no reality".to_string());
    }
    // Align with text column (col 13: 2 + 5 + 1 + 4 + 1)
    println!(
        "  {:<5} {:<4} {}",
        "",
        "",
        palette.chrome(&parts.join(" \u{00b7} "))
    );
}

fn render_notes(
    mutations: &[werk_core::Mutation],
    now: DateTime<Utc>,
    palette: &werk_shared::cli_display::Palette,
    flags: &ShowFlags,
) {
    let notes: Vec<&werk_core::Mutation> = mutations
        .iter()
        .filter(|m| m.field() == "note" && m.old_value().is_none())
        .collect();
    if notes.is_empty() {
        return;
    }

    println!();
    println!(
        "{}",
        section_header("Notes", &format!("{}", notes.len()), palette)
    );

    // Layout: "  {timestamp:>12}  {text}"
    // Text column starts at position 2 + 12 + 2 = 16.
    // Continuation lines pad to the same column 16.
    let ts_col = 12;
    let text_start = 2 + ts_col + 2; // column where text begins
    let text_width = CONTENT_WIDTH.saturating_sub(text_start).max(40);

    for note in notes.iter().rev() {
        let ts = relative_time(note.timestamp(), now);
        let text = note.new_value();

        let (lines, truncated) = if flags.notes {
            (wrap_text(text, text_width), false)
        } else {
            // Wrap the full text, then take exactly 2 lines.
            let all_lines = wrap_text(text, text_width);
            if all_lines.len() <= 2 {
                (all_lines, false)
            } else {
                let mut kept: Vec<String> = all_lines.into_iter().take(2).collect();
                // Replace last line's trailing chars with ellipsis
                let last = kept.last_mut().unwrap();
                let chars: Vec<char> = last.chars().collect();
                if chars.len() >= 2 {
                    let trimmed: String = chars[..chars.len() - 1].iter().collect();
                    *last = format!("{}\u{2026}", trimmed);
                }
                (kept, true)
            }
        };
        let _ = truncated;
        let ts_padded = format!("{:>width$}", ts, width = ts_col);
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                println!(
                    "  {}  {}",
                    palette.chrome(&ts_padded),
                    palette.testimony(line),
                );
            } else {
                println!(
                    "{:width$}{}",
                    "",
                    palette.testimony(line),
                    width = text_start,
                );
            }
        }
        if flags.notes {
            println!(); // breathing room between full notes
        }
    }
}

fn render_activity(
    mutation_infos: &[ShowMutationInfo],
    now: DateTime<Utc>,
    palette: &werk_shared::cli_display::Palette,
    flags: &ShowFlags,
    total_mutations: usize,
) {
    let non_notes: Vec<&ShowMutationInfo> = mutation_infos
        .iter()
        .filter(|m| m.field != "note")
        .collect();
    if non_notes.is_empty() {
        return;
    }

    println!();
    println!("{}", section_header("Activity", "", palette));

    let ts_col = 12;
    let text_start = 2 + ts_col + 2;

    // Most recent first, group same-timestamp
    let shown = non_notes.len();
    let reversed: Vec<&&ShowMutationInfo> = non_notes.iter().rev().collect();
    let mut i = 0;
    while i < reversed.len() {
        let m = reversed[i];
        let ts = DateTime::parse_from_rfc3339(&m.timestamp)
            .map(|dt| relative_time(dt.with_timezone(&Utc), now))
            .unwrap_or_else(|_| m.timestamp[..19].replace('T', " "));

        let mut summaries = vec![format_mutation_summary(
            &m.field,
            m.old_value.as_deref(),
            &m.new_value,
        )];
        let mut j = i + 1;
        while j < reversed.len() && reversed[j].timestamp == m.timestamp {
            summaries.push(format_mutation_summary(
                &reversed[j].field,
                reversed[j].old_value.as_deref(),
                &reversed[j].new_value,
            ));
            j += 1;
        }
        let deduped = dedup_summaries(&summaries);
        let ts_padded = format!("{:>width$}", ts, width = ts_col);
        println!("  {}  {}", palette.chrome(&ts_padded), deduped,);
        i = j;
    }

    let total_non_note =
        total_mutations - mutation_infos.iter().filter(|m| m.field == "note").count();
    if !flags.activity && shown < total_non_note {
        println!(
            "{:width$}{}",
            "",
            palette.chrome(&format!("\u{2026} {} earlier", total_non_note - shown)),
            width = text_start,
        );
    }
}

fn render_epochs(
    epochs: &[werk_core::EpochRecord],
    now: DateTime<Utc>,
    palette: &werk_shared::cli_display::Palette,
    flags: &ShowFlags,
) {
    if epochs.is_empty() {
        return;
    }

    if flags.epochs {
        println!();
        println!(
            "{}",
            section_header("Epochs", &epochs.len().to_string(), palette)
        );
        for (i, e) in epochs.iter().enumerate().rev() {
            let e_age = relative_time(e.timestamp, now);
            println!("  {:>3}.  {}", i + 1, palette.chrome(&e_age));
            println!(
                "        {}  {}",
                palette.chrome("desire:"),
                truncate(&e.desire_snapshot, 55)
            );
            println!(
                "        {} {}",
                palette.chrome("reality:"),
                truncate(&e.reality_snapshot, 55)
            );
        }
    } else {
        let latest = &epochs[epochs.len() - 1];
        let age = relative_time(latest.timestamp, now);
        println!();
        println!(
            "{}",
            palette.chrome(&format!("{} epochs (latest {})", epochs.len(), age))
        );
    }
}

fn render_context(result: &ShowResult, palette: &werk_shared::cli_display::Palette) {
    if let Some(ref ancestors) = result.ancestors {
        if !ancestors.is_empty() {
            println!();
            println!("{}", section_header("Ancestors", "", palette));
            for a in ancestors {
                let sc = display_id(a.short_code, &a.id);
                println!("  {:<6} {}", sc, truncate(&a.desired, 55));
            }
        }
    }
    if let Some(ref siblings) = result.siblings {
        if !siblings.is_empty() {
            println!();
            println!("{}", section_header("Siblings", "", palette));
            for s in siblings {
                let sc = display_id(s.short_code, &s.id);
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
        println!("{}", section_header("Engagement", "", palette));
        if let Some(freq) = eng.get("frequency_per_day").and_then(|v| v.as_f64()) {
            println!("  Frequency: {:.1}/day", freq);
        }
        if let Some(trend) = eng.get("frequency_trend").and_then(|v| v.as_f64()) {
            let tw = if trend > 0.1 {
                "accelerating"
            } else if trend < -0.1 {
                "declining"
            } else {
                "steady"
            };
            println!("  Trend: {}", tw);
        }
        if let Some(count) = eng.get("mutation_count").and_then(|v| v.as_u64()) {
            println!("  Mutations: {} in window", count);
        }
    }
}

fn render_hint(
    tension: &werk_core::Tension,
    overdue: bool,
    children: &[ChildInfo],
    palette: &werk_shared::cli_display::Palette,
) {
    let hint_id: String = match tension.short_code {
        Some(c) => c.to_string(),
        None => tension.id[..8.min(tension.id.len())].to_string(),
    };
    let has_no_positioned = children
        .iter()
        .filter(|c| c.status == "Active")
        .all(|c| c.position.is_none());
    let has_children = !children.is_empty();

    let hint = if overdue {
        format!(
            "overdue \u{2014} `werk reality {} ...` to update, `werk horizon {} <new>` to reschedule",
            hint_id, hint_id
        )
    } else if tension.status == TensionStatus::Active {
        if has_children && has_no_positioned {
            format!("`werk position {} <n>` to commit a sequence", hint_id)
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
    crate::hints::print_hint(palette, &hint);
}

// ============================================================================
// Utilities
// ============================================================================

/// Print a signal line: `  glyph LABEL      description`
/// Pads the label to 10 chars BEFORE colorizing, so ANSI codes don't
/// break the alignment.
fn signal_line(
    glyph: &str,
    label: &str,
    desc: &str,
    color_fn: fn(&werk_shared::cli_display::Palette, &str) -> String,
    palette: &werk_shared::cli_display::Palette,
) {
    let padded_label = format!("{:<10}", label);
    println!(
        "  {} {} {}",
        color_fn(palette, glyph),
        color_fn(palette, &padded_label),
        desc
    );
}

/// Format a sorted list of missing position numbers as a compact string.
/// `[2,3,4,5,6]` -> `2..6`, `[2,5]` -> `2,5`, `[2,3,7]` -> `2,3,7`.
fn format_missing_positions(missing: &[i32]) -> String {
    if missing.is_empty() {
        return String::new();
    }
    let is_contiguous = missing.len() > 2 && missing.windows(2).all(|w| w[1] == w[0] + 1);
    if is_contiguous {
        format!("{}..{}", missing[0], missing[missing.len() - 1])
    } else {
        missing
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    textwrap::wrap(text, width)
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect()
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_escape = false;
    for c in s.chars() {
        if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if c == '\x1b' {
            in_escape = true;
        } else {
            result.push(c);
        }
    }
    result
}

fn dedup_summaries(summaries: &[String]) -> String {
    if summaries.len() == 1 {
        return summaries[0].clone();
    }
    let mut seen: Vec<(String, usize)> = Vec::new();
    for s in summaries {
        if let Some(entry) = seen.iter_mut().find(|(text, _)| text == s) {
            entry.1 += 1;
        } else {
            seen.push((s.clone(), 1));
        }
    }
    seen.iter()
        .map(|(text, count)| {
            if *count > 1 {
                format!("{} \u{00d7}{}", text, count)
            } else {
                text.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" \u{00b7} ")
}

fn format_mutation_summary(field: &str, old_value: Option<&str>, new_value: &str) -> String {
    match field {
        "created" => "created".to_string(),
        "status" => match new_value {
            "Resolved" => "resolved".to_string(),
            "Released" => "released".to_string(),
            "Active" => match old_value {
                Some(old) => format!("reopened (was {})", old.to_lowercase()),
                None => "reopened".to_string(),
            },
            _ => format!("status -> {}", new_value),
        },
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
        _ => format!("{} updated", field),
    }
}
