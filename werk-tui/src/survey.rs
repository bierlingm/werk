//! Survey view — time-first orientation.
//!
//! Shows all active tensions organised by temporal urgency. Horizons are
//! inherited from ancestors: a tension without its own deadline inherits the
//! nearest ancestor's deadline as its effective temporal frame.
//!
//! Layout follows the one spatial law (desire/future above, reality/past below):
//!
//!   ── later ────────────────────────────────────────────────
//!   Jun     ▸ conceptual foundation...      #13
//!      ⌐Jun · state machine spec...         #16 ← #13
//!
//!   ── this month ───────────────────────────────────────────
//!   May 30  ▸ FrankenTUI-first...           #3
//!      ⌐30  · survey view designed...       #18 ← #15
//!
//!   ── overdue ──────────────────────────────────────────────
//!   Mar 10  ▸ overdue tension               #99
//!
//! Zone compression: each band is always visible (min 1 summary line).
//! Cursor's band gets priority expansion. Other bands compress to summaries.

use chrono::{DateTime, Utc};
use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use sd_core::TensionStatus;

use crate::app::InstrumentApp;
use crate::glyphs;
use crate::theme::STYLES;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Which temporal band a tension belongs to.
/// Variant order determines display order: top of screen (future/desire)
/// to bottom (past/reality), honouring the one spatial law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TimeBand {
    NoDeadline,
    Later,
    ThisMonth,
    ThisWeek,
    Overdue,
}

impl TimeBand {
    pub fn label(&self) -> &'static str {
        match self {
            TimeBand::Overdue => "overdue",
            TimeBand::ThisWeek => "imminent",
            TimeBand::ThisMonth => "approaching",
            TimeBand::Later => "later",
            TimeBand::NoDeadline => "unframed",
        }
    }

}

/// A single selectable row in the survey view.
#[derive(Debug, Clone)]
pub struct SurveyItem {
    pub tension_id: String,
    pub short_code: Option<i32>,
    pub desired: String,
    /// Own horizon label (set directly on this tension).
    pub own_horizon_label: Option<String>,
    /// Effective horizon label (own or inherited from ancestor).
    pub effective_horizon_label: Option<String>,
    /// Whether the effective horizon is inherited (not own).
    pub horizon_inherited: bool,
    /// Effective deadline for band classification and sorting.
    pub effective_horizon_end: Option<DateTime<Utc>>,
    /// Whether this tension has children.
    pub has_children: bool,
    /// Closure ratio: (resolved_children, total_children). Only meaningful if has_children.
    pub closure: (usize, usize),
    /// Structural parent ID (for tree building within provider groups).
    pub parent_id: Option<String>,
    /// ID of the ancestor providing the effective horizon (None = own deadline).
    pub horizon_provider_id: Option<String>,
    /// Tree prefix string (e.g. "│ ├ ") computed during load for proper rendering.
    /// Empty for provider/standalone items.
    pub tree_prefix: String,
    pub band: TimeBand,
    /// 0.0 = fresh, 1.0 = at deadline, >1.0 = overdue.
    pub urgency: f64,
}

// ---------------------------------------------------------------------------
// Loading — with inherited horizons
// ---------------------------------------------------------------------------

