//! Deck view — the new TUI rendering for the operative instrument.
//!
//! V1: Skeleton with column layout.
//! V2: Frontier computation + console. Children classified into zones,
//!     rendered in the middle area. Pitch navigation through selectable items.

use ftui::Frame;
use ftui::PackedRgba;
use ftui::layout::{Constraint, Flex, Rect};
use ftui::style::Style;
use ftui::text::{Line, Span, Text};
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use sd_core::TensionStatus;

/// An item in the accumulated zone — either a resolved/released child or a parent note.
#[derive(Debug, Clone)]
pub enum AccumulatedItem {
    /// A resolved or released child tension (index into siblings vec).
    Child(usize),
    /// A note on the parent tension.
    Note {
        text: String,
        age: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

impl AccumulatedItem {
    /// Get the sibling index if this is a Child variant.
    pub fn child_index(&self) -> Option<usize> {
        match self {
            AccumulatedItem::Child(i) => Some(*i),
            AccumulatedItem::Note { .. } => None,
        }
    }
}

use crate::app::InstrumentApp;
use crate::glyphs;
use crate::state::FieldEntry;
use crate::theme::*;

// ---------------------------------------------------------------------------
// Column layout
// ---------------------------------------------------------------------------

/// Computed column widths for the deck layout.
#[derive(Debug, Clone)]
pub struct ColumnLayout {
    /// Width of the left column (deadline display).
    pub left: usize,
    /// Start position of the main (text) column, relative to content area.
    pub main_start: usize,
    /// Width of the main text column.
    pub main: usize,
    /// Total width of the right section.
    pub right: usize,
    /// Right sub-column: ID width (zero-padded, adapts to max ID).
    pub id_width: usize,
    /// Right sub-column: age width.
    pub age_width: usize,
}

/// Gutter width between columns.
const GUTTER: usize = 2;
/// Minimum left column width (enough for "Mar 30").
const MIN_LEFT: usize = 6;
/// Maximum content width (matches existing render.rs).
const MAX_CONTENT_WIDTH: u16 = 104;
/// Left/right margin from screen edges.

// ---------------------------------------------------------------------------
// Deck configuration (V6)
// ---------------------------------------------------------------------------

/// Chrome mode controls when visual separators appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeMode {
    /// Minimal — no separators, whitespace only.
    Quiet,
    /// Default — separators appear when there are 2+ content zones.
    Adaptive,
    /// Always show separators and structural markers.
    Structured,
}

impl ChromeMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "quiet" => Self::Quiet,
            "structured" => Self::Structured,
            _ => Self::Adaptive,
        }
    }
}

/// Configuration for deck rendering, read from `deck.*` config keys.
#[derive(Debug, Clone)]
pub struct DeckConfig {
    pub chrome: ChromeMode,
}

impl Default for DeckConfig {
    fn default() -> Self {
        Self {
            chrome: ChromeMode::Adaptive,
        }
    }
}

impl DeckConfig {
    /// Load deck config from the werk config system.
    pub fn load(config: &werk_shared::Config) -> Self {
        let chrome = config.get("deck.chrome")
            .map(|v| ChromeMode::from_str(v))
            .unwrap_or(ChromeMode::Adaptive);
        Self { chrome }
    }
}
const EDGE_MARGIN: u16 = 2;
/// Extra indent for held (unpositioned) items (Q22).
const HELD_INDENT: usize = 2;

impl ColumnLayout {
    /// Compute column layout from the current data in view.
    /// `max_id` is the highest short_code visible (determines ID column width).
    /// `max_age_len` is the longest age string length visible.
    pub fn compute(total_width: usize, deadline_label: Option<&str>, max_id: usize, max_age_len: usize) -> Self {
        // Left column = max of all deadlines in view (min 6)
        let left = deadline_label
            .map(|d| d.chars().count().max(MIN_LEFT))
            .unwrap_or(MIN_LEFT);

        // Right sub-columns: [id][space][→][space][age]
        // ID: zero-padded to consistent width based on max visible ID
        let id_width = if max_id >= 100 { 3 } else { 2 };
        // Arrow: always 1 char (→ or space)
        // Age: at least 2 chars, adapts to max
        let age_width = max_age_len.max(2);
        // Right total: id + space + arrow + space + age
        let right = id_width + 1 + 1 + 1 + age_width;

        let main_start = left + GUTTER;
        let main = total_width.saturating_sub(main_start + GUTTER + right);

        Self {
            left,
            main_start,
            main,
            right,
            id_width,
            age_width,
        }
    }
}

// ---------------------------------------------------------------------------
// Frontier — classifies children into deck zones
// ---------------------------------------------------------------------------

/// Indices into the siblings vec, classified by frontier zone.
#[derive(Debug, Clone, Default)]
pub struct Frontier {
    /// Positioned steps that are not overdue and not the next step (future theory).
    /// Ordered highest-position-first (furthest from frontier at top).
    pub route: Vec<usize>,
    /// Positioned steps with a past deadline.
    pub overdue: Vec<usize>,
    /// The next committed step (lowest-position non-overdue positioned active).
    pub next: Option<usize>,
    /// Unpositioned active steps (held in reserve).
    pub held: Vec<usize>,
    /// Temporal events: resolved/released steps and parent notes (accumulated since last epoch).
    pub accumulated: Vec<AccumulatedItem>,
    /// How many route items to show individually (rest compressed to summary).
    pub show_route: usize,
    /// How many held items to show individually (0 = compressed indicator only).
    pub show_held: usize,
    /// How many accumulated items to show individually (0 = compressed indicator only).
    pub show_accumulated: usize,
    /// Whether the desire anchor is selectable (true when descended into a parent).
    pub has_desire_anchor: bool,
    /// Whether the reality anchor is selectable (true when descended and reality non-empty).
    pub has_reality_anchor: bool,
}

impl Frontier {
    /// Classify siblings into frontier zones.
    ///
    /// Siblings are sorted: positioned DESC first (highest position = furthest future),
    /// then unpositioned. Position 1 is the FIRST/nearest step.
    ///
    /// When `trajectory` is true, positioned resolved/released items stay on the route
    /// (shown in-place with their glyphs) instead of moving to accumulated. This gives
    /// a progress view showing the full route including accomplished steps.
    ///
    /// When `epoch_boundary` is Some, only resolved/released items whose last status
    /// change is after the boundary are shown in accumulated. Items from prior epochs
    /// are hidden (they belong to the log, not the current epoch).
    pub fn compute(siblings: &[FieldEntry], trajectory: bool, epoch_boundary: Option<chrono::DateTime<chrono::Utc>>) -> Self {
        let mut frontier = Frontier::default();

        // Separate into groups
        let mut positioned_active: Vec<usize> = Vec::new();

        for (i, entry) in siblings.iter().enumerate() {
            match entry.status {
                TensionStatus::Active => {
                    if entry.position.is_some() {
                        positioned_active.push(i);
                    } else {
                        frontier.held.push(i);
                    }
                }
                TensionStatus::Resolved | TensionStatus::Released => {
                    if trajectory && entry.position.is_some() {
                        // Trajectory mode: positioned resolved/released stay on the route
                        positioned_active.push(i);
                    } else {
                        // V5: only include in accumulated if resolved/released after epoch boundary
                        let in_current_epoch = match epoch_boundary {
                            Some(boundary) => entry.last_status_change >= boundary,
                            None => true, // no epoch boundary = show all (stub behavior)
                        };
                        if in_current_epoch {
                            frontier.accumulated.push(AccumulatedItem::Child(i));
                        }
                    }
                }
            }
        }

        // Among positioned active: find overdue, next, and route.
        // Overdue = has deadline AND urgency > 1.0 (past deadline).
        // Next = lowest position number among non-overdue positioned active.
        // Route = everything else (positioned, not overdue, not next).

        // First, identify overdue items
        let mut overdue_set = std::collections::HashSet::new();
        for &idx in &positioned_active {
            let entry = &siblings[idx];
            if entry.temporal_urgency > 1.0 {
                overdue_set.insert(idx);
                frontier.overdue.push(idx);
            }
        }

        // Find the "next" step: the one with the lowest position value among non-overdue.
        // Since siblings are sorted position DESC, the last positioned active non-overdue
        // in the list has the lowest position number (closest to frontier).
        let non_overdue_positioned: Vec<usize> = positioned_active.iter()
            .copied()
            .filter(|idx| !overdue_set.contains(idx))
            .collect();

        if let Some(&next_idx) = non_overdue_positioned.last() {
            frontier.next = Some(next_idx);
        }

        // Route = non-overdue positioned that are NOT the next step.
        // They appear in the same order as siblings (position DESC = furthest first).
        for &idx in &non_overdue_positioned {
            if Some(idx) != frontier.next {
                frontier.route.push(idx);
            }
        }

        // Sort overdue by position (lowest first = nearest to frontier first)
        frontier.overdue.sort_by(|&a, &b| {
            let pa = siblings[a].position.unwrap_or(0);
            let pb = siblings[b].position.unwrap_or(0);
            pa.cmp(&pb)
        });

        // Reverse accumulated so most recent (last in siblings) appears first
        // in the vec → renders at top of accumulated zone (nearest breathing space)
        frontier.accumulated.reverse();

        frontier
    }

