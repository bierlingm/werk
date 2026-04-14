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

use werk_core::TensionStatus;

use crate::app::InstrumentApp;
use crate::glyphs;
use crate::theme::InstrumentStyles;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Field-wide vitals for the NOW zone in the survey.
#[derive(Debug, Clone, Default)]
pub struct FieldVitals {
    pub active: usize,
    pub overdue: usize,
    pub imminent: usize,
    pub approaching: usize,
    pub held_unframed: usize,
    pub stale_realities: usize,
    /// Count of tensions with any structural signal (excluding plain overdue, which is in overdue count).
    pub signaled: usize,
}

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

    /// Count items in this band from a sorted items list.
    pub fn count_in(&self, items: &[SurveyItem]) -> usize {
        items.iter().filter(|it| it.band == *self).count()
    }

    /// All band variants in display order.
    pub fn all() -> &'static [TimeBand] {
        &[
            TimeBand::Overdue,
            TimeBand::ThisWeek,
            TimeBand::ThisMonth,
            TimeBand::Later,
            TimeBand::NoDeadline,
        ]
    }

    /// Create a HashMap of per-band ListStates (used by app initialization).
    pub fn all_band_states()
    -> std::collections::HashMap<TimeBand, std::cell::RefCell<ftui::widgets::list::ListState>> {
        let mut m = std::collections::HashMap::new();
        for &b in Self::all() {
            m.insert(
                b,
                std::cell::RefCell::new(ftui::widgets::list::ListState::default()),
            );
        }
        m
    }
}

// ---------------------------------------------------------------------------
// Band ranges — identify contiguous band regions in survey_items
// ---------------------------------------------------------------------------

/// A contiguous range of items belonging to one time band.
#[derive(Debug, Clone)]
pub struct BandRange {
    pub band: TimeBand,
    /// Start index in survey_items (inclusive).
    pub start: usize,
    /// Item count in this band.
    pub count: usize,
}