impl InstrumentApp {
    /// Build the survey item list from all active tensions.
    ///
    /// Horizons are inherited: a tension without its own deadline inherits
    /// the nearest ancestor's deadline as its effective temporal frame.
    pub fn load_survey_items(&mut self) {
        let now = Utc::now();
        let week = chrono::Duration::days(7);
        let month = chrono::Duration::days(30);

        let all = match self.engine.store().list_tensions() {
            Ok(ts) => ts,
            Err(_) => return,
        };

        // Build id → tension map for parent/ancestor lookups.
        let tension_map: std::collections::HashMap<&str, &sd_core::Tension> =
            all.iter().map(|t| (t.id.as_str(), t)).collect();

        // Child count and resolved count per tension (for closure ratio).
        let mut child_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        let mut resolved_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for t in &all {
            if let Some(ref pid) = t.parent_id {
                *child_counts.entry(pid.as_str()).or_insert(0) += 1;
                if t.status == TensionStatus::Resolved {
                    *resolved_counts.entry(pid.as_str()).or_insert(0) += 1;
                }
            }
        }

        let now_year = chrono::Datelike::year(&now);

        let mut items: Vec<SurveyItem> = all
            .iter()
            .filter(|t| t.status == TensionStatus::Active)
            .map(|t| {
                // Walk ancestry to find effective horizon.
                let own_end = t.horizon.as_ref().map(|h| h.range_end());
                let own_label = t.horizon.as_ref()
                    .map(|h| glyphs::compact_horizon(h, now_year));

                let (effective_end, effective_label, inherited, provider_id) = if own_end.is_some() {
                    (own_end, own_label.clone(), false, None)
                } else {
                    let (end, label, inh, pid) = find_ancestor_horizon(t, &tension_map, now_year);
                    (end, label, inh, pid)
                };

                let (band, urgency) = classify_band(&effective_end, now, week, month);
                let total_children = child_counts.get(t.id.as_str()).copied().unwrap_or(0);
                let has_children = total_children > 0;
                let resolved_children = resolved_counts.get(t.id.as_str()).copied().unwrap_or(0);

                SurveyItem {
                    tension_id: t.id.clone(),
                    short_code: t.short_code,
                    desired: t.desired.clone(),
                    own_horizon_label: own_label,
                    effective_horizon_label: effective_label,
                    horizon_inherited: inherited,
                    effective_horizon_end: effective_end,
                    has_children,
                    closure: (resolved_children, total_children),
                    parent_id: t.parent_id.clone(),
                    horizon_provider_id: provider_id,
                    tree_prefix: String::new(), // computed after tree ordering
                    band,
                    urgency,
                }
            })
            .collect();

        // Phase 1: Sort by band, then by effective deadline descending.
        items.sort_by(|a, b| {
            a.band.cmp(&b.band)
                .then_with(|| match (&a.effective_horizon_end, &b.effective_horizon_end) {
                    (Some(ae), Some(be)) => be.cmp(ae),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => a.tension_id.cmp(&b.tension_id),
                })
        });

        // Phase 2: Within each band, reorder into depth-first tree order
        // grouped by provider, and compute tree prefixes.
        items = tree_order_within_bands(items);

        // Clamp cursor.
        if !items.is_empty() && self.survey_cursor >= items.len() {
            self.survey_cursor = items.len() - 1;
        }

        self.survey_items = items;
    }
}

/// Walk up the ancestry chain to find the nearest horizon.
/// Returns (end, label, inherited, provider_id).
fn find_ancestor_horizon(
    tension: &sd_core::Tension,
    map: &std::collections::HashMap<&str, &sd_core::Tension>,
    now_year: i32,
) -> (Option<DateTime<Utc>>, Option<String>, bool, Option<String>) {
    let mut current_pid = tension.parent_id.as_deref();
    // Guard against cycles: max 20 levels deep.
    for _ in 0..20 {
        match current_pid {
            None => return (None, None, false, None),
            Some(pid) => {
                if let Some(ancestor) = map.get(pid) {
                    if let Some(ref h) = ancestor.horizon {
                        let label = glyphs::compact_horizon(h, now_year);
                        return (Some(h.range_end()), Some(label), true, Some(ancestor.id.clone()));
                    }
                    current_pid = ancestor.parent_id.as_deref();
                } else {
                    return (None, None, false, None);
                }
            }
        }
    }
    (None, None, false, None)
}