    /// Build a frontier for reorder mode: all active items go into `route`
    /// in their current array order (ignoring position fields). Resolved/released
    /// go to `accumulated` filtered by epoch boundary. No overdue classification.
    pub fn from_raw_order(siblings: &[FieldEntry], epoch_boundary: Option<chrono::DateTime<chrono::Utc>>) -> Self {
        let mut frontier = Frontier::default();

        for (i, entry) in siblings.iter().enumerate() {
            match entry.status {
                TensionStatus::Active => {
                    if entry.position.is_some() {
                        frontier.route.push(i);
                    } else {
                        frontier.held.push(i);
                    }
                }
                TensionStatus::Resolved | TensionStatus::Released => {
                    let in_current_epoch = match epoch_boundary {
                        Some(boundary) => entry.last_status_change >= boundary,
                        None => true,
                    };
                    if in_current_epoch {
                        frontier.accumulated.push(AccumulatedItem::Child(i));
                    }
                }
            }
        }

        // Last active item in array order is closest to the frontier
        if let Some(&last) = frontier.route.last() {
            frontier.next = Some(last);
            frontier.route.pop();
        }

        frontier.accumulated.reverse();
        frontier
    }

    /// Inject parent notes into the accumulated zone, then sort all items
    /// by timestamp descending (most recent first).
    /// `siblings` is needed to look up `last_status_change` for child items.
    pub fn inject_notes(&mut self, notes: Vec<AccumulatedItem>, siblings: &[FieldEntry]) {
        self.accumulated.extend(notes);
        // Sort: most recent first (descending timestamp)
        self.accumulated.sort_by(|a, b| {
            let ts_a = match a {
                AccumulatedItem::Child(idx) => siblings[*idx].last_status_change,
                AccumulatedItem::Note { timestamp, .. } => *timestamp,
            };
            let ts_b = match b {
                AccumulatedItem::Child(idx) => siblings[*idx].last_status_change,
                AccumulatedItem::Note { timestamp, .. } => *timestamp,
            };
            ts_b.cmp(&ts_a) // descending
        });
    }

    /// Determine how many route/held/accumulated items to show individually.
    ///
    /// Two-pass algorithm:
    /// 1. Reserve 1 summary line per non-empty compressible category (guarantees
    ///    every category stays visible even under extreme compression).
    /// 2. Distribute remaining space by priority: route first, held second,
    ///    accumulated last. Each category upgrades from summary → partial → full.
    pub fn compute_expansion(&mut self, available_lines: usize) {
        // Fixed items: overdue (always shown fully), next, input point, separator
        let has_ordered = !self.route.is_empty() || !self.overdue.is_empty() || self.next.is_some();
        let has_unordered = !self.held.is_empty() || !self.accumulated.is_empty();
        let fixed = self.overdue.len()
            + if self.next.is_some() { 1 } else { 0 }
            + 1  // input point
            + if has_ordered && has_unordered { 1 } else { 0 }; // separator

        let mut free = available_lines.saturating_sub(fixed);

        // Pass 1: reserve 1 summary line per non-empty category
        let has_route = !self.route.is_empty();
        let has_held = !self.held.is_empty();
        let has_accumulated = !self.accumulated.is_empty();
        let reserved = (if has_route { 1 } else { 0 })
            + (if has_held { 1 } else { 0 })
            + (if has_accumulated { 1 } else { 0 });

        if free < reserved {
            // Not enough space even for summaries — show what fits in priority order
            // (accumulated summary first since it compresses first, but route summary
            // is most valuable). Give each what we can.
            self.show_route = 0;
            self.show_held = 0;
            self.show_accumulated = 0;
            // Even here, the summaries will render if my < middle_end in the render loop
            return;
        }

        // Deduct reserved lines — these are guaranteed summary slots
        free -= reserved;

        // Pass 2: distribute remaining space by priority (route > held > accumulated)
        // Each category can upgrade: summary (0 shown) → partial → full.
        // Upgrading from summary to N items costs N lines (the summary line is already reserved,
        // but showing items replaces it — so showing all N items costs N-1 extra if N == count,
        // or N extra if we still need a summary for the remainder).

        // Route
        if has_route {
            let count = self.route.len();
            if free >= count {
                // Show all — reclaim the reserved summary line
                self.show_route = count;
                free -= count - 1; // -1 because summary line is freed
            } else if free >= 1 {
                // Show some + summary (summary already reserved)
                self.show_route = free;
                free = 0;
            } else {
                self.show_route = 0;
            }
        }

        // Held
        if has_held {
            let count = self.held.len();
            if free >= count {
                self.show_held = count;
                free -= count - 1;
            } else if free >= 1 {
                self.show_held = free;
                free = 0;
            } else {
                self.show_held = 0;
            }
        }

        // Accumulated
        if has_accumulated {
            let count = self.accumulated.len();
            if free >= count {
                self.show_accumulated = count;
                // free -= count - 1; // not needed, last category
            } else if free >= 1 {
                self.show_accumulated = free;
            } else {
                self.show_accumulated = 0;
            }
        }
    }

    /// Total number of selectable items in the deck.
    pub fn selectable_count(&self) -> usize {
        let mut count = 0;
        if self.has_desire_anchor { count += 1; }
        count += self.show_route;
        if self.show_route < self.route.len() { count += 1; } // route summary
        count += self.overdue.len();
        if self.next.is_some() { count += 1; }
        if !self.held.is_empty() {
            count += self.show_held; // individual items
            if self.show_held < self.held.len() { count += 1; } // summary for rest
        }
        count += 1; // input point (always present)
        if !self.accumulated.is_empty() {
            count += self.show_accumulated;
            if self.show_accumulated < self.accumulated.len() { count += 1; } // summary
        }
        if self.has_reality_anchor { count += 1; }
        count
    }

    /// Get the default cursor position — rests at the input point (NOW/frontier).
    pub fn default_cursor(&self) -> usize {
        let mut pos = 0;
        if self.has_desire_anchor { pos += 1; }
        pos += self.show_route;
        if self.show_route < self.route.len() { pos += 1; } // route summary
        pos += self.overdue.len();
        if self.next.is_some() { pos += 1; }
        if !self.held.is_empty() {
            pos += self.show_held;
            if self.show_held < self.held.len() { pos += 1; } // summary line
        }
        pos // input point is always right after held
    }