/// Compute band ranges from sorted survey_items.
pub fn compute_band_ranges(items: &[SurveyItem]) -> Vec<BandRange> {
    let mut ranges = Vec::new();
    let mut i = 0;
    while i < items.len() {
        let band = items[i].band;
        let start = i;
        while i < items.len() && items[i].band == band {
            i += 1;
        }
        ranges.push(BandRange {
            band,
            start,
            count: i - start,
        });
    }
    ranges
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
    /// True if the tension has no position (acknowledged but uncommitted to sequence).
    pub is_held: bool,
    /// True if this is the next positioned step in its parent's sequence (lowest position).
    pub is_next: bool,
    /// Signal glyphs (by exception — empty for most tensions).
    pub signal_glyphs: Vec<&'static str>,
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
        let tension_map: std::collections::HashMap<&str, &werk_core::Tension> =
            all.iter().map(|t| (t.id.as_str(), t)).collect();

        // Child count and resolved count per tension (for closure ratio).
        let mut child_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        let mut resolved_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        // Track the lowest-positioned active child per parent (= "next step").
        let mut next_step_per_parent: std::collections::HashMap<&str, (&str, i32)> =
            std::collections::HashMap::new();
        for t in &all {
            if let Some(ref pid) = t.parent_id {
                *child_counts.entry(pid.as_str()).or_insert(0) += 1;
                if t.status == TensionStatus::Resolved {
                    *resolved_counts.entry(pid.as_str()).or_insert(0) += 1;
                }
                // Track next step: lowest position among active positioned children.
                if t.status == TensionStatus::Active {
                    if let Some(pos) = t.position {
                        let entry = next_step_per_parent.entry(pid.as_str());
                        entry
                            .and_modify(|(id, cur_pos)| {
                                if pos < *cur_pos {
                                    *id = t.id.as_str();
                                    *cur_pos = pos;
                                }
                            })
                            .or_insert((t.id.as_str(), pos));
                    }
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
                let own_label = t
                    .horizon
                    .as_ref()
                    .map(|h| glyphs::compact_horizon(h, now_year));

                let (effective_end, effective_label, inherited, provider_id) = if own_end.is_some()
                {
                    (own_end, own_label.clone(), false, None)
                } else {
                    let (end, label, inh, pid) = find_ancestor_horizon(t, &tension_map, now_year);
                    (end, label, inh, pid)
                };

                let (band, urgency) = classify_band(&effective_end, now, week, month);
                let total_children = child_counts.get(t.id.as_str()).copied().unwrap_or(0);
                let has_children = total_children > 0;
                let resolved_children = resolved_counts.get(t.id.as_str()).copied().unwrap_or(0);

                // Is this tension the "next step" in its parent's sequence?
                let is_next = t
                    .parent_id
                    .as_deref()
                    .and_then(|pid| next_step_per_parent.get(pid))
                    .map(|(next_id, _)| *next_id == t.id.as_str())
                    .unwrap_or(false);

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
                    is_held: t.position.is_none(),
                    is_next,
                    signal_glyphs: vec![],
                }
            })
            .collect();

        // Compute structural signals for each item.
        if let Ok(forest) = werk_core::Forest::from_tensions(all.clone()) {
            let field_structural = werk_core::compute_structural_signals(&forest);

            for idx in 0..items.len() {
                let tension_id = items[idx].tension_id.clone();
                let temporal = werk_core::compute_temporal_signals(&forest, &tension_id, now);

                // Overdue glyph (band already colors it, but glyph is explicit signal mark)
                if items[idx].band == TimeBand::Overdue {
                    items[idx].signal_glyphs.push("!");
                }
                if temporal.on_critical_path {
                    items[idx].signal_glyphs.push("\u{2021}"); // ‡
                }
                if temporal.has_containment_violation {
                    items[idx].signal_glyphs.push("\u{21a5}"); // ↥
                }
                if !temporal.sequencing_pressures.is_empty() {
                    items[idx].signal_glyphs.push("\u{21c5}"); // ⇅
                }
                if !temporal.critical_path.is_empty() && !temporal.on_critical_path {
                    items[idx].signal_glyphs.push("\u{2021}"); // ‡ (as parent)
                }
                if !temporal.containment_violations.is_empty()
                    && !temporal.has_containment_violation
                {
                    items[idx].signal_glyphs.push("\u{21a5}"); // ↥ (as parent)
                }

                // Horizon drift — only RepeatedPostponement/Oscillating (noise threshold)
                let t = all.iter().find(|t| t.id == tension_id);
                if t.is_some_and(|t| t.horizon.is_some()) {
                    if let Ok(mutations) = self.engine.store().get_mutations(&tension_id) {
                        let drift = werk_core::detect_horizon_drift(&tension_id, &mutations);
                        match drift.drift_type {
                            werk_core::HorizonDriftType::RepeatedPostponement
                            | werk_core::HorizonDriftType::Oscillating => {
                                items[idx].signal_glyphs.push("\u{219d}"); // ↝
                            }
                            _ => {}
                        }
                    }
                }

                // Structural signals
                if let Some(ss) = field_structural.signals.get(&tension_id) {
                    if ss
                        .centrality
                        .map(|c| c > self.signal_thresholds.hub_centrality)
                        .unwrap_or(false)
                    {
                        items[idx].signal_glyphs.push("\u{25c9}"); // ◉ HUB
                    }
                    if ss.on_longest_path {
                        items[idx].signal_glyphs.push("\u{2503}"); // ┃ SPINE
                    }
                }
            }
        }

        // Phase 1: Sort by band, then by effective deadline descending.
        items.sort_by(|a, b| {
            a.band.cmp(&b.band).then_with(|| {
                match (&a.effective_horizon_end, &b.effective_horizon_end) {
                    (Some(ae), Some(be)) => be.cmp(ae),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => a.tension_id.cmp(&b.tension_id),
                }
            })
        });

        // Phase 2: Within each band, reorder into depth-first tree order
        // grouped by provider, and compute tree prefixes.
        items = tree_order_within_bands(items);

        // Compute field vitals for the NOW zone.
        let mut vitals = FieldVitals {
            active: items.len(),
            ..Default::default()
        };
        for item in &items {
            match item.band {
                TimeBand::Overdue => vitals.overdue += 1,
                TimeBand::ThisWeek => vitals.imminent += 1,
                TimeBand::ThisMonth => vitals.approaching += 1,
                _ => {}
            }
            if item.is_held && item.band == TimeBand::NoDeadline {
                vitals.held_unframed += 1;
            }
            // Count tensions with non-overdue signals (overdue already counted above)
            if item.signal_glyphs.iter().any(|g| *g != "!") {
                vitals.signaled += 1;
            }
        }
        // Stale realities: active tensions whose last 'actual' mutation is >7 days old.
        let active_ids: Vec<&str> = all
            .iter()
            .filter(|t| t.status == TensionStatus::Active)
            .map(|t| t.id.as_str())
            .collect();
        if let Ok(last_actuals) = self
            .engine
            .store()
            .get_last_mutation_timestamps(&active_ids, &["actual"])
        {
            let stale_cutoff = now - chrono::Duration::days(7);
            for t in all.iter().filter(|t| t.status == TensionStatus::Active) {
                match last_actuals.get(&t.id) {
                    Some(ts) if *ts < stale_cutoff => vitals.stale_realities += 1,
                    None => {} // never updated — not stale, just new
                    _ => {}
                }
            }
        }
        self.field_vitals = vitals;

        // Clamp cursor.
        if !items.is_empty() && self.survey_cursor >= items.len() {
            self.survey_cursor = items.len() - 1;
        }

        self.survey_items = items;
        self.sync_survey_band_states();
    }

    /// Compute band + offset from survey_cursor and sync all per-band ListStates.
    /// Active band gets `select(Some(offset))`, others get `selected = None`.
    pub fn sync_survey_band_states(&self) {
        let ranges = compute_band_ranges(&self.survey_items);
        let focused_band = self.survey_items.get(self.survey_cursor).map(|it| it.band);

        for range in &ranges {
            if let Some(state_cell) = self.survey_band_states.get(&range.band) {
                let mut state = state_cell.borrow_mut();
                if Some(range.band) == focused_band {
                    let offset = self.survey_cursor.saturating_sub(range.start);
                    state.select(Some(offset));
                } else {
                    state.selected = None;
                }
            }
        }
    }

    /// Get the focused band (the band containing survey_cursor).
    pub fn survey_focused_band(&self) -> Option<TimeBand> {
        self.survey_items.get(self.survey_cursor).map(|it| it.band)
    }
}

