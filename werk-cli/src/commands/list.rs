//! List command handler — the general-purpose query engine.
//!
//! Flat or tree listing of tensions with rich filtering, sorting, and
//! time-windowed change detection. Absorbs survey and diff functionality.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use sd_core::{compute_temporal_signals, compute_urgency, detect_horizon_drift, Forest, HorizonDriftType, Tension, TensionStatus};
use werk_shared::truncate;

/// JSON output structure for a tension in list.
#[derive(Serialize)]
struct ListTensionJson {
    id: String,
    short_code: Option<i32>,
    desired: String,
    actual: String,
    status: String,
    parent_id: Option<String>,
    urgency: Option<f64>,
    horizon: Option<String>,
    overdue: bool,
    position: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_desired: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changed_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    signals: Vec<String>,
}

/// JSON output structure for list.
#[derive(Serialize)]
struct ListJson {
    tensions: Vec<ListTensionJson>,
    count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<String>,
}

/// All the parameters for list, collected from the clap definition.
pub struct ListParams {
    pub all: bool,
    pub status: Option<String>,
    pub overdue: bool,
    pub approaching: Option<i64>,
    pub stale: Option<i64>,
    pub held: bool,
    pub positioned: bool,
    pub root: bool,
    pub parent: Option<String>,
    pub has_deadline: bool,
    pub changed: Option<String>,
    pub signals: bool,
    pub sort: String,
    pub reverse: bool,
    pub tree: bool,
    pub long: bool,
    pub search: Option<String>,
}

/// Computed row data for filtering, sorting, and display.
struct TensionRow {
    id: String,
    short_code: Option<i32>,
    desired: String,
    actual: String,
    status: TensionStatus,
    parent_id: Option<String>,
    urgency: Option<f64>,
    horizon_display: String,
    horizon_raw: Option<String>,
    overdue: bool,
    position: Option<i32>,
    category: Option<String>,
    parent_desired: Option<String>,
    depth: usize,
    changed_fields: Option<Vec<String>>,
    /// Signal glyphs for this tension (computed when --long, --signals, or --json).
    signal_glyphs: Vec<&'static str>,
    /// Signal labels for JSON output.
    signal_labels: Vec<String>,
}

fn format_horizon(tension: &Tension, now: DateTime<Utc>) -> String {
    match &tension.horizon {
        Some(h) => {
            let days = h.range_end().signed_duration_since(now).num_days();
            if days < 0 {
                format!("{}d past", -days)
            } else if days == 0 {
                "today".to_string()
            } else if days <= 30 {
                format!("{}d", days)
            } else {
                h.to_string()
            }
        }
        None => "\u{2014}".to_string(),
    }
}