/// Reorder items within each band into depth-first tree order, grouped by
/// horizon provider. Computes proper tree prefix strings (│ ├ └) for each item.
fn tree_order_within_bands(items: Vec<SurveyItem>) -> Vec<SurveyItem> {
    let mut result = Vec::with_capacity(items.len());

    // Process each band separately.
    let mut i = 0;
    while i < items.len() {
        let band = items[i].band;
        let band_start = i;
        while i < items.len() && items[i].band == band {
            i += 1;
        }
        let band_items = &items[band_start..i];

        // Identify provider groups: items sharing the same provider_id.
        // Standalone items (no provider) are their own group.
        let mut groups: Vec<Vec<&SurveyItem>> = Vec::new();
        let mut group_map: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

        for item in band_items {
            match item.horizon_provider_id.as_deref() {
                Some(pid) => {
                    if let Some(&group_idx) = group_map.get(pid) {
                        groups[group_idx].push(item);
                    } else {
                        // Check if the provider itself is already a standalone group.
                        if let Some(&group_idx) = group_map.get(item.tension_id.as_str()) {
                            groups[group_idx].push(item);
                            group_map.insert(pid, group_idx);
                        } else {
                            let idx = groups.len();
                            group_map.insert(pid, idx);
                            groups.push(vec![item]);
                        }
                    }
                }
                None => {
                    // Standalone: might be a provider for later items.
                    if let Some(&group_idx) = group_map.get(item.tension_id.as_str()) {
                        // Insert provider at the front of its group.
                        groups[group_idx].insert(0, item);
                    } else {
                        let idx = groups.len();
                        group_map.insert(&item.tension_id, idx);
                        groups.push(vec![item]);
                    }
                }
            }
        }

        // For each group, do a depth-first tree walk.
        for group in &groups {
            if group.len() == 1 {
                // Single item — standalone, no tree prefix.
                let mut item = group[0].clone();
                item.tree_prefix = String::new();
                result.push(item);
                continue;
            }

            // Find the provider (first item without horizon_provider_id, or first item).
            let provider_idx = group.iter().position(|it| it.horizon_provider_id.is_none())
                .unwrap_or(0);
            let provider = group[provider_idx];

            // Push the provider first (no prefix).
            let mut p = provider.clone();
            p.tree_prefix = String::new();
            result.push(p);

            // Build parent → children mapping for the remaining items.
            let inheritors: Vec<&SurveyItem> = group.iter()
                .filter(|it| it.tension_id != provider.tension_id)
                .copied()
                .collect();

            // Map from id → [children in this group].
            let item_ids: std::collections::HashSet<&str> = group.iter()
                .map(|it| it.tension_id.as_str())
                .collect();
            let mut children_of: std::collections::HashMap<&str, Vec<&SurveyItem>> =
                std::collections::HashMap::new();

            for item in &inheritors {
                // Walk up from this item to find the nearest ancestor that's IN the group.
                let effective_parent = item.parent_id.as_deref();
                // Walk up until we find an ancestor in the group or run out.
                let mut found_parent: &str = &provider.tension_id;
                if let Some(pid) = effective_parent {
                    // Check each ancestor: is it in the group?
                    let mut current = pid;
                    for _ in 0..20 {
                        if item_ids.contains(current) {
                            found_parent = current;
                            break;
                        }
                        // Walk up: find this id's parent in the group items.
                        // We need the raw parent_id chain. Check if any group item has this id.
                        let parent_of_current = group.iter()
                            .find(|it| it.tension_id == current)
                            .and_then(|it| it.parent_id.as_deref());
                        match parent_of_current {
                            Some(pp) => current = pp,
                            None => break,
                        }
                    }
                }
                children_of.entry(found_parent).or_default().push(item);
            }

            // Depth-first walk from the provider.
            let mut stack: Vec<(&str, Vec<bool>)> = Vec::new();
            // is_last_stack tracks whether each ancestor was the last child.
            // Start with provider's children.
            if let Some(kids) = children_of.get(provider.tension_id.as_str()) {
                for (ci, child) in kids.iter().enumerate().rev() {
                    let is_last = ci == kids.len() - 1;
                    stack.push((&child.tension_id, vec![is_last]));
                }
            }

            while let Some((node_id, is_last_stack)) = stack.pop() {
                // Build prefix from is_last_stack.
                let mut prefix = String::new();
                for (level, &is_last) in is_last_stack.iter().enumerate() {
                    if level < is_last_stack.len() - 1 {
                        // Ancestor column: │ if ancestor has more siblings, space if not.
                        if is_last {
                            prefix.push_str("  ");
                        } else {
                            prefix.push_str("\u{2502} "); // │ + space
                        }
                    } else {
                        // This node's own level: ├ or └.
                        if is_last {
                            prefix.push_str("\u{2514} "); // └ + space
                        } else {
                            prefix.push_str("\u{251c} "); // ├ + space
                        }
                    }
                }

                // Find the item and push it.
                if let Some(item) = inheritors.iter().find(|it| it.tension_id == node_id) {
                    let mut clone = (*item).clone();
                    clone.tree_prefix = prefix;
                    result.push(clone);

                    // Push children (in reverse so first child is popped first).
                    if let Some(kids) = children_of.get(node_id) {
                        for (ci, child) in kids.iter().enumerate().rev() {
                            let is_last = ci == kids.len() - 1;
                            let mut child_stack = is_last_stack.clone();
                            child_stack.push(is_last);
                            stack.push((&child.tension_id, child_stack));
                        }
                    }
                }
            }
        }
    }

    result
}