    /// Find the cursor index that points to a given sibling index.
    /// Returns None if the sibling is not visible (compressed into a summary).
    pub fn cursor_for_sibling(&self, sibling_idx: usize) -> Option<usize> {
        let count = self.selectable_count();
        for i in 0..count {
            if let Some(idx) = self.cursor_target(i).sibling_index() {
                if idx == sibling_idx {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Map a cursor index to what it points at.
    pub fn cursor_target(&self, cursor: usize) -> CursorTarget {
        let mut offset = 0;

        // Desire anchor
        if self.has_desire_anchor {
            if cursor == 0 {
                return CursorTarget::Desire;
            }
            offset += 1;
        }

        // Route — show_route individual items, then possibly summary
        let shown_route = self.show_route.min(self.route.len());
        if cursor < offset + shown_route {
            return CursorTarget::Route(self.route[cursor - offset]);
        }
        offset += shown_route;
        if shown_route < self.route.len() {
            if cursor == offset {
                return CursorTarget::RouteSummary;
            }
            offset += 1;
        }

        // Overdue
        if cursor < offset + self.overdue.len() {
            return CursorTarget::Overdue(self.overdue[cursor - offset]);
        }
        offset += self.overdue.len();

        // Next
        if let Some(next_idx) = self.next {
            if cursor == offset {
                return CursorTarget::Next(next_idx);
            }
            offset += 1;
        }

        // Held — show_held individual items, then possibly a summary
        if !self.held.is_empty() {
            // Individual held items
            let shown = self.show_held.min(self.held.len());
            if cursor < offset + shown {
                return CursorTarget::HeldItem(self.held[cursor - offset]);
            }
            offset += shown;
            // Summary line for remaining
            if shown < self.held.len() {
                if cursor == offset {
                    return CursorTarget::Held;
                }
                offset += 1;
            }
        }

        // Input point
        if cursor == offset {
            return CursorTarget::InputPoint;
        }
        offset += 1;

        // Accumulated — show_accumulated individual items, then possibly a summary
        if !self.accumulated.is_empty() {
            let shown = self.show_accumulated.min(self.accumulated.len());
            if cursor < offset + shown {
                let acc_idx = cursor - offset;
                return match &self.accumulated[acc_idx] {
                    AccumulatedItem::Child(sibling_idx) => CursorTarget::AccumulatedItem(*sibling_idx),
                    AccumulatedItem::Note { .. } => CursorTarget::NoteItem(acc_idx),
                };
            }
            offset += shown;
            if shown < self.accumulated.len() {
                if cursor == offset {
                    return CursorTarget::Accumulated;
                }
            }
        }

        // Reality anchor
        if self.has_reality_anchor {
            return CursorTarget::Reality;
        }

        CursorTarget::InputPoint // fallback
    }
}

/// What the deck cursor is pointing at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorTarget {
    /// The desire anchor (parent's desired outcome).
    Desire,
    /// A route step (index into siblings).
    Route(usize),
    /// Route summary line (compressed remaining route steps).
    RouteSummary,
    /// An overdue step (index into siblings).
    Overdue(usize),
    /// The next committed step (index into siblings).
    Next(usize),
    /// The held indicator (compressed).
    Held,
    /// An individual held item (expanded).
    HeldItem(usize),
    /// The input point.
    InputPoint,
    /// The accumulated indicator (compressed).
    Accumulated,
    /// An individual accumulated child item (expanded, sibling index).
    AccumulatedItem(usize),
    /// An individual accumulated note (expanded, index into accumulated vec).
    NoteItem(usize),
    /// The reality anchor (parent's current ground truth).
    Reality,
}

impl CursorTarget {
    /// Get the sibling index this target refers to, if it points to a specific child.
    pub fn sibling_index(&self) -> Option<usize> {
        match self {
            CursorTarget::Route(i) | CursorTarget::Overdue(i) | CursorTarget::Next(i)
            | CursorTarget::HeldItem(i) | CursorTarget::AccumulatedItem(i) => Some(*i),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Deck cursor — V2: index into flat selectable list
// ---------------------------------------------------------------------------

/// Which zone the cursor is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckZone {
    Console,
}

/// Zone-aware cursor for the deck.
#[derive(Debug, Clone)]
pub struct DeckCursor {
    pub zone: DeckZone,
    /// Index into the flat selectable items list.
    pub index: usize,
}

impl Default for DeckCursor {
    fn default() -> Self {
        Self {
            zone: DeckZone::Console,
            index: 0,
        }
    }
}

impl DeckCursor {
    /// Move cursor up (toward desire / route).
    pub fn pitch_up(&mut self, selectable_count: usize) {
        if selectable_count == 0 { return; }
        if self.index > 0 {
            self.index -= 1;
        }
    }

    /// Move cursor down (toward reality / accumulated).
    pub fn pitch_down(&mut self, selectable_count: usize) {
        if selectable_count == 0 { return; }
        if self.index < selectable_count - 1 {
            self.index += 1;
        }
    }

    /// Clamp cursor to valid range.
    pub fn clamp(&mut self, selectable_count: usize) {
        if selectable_count == 0 {
            self.index = 0;
        } else {
            self.index = self.index.min(selectable_count - 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Zoom level (V1: Normal only)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomLevel {
    Normal,
    Focus,
    Peek,
    #[allow(dead_code)]
    Orient,
}

impl ZoomLevel {
    /// True if inline detail is showing (either Focus or Peek).
    pub fn has_detail(&self) -> bool {
        matches!(self, ZoomLevel::Focus | ZoomLevel::Peek)
    }
}

/// Detail for a focused note in the accumulated zone.
#[derive(Debug, Clone)]
pub struct FocusedNote {
    /// Index into the accumulated vec.
    pub acc_index: usize,
    pub text: String,
    pub age: String,
}

/// Detail for a focused element (V7 → detail card).
#[derive(Debug, Clone)]
pub struct FocusedDetail {
    /// The sibling index of the focused element.
    pub sibling_index: usize,
    /// The focused child's desire text.
    pub desired: String,
    /// The focused child's reality text.
    pub actual: String,
    /// The focused child's short code.
    pub short_code: Option<i32>,
    /// The focused child's deadline label.
    pub deadline_label: Option<String>,
    // Temporal facts
    pub created_age: String,
    pub last_reality_age: String,
    pub last_desire_age: String,
    pub temporal_urgency: f64,
    // Structure
    pub child_count: usize,
    pub child_active: usize,
    pub child_resolved: usize,
    pub child_released: usize,
    pub child_held: usize,
    // Notes (most recent first, capped)
    pub recent_notes: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// Rendering — render_deck
// ---------------------------------------------------------------------------

impl InstrumentApp {
    /// Constrain area to max content width for the deck, centered horizontally,
    /// with edge margins so text doesn't press against terminal edges.
    fn deck_content_area(&self, area: Rect) -> Rect {
        // Apply edge margins first
        let usable_width = area.width.saturating_sub(EDGE_MARGIN * 2);
        let width = usable_width.min(MAX_CONTENT_WIDTH);
        let x_offset = if usable_width > MAX_CONTENT_WIDTH {
            EDGE_MARGIN + (usable_width - MAX_CONTENT_WIDTH) / 2
        } else {
            EDGE_MARGIN
        };
        let top_pad = if area.height > 30 { 1 } else { 0 };
        Rect::new(
            area.x + x_offset,
            area.y + top_pad,
            width,
            area.height.saturating_sub(top_pad),
        )
    }

    /// Main deck render entry point.
    ///
    /// Uses Flex layout to split into three vertical zones:
    /// - Top: breadcrumb + desire + rule (Fixed height)
    /// - Middle: route + console + accumulated (Fill)
    /// - Bottom: reality + rule (Fixed height)
    pub fn render_deck(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.deck_content_area(*area);
        let w = area.width as usize;

        if w < 20 || area.height < 8 {
            return;
        }

        let parent = self.parent_tension.as_ref();

        // --- Column layout for child lines ---
        let deadline_label = self.parent_horizon_label.as_deref();
        let max_child_deadline = self.siblings.iter()
            .filter_map(|s| s.horizon_label.as_deref())
            .max_by_key(|s| s.len());
        let widest_deadline = match (deadline_label, max_child_deadline) {
            (Some(a), Some(b)) => Some(if a.len() >= b.len() { a } else { b }),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let max_id = self.siblings.iter()
            .filter_map(|s| s.short_code)
            .max()
            .unwrap_or(0) as usize;
        let max_age_len = self.siblings.iter()
            .map(|s| s.created_age.chars().count())
            .max()
            .unwrap_or(2);
        let cols = ColumnLayout::compute(w, widest_deadline, max_id, max_age_len);

        // --- Frontier classification (use cached, or compute fresh) ---
        let mut frontier = self.cached_frontier.clone()
            .unwrap_or_else(|| {
                if matches!(self.input_mode, crate::state::InputMode::Reordering { .. }) {
                    Frontier::from_raw_order(&self.siblings, self.epoch_boundary)
                } else {
                    Frontier::compute(&self.siblings, self.trajectory_mode, self.epoch_boundary)
                }
            });

        // --- Measure zones for Flex layout ---
        let has_breadcrumb = self.grandparent_display.is_some();
        let has_deadline = deadline_label.is_some() && !deadline_label.unwrap_or("").is_empty();

        // At root level (no parent), skip desire/reality zones entirely
        let (top_height, bottom_height, desire_lines): (u16, u16, Vec<String>) = if let Some(p) = parent {
            let desire_indent = if has_deadline { cols.left + GUTTER } else { 0 };
            let right_col_reserve = GUTTER + cols.right;
            let desire_wrap_width = w.saturating_sub(desire_indent + right_col_reserve);
            let d_lines = word_wrap(&p.desired, desire_wrap_width);

            let th: u16 = {
                let mut h: u16 = 0;
                if has_breadcrumb { h += 1; }
                h += 1; // blank line before desire
                h += d_lines.len() as u16;
                h += 1; // desire rule
                h
            };

            // Reality height: adaptive. Compute what the middle zone needs,
            // then reality gets the remaining space (minimum 1 content line).
            let reality_full_lines = if p.actual.is_empty() {
                0u16
            } else {
                word_wrap(&p.actual, w.saturating_sub(2)).len() as u16
            };
            // Estimate middle zone need from frontier content
            let middle_need = (frontier.selectable_count() as u16) + 4; // selectables + chrome
            let space_for_reality = area.height
                .saturating_sub(th)
                .saturating_sub(middle_need)
                .saturating_sub(2); // rule + blank
            let reality_lines = if p.actual.is_empty() { 0 } else {
                reality_full_lines.min(space_for_reality.max(1))
            };
            let bh: u16 = {
                let mut h: u16 = 0;
                if reality_lines > 0 {
                    h += 1; // blank line before reality
                    h += reality_lines;
                }
                h += 1; // reality rule
                h
            };

            (th, bh, d_lines)
        } else {
            (0, 0, Vec::new())
        };

        // === Flex vertical split: top (Fixed) | middle (Fill) | bottom (Fixed) ===
        let zones = Flex::vertical()
            .constraints([
                Constraint::Fixed(top_height),
                Constraint::Fill,
                Constraint::Fixed(bottom_height),
            ])
            .split(area);
        let top_zone = zones[0];
        let middle_zone = zones[1];
        let bottom_zone = zones[2];

        // === Render top zone (desire anchor) — only when descended ===
        if let Some(p) = parent {
            let reality_age_str = self.parent_reality_age.as_deref().unwrap_or("");
            // Check anchor selection using the frontier's cursor_target
            let desire_selected = frontier.has_desire_anchor
                && frontier.cursor_target(self.deck_cursor.index) == CursorTarget::Desire;
            let reality_selected = frontier.has_reality_anchor
                && frontier.cursor_target(self.deck_cursor.index) == CursorTarget::Reality;
            self.render_desire_zone(frame, top_zone, w, &cols, p, &desire_lines,
                has_breadcrumb, has_deadline, deadline_label.unwrap_or(""), desire_selected);
            self.render_reality_zone(frame, bottom_zone, w, p, reality_age_str, reality_selected);
        }

        // === Render middle zone ===
        if middle_zone.height == 0 {
            return;
        }

        // Reorder mode: keep the full deck visible. The frontier uses stale
        // position fields during drag, so items may appear in zones that don't
        // reflect the pending reorder — but the full context (console, accumulated,
        // desire, reality) stays visible. Positions are finalized on commit.

        // V7: measure inline focus detail height (used for accumulated item spacing)
        let focus_detail_height: usize = if self.deck_zoom.has_detail() {
            if let Some(ref detail) = self.focused_detail {
                let text_w = w.saturating_sub(cols.left + GUTTER + HELD_INDENT * 2);
                let reality_lines = if detail.actual.is_empty() { 0 } else {
                    word_wrap(&detail.actual, text_w).len()
                };
                let meta_line = 1; // combined temporal + structure
                let note_lines = detail.recent_notes.len();
                reality_lines + meta_line + note_lines
            } else { 0 }
        } else { 0 };

        // === Two-pass layout: measure → reconcile → render ===
        let middle_start = middle_zone.y;
        let middle_end = middle_zone.y + middle_zone.height;
        let middle_lines = middle_zone.height as usize;

        // Pass 1: initial expansion (optimistic allocation)
        frontier.compute_expansion(middle_lines);
        self.last_render_lines.set(middle_lines);

        // Pass 2+3: reconcile — compress until total fits middle_lines.
        //
        // Guarantee: every non-empty zone keeps at least 1 line (summary).
        // Summaries never disappear. Compression priority (first to shrink):
        //   1. Accumulated individuals → summary
        //   2. Held individuals → summary
        //   3. Focus detail height (reduced toward 0)
        //   4. Route individuals → summary (not yet — route is high priority)
        let mut final_show_acc = frontier.show_accumulated.min(frontier.accumulated.len());
        let mut final_show_held = frontier.show_held.min(frontier.held.len());
        let mut effective_focus_height = focus_detail_height;

        let recount = |sa: usize, sh: usize, fdh: usize| -> usize {
            let mut top: usize = 0;
            // Route
            let sr = frontier.show_route.min(frontier.route.len());
            top += sr;
            if sr < frontier.route.len() { top += 1; }
            top += frontier.overdue.len();
            if frontier.next.is_some() { top += 1; }
            if frontier.route.is_empty() && frontier.overdue.is_empty()
                && frontier.next.is_none() && !frontier.held.is_empty() {
                top += 2; // "no committed next step" + breathing
            }
            let has_content = !frontier.route.is_empty() || !frontier.overdue.is_empty()
                || frontier.next.is_some() || !frontier.held.is_empty()
                || !frontier.accumulated.is_empty();
            if has_content { top += 2; } // console + breathing
            // Inline focus detail (V7) takes extra lines in the top-down pass
            top += fdh;
            if !frontier.held.is_empty() {
                top += sh;
                if sh < frontier.held.len() { top += 1; } // summary (always present)
            }
            top += 1; // input point
            // Accumulated — always at least a summary line if non-empty
            if !frontier.accumulated.is_empty() {
                top += sa;
                if sa < frontier.accumulated.len() { top += 1; } // summary
            }
            top
        };

        // Step 1: reduce accumulated individuals
        // But never hide the focused item if it's in the accumulated zone.
        let min_show_acc = if self.deck_zoom.has_detail() {
            if let Some(ref detail) = self.focused_detail {
                frontier.accumulated.iter().position(|item| item.child_index() == Some(detail.sibling_index))
                    .map(|pos| pos + 1)
                    .unwrap_or(0)
            } else { 0 }
        } else { 0 };
        while recount(final_show_acc, final_show_held, effective_focus_height) > middle_lines
            && final_show_acc > min_show_acc
        {
            final_show_acc -= 1;
        }
        // Step 2: reduce held individuals (summary always stays)
        // But never hide the focused item — it must remain individually visible.
        let min_show_held = if self.deck_zoom.has_detail() {
            if let Some(ref detail) = self.focused_detail {
                frontier.held.iter().position(|&si| si == detail.sibling_index)
                    .map(|pos| pos + 1)  // must show at least up to this index
                    .unwrap_or(0)
            } else { 0 }
        } else { 0 };
        while recount(final_show_acc, final_show_held, effective_focus_height) > middle_lines
            && final_show_held > min_show_held
        {
            final_show_held -= 1;
        }
        // Step 3: reduce focus detail height (sacrifice luxury before summaries)
        while recount(final_show_acc, final_show_held, effective_focus_height) > middle_lines
            && effective_focus_height > 0
        {
            effective_focus_height -= 1;
        }

        frontier.show_accumulated = final_show_acc;
        frontier.show_held = final_show_held;
        // Focus detail may have been truncated by reconciliation
        let focus_detail_height = effective_focus_height;

        // Cursor and focus setup
        let cursor_idx = if let crate::state::InputMode::Reordering { ref tension_id } = self.input_mode {
            frontier.cursor_for_sibling(
                self.siblings.iter().position(|s| s.id == *tension_id).unwrap_or(0)
            ).unwrap_or(0)
        } else {
            self.deck_cursor.index.min(
                frontier.selectable_count().saturating_sub(1)
            )
        };
        let focused_sibling = if self.deck_zoom.has_detail() {
            self.focused_detail.as_ref().map(|d| d.sibling_index)
        } else {
            None
        };
        let _focused_note_acc = if self.deck_zoom.has_detail() {
            self.focused_note.as_ref().map(|n| n.acc_index)
        } else {
            None
        };

        // === Bottom-up pass: accumulated items (gravity toward reality) ===
        let mut acc_top = middle_end;

        if !frontier.accumulated.is_empty() {
            let shown = frontier.show_accumulated;
            let remaining = frontier.accumulated.len() - shown;

            // Remaining: show individually if only 1, else summary
            if remaining == 1 && acc_top > middle_start {
                acc_top -= 1;
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Accumulated;
                match &frontier.accumulated[shown] {
                    AccumulatedItem::Child(sibling_idx) => {
                        let entry = &self.siblings[*sibling_idx];
                        let glyph = status_glyph(entry.status);
                        self.render_child_line(frame, area.x, acc_top, w, &cols, entry, glyph, is_selected, false, 0, None);
                    }
                    AccumulatedItem::Note { text, age, .. } => {
                        self.render_note_line(frame, area.x, acc_top, w, &cols, text, age, is_selected);
                    }
                }
            } else if remaining > 1 && acc_top > middle_start {
                acc_top -= 1;
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Accumulated;

                let remaining_resolved = frontier.accumulated[shown..].iter()
                    .filter(|item| matches!(item, AccumulatedItem::Child(idx) if self.siblings[*idx].status == TensionStatus::Resolved))
                    .count();
                let remaining_released = frontier.accumulated[shown..].iter()
                    .filter(|item| matches!(item, AccumulatedItem::Child(idx) if self.siblings[*idx].status == TensionStatus::Released))
                    .count();
                let remaining_notes = frontier.accumulated[shown..].iter()
                    .filter(|item| matches!(item, AccumulatedItem::Note { .. }))
                    .count();

                let mut parts = Vec::new();
                if remaining_resolved > 0 {
                    parts.push(format!("\u{2713} {} more resolved", remaining_resolved));
                }
                if remaining_released > 0 {
                    parts.push(format!("~ {} more released", remaining_released));
                }
                if remaining_notes > 0 {
                    parts.push(format!("\u{203b} {} more notes", remaining_notes));
                }
                if parts.is_empty() {
                    parts.push(format!("{} more", remaining));
                }
                let acc_text = parts.join(" \u{00B7} ");
                self.render_indicator_line(frame, area.x, acc_top, w, &cols, &acc_text, is_selected, STYLES.dim, 0);
            }

            // Individual accumulated items (shown items above the summary, in order)
            // V7: if an accumulated item is focused, reserve space for inline detail
            let _acc_focus_reserve: u16 = if let Some(fi) = focused_sibling {
                if frontier.accumulated[..shown].iter().any(|item| item.child_index() == Some(fi)) {
                    focus_detail_height as u16
                } else { 0 }
            } else { 0 };

            // Track which accumulated items need focus detail rendered after placement
            let mut acc_focus_y: Option<(u16, u16)> = None; // (start_y, limit_y) for deferred focus render

            for i in (0..shown).rev() {
                if acc_top <= middle_start { break; }
                let item = frontier.accumulated[i].clone();

                // Reserve extra space for focus detail below child items
                let is_focused_child = match &item {
                    AccumulatedItem::Child(idx) => focused_sibling == Some(*idx),
                    AccumulatedItem::Note { .. } => false,
                };
                if is_focused_child {
                    let reserve = focus_detail_height as u16;
                    acc_top = acc_top.saturating_sub(reserve);
                    acc_focus_y = Some((acc_top, acc_top + reserve));
                }
                if acc_top <= middle_start { break; }
                acc_top -= 1;

                match &item {
                    AccumulatedItem::Child(sibling_idx) => {
                        let entry = &self.siblings[*sibling_idx];
                        let glyph = match entry.status {
                            TensionStatus::Resolved => "\u{2713}",  // ✓
                            TensionStatus::Released => "~",
                            _ => "\u{25c6}",                        // ◆ fallback
                        };
                        let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::AccumulatedItem(*sibling_idx);
                        self.render_child_line(frame, area.x, acc_top, w, &cols, entry, glyph, is_selected, false, 0, None);

                        // Render focus detail below this accumulated item
                        if is_focused_child {
                            if let (Some((_fy_start, fy_limit)), Some(detail)) = (acc_focus_y, &self.focused_detail) {
                                self.render_inline_focus(frame, area.x, acc_top + 1, fy_limit, w, &cols, detail, 0);
                            }
                        }
                    }
                    AccumulatedItem::Note { text, age, .. } => {
                        let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::NoteItem(i);
                        self.render_note_line(frame, area.x, acc_top, w, &cols, text, age, is_selected);
                    }
                }
            }
        }

        // top_limit = where the bottom-up pass stopped
        let top_limit = acc_top;
        let mut my = middle_start;

        // === Top-down pass: route → overdue → next → separator → held → input ===

        // --- Q28: Check if route and held both fully compressed → unified summary ---
        let shown_route = frontier.show_route.min(frontier.route.len());
        let route_remaining = frontier.route.len() - shown_route;
        let shown_held = frontier.show_held.min(frontier.held.len());
        let held_remaining = frontier.held.len() - shown_held;
        let unified_summary = shown_route == 0 && route_remaining > 0
            && shown_held == 0 && held_remaining > 0;

        if unified_summary && my < top_limit {
            // Merge route + next + held into one summary line above NOW (Q28)
            // Next and overdue are absorbed into the route count
            let total_route = frontier.route.len()
                + frontier.overdue.len()
                + if frontier.next.is_some() { 1 } else { 0 };
            let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::RouteSummary
                || frontier.cursor_target(cursor_idx) == CursorTarget::Held
                || frontier.cursor_target(cursor_idx) == CursorTarget::Next(frontier.next.unwrap_or(0));
            let text = format!(
                "\u{25B8} {} route \u{00B7} {} held",
                total_route,
                frontier.held.len()
            );
            self.render_indicator_line(frame, area.x, my, w, &cols, &text, is_selected, STYLES.dim, 0);
            my += 1;
        } else {
            // --- Route zone (gradual: show N individually, summary for rest) ---
            for i in 0..shown_route {
                if my >= top_limit { break; }
                let sibling_idx = frontier.route[i];
                let entry = &self.siblings[sibling_idx];
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Route(sibling_idx);
                let glyph = status_glyph(entry.status);
                self.render_child_line(frame, area.x, my, w, &cols, entry, glyph, is_selected, false, 0, Some(STYLES.cyan));
                my += 1;
                // V7: inline focus expansion
                if focused_sibling == Some(sibling_idx) {
                    if let Some(ref detail) = self.focused_detail {
                        my += self.render_inline_focus(frame, area.x, my, top_limit, w, &cols, detail, 0);
                    }
                }
            }
            // Route: show remaining items individually if only 1, else summary
            if route_remaining == 1 && my < top_limit {
                // Just show the single remaining item — a summary for 1 is silly
                let sibling_idx = frontier.route[shown_route];
                let entry = &self.siblings[sibling_idx];
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::RouteSummary;
                let glyph = status_glyph(entry.status);
                self.render_child_line(frame, area.x, my, w, &cols, entry, glyph, is_selected, false, 0, Some(STYLES.cyan));
                my += 1;
                if focused_sibling == Some(sibling_idx) {
                    if let Some(ref detail) = self.focused_detail {
                        my += self.render_inline_focus(frame, area.x, my, top_limit, w, &cols, detail, 0);
                    }
                }
            } else if route_remaining > 1 && my < top_limit {
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::RouteSummary;
                let count = if shown_route == 0 { frontier.route.len() } else { route_remaining };
                let next_deadline = frontier.route[shown_route..].iter()
                    .filter_map(|&idx| self.siblings[idx].horizon_label.as_deref())
                    .next();
                // "N more" when some items visible above, "N route steps" when fully compressed
                let label = if shown_route > 0 { "more" } else { "route steps" };
                let text = match next_deadline {
                    Some(dl) => format!("\u{25B8} {} {} \u{00B7} next {}", count, label, dl),
                    None => format!("\u{25B8} {} {}", count, label),
                };
                self.render_indicator_line(frame, area.x, my, w, &cols, &text, is_selected, STYLES.dim, 0);
                my += 1;
            }

            // --- Overdue steps (part of the action sequence, above the separator) ---
            for &sibling_idx in &frontier.overdue {
                if my >= top_limit { break; }
                let entry = &self.siblings[sibling_idx];
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Overdue(sibling_idx);
                self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{25c6}", is_selected, true, 0, Some(STYLES.cyan));
                my += 1;
                if focused_sibling == Some(sibling_idx) {
                    if let Some(ref detail) = self.focused_detail {
                        my += self.render_inline_focus(frame, area.x, my, top_limit, w, &cols, detail, 0);
                    }
                }
            }

            // --- Next committed step (still part of the ordered sequence) ---
            if let Some(next_idx) = frontier.next {
                if my < top_limit {
                    let entry = &self.siblings[next_idx];
                    let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Next(next_idx);
                    self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{25c6}", is_selected, false, 0, Some(STYLES.green));
                    my += 1;
                    if focused_sibling == Some(next_idx) {
                        if let Some(ref detail) = self.focused_detail {
                            my += self.render_inline_focus(frame, area.x, my, top_limit, w, &cols, detail, 0);
                        }
                    }
                }
            }

            // S4: "no committed next step" appears in the route zone (above console)
            let s4_held_only = frontier.route.is_empty()
                && frontier.overdue.is_empty()
                && frontier.next.is_none()
                && !frontier.held.is_empty();
            if s4_held_only && my < top_limit {
                if my > middle_start { my += 1; } // breathing
                let msg = "no committed next step";
                let prefix_len = cols.left + GUTTER;
                let pad_right = w.saturating_sub(prefix_len + msg.len());
                render_line(frame, area.x, my, area.width, &[
                    (" ".repeat(prefix_len), Style::new()),
                    (msg.to_string(), STYLES.dim),
                    (" ".repeat(pad_right), Style::new()),
                ]);
                my += 1;
            }

            // --- Console header: enriched boundary with structural readouts (S3) ---
            let has_content = !frontier.route.is_empty() || !frontier.overdue.is_empty()
                || frontier.next.is_some() || !frontier.held.is_empty()
                || !frontier.accumulated.is_empty();
            if has_content && my < top_limit {
                // S5: Breathing line above console header
                if my > middle_start && middle_zone.height > 10 {
                    my += 1;
                }

                // Build readout as colored spans
                let total_children = self.siblings.len();
                let done_count = self.siblings.iter()
                    .filter(|s| s.status == TensionStatus::Resolved || s.status == TensionStatus::Released)
                    .count();
                let epoch_display = self.epoch_boundary.map(|boundary| {
                    let delta = chrono::Utc::now().signed_duration_since(boundary);
                    let hours = delta.num_hours();
                    let days = delta.num_days();
                    let text = if hours < 1 { "fresh".to_string() }
                        else if hours < 24 { format!("epoch {}h", hours) }
                        else { format!("epoch {}d", days) };
                    let style = if hours < 1 { STYLES.green }
                        else if days > 7 { STYLES.amber }
                        else { STYLES.dim };
                    (text, style)
                });
                // Show the next step's deadline, or if none, the nearest route deadline.
                // Route items are sorted by position DESC (last = nearest to frontier).
                let next_dl = frontier.next.iter()
                    .filter_map(|&idx| self.siblings.get(idx))
                    .filter_map(|s| s.horizon_label.as_deref())
                    .next()
                    .or_else(|| {
                        frontier.route.iter().rev()
                            .filter_map(|&idx| self.siblings.get(idx))
                            .filter_map(|s| s.horizon_label.as_deref())
                            .next()
                    });

                // Collect readout cells: (text, style)
                let mut cells: Vec<(String, Style)> = Vec::new();
                if total_children > 0 {
                    cells.push((format!("{}/{}", done_count, total_children), STYLES.text));
                }
                if let Some((ref text, style)) = epoch_display {
                    cells.push((text.clone(), style));
                }
                if let Some(dl) = next_dl {
                    cells.push((format!("next {}", dl), STYLES.dim));
                }
                if !frontier.overdue.is_empty() {
                    cells.push((
                        format!("\u{26A0} {} overdue", frontier.overdue.len()),
                        STYLES.amber,
                    ));
                }

                // Calculate total readout width for centering
                let separator = " \u{00B7} ";
                let sep_w = separator.chars().count();
                let readout_w: usize = cells.iter().map(|(t, _)| t.chars().count()).sum::<usize>()
                    + if cells.len() > 1 { sep_w * (cells.len() - 1) } else { 0 };

                // Render: rule background with colored readout overlay
                let rule_char = glyphs::LIGHT_RULE.to_string();
                let pad_total = w.saturating_sub(readout_w + 4);
                let left_rules = pad_total / 2;
                let right_rules = pad_total - left_rules;

                let mut header_spans: Vec<Span> = Vec::new();
                header_spans.push(Span::styled(rule_char.repeat(left_rules), STYLES.dim));
                header_spans.push(Span::styled(" ", STYLES.dim));
                for (i, (text, style)) in cells.iter().enumerate() {
                    if i > 0 {
                        header_spans.push(Span::styled(separator, STYLES.dim));
                    }
                    header_spans.push(Span::styled(text.clone(), *style));
                }
                header_spans.push(Span::styled(" ", STYLES.dim));
                header_spans.push(Span::styled(rule_char.repeat(right_rules), STYLES.dim));

                Paragraph::new(Text::from(Line::from_spans(header_spans)))
                    .render(Rect::new(area.x, my, area.width, 1), frame);
                my += 1;

                // (S4 "no committed next step" moved above console header)
            }

            // --- Held items (gradual: show N individually, summary for rest) ---
            if !frontier.held.is_empty() && my < top_limit {
                // Individual items
                for i in 0..shown_held {
                    if my >= top_limit { break; }
                    let sibling_idx = frontier.held[i];
                    let entry = &self.siblings[sibling_idx];
                    let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::HeldItem(sibling_idx);
                    self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{2727}", is_selected, false, HELD_INDENT, Some(STYLES.subdued));
                    my += 1;
                    if focused_sibling == Some(sibling_idx) {
                        if let Some(ref detail) = self.focused_detail {
                            my += self.render_inline_focus(frame, area.x, my, top_limit, w, &cols, detail, HELD_INDENT);
                        }
                    }
                }
                // Remaining: show individually if only 1, else summary
                if held_remaining == 1 && my < top_limit {
                    let sibling_idx = frontier.held[shown_held];
                    let entry = &self.siblings[sibling_idx];
                    let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Held;
                    self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{2727}", is_selected, false, HELD_INDENT, Some(STYLES.subdued));
                    my += 1;
                    if focused_sibling == Some(sibling_idx) {
                        if let Some(ref detail) = self.focused_detail {
                            my += self.render_inline_focus(frame, area.x, my, top_limit, w, &cols, detail, HELD_INDENT);
                        }
                    }
                } else if held_remaining > 1 && my < top_limit {
                    let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Held;
                    let text = if shown_held == 0 {
                        format!("\u{2727} {} held", frontier.held.len())
                    } else {
                        format!("\u{2727} {} more held", held_remaining)
                    };
                    self.render_indicator_line(frame, area.x, my, w, &cols, &text, is_selected, STYLES.dim, HELD_INDENT);
                    my += 1;
                }
            }
        }

        // --- S4: Empty console state ---
        let is_empty_console = frontier.route.is_empty()
            && frontier.overdue.is_empty()
            && frontier.next.is_none()
            && frontier.held.is_empty();

        if is_empty_console && my < top_limit {
            // Truly empty: no children or all accumulated
            if my < top_limit { my += 1; } // breathing
            let msg = if frontier.accumulated.is_empty() && self.siblings.is_empty() {
                "no steps yet"
            } else {
                "no active steps"
            };
            let prefix_len = cols.left + GUTTER;
            let pad_right = w.saturating_sub(prefix_len + msg.len());
            render_line(frame, area.x, my, area.width, &[
                (" ".repeat(prefix_len), Style::new()),
                (msg.to_string(), STYLES.dim),
                (" ".repeat(pad_right), Style::new()),
            ]);
            my += 1;
            if my < top_limit { my += 1; } // breathing
        }

        // --- Input line: the action surface at the console's heart ---
        if my < top_limit {
            let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::InputPoint;

            let content = if is_selected {
                // Active: show available gestures
                if is_empty_console {
                    "\u{25B8} a add first step \u{00B7} n note \u{00B7} ! desire \u{00B7} ? reality"
                } else {
                    "\u{25B8} a add \u{00B7} n note \u{00B7} ! desire \u{00B7} ? reality"
                }.to_string()
            } else {
                // Resting: minimal affordance
                "\u{25B8} ___".to_string()
            };

            let style = if is_selected {
                STYLES.selected
            } else {
                STYLES.dim
            };

            let prefix_len = cols.left + GUTTER;
            let pad_right = w.saturating_sub(prefix_len + content.chars().count());
            let line = Line::from_spans([
                Span::styled(" ".repeat(prefix_len), style),
                Span::styled(content, style),
                Span::styled(" ".repeat(pad_right), style),
            ]);
            Paragraph::new(Text::from(line))
                .render(Rect::new(area.x, my, area.width, 1), frame);
        }
    }

    /// Render a single child line in the 3-column layout.
    /// `extra_indent` adds chars before the glyph (used for held items, Q22).
    fn render_child_line(
        &self,
        frame: &mut Frame<'_>,
        x: u16,
        y: u16,
        w: usize,
        cols: &ColumnLayout,
        entry: &FieldEntry,
        glyph: &str,
        is_selected: bool,
        is_overdue: bool,
        extra_indent: usize,
        glyph_color: Option<Style>,
    ) {
        let is_done = entry.status == TensionStatus::Resolved
            || entry.status == TensionStatus::Released;

        let base_style = if is_selected {
            STYLES.selected
        } else if is_overdue {
            // S2: Time-amplified overdue intensity (N7/C5)
            if entry.temporal_urgency > 2.0 {
                Style::new().fg(PackedRgba::rgb(230, 190, 60)).bold()
            } else if entry.temporal_urgency > 1.3 {
                Style::new().fg(CLR_AMBER).bold()
            } else {
                STYLES.amber
            }
        } else if is_done {
            STYLES.dim
        } else {
            STYLES.text
        };

        let glyph_style = if is_selected || is_overdue {
            base_style
        } else {
            glyph_color.unwrap_or(base_style)
        };

        // Left column: deadline label
        let left_str = match entry.horizon_label.as_deref() {
            Some(dl) => dl.to_string(),
            None => String::new(),
        };
        let left_padded = format!("{:<width$}", left_str, width = cols.left);

        // Right sub-columns: [id] [→] [age]
        let id_num = entry.short_code
            .map(|sc| format!("{:0>width$}", sc, width = cols.id_width))
            .unwrap_or_else(|| entry.id[..cols.id_width.min(entry.id.len())].to_string());

        let arrow = if entry.child_count > 0 { "\u{2192}" } else { " " };

        let age_str = format!("{:>width$}", entry.created_age, width = cols.age_width);

        let right_str = format!("{} {} {}", id_num, arrow, age_str);

        // S7: OVERDUE tag for overdue items
        let overdue_tag = if is_overdue && !is_selected { "OVERDUE  " } else { "" };
        let overdue_tag_w = overdue_tag.chars().count();

        // Main column: glyph + text — maximum budget
        let glyph_w = 2; // glyph + space
        let right_w = right_str.chars().count();
        let text_budget = w
            .saturating_sub(cols.left + GUTTER + extra_indent + glyph_w + overdue_tag_w + GUTTER + right_w);
        let main_text = truncate_str(&entry.desired, text_budget);

        // Build the line
        let mut spans: Vec<Span> = Vec::new();

        let left_style = if is_selected { base_style } else if is_overdue { base_style } else { STYLES.dim };
        let right_style = if is_selected { base_style } else { STYLES.dim };

        spans.push(Span::styled(left_padded, left_style));
        spans.push(Span::styled(" ".repeat(GUTTER + extra_indent), base_style));
        spans.push(Span::styled(format!("{} ", glyph), glyph_style));
        spans.push(Span::styled(&main_text, base_style));

        // Pad between text and OVERDUE tag / right sub-columns
        let used: usize = cols.left + GUTTER + extra_indent + glyph_w + main_text.chars().count();
        let gap = w.saturating_sub(used + overdue_tag_w + right_w);
        spans.push(Span::styled(" ".repeat(gap), base_style));
        if !overdue_tag.is_empty() {
            spans.push(Span::styled(overdue_tag, base_style));
        }
        spans.push(Span::styled(right_str, right_style));

        // Pad to full width for selection highlight
        let total_rendered: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        if total_rendered < w {
            spans.push(Span::styled(" ".repeat(w - total_rendered), base_style));
        }

        let line = Line::from_spans(spans);
        Paragraph::new(Text::from(line))
            .render(Rect::new(x, y, w as u16, 1), frame);
    }

    /// Render a note line in the accumulated zone.
    fn render_note_line(
        &self,
        frame: &mut Frame<'_>,
        x: u16,
        y: u16,
        w: usize,
        cols: &ColumnLayout,
        text: &str,
        age: &str,
        is_selected: bool,
    ) {
        let base_style = if is_selected { STYLES.selected } else { STYLES.dim };
        let glyph = "\u{203b}"; // ※
        let glyph_w = 2; // glyph + space
        let age_w = age.chars().count();
        let text_budget = w.saturating_sub(cols.left + GUTTER + glyph_w + GUTTER + age_w);
        let main_text = if text.chars().count() > text_budget {
            let t: String = text.chars().take(text_budget.saturating_sub(1)).collect();
            format!("{}\u{2026}", t)
        } else {
            text.to_string()
        };

        let used = cols.left + GUTTER + glyph_w + main_text.chars().count();
        let gap = w.saturating_sub(used + age_w);

        let mut spans = vec![
            Span::styled(format!("{:<width$}", "", width = cols.left), base_style),
            Span::styled(" ".repeat(GUTTER), base_style),
            Span::styled(format!("{} ", glyph), base_style),
            Span::styled(&main_text, base_style),
            Span::styled(" ".repeat(gap), base_style),
            Span::styled(age.to_string(), base_style),
        ];

        let total_rendered: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        if total_rendered < w {
            spans.push(Span::styled(" ".repeat(w - total_rendered), base_style));
        }

        let line = Line::from_spans(spans);
        Paragraph::new(Text::from(line))
            .render(Rect::new(x, y, w as u16, 1), frame);
    }

    /// Render an indicator line (held, accumulated) in the deck.
    /// `extra_indent` adds chars before the text (used for held items, Q22).
    fn render_indicator_line(
        &self,
        frame: &mut Frame<'_>,
        x: u16,
        y: u16,
        w: usize,
        cols: &ColumnLayout,
        text: &str,
        is_selected: bool,
        _base_style: Style,
        extra_indent: usize,
    ) {
        let style = if is_selected {
            STYLES.selected
        } else {
            STYLES.dim
        };

        let prefix_len = cols.left + GUTTER + extra_indent;
        let pad_right = w.saturating_sub(prefix_len + text.chars().count());
        let line = Line::from_spans([
            Span::styled(" ".repeat(prefix_len), style),
            Span::styled(text.to_string(), style),
            Span::styled(" ".repeat(pad_right), style),
        ]);
        Paragraph::new(Text::from(line))
            .render(Rect::new(x, y, w as u16, 1), frame);
    }

    /// Render the desire zone (top anchor): breadcrumb + blank + desire text + rule.
    fn render_desire_zone(
        &self,
        frame: &mut Frame<'_>,
        zone: Rect,
        w: usize,
        cols: &ColumnLayout,
        parent: &sd_core::Tension,
        desire_lines: &[String],
        has_breadcrumb: bool,
        has_deadline: bool,
        deadline_str: &str,
        selected: bool,
    ) {
        let mut y = zone.y;

        // Breadcrumb
        if has_breadcrumb {
            if let Some((ref gp_id, ref gp_desired)) = self.grandparent_display {
                let breadcrumb = format!(
                    "\u{2190} {} {}",
                    gp_id,
                    truncate_str(gp_desired, cols.main.min(60))
                );
                render_line(frame, zone.x, y, zone.width, &[
                    (pad_left(cols), STYLES.dim),
                    (breadcrumb, STYLES.dim),
                ]);
                y += 1;
            }
        }

        // Blank line
        y += 1;

        // Desire text with right-column facts
        let desire_age = self.parent_desire_age.as_deref().unwrap_or("")
            .trim_end_matches(" ago").to_string();
        let desire_id = parent.short_code
            .map(|sc| format!("{:0>width$}", sc, width = cols.id_width))
            .unwrap_or_default();
        let desire_right = format!("{} {}", desire_id, desire_age);
        let desire_right_w = desire_right.chars().count();

        for (i, line_text) in desire_lines.iter().enumerate() {
            if y >= zone.y + zone.height { break; }
            let mut spans: Vec<(String, Style)> = Vec::new();

            if has_deadline {
                let left_content = if i == 0 {
                    format!("{:<width$}", deadline_str, width = cols.left)
                } else {
                    " ".repeat(cols.left)
                };
                spans.push((left_content, STYLES.dim));
                spans.push((" ".repeat(GUTTER), Style::new()));
            }

            let text_style = if selected { STYLES.selected } else { STYLES.text_bold };
            if i == 0 {
                let glyph_style = if selected { STYLES.selected } else { STYLES.cyan };
                spans.push(("\u{25c6} ".to_string(), glyph_style));
            } else {
                spans.push(("  ".to_string(), text_style)); // align continuation
            }
            spans.push((line_text.clone(), text_style));

            if i == 0 {
                let text_used = if has_deadline { cols.left + GUTTER } else { 0 } + 2 + line_text.chars().count();
                let gap = w.saturating_sub(text_used + desire_right_w);
                if gap >= GUTTER {
                    let right_style = if selected { STYLES.selected } else { STYLES.dim };
                    spans.push((" ".repeat(gap), right_style));
                    spans.push((desire_right.clone(), right_style));
                }
            }

            render_line(frame, zone.x, y, zone.width, &spans);
            y += 1;
        }

        // Desire rule
        if y < zone.y + zone.height {
            let rule = glyphs::HEAVY_RULE.to_string().repeat(w);
            render_line(frame, zone.x, y, zone.width, &[(rule, STYLES.dim)]);
        }
    }

    /// Render the reality zone (bottom anchor): blank + reality text (word-wrapped) + rule.
    fn render_reality_zone(
        &self,
        frame: &mut Frame<'_>,
        zone: Rect,
        w: usize,
        parent: &sd_core::Tension,
        reality_age: &str,
        selected: bool,
    ) {
        if zone.height == 0 { return; }

        // Use Flex to split: [blank line] [reality text] [rule]
        let has_reality = !parent.actual.is_empty();
        let zones = Flex::vertical()
            .constraints(if has_reality {
                vec![Constraint::Fixed(1), Constraint::Fill, Constraint::Fixed(1)]
            } else {
                vec![Constraint::Fill, Constraint::Fixed(1)]
            })
            .split(zone);

        // Reality rule at the bottom
        let rule_zone = match zones.last() {
            Some(z) => z,
            None => return, // no space to render
        };
        let rule = glyphs::RULE.to_string().repeat(w);
        render_line(frame, rule_zone.x, rule_zone.y, rule_zone.width, &[(rule, STYLES.dim)]);

        if !has_reality { return; }

        let text_zone = zones[1];
        if text_zone.height == 0 { return; }

        let age_suffix = if reality_age.is_empty() {
            String::new()
        } else {
            format!(" \u{00B7} {}", reality_age)
        };
        let age_w = age_suffix.chars().count();

        let glyph_prefix = "\u{25c7} "; // ◇ prefix
        let full_lines = word_wrap(&parent.actual, w.saturating_sub(2));
        let fits = full_lines.len() as u16 <= text_zone.height;

        let text_style = if selected { STYLES.selected } else { STYLES.dim };
        let glyph_style = if selected { STYLES.selected } else { STYLES.subdued };

        if fits {
            // Full text fits — render all lines, age on last line
            let mut lines: Vec<Line> = Vec::new();
            for (i, line_text) in full_lines.iter().enumerate() {
                let prefix = if i == 0 { glyph_prefix } else { "  " };
                let prefix_style = if i == 0 { glyph_style } else { text_style };
                if i == full_lines.len() - 1 && !age_suffix.is_empty() {
                    lines.push(Line::from_spans([
                        Span::styled(prefix, prefix_style),
                        Span::styled(line_text.as_str(), text_style),
                        Span::styled(&age_suffix, text_style),
                    ]));
                } else {
                    lines.push(Line::from_spans([
                        Span::styled(prefix, prefix_style),
                        Span::styled(line_text.as_str(), text_style),
                    ]));
                }
            }
            Paragraph::new(Text::from_lines(lines))
                .render(text_zone, frame);
        } else if text_zone.height == 1 {
            // Single line: truncate with "..." + age
            let text_budget = w.saturating_sub(age_w + 3 + 2); // 3 for "...", 2 for glyph
            let truncated: String = parent.actual.chars().take(text_budget).collect();
            render_line(frame, text_zone.x, text_zone.y, text_zone.width, &[
                (glyph_prefix.to_string(), glyph_style),
                (format!("{}...", truncated), text_style),
                (age_suffix, text_style),
            ]);
        } else {
            // Multi-line truncated: show N-1 full lines, last line = "..." + age
            let avail = (text_zone.height - 1) as usize;
            let mut lines: Vec<Line> = full_lines.iter().enumerate().take(avail)
                .map(|(i, l)| {
                    let pfx = if i == 0 { glyph_prefix } else { "  " };
                    let ps = if i == 0 { glyph_style } else { text_style };
                    Line::from_spans([Span::styled(pfx, ps), Span::styled(l.as_str(), text_style)])
                })
                .collect();
            lines.push(Line::from_spans([
                Span::styled("  ", text_style),
                Span::styled("...", text_style),
                Span::styled(&age_suffix, text_style),
            ]));
            Paragraph::new(Text::from_lines(lines))
                .render(text_zone, frame);
        }
    }

    /// Render inline focus detail card below a child line.
    /// The focused line already shows desire — the card shows everything else:
    /// reality, temporal facts, child count, recent notes.
    /// `parent_indent` is the indent of the parent item (0 for route, HELD_INDENT for held).
    /// Returns the number of lines consumed.
    fn render_inline_focus(
        &self,
        frame: &mut Frame<'_>,
        x: u16,
        start_y: u16,
        limit_y: u16,
        w: usize,
        cols: &ColumnLayout,
        detail: &FocusedDetail,
        parent_indent: usize,
    ) -> u16 {
        let mut y = start_y;
        if y >= limit_y { return 0; }

        let child_indent = parent_indent + HELD_INDENT;
        let indent_str = " ".repeat(cols.left + GUTTER + child_indent);
        let text_w = w.saturating_sub(cols.left + GUTTER + child_indent);
        if text_w == 0 { return 0; }

        // 1. Reality (the thing you never see in the list — subdued weight)
        if !detail.actual.is_empty() {
            let reality_lines = word_wrap(&detail.actual, text_w);
            for line_text in &reality_lines {
                if y >= limit_y { return y - start_y; }
                render_line(frame, x, y, w as u16, &[
                    (indent_str.clone(), STYLES.subdued),
                    (line_text.clone(), STYLES.subdued),
                ]);
                y += 1;
            }
        }

        // 2. Metadata line: intent → bridge → trace (mirrors deck top→bottom)
        if y >= limit_y { return y - start_y; }
        {
            let mut parts: Vec<String> = Vec::new();
            // Intent (left): deadline/horizon — the aimed-at future
            if let Some(ref dl) = detail.deadline_label {
                parts.push(dl.clone());
            }
            // Bridge (middle): child count — theory of closure
            if detail.child_count > 0 {
                let done = detail.child_resolved + detail.child_released;
                let mut child_part = format!("{}/{}", done, detail.child_count);
                if detail.child_held > 0 {
                    child_part.push_str(&format!(" {} held", detail.child_held));
                }
                parts.push(child_part);
            }
            // Trace (right): temporal ages — what happened when
            parts.push(format!("born {}", detail.created_age));
            if detail.last_desire_age != detail.created_age {
                parts.push(format!("\u{25c6} {}", detail.last_desire_age));
            }
            if detail.last_reality_age != detail.created_age {
                parts.push(format!("\u{25c7} {}", detail.last_reality_age));
            }
            let meta_text = parts.join(" \u{00b7} ");
            render_line(frame, x, y, w as u16, &[
                (indent_str.clone(), STYLES.dim),
                (meta_text, STYLES.dim),
            ]);
            y += 1;
        }

        // 3. Notes (fill remaining space, bounded by limit_y)
        for (age, text) in detail.recent_notes.iter() {
            if y >= limit_y { break; }
            let age_w = age.len() + 2;
            let note_avail = text_w.saturating_sub(age_w + 2);
            let truncated = if text.chars().count() > note_avail {
                let t: String = text.chars().take(note_avail.saturating_sub(1)).collect();
                format!("{}\u{2026}", t)
            } else {
                text.clone()
            };
            let display_w = truncated.chars().count();
            let pad = text_w.saturating_sub(2 + display_w + age.len());
            let note_line = format!("\u{203b} {}{}{}", truncated, " ".repeat(pad), age);
            render_line(frame, x, y, w as u16, &[
                (indent_str.clone(), STYLES.dim),
                (note_line, STYLES.dim),
            ]);
            y += 1;
        }

        y - start_y
    }

    /// Handle pitch up (k / Up) in deck mode.
    pub fn deck_pitch_up(&mut self) {
        let count = self.ensure_frontier().selectable_count();
        self.deck_cursor.pitch_up(count);
    }

    /// Handle pitch down (j / Down) in deck mode.
    pub fn deck_pitch_down(&mut self) {
        let count = self.ensure_frontier().selectable_count();
        self.deck_cursor.pitch_down(count);
    }

    /// Reset deck cursor to default position after data reload.
    pub fn deck_cursor_reset(&mut self) {
        let default = self.ensure_frontier().default_cursor();
        self.deck_cursor.index = default;
    }

    /// Set deck cursor to point at a specific sibling index.
    /// Falls back to default cursor if the sibling isn't visible.
    pub fn deck_cursor_to_sibling(&mut self, sibling_idx: usize) {
        let frontier = self.ensure_frontier();
        if let Some(cursor_idx) = frontier.cursor_for_sibling(sibling_idx) {
            self.deck_cursor.index = cursor_idx;
        } else {
            self.deck_cursor.index = frontier.default_cursor();
        }
    }

    /// Get the sibling index the deck cursor currently points to (if any).
    pub fn deck_selected_sibling_index(&self) -> Option<usize> {
        let frontier = self.cached_frontier.as_ref()?;
        Some(frontier.cursor_target(self.deck_cursor.index).sibling_index()?)
    }

    /// Render the deck bottom bar using ftui StatusLine.
    /// S6: Context-sensitive gesture hints based on cursor target (Normal mode only).
    pub fn render_deck_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        let content = self.deck_content_area(Rect::new(area.x, area.y, area.width, area.height + 10));
        let bar_area = Rect::new(content.x, area.y, content.width, 1);

        // S6: Context-sensitive hints — only in Normal input mode
        let hints_text = if matches!(self.input_mode, crate::state::InputMode::Normal) {
            let frontier = match self.cached_frontier.as_ref() {
                Some(f) => f,
                None => { return; }
            };
            let target = frontier.cursor_target(self.deck_cursor.index);
            match target {
                CursorTarget::Desire =>
                    "e edit desire \u{00B7} Enter focus".to_string(),
                CursorTarget::Route(_) | CursorTarget::Next(_) =>
                    "e edit \u{00B7} Enter focus \u{00B7} l descend \u{00B7} p hold \u{00B7} r resolve".to_string(),
                CursorTarget::Overdue(_) =>
                    "r resolve \u{00B7} ~ release \u{00B7} l descend".to_string(),
                CursorTarget::HeldItem(_) =>
                    "e edit \u{00B7} p position \u{00B7} Enter focus \u{00B7} r resolve".to_string(),
                CursorTarget::AccumulatedItem(_) =>
                    "l descend \u{00B7} Enter focus".to_string(),
                CursorTarget::NoteItem(_) =>
                    "Enter focus".to_string(),
                CursorTarget::InputPoint =>
                    "a add \u{00B7} n note \u{00B7} ! desire \u{00B7} ? reality".to_string(),
                CursorTarget::RouteSummary | CursorTarget::Held | CursorTarget::Accumulated =>
                    "Enter expand \u{00B7} j/k navigate".to_string(),
                CursorTarget::Reality =>
                    "e edit reality \u{00B7} Enter focus".to_string(),
            }
        } else {
            String::new()
        };

        // Build dynamic text, then borrow for StatusLine
        let events_text = if self.parent_mutation_count > 0 {
            format!("\u{2193} {} prior epochs", self.parent_mutation_count)
        } else {
            String::new()
        };

        // Render bar manually for true centering of hints
        let w = bar_area.width as usize;
        let left_w = events_text.chars().count();
        let right_text = "? help";
        let right_w = right_text.chars().count();
        let hints_w = hints_text.chars().count();

        // True center position for hints
        let hints_start = if hints_w > 0 {
            (w.saturating_sub(hints_w)) / 2
        } else { 0 };

        let mut spans: Vec<Span> = Vec::new();

        // Left: events text
        if !events_text.is_empty() {
            spans.push(Span::styled(&events_text, STYLES.lever));
        }

        // Gap between left and centered hints
        let after_left = left_w;
        if hints_w > 0 {
            let gap = hints_start.saturating_sub(after_left);
            spans.push(Span::styled(" ".repeat(gap), STYLES.lever));
            spans.push(Span::styled(&hints_text, STYLES.lever));
        }

        // Gap between hints and right
        let after_hints = if hints_w > 0 { hints_start + hints_w } else { after_left };
        let right_start = w.saturating_sub(right_w);
        let gap = right_start.saturating_sub(after_hints);
        spans.push(Span::styled(" ".repeat(gap), STYLES.lever));
        spans.push(Span::styled(right_text, STYLES.lever));

        Paragraph::new(Text::from(Line::from_spans(spans)))
            .render(bar_area, frame);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Render a single line from styled string pairs.
/// Return the appropriate glyph for a tension's status.
/// All items get a glyph for visual rhythm — · for active, ✓ for resolved, ~ for released.
fn status_glyph(status: TensionStatus) -> &'static str {
    match status {
        TensionStatus::Active => "\u{25c6}",     // ◆ committed, declared
        TensionStatus::Resolved => "\u{2713}",   // ✓
        TensionStatus::Released => "~",
    }
}

/// Render a single styled line at (x, y) using Paragraph.
fn render_line(frame: &mut Frame<'_>, x: u16, y: u16, width: u16, parts: &[(String, Style)]) {
    let spans: Vec<Span> = parts.iter().map(|(text, style)| Span::styled(text.clone(), *style)).collect();
    Paragraph::new(Text::from(Line::from_spans(spans)))
        .render(Rect::new(x, y, width, 1), frame);
}

/// Left padding string (empty left column).
fn pad_left(cols: &ColumnLayout) -> String {
    format!("{}{}", " ".repeat(cols.left), " ".repeat(GUTTER))
}

/// Word-wrap text to a given width.
pub fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    if text.chars().count() <= width {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.chars().count() + 1 + word.chars().count() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(text.to_string());
    }
    lines
}

/// Truncate a string to at most `max` chars, appending "..." if needed.
pub fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