/// Walk up the ancestry chain to find the nearest horizon.
/// Returns (end, label, inherited, provider_id).
fn find_ancestor_horizon(
    tension: &werk_core::Tension,
    map: &std::collections::HashMap<&str, &werk_core::Tension>,
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
                        return (
                            Some(h.range_end()),
                            Some(label),
                            true,
                            Some(ancestor.id.clone()),
                        );
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
        let mut group_map: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();

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
            let provider_idx = group
                .iter()
                .position(|it| it.horizon_provider_id.is_none())
                .unwrap_or(0);
            let provider = group[provider_idx];

            // Push the provider first (no prefix).
            let mut p = provider.clone();
            p.tree_prefix = String::new();
            result.push(p);

            // Build parent → children mapping for the remaining items.
            let inheritors: Vec<&SurveyItem> = group
                .iter()
                .filter(|it| it.tension_id != provider.tension_id)
                .copied()
                .collect();

            // Map from id → [children in this group].
            let item_ids: std::collections::HashSet<&str> =
                group.iter().map(|it| it.tension_id.as_str()).collect();
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
                        let parent_of_current = group
                            .iter()
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
        format!(
            "{}…",
            chars[..max_chars.saturating_sub(1)]
                .iter()
                .collect::<String>()
        )
    }
}