fn classify_band(
    horizon_end: &Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    week: chrono::Duration,
    month: chrono::Duration,
) -> (TimeBand, f64) {
    match horizon_end {
        None => (TimeBand::NoDeadline, 0.0),
        Some(end) => {
            let days_until = (*end - now).num_hours() as f64 / 24.0;
            if days_until < 0.0 {
                (TimeBand::Overdue, 1.0 + (-days_until / 30.0).min(9.0))
            } else if *end <= now + week {
                (TimeBand::ThisWeek, days_until / 7.0)
            } else if *end <= now + month {
                (TimeBand::ThisMonth, days_until / 30.0)
            } else {
                (TimeBand::Later, 0.0)
            }
        }
    }
}

fn truncate_desired(s: &str, max_chars: usize) -> String {
    let s = s.trim();
    let first_line = s.lines().next().unwrap_or(s);
    let chars: Vec<char> = first_line.chars().collect();
    if chars.len() <= max_chars {
        first_line.to_string()
    } else {
        format!("{}…", chars[..max_chars.saturating_sub(1)].iter().collect::<String>())
    }
}

// ---------------------------------------------------------------------------
// Zone expansion — decide how many items to show per band
// ---------------------------------------------------------------------------

/// How a band should be rendered.
struct BandExpansion {
    /// First item index in the global survey_items list.
    start: usize,
    /// Number of items in this band.
    count: usize,
    /// How many to show individually.
    show: usize,
}