pub fn cmd_list(output: &Output, params: ListParams) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let now = Utc::now();

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;

    if tensions.is_empty() {
        if output.is_structured() {
            let result = ListJson {
                tensions: vec![],
                count: 0,
                since: None,
            };
            output.print_structured(&result).map_err(WerkError::IoError)?;
        } else {
            output
                .info("No tensions found")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
        return Ok(());
    }

    // Build parent lookup for structural context
    let parent_lookup: HashMap<String, (Option<i32>, String)> = tensions
        .iter()
        .map(|t| (t.id.clone(), (t.short_code, t.desired.clone())))
        .collect();

    // If --changed, parse the since value and find changed tension IDs
    let (since_dt, changed_tension_fields) = if let Some(ref since_str) = params.changed {
        let dt = crate::commands::diff::parse_since(since_str, now)?;
        let mutations = store.mutations_between(dt, now).map_err(WerkError::StoreError)?;
        let mut changed: HashMap<String, Vec<String>> = HashMap::new();
        for m in &mutations {
            changed
                .entry(m.tension_id().to_owned())
                .or_default()
                .push(m.field().to_owned());
        }
        // Deduplicate fields per tension
        for fields in changed.values_mut() {
            fields.sort();
            fields.dedup();
        }
        (Some(dt), Some(changed))
    } else {
        (None, None)
    };

    // If --stale, compute last mutation timestamps
    let stale_threshold = params.stale.map(|days| {
        now - chrono::Duration::days(days)
    });

    let last_mutation_ts: Option<HashMap<String, DateTime<Utc>>> = if stale_threshold.is_some() {
        let ids: Vec<&str> = tensions.iter().map(|t| t.id.as_str()).collect();
        let fields: Vec<&str> = vec!["actual", "desired", "status", "note"];
        Some(
            store
                .get_last_mutation_timestamps(&ids, &fields)
                .map_err(WerkError::StoreError)?,
        )
    } else {
        None
    };

    // Resolve --parent prefix
    let parent_filter_id = if let Some(ref prefix) = params.parent {
        let resolver = crate::prefix::PrefixResolver::new(tensions.clone());
        Some(resolver.resolve(prefix)?.id.clone())
    } else {
        None
    };

    // Build rows
    let mut rows: Vec<TensionRow> = Vec::new();

    for tension in &tensions {
        let urgency_val = compute_urgency(tension, now).map(|u| u.value);
        let horizon_display = format_horizon(tension, now);

        let is_overdue = tension.status == TensionStatus::Active
            && tension
                .horizon
                .as_ref()
                .map(|h| h.is_past(now))
                .unwrap_or(false);

        let (_parent_sc, parent_desired) = tension
            .parent_id
            .as_ref()
            .and_then(|pid| parent_lookup.get(pid))
            .map(|(sc, d)| (*sc, Some(d.clone())))
            .unwrap_or((None, None));

        // Category for survey-style output
        let category = if tension.status == TensionStatus::Resolved
            || tension.status == TensionStatus::Released
        {
            Some("resolved".to_string())
        } else if is_overdue {
            Some("overdue".to_string())
        } else if tension.position.is_none() {
            Some("held".to_string())
        } else {
            Some("active".to_string())
        };

        let changed_fields = changed_tension_fields
            .as_ref()
            .and_then(|cf| cf.get(&tension.id))
            .cloned();

        rows.push(TensionRow {
            id: tension.id.clone(),
            short_code: tension.short_code,
            desired: tension.desired.clone(),
            actual: tension.actual.clone(),
            status: tension.status,
            parent_id: tension.parent_id.clone(),
            urgency: urgency_val,
            horizon_display,
            horizon_raw: tension.horizon.as_ref().map(|h| h.to_string()),
            overdue: is_overdue,
            position: tension.position,
            category,
            parent_desired,
            depth: 0, // set below if --tree
            changed_fields,
            signal_glyphs: vec![],
            signal_labels: vec![],
        });
    }

    // ── Compute signals (when needed for display or filtering) ────
    let compute_signals = params.long || params.signals || output.is_structured();
    if compute_signals {
        let forest = Forest::from_tensions(tensions.clone())
            .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

        for row in &mut rows {
            // Only compute for active tensions
            if row.status != TensionStatus::Active {
                continue;
            }

            // Overdue (already computed, just add glyph)
            if row.overdue {
                row.signal_glyphs.push("!");
                row.signal_labels.push("overdue".to_string());
            }

            // Temporal signals from forest
            let temporal = compute_temporal_signals(&forest, &row.id, now);

            if temporal.on_critical_path {
                row.signal_glyphs.push("\u{2021}");
                row.signal_labels.push("critical_path".to_string());
            }
            if temporal.has_containment_violation {
                row.signal_glyphs.push("\u{21a5}");
                row.signal_labels.push("containment_violation".to_string());
            }
            if !temporal.sequencing_pressures.is_empty() {
                row.signal_glyphs.push("\u{21c5}");
                row.signal_labels.push("sequencing_pressure".to_string());
            }
            // Children signals (critical path and containment on children)
            if !temporal.critical_path.is_empty() && !temporal.on_critical_path {
                row.signal_glyphs.push("\u{2021}");
                row.signal_labels.push("critical_path_parent".to_string());
            }
            if !temporal.containment_violations.is_empty() && !temporal.has_containment_violation {
                row.signal_glyphs.push("\u{21a5}");
                row.signal_labels.push("containment_violation_parent".to_string());
            }

            // Horizon drift — only RepeatedPostponement and Oscillating in list (noise threshold)
            if let Some(horizon) = tensions.iter().find(|t| t.id == row.id).and_then(|t| t.horizon.as_ref()) {
                let _ = horizon; // confirm horizon exists
                let mutations = store.get_mutations(&row.id).map_err(WerkError::StoreError)?;
                let drift = detect_horizon_drift(&row.id, &mutations);
                match drift.drift_type {
                    HorizonDriftType::RepeatedPostponement | HorizonDriftType::Oscillating => {
                        row.signal_glyphs.push("\u{219d}");
                        row.signal_labels.push(format!("drift:{:?}", drift.drift_type));
                    }
                    _ => {}
                }
            }
        }
    }

    // ── Apply filters ──────────────────────────────────────────────

    // Status filter
    if let Some(ref status_filter) = params.status {
        let target = match status_filter.to_lowercase().as_str() {
            "active" => TensionStatus::Active,
            "resolved" => TensionStatus::Resolved,
            "released" => TensionStatus::Released,
            _ => {
                return Err(WerkError::InvalidInput(format!(
                    "unknown status '{}'. Use active, resolved, or released.",
                    status_filter
                )))
            }
        };
        rows.retain(|r| r.status == target);
    } else if !params.all {
        // Default: active only
        rows.retain(|r| r.status == TensionStatus::Active);
    }

    if params.overdue {
        rows.retain(|r| r.overdue);
    }

    if let Some(approaching_days) = params.approaching {
        let frame_end = now + chrono::Duration::days(approaching_days);
        rows.retain(|r| {
            r.overdue
                || tensions
                    .iter()
                    .find(|t| t.id == r.id)
                    .and_then(|t| t.horizon.as_ref())
                    .map(|h| h.range_end() <= frame_end)
                    .unwrap_or(false)
        });
    }

    if let Some(ref stale_ts) = stale_threshold {
        if let Some(ref ts_map) = last_mutation_ts {
            rows.retain(|r| {
                r.status == TensionStatus::Active
                    && ts_map
                        .get(&r.id)
                        .map(|last| last < stale_ts)
                        .unwrap_or(true) // no mutations at all = stale
            });
        }
    }

    if params.held {
        rows.retain(|r| r.position.is_none() && r.status == TensionStatus::Active);
    }

    if params.positioned {
        rows.retain(|r| r.position.is_some() && r.status == TensionStatus::Active);
    }

    if params.root {
        rows.retain(|r| r.parent_id.is_none());
    }

    if let Some(ref pid) = parent_filter_id {
        rows.retain(|r| r.parent_id.as_deref() == Some(pid.as_str()));
    }

    if params.has_deadline {
        rows.retain(|r| r.horizon_raw.is_some());
    }

    if changed_tension_fields.is_some() {
        rows.retain(|r| r.changed_fields.is_some());
    }

    if params.signals {
        rows.retain(|r| !r.signal_glyphs.is_empty());
    }

    // ── Search (FrankenSearch content retrieval) ───────────────────
    // When --search is active, filter to matching tensions and sort by
    // relevance score (overrides the normal sort).
    let search_active = params.search.is_some();
    if let Some(ref query) = params.search {
        let index = sd_core::SearchIndex::build(&store);
        if let Some(ref idx) = index {
            let hits = idx.search(query, 100);
            let hit_order: std::collections::HashMap<String, (usize, f32)> = hits
                .iter()
                .enumerate()
                .map(|(i, h)| (h.doc_id.clone(), (i, h.score)))
                .collect();
            rows.retain(|r| hit_order.contains_key(&r.id));
            rows.sort_by(|a, b| {
                let oa = hit_order.get(&a.id).map(|(i, _)| *i).unwrap_or(usize::MAX);
                let ob = hit_order.get(&b.id).map(|(i, _)| *i).unwrap_or(usize::MAX);
                oa.cmp(&ob)
            });
        } else {
            // Fallback: substring filter when index unavailable
            let q = query.to_lowercase();
            rows.retain(|r| {
                r.desired.to_lowercase().contains(&q)
                    || r.actual.to_lowercase().contains(&q)
            });
        }
    }

    // ── Sort (skipped when --search provides relevance ranking) ────

    if search_active {
        // Search already sorted by relevance — don't re-sort unless
        // the user explicitly asked for a different sort.
    } else { match params.sort.as_str() {
        "urgency" => {
            rows.sort_by(|a, b| {
                let ua = a.urgency.unwrap_or(-1.0);
                let ub = b.urgency.unwrap_or(-1.0);
                ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        "name" => {
            rows.sort_by(|a, b| a.desired.to_lowercase().cmp(&b.desired.to_lowercase()));
        }
        "deadline" | "horizon" => {
            rows.sort_by(|a, b| match (&a.horizon_raw, &b.horizon_raw) {
                (Some(ha), Some(hb)) => ha.cmp(hb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }
        "created" => {
            // Sort by short_code as proxy for creation order
            rows.sort_by(|a, b| a.short_code.cmp(&b.short_code));
        }
        "updated" => {
            // Would need last mutation timestamp; fall back to urgency if not available
            if let Some(ref ts_map) = last_mutation_ts {
                rows.sort_by(|a, b| {
                    let ta = ts_map.get(&a.id);
                    let tb = ts_map.get(&b.id);
                    tb.cmp(&ta)
                });
            }
        }
        "position" => {
            rows.sort_by(|a, b| {
                let pa = a.position.unwrap_or(i32::MAX);
                let pb = b.position.unwrap_or(i32::MAX);
                pa.cmp(&pb)
            });
        }
        _ => {
            // Default: urgency
            rows.sort_by(|a, b| {
                let ua = a.urgency.unwrap_or(-1.0);
                let ub = b.urgency.unwrap_or(-1.0);
                ub.partial_cmp(&ua).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    } }

    if params.reverse {
        rows.reverse();
    }

    // ── Tree mode ──────────────────────────────────────────────────

    if params.tree {
        // Compute depth for retained rows, then sort by tree order
        let _retained_ids: HashSet<String> = rows.iter().map(|r| r.id.clone()).collect();
        let tension_map: HashMap<String, &Tension> = tensions
            .iter()
            .map(|t| (t.id.clone(), t))
            .collect();

        // Compute depths
        for row in &mut rows {
            let mut depth = 0;
            let mut pid = row.parent_id.clone();
            while let Some(p) = pid {
                depth += 1;
                pid = tension_map.get(&p).and_then(|t| t.parent_id.clone());
            }
            row.depth = depth;
        }

        // Sort by tree order: root order, then children by position/creation
        rows.sort_by(|a, b| {
            let a_path = build_tree_path(a, &tension_map);
            let b_path = build_tree_path(b, &tension_map);
            a_path.cmp(&b_path)
        });
    }

    // ── Output ─────────────────────────────────────────────────────

    if output.is_structured() {
        let json_tensions: Vec<ListTensionJson> = rows
            .iter()
            .map(|r| ListTensionJson {
                id: r.id.clone(),
                short_code: r.short_code,
                desired: r.desired.clone(),
                actual: r.actual.clone(),
                status: r.status.to_string(),
                parent_id: r.parent_id.clone(),
                urgency: r.urgency,
                horizon: r.horizon_raw.clone(),
                overdue: r.overdue,
                position: r.position,
                category: r.category.clone(),
                parent_desired: r.parent_desired.clone(),
                changed_fields: r.changed_fields.clone(),
                signals: r.signal_labels.clone(),
            })
            .collect();

        let count = json_tensions.len();
        let result = ListJson {
            tensions: json_tensions,
            count,
            since: since_dt.map(|dt| dt.to_rfc3339()),
        };
        output.print_structured(&result).map_err(WerkError::IoError)?;
    } else {
        if rows.is_empty() {
            output
                .info("No tensions match the filter")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
            return Ok(());
        }

        if params.tree {
            print_tree_rows(&rows);
        } else if params.long {
            print_long_rows(&rows, now);
        } else if params.changed.is_some() {
            print_changed_rows(&rows, now);
        } else {
            print_default_rows(&rows);
        }

        // Legend: show glyph key if any signals are present in the output
        let has_any_signals = rows.iter().any(|r| !r.signal_glyphs.is_empty());
        println!();
        if has_any_signals {
            println!("{} tension(s)  ! overdue  \u{2021} critical path  \u{21a5} containment  \u{21c5} sequencing  \u{219d} drift", rows.len());
        } else {
            println!("{} tension(s)", rows.len());
        }
    }

    Ok(())
}

// ── Display functions ──────────────────────────────────────────────

fn print_default_rows(rows: &[TensionRow]) {
    for row in rows {
        let id_display = match row.short_code {
            Some(c) => format!("#{:<4}", c),
            None => format!("{:<8}", &row.id[..8.min(row.id.len())]),
        };

        let overdue_marker = if row.overdue { " OVERDUE" } else { "" };

        let urgency_display = match row.urgency {
            Some(u) => format!("{:>3.0}%", u * 100.0),
            None => " \u{2014} ".to_string(),
        };

        let signal_display = if row.signal_glyphs.is_empty() {
            String::new()
        } else {
            format!(" {}", row.signal_glyphs.join(""))
        };

        println!(
            "{}  {:<30}  {:>8}{}  {}{}",
            id_display,
            truncate(&row.desired, 30),
            row.horizon_display,
            overdue_marker,
            urgency_display,
            signal_display,
        );
    }
}

fn print_long_rows(rows: &[TensionRow], _now: DateTime<Utc>) {
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            println!();
        }
        let id_display = match row.short_code {
            Some(c) => format!("#{}", c),
            None => row.id[..8.min(row.id.len())].to_string(),
        };

        let status = match row.status {
            TensionStatus::Active => "active",
            TensionStatus::Resolved => "resolved",
            TensionStatus::Released => "released",
        };

        println!("{} [{}] {}", id_display, status, truncate(&row.desired, 60));
        println!("  Reality: {}", truncate(&row.actual, 60));
        if let Some(ref h) = row.horizon_raw {
            let overdue_marker = if row.overdue { " OVERDUE" } else { "" };
            println!("  Deadline: {}{}", h, overdue_marker);
        }
        if let Some(u) = row.urgency {
            println!("  Urgency: {:.0}%", u * 100.0);
        }
        if !row.signal_glyphs.is_empty() {
            let signal_str: Vec<String> = row.signal_glyphs.iter()
                .zip(row.signal_labels.iter())
                .map(|(g, l)| format!("{} {}", g, l))
                .collect();
            println!("  Signals: {}", signal_str.join(", "));
        }
        if let Some(ref pd) = row.parent_desired {
            let psc = row.parent_id.as_ref().map(|_| "parent").unwrap_or("");
            println!("  {}: {}", psc, truncate(pd, 50));
        }
    }
}

fn print_changed_rows(rows: &[TensionRow], _now: DateTime<Utc>) {
    for row in rows {
        let id_display = match row.short_code {
            Some(c) => format!("#{:<4}", c),
            None => format!("{:<8}", &row.id[..8.min(row.id.len())]),
        };

        let fields = row
            .changed_fields
            .as_ref()
            .map(|f| f.join(", "))
            .unwrap_or_default();

        println!(
            "{}  {:<35}  [{}]",
            id_display,
            truncate(&row.desired, 35),
            fields,
        );
    }
}

fn print_tree_rows(rows: &[TensionRow]) {
    for row in rows {
        let id_display = match row.short_code {
            Some(c) => format!("#{}", c),
            None => row.id[..8.min(row.id.len())].to_string(),
        };

        let indent = "  ".repeat(row.depth);
        let overdue_marker = if row.overdue { " OVERDUE" } else { "" };
        let deadline = row.horizon_raw.as_deref().unwrap_or("");
        let deadline_display = if deadline.is_empty() {
            String::new()
        } else {
            format!(" [{}]", deadline)
        };

        println!(
            "{}{} {}{}{}",
            indent,
            id_display,
            truncate(&row.desired, 50 - (row.depth * 2).min(40)),
            deadline_display,
            overdue_marker,
        );
    }
}

/// Build a sort key for tree ordering: sequence of (position, short_code) tuples from root to node.
fn build_tree_path(row: &TensionRow, tension_map: &HashMap<String, &Tension>) -> Vec<(i32, i32)> {
    let mut path = Vec::new();
    let mut current_id = Some(row.id.clone());

    while let Some(id) = current_id {
        if let Some(t) = tension_map.get(&id) {
            path.push((
                t.position.unwrap_or(i32::MAX),
                t.short_code.unwrap_or(i32::MAX),
            ));
            current_id = t.parent_id.clone();
        } else {
            break;
        }
    }

    path.reverse();
    path
}