// ---------------------------------------------------------------------------
// Rendering — List widget (ftui)
// ---------------------------------------------------------------------------

const HORIZON_COL_W: usize = 9; // "Apr 10   " — date + trailing padding
const SURVEY_INDENT: &str = "  ";

impl InstrumentApp {
    pub fn render_survey(&self, area: &Rect, frame: &mut Frame<'_>) {
        use ftui::layout::{Constraint, Flex};
        use ftui::widgets::StatefulWidget;
        use ftui::widgets::block::Block;
        use ftui::widgets::borders::{BorderType, Borders};
        use ftui::widgets::list::{List, ListItem};

        let full_area = self.layout.content_area(*area);

        if self.survey_items.is_empty() {
            let line = Line::from_spans([Span::styled("  No active tensions.", self.styles.dim)]);
            Paragraph::new(Text::from_lines(vec![line])).render(full_area, frame);
            return;
        }

        let area = full_area;

        let ranges = compute_band_ranges(&self.survey_items);
        let focused_band = self.survey_focused_band();

        // Sync per-band ListStates from survey_cursor.
        self.sync_survey_band_states();

        // Height allocation:
        // 1. Each band's natural height = item count + 2 (borders).
        // 2. If all bands fit, use natural heights (no wasted space).
        // 3. If they don't fit, focused band gets Fill (scrollable),
        //    others get capped to share remaining space evenly.
        let total_h = area.height as usize;
        let min_band: usize = 3; // border + 1 item + border

        let natural: Vec<usize> = ranges.iter().map(|r| r.count + 2).collect();
        let total_natural: usize = natural.iter().sum();

        let constraints: Vec<Constraint> = if total_natural <= total_h {
            // Everything fits — use exact natural heights, no empty Fill space.
            natural
                .iter()
                .map(|&h| Constraint::Fixed(h as u16))
                .collect()
        } else {
            // Overflow — largest band gets Fill (it needs scrolling most),
            // all others get capped Fixed heights.
            let largest_idx = natural
                .iter()
                .enumerate()
                .max_by_key(|&(_, h)| h)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let others_count = ranges.len().saturating_sub(1);
            let others_budget = total_h.saturating_sub(5); // reserve min 5 for Fill band
            let per_band_budget = if others_count > 0 {
                others_budget / others_count
            } else {
                0
            };

            ranges
                .iter()
                .enumerate()
                .map(|(i, range)| {
                    if i == largest_idx {
                        Constraint::Fill
                    } else {
                        let h = (range.count + 2).min(per_band_budget).max(min_band);
                        Constraint::Fixed(h as u16)
                    }
                })
                .collect()
        };

        let slots = Flex::vertical().constraints(constraints).split(area);

        // Render each band as a List inside a Block.
        for (bi, range) in ranges.iter().enumerate() {
            let slot = slots[bi];
            let is_focused = Some(range.band) == focused_band;
            // -2 for block borders, -1 for scroll indicator gutter
            let inner_w = slot.width.saturating_sub(3) as usize;

            let list_items: Vec<ListItem> = (range.start..range.start + range.count)
                .map(|data_idx| {
                    let item = &self.survey_items[data_idx];
                    let line = if item.tree_prefix.is_empty() {
                        render_provider_line(item, false, inner_w, &self.styles)
                    } else {
                        render_tree_child_line(item, false, inner_w, &self.styles)
                    };
                    ListItem::new(line).marker("")
                })
                .collect();

            let title = format!(" {} ({}) ", range.band.label(), range.count);
            let border_color = if is_focused {
                band_accent(range.band, &self.styles)
            } else {
                self.styles.dim
            };
            let block = Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_color)
                .title(title.as_str())
                .style(ftui::style::Style::new().bg(self.styles.clr_bg));

            let list = List::new(list_items)
                .block(block)
                .style(self.styles.text)
                .highlight_style(self.styles.selected);

            if let Some(state_cell) = self.survey_band_states.get(&range.band) {
                let mut state = state_cell.borrow_mut();
                StatefulWidget::render(&list, slot, frame, &mut state);
            }
        }
    }

    /// Render the survey bottom bar with field vitals.
    pub fn render_survey_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        let content =
            self.layout
                .content_area(Rect::new(area.x, area.y, area.width, area.height + 10));
        let bar_area = Rect::new(content.x, area.y, content.width, 1);

        let v = &self.field_vitals;

        // Only show aggregate stats that aren't already in band headers.
        // Band counts (overdue, imminent) are visible in the band titles.
        let mut vitals_parts: Vec<String> = Vec::new();
        vitals_parts.push(format!("{} active", v.active));
        if v.held_unframed > 0 {
            vitals_parts.push(format!("{} held", v.held_unframed));
        }
        if v.stale_realities > 0 {
            vitals_parts.push(format!("{} stale", v.stale_realities));
        }
        if v.signaled > 0 {
            vitals_parts.push(format!("{} signaled", v.signaled));
        }
        let left = vitals_parts.join(" \u{00B7} ");

        let center = "j/k navigate \u{00B7} J/K band \u{00B7} Tab pivot \u{00B7} Enter descend";
        let right_text = "? help";

        let w = bar_area.width as usize;
        let left_w = left.chars().count();
        let center_w = center.chars().count();
        let right_w = right_text.chars().count();
        let center_start = w.saturating_sub(center_w) / 2;

        let mut spans: Vec<Span> = Vec::new();
        let has_attention = v.stale_realities > 0 || v.signaled > 0;
        spans.push(Span::styled(
            &left,
            if has_attention {
                self.styles.amber
            } else {
                self.styles.dim
            },
        ));

        if center_start > left_w + 1 {
            let pad = " ".repeat(center_start - left_w);
            spans.push(Span::styled(pad, self.styles.dim));
            spans.push(Span::styled(center, self.styles.dim));
        }

        let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let right_start = w.saturating_sub(right_w);
        if used < right_start {
            spans.push(Span::styled(
                " ".repeat(right_start - used),
                self.styles.dim,
            ));
            spans.push(Span::styled(right_text, self.styles.dim));
        } else if used < w {
            spans.push(Span::styled(" ".repeat(w - used), self.styles.dim));
        }

        Paragraph::new(Text::from_lines(vec![Line::from_spans(spans)])).render(bar_area, frame);
    }
}