/// Compute how many items to show per band given available rows.
fn compute_band_expansion(
    items: &[SurveyItem],
    cursor: usize,
    available_rows: usize,
) -> Vec<(TimeBand, BandExpansion)> {
    // Identify non-empty bands and their ranges.
    let mut bands: Vec<(TimeBand, usize, usize)> = Vec::new(); // (band, start, count)
    let mut i = 0;
    while i < items.len() {
        let band = items[i].band;
        let start = i;
        while i < items.len() && items[i].band == band {
            i += 1;
        }
        bands.push((band, start, i - start));
    }

    if bands.is_empty() {
        return Vec::new();
    }

    let cursor_band = items.get(cursor).map(|it| it.band);

    // Each band needs: 1 header line + at least 1 content line (item or summary).
    // Blank line between bands (except first): bands.len() - 1.
    let overhead = bands.len() * 2 + bands.len().saturating_sub(1);
    let content_rows = available_rows.saturating_sub(overhead);

    // First pass: give every band 1 line (summary).
    let mut allocs: Vec<usize> = vec![1; bands.len()];
    let mut used = bands.len(); // 1 per band

    // Second pass: expand the cursor's band up to its full count.
    if let Some(cursor_idx) = bands.iter().position(|b| Some(b.0) == cursor_band) {
        let max = bands[cursor_idx].2;
        let can_give = content_rows.saturating_sub(used).min(max.saturating_sub(1));
        allocs[cursor_idx] += can_give;
        used += can_give;
    }

    // Third pass: distribute remaining rows to other bands (overdue first, then this_week, etc).
    // Iterate in reverse order of TimeBand (overdue = most important to expand).
    let priority_order: Vec<usize> = {
        let mut idxs: Vec<usize> = (0..bands.len()).collect();
        idxs.sort_by(|&a, &b| bands[b].0.cmp(&bands[a].0));
        idxs
    };
    for &band_idx in &priority_order {
        if used >= content_rows {
            break;
        }
        let max = bands[band_idx].2;
        let can_give = content_rows.saturating_sub(used).min(max.saturating_sub(allocs[band_idx]));
        allocs[band_idx] += can_give;
        used += can_give;
    }

    bands.into_iter().enumerate().map(|(i, (band, start, count))| {
        (band, BandExpansion {
            start,
            count,
            show: allocs[i].min(count),
        })
    }).collect()
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

const HORIZON_COL_W: usize = 9; // "Apr 10   " — date + trailing padding
const SURVEY_INDENT: &str = "  ";

impl InstrumentApp {
    pub fn render_survey(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);

        let w = area.width as usize;
        let items = &self.survey_items;

        if items.is_empty() {
            let line = Line::from_spans([
                Span::styled("  No active tensions.", STYLES.dim),
            ]);
            Paragraph::new(Text::from_lines(vec![line])).render(area, frame);
            return;
        }

        let left_w = SURVEY_INDENT.len() + HORIZON_COL_W;
        // Main column fills most of the width. Right column is computed per-line
        // from actual content (closure ratio is short, no need to reserve fixed space).
        let main_w = w.saturating_sub(left_w);

        let expansions = compute_band_expansion(items, self.survey_cursor, area.height as usize);

        let mut lines: Vec<Line> = Vec::new();
        let mut cursor_line: usize = 0;

        for (band, exp) in &expansions {
            // Blank line before band (except first).
            if !lines.is_empty() {
                lines.push(Line::from_spans([Span::raw("")]));
            }

            // Band header.
            let band_label = band.label();
            let count_label = format!(" ({})", exp.count);
            let rule_w = w.saturating_sub(4 + band_label.len() + count_label.len() + 3);
            let rule = "\u{2500}".repeat(rule_w);
            lines.push(Line::from_spans([
                Span::styled(
                    format!("{SURVEY_INDENT}\u{2500}\u{2500} {band_label}{count_label} {rule}"),
                    STYLES.dim,
                ),
            ]));

            // Which items from this band to show.
            let (show_start, show_end) = if exp.show >= exp.count {
                (exp.start, exp.start + exp.count)
            } else {
                let cursor_in_band = self.survey_cursor >= exp.start
                    && self.survey_cursor < exp.start + exp.count;
                if cursor_in_band {
                    let cursor_offset = self.survey_cursor - exp.start;
                    let half = exp.show / 2;
                    let win_start = if cursor_offset > half {
                        (cursor_offset - half).min(exp.count - exp.show)
                    } else {
                        0
                    };
                    (exp.start + win_start, exp.start + win_start + exp.show)
                } else {
                    (exp.start, exp.start + exp.show)
                }
            };

            let hidden_above = show_start - exp.start;
            let hidden_below = (exp.start + exp.count) - show_end;

            // "... N above" summary at top of visible window.
            if hidden_above > 0 {
                let style = STYLES.dim;
                lines.push(Line::from_spans([
                    Span::styled(
                        format!("{SURVEY_INDENT}{:>width$}\u{2191} {hidden_above} above", "", width = HORIZON_COL_W),
                        style,
                    ),
                ]));
            }

            // Render visible items. Tree prefixes are pre-computed during loading.
            for idx in show_start..show_end {
                let item = &items[idx];
                let is_selected = idx == self.survey_cursor;
                if is_selected {
                    cursor_line = lines.len();
                }

                if item.tree_prefix.is_empty() {
                    lines.push(render_provider_line(item, is_selected, w, main_w));
                } else {
                    lines.push(render_tree_child_line(item, is_selected, w, main_w));
                }
            }

            // "... N below" summary at bottom of visible window.
            if hidden_below > 0 {
                let summary_selected = self.survey_cursor >= show_end
                    && self.survey_cursor < exp.start + exp.count;
                if summary_selected {
                    cursor_line = lines.len();
                }
                let style = if summary_selected { STYLES.selected } else { STYLES.dim };
                lines.push(Line::from_spans([
                    Span::styled(
                        format!("{SURVEY_INDENT}{:>width$}\u{2193} {hidden_below} below", "", width = HORIZON_COL_W),
                        style,
                    ),
                ]));
            }
        }

        // Scroll so the cursor line is visible.
        let available = area.height as usize;
        let scroll = compute_scroll(cursor_line, available, lines.len());

        let text = Text::from_lines(lines);
        Paragraph::new(text).scroll((scroll as u16, 0)).render(area, frame);
    }

    /// Render the survey bottom bar.
    pub fn render_survey_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        let content = self.content_area(Rect::new(area.x, area.y, area.width, area.height + 10));
        let bar_area = Rect::new(content.x, area.y, content.width, 1);

        let total = self.survey_items.len();
        let overdue = self.survey_items.iter().filter(|i| i.band == TimeBand::Overdue).count();

        let left = if overdue > 0 {
            format!("{overdue} overdue")
        } else {
            format!("{total} active")
        };
        let center = "Tab pivot \u{00B7} Shift+Tab return \u{00B7} j/k navigate \u{00B7} Enter descend";

        let w = bar_area.width as usize;
        let left_w = left.chars().count();
        let center_w = center.chars().count();
        let center_start = w.saturating_sub(center_w) / 2;

        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::styled(&left, if overdue > 0 { STYLES.amber } else { STYLES.dim }));

        if center_start > left_w + 1 {
            let pad = " ".repeat(center_start - left_w);
            spans.push(Span::styled(pad, STYLES.dim));
            spans.push(Span::styled(center, STYLES.dim));
        }

        // Pad to full width.
        let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        if used < w {
            spans.push(Span::styled(" ".repeat(w - used), STYLES.dim));
        }

        Paragraph::new(Text::from_lines(vec![Line::from_spans(spans)]))
            .render(bar_area, frame);
    }
}

