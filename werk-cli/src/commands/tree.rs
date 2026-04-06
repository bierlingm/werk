//! Tree command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use owo_colors::OwoColorize;
use sd_core::{
    Forest, TensionStatus, compute_structural_signals, compute_temporal_signals,
    detect_containment_violations, detect_sequencing_pressure, FieldStructuralSignals,
};
use serde::Serialize;
use std::collections::HashMap;
use std::io::IsTerminal;

/// JSON output structure for a tension in tree.
#[derive(Serialize)]
struct TensionJson {
    id: String,
    short_code: Option<i32>,
    desired: String,
    actual: String,
    status: String,
    parent_id: Option<String>,
    created_at: String,
    horizon: Option<String>,
    overdue: bool,
    containment_violation: bool,
    sequencing_pressure: bool,
    closure_resolved: usize,
    closure_total: usize,
}

/// JSON output structure for tree.
#[derive(Serialize)]
struct TreeJson {
    tensions: Vec<TensionJson>,
    summary: TreeSummary,
}

#[derive(Serialize)]
struct TreeSummary {
    total: usize,
    active: usize,
    resolved: usize,
    released: usize,
}

/// Filter for tree display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Active,
    Resolved,
    Released,
}

pub fn cmd_tree(
    output: &Output,
    id: Option<String>,
    _open: bool,
    all: bool,
    resolved: bool,
    released: bool,
    stats: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;

    let forest = Forest::from_tensions(tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // If an ID is provided, resolve it and show subtree
    let (forest, tensions) = if let Some(ref id_str) = id {
        let resolver = PrefixResolver::new(tensions.clone());
        let root_id = resolver.resolve(id_str)?.id.clone();
        drop(resolver);
        let sub = forest
            .subtree(&root_id)
            .ok_or_else(|| WerkError::InvalidInput(format!("no subtree found for {}", root_id)))?;
        let sub_tensions: Vec<_> = tensions
            .into_iter()
            .filter(|t| sub.find(&t.id).is_some())
            .collect();
        (sub, sub_tensions)
    } else {
        (forest, tensions)
    };

    let now = Utc::now();

    let filter = if all {
        Filter::All
    } else if resolved {
        Filter::Resolved
    } else if released {
        Filter::Released
    } else {
        Filter::Active
    };

    let filtered_tensions: Vec<_> = tensions
        .iter()
        .filter(|t| match filter {
            Filter::All => true,
            Filter::Active => t.status == TensionStatus::Active,
            Filter::Resolved => t.status == TensionStatus::Resolved,
            Filter::Released => t.status == TensionStatus::Released,
        })
        .collect();

    if filtered_tensions.is_empty() {
        if output.is_structured() {
            let result = TreeJson {
                tensions: vec![],
                summary: TreeSummary {
                    total: 0,
                    active: 0,
                    resolved: 0,
                    released: 0,
                },
            };
            output
                .print_structured(&result)
                .map_err(WerkError::IoError)?;
        } else {
            output
                .info("No tensions found")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
        return Ok(());
    }

    // JSON output
    if output.is_structured() {
        let json_tensions: Vec<TensionJson> = filtered_tensions
            .iter()
            .map(|t| {
                let all_children = forest.children(&t.id).unwrap_or_default();
                let closure_resolved = all_children
                    .iter()
                    .filter(|c| c.tension.status == TensionStatus::Resolved)
                    .count();
                let overdue = t.status == TensionStatus::Active
                    && t.horizon
                        .as_ref()
                        .map(|h| h.is_past(now))
                        .unwrap_or(false);

                let containment_violation = t.status == TensionStatus::Active
                    && t.parent_id.as_ref().map_or(false, |pid| {
                        detect_containment_violations(&forest, pid)
                            .iter()
                            .any(|v| v.tension_id == t.id)
                    });

                let sequencing_pressure = t.status == TensionStatus::Active
                    && t.parent_id.as_ref().map_or(false, |pid| {
                        detect_sequencing_pressure(&forest, pid)
                            .iter()
                            .any(|p| p.tension_id == t.id)
                    });

                TensionJson {
                    id: t.id.clone(),
                    short_code: t.short_code,
                    desired: t.desired.clone(),
                    actual: t.actual.clone(),
                    status: t.status.to_string(),
                    parent_id: t.parent_id.clone(),
                    created_at: t.created_at.to_rfc3339(),
                    horizon: t.horizon.as_ref().map(|h| h.to_string()),
                    overdue,
                    containment_violation,
                    sequencing_pressure,
                    closure_resolved,
                    closure_total: all_children.len(),
                }
            })
            .collect();

        let active_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Active)
            .count();
        let resolved_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Resolved)
            .count();
        let released_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Released)
            .count();

        let result = TreeJson {
            tensions: json_tensions,
            summary: TreeSummary {
                total: tensions.len(),
                active: active_count,
                resolved: resolved_count,
                released: released_count,
            },
        };

        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
        return Ok(());
    }

    // Human-readable tree output
    let filtered_ids: std::collections::HashSet<_> =
        filtered_tensions.iter().map(|t| t.id.as_str()).collect();

    let use_color = std::io::stdout().is_terminal()
        && std::env::var("NO_COLOR").is_err();

    // Compute structural signals once for the whole forest
    let structural_signals = compute_structural_signals(&forest);

    // Pre-compute critical path membership for all tensions
    let critical_path_set: HashMap<String, bool> = filtered_tensions
        .iter()
        .map(|t| {
            let ts = compute_temporal_signals(&forest, &t.id, now);
            (t.id.clone(), ts.on_critical_path)
        })
        .collect();

    /// Sort nodes canonically: positioned DESC, then unpositioned by horizon, then creation time.
    fn canonical_sort(nodes: &mut Vec<&sd_core::Node>) {
        nodes.sort_by(|a, b| {
            let a_pos = a.tension.position;
            let b_pos = b.tension.position;

            match (a_pos, b_pos) {
                (Some(pa), Some(pb)) => return pb.cmp(&pa),
                (Some(_), None) => return std::cmp::Ordering::Less,
                (None, Some(_)) => return std::cmp::Ordering::Greater,
                (None, None) => {}
            }

            match (&a.tension.horizon, &b.tension.horizon) {
                (Some(ha), Some(hb)) => {
                    let end_order = ha.range_end().cmp(&hb.range_end());
                    if end_order != std::cmp::Ordering::Equal {
                        return end_order;
                    }
                    let prec_order = ha.precision_level().cmp(&hb.precision_level());
                    if prec_order != std::cmp::Ordering::Equal {
                        return prec_order;
                    }
                }
                (Some(_), None) => return std::cmp::Ordering::Less,
                (None, Some(_)) => return std::cmp::Ordering::Greater,
                (None, None) => {}
            }

            a.tension.created_at.cmp(&b.tension.created_at)
        });
    }

    /// Truncate to max chars, appending … if cut.
    fn smart_truncate(s: &str, max: usize) -> String {
        if s.chars().count() <= max {
            s.to_string()
        } else {
            let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
            format!("{}\u{2026}", truncated)
        }
    }

    /// Build signal glyphs for a node (by exception).
    fn signal_glyphs(
        node_id: &str,
        structural: &FieldStructuralSignals,
        critical_path_set: &HashMap<String, bool>,
        sig: &werk_shared::SignalThresholds,
    ) -> Vec<&'static str> {
        let mut glyphs = Vec::new();

        if *critical_path_set.get(node_id).unwrap_or(&false) {
            glyphs.push("\u{2021}"); // ‡ critical path
        }
        if let Some(ss) = structural.signals.get(node_id) {
            if ss.on_longest_path {
                glyphs.push("\u{2503}"); // ┃ spine
            }
            if ss.centrality.map(|c| c > sig.hub_centrality).unwrap_or(false) {
                glyphs.push("\u{25c9}"); // ◉ hub
            }
            if ss.descendant_count.map(|c| c > sig.reach_descendants as usize).unwrap_or(false) {
                glyphs.push("\u{25ce}"); // ◎ reach
            }
        }
        glyphs
    }

    struct RenderCtx<'a> {
        forest: &'a Forest,
        filtered_ids: &'a std::collections::HashSet<&'a str>,
        structural: &'a FieldStructuralSignals,
        critical_path_set: &'a HashMap<String, bool>,
        sig: &'a werk_shared::SignalThresholds,
        term_width: usize,
        use_color: bool,
    }

    fn render_tree(
        ctx: &RenderCtx<'_>,
        root_ids: &[String],
        now: chrono::DateTime<Utc>,
        prefix: &str,
        lines: &mut Vec<String>,
    ) {
        let mut roots: Vec<_> = root_ids
            .iter()
            .filter(|id| ctx.filtered_ids.contains(id.as_str()))
            .filter_map(|id| ctx.forest.find(id))
            .collect();

        canonical_sort(&mut roots);

        for (i, node) in roots.iter().enumerate() {
            let is_last = i == roots.len() - 1;
            let connector = if is_last { "\u{2514}\u{2500}\u{2500} " } else { "\u{251c}\u{2500}\u{2500} " };

            // --- Zone 1: Identity (id + position) ---
            let id_str = werk_shared::display_id(node.tension.short_code, node.id());
            let pos_str = if node.tension.status == TensionStatus::Active {
                node.tension.position.map(|p| format!("\u{25b8}{}", p)).unwrap_or_default()
            } else {
                match node.tension.status {
                    TensionStatus::Resolved => "\u{2713}".to_string(),
                    TensionStatus::Released => "~".to_string(),
                    _ => String::new(),
                }
            };

            // --- Zone 2: Horizon ---
            let horizon_str = node.tension.horizon.as_ref().map(|h| format!("[{}]", h)).unwrap_or_default();

            // --- Zone 3: Warning signals (by exception) ---
            let mut warnings: Vec<String> = Vec::new();
            if node.tension.status == TensionStatus::Active {
                if let Some(h) = &node.tension.horizon {
                    if h.is_past(now) {
                        warnings.push("OVERDUE".to_string());
                    }
                }
                if let Some(ref parent_id) = node.tension.parent_id {
                    if detect_containment_violations(ctx.forest, parent_id)
                        .iter()
                        .any(|v| v.tension_id == node.id())
                    {
                        warnings.push("EXCEEDS_PARENT".to_string());
                    }
                    if detect_sequencing_pressure(ctx.forest, parent_id)
                        .iter()
                        .any(|p| p.tension_id == node.id())
                    {
                        warnings.push("PRESSURE".to_string());
                    }
                }
            }

            // --- Zone 4: Closure progress ---
            let all_children = ctx.forest.children(node.id()).unwrap_or_default();
            let closure_str = if !all_children.is_empty() {
                let resolved_count = all_children
                    .iter()
                    .filter(|c| c.tension.status == TensionStatus::Resolved)
                    .count();
                let released_count = all_children
                    .iter()
                    .filter(|c| c.tension.status == TensionStatus::Released)
                    .count();
                let active_count = all_children.len() - released_count;
                if released_count > 0 {
                    format!("[{}/{}] ({} released)", resolved_count, active_count, released_count)
                } else {
                    format!("[{}/{}]", resolved_count, active_count)
                }
            } else {
                String::new()
            };

            // --- Zone 5: Structural glyphs ---
            let glyphs = signal_glyphs(node.id(), ctx.structural, ctx.critical_path_set, ctx.sig);

            // --- Compute available width for desire text ---
            let prefix_chars = prefix.chars().count() + connector.chars().count();

            // Build the meta prefix: "#42 ▸3 [2026-06] "
            let mut meta_plain = id_str.clone();
            if !pos_str.is_empty() {
                meta_plain.push(' ');
                meta_plain.push_str(&pos_str);
            }
            if !horizon_str.is_empty() {
                meta_plain.push(' ');
                meta_plain.push_str(&horizon_str);
            }
            for w in &warnings {
                meta_plain.push(' ');
                meta_plain.push_str(w);
            }
            meta_plain.push(' ');

            // Suffix: "  [9/15]  ‡◉"
            let suffix_plain = {
                let mut s = String::new();
                if !closure_str.is_empty() {
                    s.push_str("  ");
                    s.push_str(&closure_str);
                }
                if !glyphs.is_empty() {
                    s.push_str("  ");
                    for g in &glyphs {
                        s.push_str(g);
                    }
                }
                s
            };

            let chrome_width = prefix_chars + meta_plain.chars().count() + suffix_plain.chars().count();
            let available = if ctx.term_width > chrome_width + 12 {
                ctx.term_width - chrome_width
            } else {
                40 // minimum readable text
            };

            let desired_text = smart_truncate(&node.tension.desired, available);

            // --- Assemble with color ---
            if ctx.use_color {
                let line = format!(
                    "{}{}{}{}{}",
                    prefix,
                    connector.dimmed(),
                    format_meta_colored(&id_str, &pos_str, &horizon_str, &warnings),
                    desired_text,
                    format_suffix_colored(&closure_str, &glyphs),
                );
                lines.push(line);
            } else {
                let line = format!(
                    "{}{}{}{}{}",
                    prefix, connector, meta_plain, desired_text, suffix_plain
                );
                lines.push(line);
            }

            // Recurse
            let child_ids: Vec<_> = node
                .children
                .iter()
                .filter(|id| ctx.filtered_ids.contains(id.as_str()))
                .cloned()
                .collect();

            if !child_ids.is_empty() {
                let new_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}\u{2502}   ", prefix)
                };
                render_tree(ctx, &child_ids, now, &new_prefix, lines);
            }
        }
    }

    fn format_meta_colored(
        id_str: &str,
        pos_str: &str,
        horizon_str: &str,
        warnings: &[String],
    ) -> String {
        let mut s = format!("{}", id_str.bold());
        if !pos_str.is_empty() {
            s.push_str(&format!(" {}", pos_str.dimmed()));
        }
        if !horizon_str.is_empty() {
            s.push_str(&format!(" {}", horizon_str.dimmed()));
        }
        for w in warnings {
            s.push_str(&format!(" {}", w.yellow().bold()));
        }
        s.push(' ');
        s
    }

    fn format_suffix_colored(closure_str: &str, glyphs: &[&str]) -> String {
        let mut s = String::new();
        if !closure_str.is_empty() {
            s.push_str(&format!("  {}", closure_str.dimmed()));
        }
        if !glyphs.is_empty() {
            s.push_str(&format!("  {}", glyphs.join("").cyan()));
        }
        s
    }

    let term_width = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(120);

    let sig = crate::commands::signal_thresholds_from(&workspace);
    let ctx = RenderCtx {
        forest: &forest,
        filtered_ids: &filtered_ids,
        structural: &structural_signals,
        critical_path_set: &critical_path_set,
        sig: &sig,
        term_width,
        use_color,
    };

    let mut lines = Vec::new();
    render_tree(&ctx, forest.root_ids(), now, "", &mut lines);

    for line in &lines {
        println!("{}", line);
    }

    // Summary footer
    let active_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .count();
    let resolved_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .count();
    let released_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Released)
        .count();

    println!();
    if use_color {
        println!(
            "{}",
            format!(
                "Total: {}  Active: {}  Resolved: {}  Released: {}",
                tensions.len(), active_count, resolved_count, released_count
            ).dimmed()
        );
    } else {
        println!(
            "Total: {}  Active: {}  Resolved: {}  Released: {}",
            tensions.len(), active_count, resolved_count, released_count
        );
    }

    if stats {
        let deadlined = tensions.iter().filter(|t| t.horizon.is_some()).count();
        let overdue = tensions
            .iter()
            .filter(|t| {
                t.status == TensionStatus::Active
                    && t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false)
            })
            .count();
        let positioned = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Active && t.position.is_some())
            .count();
        let held = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Active && t.position.is_none())
            .count();
        if use_color {
            println!(
                "{}",
                format!(
                    "Deadlined: {}  Overdue: {}  Positioned: {}  Held: {}",
                    deadlined, overdue, positioned, held
                ).dimmed()
            );
        } else {
            println!(
                "Deadlined: {}  Overdue: {}  Positioned: {}  Held: {}",
                deadlined, overdue, positioned, held
            );
        }
    }

    Ok(())
}