/// Map band to accent style for the active band border.
fn band_accent(band: TimeBand, styles: &InstrumentStyles) -> ftui::style::Style {
    match band {
        TimeBand::Overdue => styles.amber, // overdue uses amber (red reserved for errors)
        TimeBand::ThisWeek => styles.amber,
        TimeBand::ThisMonth => styles.cyan,
        TimeBand::Later => styles.cyan,
        TimeBand::NoDeadline => styles.subdued,
    }
}

// ---------------------------------------------------------------------------
// Line builders
// ---------------------------------------------------------------------------

/// Render a provider/standalone item line (has own deadline or no deadline).
///
/// Layout: `  Jun    ◆ desire text...                   #2`
fn render_provider_line(
    item: &SurveyItem,
    is_selected: bool,
    w: usize,
    styles: &InstrumentStyles,
) -> Line<'static> {
    let horizon_str = match &item.own_horizon_label {
        Some(label) => {
            let padded = format!("{:<width$}", label, width = HORIZON_COL_W);
            padded.chars().take(HORIZON_COL_W).collect::<String>()
        }
        None => " ".repeat(HORIZON_COL_W),
    };

    let glyph = position_glyph(item);
    let glyph_str = format!("{glyph} ");
    let glyph_w = 2;

    let right_text = build_right_col(item);
    let right_w = right_text.chars().count();
    let signal_str = signal_display_str(item);
    let signal_w = signal_str.chars().count();
    let left_used = SURVEY_INDENT.len() + HORIZON_COL_W + glyph_w;
    let desire_w = w.saturating_sub(left_used + right_w + signal_w + 2);
    let desire_text = truncate_desired(&item.desired, desire_w);

    let text_w = desire_text.chars().count();
    let gap = w.saturating_sub(left_used + text_w + signal_w + right_w);

    let style_text = if is_selected {
        styles.selected
    } else {
        styles.text
    };
    let style_dim = if is_selected {
        styles.selected
    } else {
        styles.dim
    };
    let style_glyph = if is_selected {
        styles.selected
    } else {
        glyph_style(item, styles)
    };
    let style_signal = if is_selected {
        styles.selected
    } else {
        styles.amber
    };
    let style_horizon = if item.urgency > 1.0 {
        if is_selected {
            styles.selected
        } else {
            styles.amber
        }
    } else {
        style_dim
    };

    let mut spans = vec![
        Span::styled(SURVEY_INDENT.to_string(), style_dim),
        Span::styled(horizon_str, style_horizon),
        Span::styled(glyph_str, style_glyph),
        Span::styled(desire_text, style_text),
        Span::styled(" ".repeat(gap), style_dim),
        Span::styled(signal_str, style_signal),
        Span::styled(right_text, style_dim),
    ];

    pad_to_width(&mut spans, w, style_dim);
    Line::from_spans(spans)
}