/// Render a provider/standalone item line (has own deadline or no deadline).
///
/// Layout: `  Jun      desire text...                   #2 → [4/8]`
fn render_provider_line(
    item: &SurveyItem,
    is_selected: bool,
    w: usize,
    main_w: usize,
) -> Line {
    // Plain text for the horizon column. Emoji-range glyphs (⏱) cause width
    // misalignment: unicode-width reports 1 cell but terminals render 2.
    let horizon_str = match &item.own_horizon_label {
        Some(label) => {
            let padded = format!("{:<width$}", label, width = HORIZON_COL_W);
            padded.chars().take(HORIZON_COL_W).collect::<String>()
        }
        None => " ".repeat(HORIZON_COL_W),
    };

    let right_text = build_right_col(item);
    let right_w = right_text.chars().count();
    let desire_w = main_w.saturating_sub(right_w + 2);
    let desire_text = truncate_desired(&item.desired, desire_w);

    let style_text = if is_selected { STYLES.selected } else { STYLES.text };
    let style_dim = if is_selected { STYLES.selected } else { STYLES.dim };
    let style_horizon = if item.urgency > 1.0 {
        if is_selected { STYLES.selected } else { STYLES.amber }
    } else {
        style_dim
    };

    let mut spans = vec![
        Span::styled(SURVEY_INDENT.to_string(), style_dim),
        Span::styled(horizon_str, style_horizon),
        Span::styled(desire_text, style_text),
        Span::styled("  ".to_string(), style_dim),
        Span::styled(right_text, style_dim),
    ];

    pad_to_width(&mut spans, w, style_dim);
    Line::from_spans(spans)
}

/// Render a tree-child item line (inherits deadline from a provider ancestor).
/// Uses the pre-computed tree_prefix for proper │/├/└ connector lines.
///
/// Layout: `          │ ├ desire text...               #18 → [0/3]`
fn render_tree_child_line(
    item: &SurveyItem,
    is_selected: bool,
    w: usize,
    main_w: usize,
) -> Line {
    let base_indent = SURVEY_INDENT.len() + HORIZON_COL_W;
    let prefix_w = item.tree_prefix.chars().count();

    let right_text = build_right_col(item);
    let right_w = right_text.chars().count();
    let desire_w = main_w.saturating_sub(prefix_w + right_w + 2);
    let desire_text = truncate_desired(&item.desired, desire_w);

    let style_text = if is_selected { STYLES.selected } else { STYLES.text };
    let style_dim = if is_selected { STYLES.selected } else { STYLES.dim };

    let mut spans = vec![
        Span::styled(" ".repeat(base_indent), style_dim),
        Span::styled(item.tree_prefix.clone(), style_dim),
        Span::styled(desire_text, style_text),
        Span::styled("  ".to_string(), style_dim),
        Span::styled(right_text, style_dim),
    ];

    pad_to_width(&mut spans, w, style_dim);
    Line::from_spans(spans)
}

/// Build the right column: #code + → (children) + closure ratio.
/// Trace-facing data (facts about the tension's state).
fn build_right_col(item: &SurveyItem) -> String {
    let code_str = item.short_code.map(|c| format!("#{c}")).unwrap_or_default();
    let children_indicator = if item.has_children { " \u{2192}" } else { "" };
    let closure_str = if item.has_children {
        format!(" [{}/{}]", item.closure.0, item.closure.1)
    } else {
        String::new()
    };
    format!("{code_str}{children_indicator}{closure_str}")
}

fn pad_to_width(spans: &mut Vec<Span>, w: usize, style: ftui::style::Style) {
    let total: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if total < w {
        spans.push(Span::styled(" ".repeat(w - total), style));
    }
}

// ---------------------------------------------------------------------------
// Scroll helpers
// ---------------------------------------------------------------------------

/// Compute scroll offset so that `target_line` is visible within `viewport`.
fn compute_scroll(target_line: usize, viewport: usize, total_lines: usize) -> usize {
    if total_lines <= viewport {
        return 0;
    }
    let context = 3_usize;
    let ideal_top = target_line.saturating_sub(context);
    let max_scroll = total_lines.saturating_sub(viewport);
    ideal_top.min(max_scroll)
}
