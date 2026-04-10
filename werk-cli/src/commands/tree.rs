//! Tree command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use sd_core::{
    Forest, TensionStatus, compute_structural_signals, compute_temporal_signals,
    detect_containment_violations, detect_sequencing_pressure, FieldStructuralSignals,
};
use serde::Serialize;
use std::collections::HashMap;
use werk_shared::cli_display::{Palette, glyphs};

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
    compact: bool,
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

    // The Output owns the shared palette; tree.rs used to build its own,
    // but Phase 2 centralizes TTY/NO_COLOR detection there.
    let palette = output.palette();

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

    /// Truncate to max chars, appending the canonical Unicode ellipsis if cut.
    fn smart_truncate(s: &str, max: usize) -> String {
        if s.chars().count() <= max {
            s.to_string()
        } else {
            let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
            format!("{}{}", truncated, glyphs::TRUNCATE_ELLIPSIS)
        }
    }

    /// Build signal glyphs for a node (by exception).
    fn signal_glyphs(
        node_id: &str,
        structural: &FieldStructuralSignals,
        critical_path_set: &HashMap<String, bool>,
        sig: &werk_shared::SignalThresholds,
    ) -> Vec<&'static str> {
        let mut out = Vec::new();

        if *critical_path_set.get(node_id).unwrap_or(&false) {
            out.push(glyphs::SIGNAL_CRITICAL_PATH);
        }
        if let Some(ss) = structural.signals.get(node_id) {
            if ss.on_longest_path {
                out.push(glyphs::SIGNAL_SPINE);
            }
            if ss.centrality.map(|c| c > sig.hub_centrality).unwrap_or(false) {
                out.push(glyphs::SIGNAL_HUB);
            }
            if ss.descendant_count.map(|c| c > sig.reach_descendants as usize).unwrap_or(false) {
                out.push(glyphs::SIGNAL_REACH);
            }
        }
        out
    }

    struct RenderCtx<'a> {
        forest: &'a Forest,
        filtered_ids: &'a std::collections::HashSet<&'a str>,
        structural: &'a FieldStructuralSignals,
        critical_path_set: &'a HashMap<String, bool>,
        sig: &'a werk_shared::SignalThresholds,
        term_width: usize,
        palette: Palette,
    }

    /// Per-node structural facts assembled once and reused by both
    /// the compact and rich renderers.
    struct NodeMeta<'a> {
        id_str: String,
        pos_str: String,
        horizon_str: String,
        warnings: Vec<String>,
        closure_str: String,
        glyphs: Vec<&'a str>,
        child_ids: Vec<String>,
    }

    fn compute_node_meta<'a>(
        ctx: &'a RenderCtx<'_>,
        node: &sd_core::Node,
        now: chrono::DateTime<Utc>,
    ) -> NodeMeta<'a> {
        let id_str = werk_shared::display_id(node.tension.short_code, node.id());
        let pos_str = if node.tension.status == TensionStatus::Active {
            node.tension
                .position
                .map(|p| format!("{}{}", glyphs::STATUS_POSITION, p))
                .unwrap_or_default()
        } else {
            match node.tension.status {
                TensionStatus::Resolved => glyphs::STATUS_RESOLVED.to_string(),
                TensionStatus::Released => glyphs::STATUS_RELEASED.to_string(),
                _ => String::new(),
            }
        };

        let horizon_str = node
            .tension
            .horizon
            .as_ref()
            .map(|h| format!("[{}]", h))
            .unwrap_or_default();

        // Warning signals are shown by exception. The strings are exactly
        // those the compact renderer has always produced so assertions
        // like `stdout.contains("OVERDUE")` keep working.
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

        let sig_glyphs = signal_glyphs(node.id(), ctx.structural, ctx.critical_path_set, ctx.sig);

        let child_ids: Vec<_> = node
            .children
            .iter()
            .filter(|id| ctx.filtered_ids.contains(id.as_str()))
            .cloned()
            .collect();

        NodeMeta {
            id_str,
            pos_str,
            horizon_str,
            warnings,
            closure_str,
            glyphs: sig_glyphs,
            child_ids,
        }
    }

    /// Compact single-line renderer — the v1.5 layout.
    ///
    /// All zones on one line. Used when `--compact` is set, under 80
    /// columns, when the terminal is not interactive (piped output,
    /// test harness), or when the palette is disabled (e.g. NO_COLOR).
    /// Tests run under `assert_cmd` with no TTY, so they always hit
    /// this path — which means test assertions stay pinned against
    /// the stable v1.5 output.
    fn render_tree_compact(
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
            let connector = if is_last { glyphs::TREE_LAST } else { glyphs::TREE_BRANCH };

            let meta = compute_node_meta(ctx, node, now);

            // --- Compute available width for desire text ---
            let prefix_chars = prefix.chars().count() + connector.chars().count();

            // Build the meta prefix: "#42 ▸3 [2026-06] "
            let mut meta_plain = meta.id_str.clone();
            if !meta.pos_str.is_empty() {
                meta_plain.push(' ');
                meta_plain.push_str(&meta.pos_str);
            }
            if !meta.horizon_str.is_empty() {
                meta_plain.push(' ');
                meta_plain.push_str(&meta.horizon_str);
            }
            for w in &meta.warnings {
                meta_plain.push(' ');
                meta_plain.push_str(w);
            }
            meta_plain.push(' ');

            let suffix_plain = {
                let mut s = String::new();
                if !meta.closure_str.is_empty() {
                    s.push_str("  ");
                    s.push_str(&meta.closure_str);
                }
                if !meta.glyphs.is_empty() {
                    s.push_str("  ");
                    for g in &meta.glyphs {
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

            let line = format!(
                "{}{}{}{}{}",
                prefix,
                ctx.palette.chrome(connector),
                format_meta(&ctx.palette, &meta.id_str, &meta.pos_str, &meta.horizon_str, &meta.warnings),
                desired_text,
                format_suffix(&ctx.palette, &meta.closure_str, &meta.glyphs),
            );
            lines.push(line);

            if !meta.child_ids.is_empty() {
                let new_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}{}", prefix, glyphs::TREE_VERTICAL)
                };
                render_tree_compact(ctx, &meta.child_ids, now, &new_prefix, lines);
            }
        }
    }

    /// Rich renderer — the Phase 3 layout.
    ///
    /// * **Depth 0 roots** render as a two-line zone: line 1 is
    ///   `╭─ #ID ▸pos  desire`, line 2 is `╰─ horizon · closure · glyphs`.
    ///   Children live at a 3-space indent (the column after the zone
    ///   edge) and an empty line is inserted between roots.
    /// * **Depth 1 siblings with children** get an empty rail line after
    ///   their subtree so long branches have breathing room.
    /// * **Warning signals** (OVERDUE, EXCEEDS_PARENT, PRESSURE) move to
    ///   their own danger-colored line below the tension rather than
    ///   getting jammed into the single-line layout where they would
    ///   fall off the right edge.
    /// * **Depth 2+** nodes render as dense single-line entries, same
    ///   shape as the compact renderer.
    fn render_tree_rich(
        ctx: &RenderCtx<'_>,
        root_ids: &[String],
        now: chrono::DateTime<Utc>,
        prefix: &str,
        depth: usize,
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
            let meta = compute_node_meta(ctx, node, now);

            if depth == 0 {
                // --- Root: two-line zone ---
                let indent_chars = 3; // `╭─ ` is 3 columns
                let head_chars = indent_chars + meta.id_str.chars().count()
                    + if meta.pos_str.is_empty() { 0 } else { meta.pos_str.chars().count() + 1 };
                let available = ctx.term_width.saturating_sub(head_chars + 2).max(40);
                let desired_text = smart_truncate(&node.tension.desired, available);

                // Line 1: identity
                let mut line1 = ctx.palette.structure(glyphs::TREE_ZONE_OPEN);
                line1.push(' ');
                line1.push_str(&ctx.palette.bold(&meta.id_str));
                if !meta.pos_str.is_empty() {
                    line1.push(' ');
                    line1.push_str(&ctx.palette.chrome(&meta.pos_str));
                }
                line1.push(' ');
                line1.push(' ');
                line1.push_str(&desired_text);
                lines.push(line1);

                // Line 2: metadata
                let mut meta_parts: Vec<String> = Vec::new();
                if !meta.horizon_str.is_empty() {
                    meta_parts.push(ctx.palette.chrome(&meta.horizon_str));
                }
                if !meta.closure_str.is_empty() {
                    meta_parts.push(ctx.palette.chrome(&meta.closure_str));
                }
                if !meta.glyphs.is_empty() {
                    meta_parts.push(ctx.palette.structure(&meta.glyphs.join("")));
                }
                let mut line2 = ctx.palette.structure(glyphs::TREE_ZONE_CLOSE);
                if !meta_parts.is_empty() {
                    line2.push(' ');
                    line2.push_str(&meta_parts.join(&ctx.palette.chrome(" · ")));
                }
                lines.push(line2);

                // Signal lines: show warnings below the zone (rich mode
                // never puts warnings on the identity line, so they
                // can't be truncated off the right edge).
                for w in &meta.warnings {
                    let mut signal_line = String::from("   ");
                    signal_line.push_str(&ctx.palette.bold(&ctx.palette.danger("! ")));
                    signal_line.push_str(&ctx.palette.danger(w));
                    lines.push(signal_line);
                }

                // Children under a root use a 3-column indent — the
                // column immediately after the zone edge. Roots do not
                // grow a vertical rail.
                if !meta.child_ids.is_empty() {
                    render_tree_rich(ctx, &meta.child_ids, now, "   ", 1, lines);
                }

                // Breathing room between roots.
                if !is_last {
                    lines.push(String::new());
                }
            } else {
                // --- Non-root: single-line like compact, plus signal lines ---
                let connector = if is_last { glyphs::TREE_LAST } else { glyphs::TREE_BRANCH };

                // Width calculation mirrors the compact renderer so the
                // truncation behavior matches.
                let prefix_chars = prefix.chars().count() + connector.chars().count();
                let mut meta_plain = meta.id_str.clone();
                if !meta.pos_str.is_empty() {
                    meta_plain.push(' ');
                    meta_plain.push_str(&meta.pos_str);
                }
                if !meta.horizon_str.is_empty() {
                    meta_plain.push(' ');
                    meta_plain.push_str(&meta.horizon_str);
                }
                meta_plain.push(' ');
                let mut suffix_plain = String::new();
                if !meta.closure_str.is_empty() {
                    suffix_plain.push_str("  ");
                    suffix_plain.push_str(&meta.closure_str);
                }
                if !meta.glyphs.is_empty() {
                    suffix_plain.push_str("  ");
                    for g in &meta.glyphs {
                        suffix_plain.push_str(g);
                    }
                }
                let chrome_width =
                    prefix_chars + meta_plain.chars().count() + suffix_plain.chars().count();
                let available = if ctx.term_width > chrome_width + 12 {
                    ctx.term_width - chrome_width
                } else {
                    40
                };
                let desired_text = smart_truncate(&node.tension.desired, available);

                // Warnings live on their own lines in rich mode, so the
                // identity line itself carries only the id/pos/horizon
                // metadata (no inline OVERDUE / EXCEEDS_PARENT).
                let empty_warnings: Vec<String> = Vec::new();
                let line = format!(
                    "{}{}{}{}{}",
                    prefix,
                    ctx.palette.chrome(connector),
                    format_meta(&ctx.palette, &meta.id_str, &meta.pos_str, &meta.horizon_str, &empty_warnings),
                    desired_text,
                    format_suffix(&ctx.palette, &meta.closure_str, &meta.glyphs),
                );
                lines.push(line);

                // Signal lines under the node, indented to match the
                // child rail position.
                if !meta.warnings.is_empty() {
                    let signal_prefix = if is_last {
                        format!("{}    ", prefix)
                    } else {
                        format!("{}{}", prefix, glyphs::TREE_VERTICAL)
                    };
                    for w in &meta.warnings {
                        let mut signal_line = signal_prefix.clone();
                        signal_line.push_str(&ctx.palette.bold(&ctx.palette.danger("! ")));
                        signal_line.push_str(&ctx.palette.danger(w));
                        lines.push(signal_line);
                    }
                }

                if !meta.child_ids.is_empty() {
                    let new_prefix = if is_last {
                        format!("{}    ", prefix)
                    } else {
                        format!("{}{}", prefix, glyphs::TREE_VERTICAL)
                    };
                    render_tree_rich(ctx, &meta.child_ids, now, &new_prefix, depth + 1, lines);

                    // Depth-1 siblings with children get breathing room:
                    // an empty rail line between this sibling's subtree
                    // and the next sibling. The last sibling gets no
                    // trailing blank.
                    if depth == 1 && !is_last {
                        lines.push(prefix.trim_end().to_string());
                    }
                }
            }
        }
    }

    fn format_meta(
        palette: &Palette,
        id_str: &str,
        pos_str: &str,
        horizon_str: &str,
        warnings: &[String],
    ) -> String {
        // ID is always bold — identity emphasis, earned by the primacy of
        // the short-code as an addressing scheme.
        let mut s = palette.bold(id_str);
        if !pos_str.is_empty() {
            s.push(' ');
            s.push_str(&palette.chrome(pos_str));
        }
        if !horizon_str.is_empty() {
            s.push(' ');
            s.push_str(&palette.chrome(horizon_str));
        }
        for w in warnings {
            // Warnings are bold danger — they demand attention and break
            // the signal-by-exception silence.
            s.push(' ');
            s.push_str(&palette.bold(&palette.danger(w)));
        }
        s.push(' ');
        s
    }

    fn format_suffix(palette: &Palette, closure_str: &str, glyphs: &[&str]) -> String {
        let mut s = String::new();
        if !closure_str.is_empty() {
            s.push_str("  ");
            s.push_str(&palette.chrome(closure_str));
        }
        if !glyphs.is_empty() {
            s.push_str("  ");
            s.push_str(&palette.structure(&glyphs.join("")));
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
        palette,
    };

    // Dispatch: rich layout only when the palette is enabled (which
    // already bundles interactive stdout + no NO_COLOR + not --json),
    // the terminal is wide enough for two-line roots, and the caller
    // has not forced --compact. Everything else — pipes, tests,
    // narrow terminals, NO_COLOR, explicit --compact — gets the
    // stable v1.5 single-line layout.
    let rich_mode = palette.is_enabled() && term_width >= 80 && !compact;

    let mut lines = Vec::new();
    if rich_mode {
        render_tree_rich(&ctx, forest.root_ids(), now, "", 0, &mut lines);
    } else {
        render_tree_compact(&ctx, forest.root_ids(), now, "", &mut lines);
    }

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
    println!(
        "{}",
        palette.chrome(&format!(
            "Total: {}  Active: {}  Resolved: {}  Released: {}",
            tensions.len(), active_count, resolved_count, released_count
        ))
    );

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
        println!(
            "{}",
            palette.chrome(&format!(
                "Deadlined: {}  Overdue: {}  Positioned: {}  Held: {}",
                deadlined, overdue, positioned, held
            ))
        );
    }

    Ok(())
}