/// Render a tree-child item line (inherits deadline from a provider ancestor).
fn render_tree_child_line(
    item: &SurveyItem,
    is_selected: bool,
    w: usize,
    styles: &InstrumentStyles,
) -> Line<'static> {
    let base_indent = SURVEY_INDENT.len() + HORIZON_COL_W;
    let prefix_w = item.tree_prefix.chars().count();
    let glyph = position_glyph(item);
    let glyph_str = format!("{glyph} ");
    let glyph_w = 2;

    let right_text = build_right_col(item);
    let right_w = right_text.chars().count();
    let signal_str = signal_display_str(item);
    let signal_w = signal_str.chars().count();
    let left_used = base_indent + prefix_w + glyph_w;
    let desire_w = w.saturating_sub(left_used + right_w + signal_w + 2);
    let desire_text = truncate_desired(&item.desired, desire_w);

    let text_w = desire_text.chars().count();
    let gap = w.saturating_sub(left_used + text_w + signal_w + right_w);

    let style_text = if is_selected {
        styles.selected
    } else {
        styles.text
    };
    let style_dim = if is_selected {
        styles.selected
    } else {
        styles.dim
    };
    let style_glyph = if is_selected {
        styles.selected
    } else {
        glyph_style(item, styles)
    };
    let style_signal = if is_selected {
        styles.selected
    } else {
        styles.amber
    };

    let mut spans = vec![
        Span::styled(" ".repeat(base_indent), style_dim),
        Span::styled(item.tree_prefix.clone(), style_dim),
        Span::styled(glyph_str, style_glyph),
        Span::styled(desire_text, style_text),
        Span::styled(" ".repeat(gap), style_dim),
        Span::styled(signal_str, style_signal),
        Span::styled(right_text, style_dim),
    ];

    pad_to_width(&mut spans, w, style_dim);
    Line::from_spans(spans)
}

fn position_glyph(item: &SurveyItem) -> &'static str {
    if item.is_held { "\u{2727}" } else { "\u{25c6}" }
}

fn glyph_style(item: &SurveyItem, styles: &InstrumentStyles) -> ftui::style::Style {
    if item.is_held {
        styles.subdued
    } else if item.is_next {
        styles.green
    } else {
        styles.cyan
    }
}

fn signal_display_str(item: &SurveyItem) -> String {
    if item.signal_glyphs.is_empty() {
        String::new()
    } else {
        format!("{} ", item.signal_glyphs.join(""))
    }
}

fn build_right_col(item: &SurveyItem) -> String {
    item.short_code
        .map(|c| format!("{c:02}"))
        .unwrap_or_default()
}

fn pad_to_width(spans: &mut Vec<Span>, w: usize, style: ftui::style::Style) {
    let total: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if total < w {
        spans.push(Span::styled(" ".repeat(w - total), style));
    }
}
