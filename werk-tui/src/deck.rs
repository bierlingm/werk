//! Deck view — the new TUI rendering for the operative instrument.
//!
//! V1: Skeleton with column layout.
//! V2: Frontier computation + console. Children classified into zones,
//!     rendered in the middle area. Pitch navigation through selectable items.

use ftui::Frame;
use ftui::layout::{Constraint, Flex, Rect};
use ftui::style::Style;
use ftui::text::{Line, Span, Text, WrapMode};
use ftui::widgets::status_line::{StatusLine, StatusItem};
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use sd_core::TensionStatus;

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
    /// Resolved or released steps (accumulated since last epoch).
    pub accumulated: Vec<usize>,
    /// How many route items to show individually (rest compressed to summary).
    pub show_route: usize,
    /// How many held items to show individually (0 = compressed indicator only).
    pub show_held: usize,
    /// How many accumulated items to show individually (0 = compressed indicator only).
    pub show_accumulated: usize,
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
                            frontier.accumulated.push(i);
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
        count
    }

    /// Get the default cursor position — rests at the input point (NOW/frontier).
    pub fn default_cursor(&self) -> usize {
        let mut pos = self.show_route;
        if self.show_route < self.route.len() { pos += 1; } // route summary
        pos += self.overdue.len();
        if self.next.is_some() { pos += 1; }
        if !self.held.is_empty() {
            pos += self.show_held;
            if self.show_held < self.held.len() { pos += 1; } // summary line
        }
        pos // input point is always right after held
    }

    /// Map a cursor index to what it points at.
    pub fn cursor_target(&self, cursor: usize) -> CursorTarget {
        let mut offset = 0;

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
                return CursorTarget::AccumulatedItem(self.accumulated[cursor - offset]);
            }
            offset += shown;
            if shown < self.accumulated.len() {
                if cursor == offset {
                    return CursorTarget::Accumulated;
                }
            }
        }

        CursorTarget::InputPoint // fallback
    }
}

/// What the deck cursor is pointing at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorTarget {
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
    /// An individual accumulated item (expanded).
    AccumulatedItem(usize),
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
    #[allow(dead_code)]
    Orient,
}

/// Detail for a focused element (V7).
#[derive(Debug, Clone)]
pub struct FocusedDetail {
    /// The sibling index of the focused element.
    pub sibling_index: usize,
    /// The focused child's desire text.
    pub desired: String,
    /// The focused child's reality text.
    pub actual: String,
    /// The focused child's children as FieldEntries (for render_child_line).
    pub children: Vec<FieldEntry>,
    /// The focused child's short code.
    pub short_code: Option<i32>,
    /// The focused child's deadline label.
    pub deadline_label: Option<String>,
}

// ---------------------------------------------------------------------------
// DeckState — lives alongside InstrumentApp
// ---------------------------------------------------------------------------

/// State for the new deck rendering.
pub struct DeckState {
    pub columns: ColumnLayout,
    pub zoom: ZoomLevel,
    pub cursor: DeckCursor,
}

impl DeckState {
    pub fn new(total_width: usize, deadline_label: Option<&str>, max_id: usize, max_age_len: usize) -> Self {
        Self {
            columns: ColumnLayout::compute(total_width, deadline_label, max_id, max_age_len),
            zoom: ZoomLevel::Normal,
            cursor: DeckCursor::default(),
        }
    }

    /// Recompute column layout (e.g. on terminal resize or data change).
    pub fn recompute_columns(&mut self, total_width: usize, deadline_label: Option<&str>, max_id: usize, max_age_len: usize) {
        self.columns = ColumnLayout::compute(total_width, deadline_label, max_id, max_age_len);
    }
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

        let parent = match &self.parent_tension {
            Some(p) => p,
            None => {
                self.render_field(&area, frame);
                return;
            }
        };

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

        // --- Frontier classification ---
        let mut frontier = Frontier::compute(&self.siblings, self.trajectory_mode, self.epoch_boundary);

        // --- Measure zones for Flex layout ---
        let has_breadcrumb = self.grandparent_display.is_some();
        let has_deadline = deadline_label.is_some() && !deadline_label.unwrap_or("").is_empty();
        let desire_indent = if has_deadline { cols.left + GUTTER } else { 0 };
        let right_col_reserve = GUTTER + cols.right;
        let desire_wrap_width = w.saturating_sub(desire_indent + right_col_reserve);
        let desire_lines = word_wrap(&parent.desired, desire_wrap_width);

        let top_height: u16 = {
            let mut h: u16 = 0;
            if has_breadcrumb { h += 1; }
            h += 1; // blank line before desire
            h += desire_lines.len() as u16;
            h += 1; // desire rule
            h
        };

        // Reality: use Paragraph with word wrap for measurement
        // Reality: measure height for Flex constraint (Paragraph handles actual wrapping)
        let reality_age_str = self.parent_reality_age.as_deref().unwrap_or("");
        let reality_age_reserve = if reality_age_str.is_empty() { 0 } else { 3 + reality_age_str.chars().count() };
        let reality_line_count = if parent.actual.is_empty() {
            0u16
        } else {
            word_wrap(&parent.actual, w.saturating_sub(reality_age_reserve)).len() as u16
        };
        let bottom_height: u16 = {
            let mut h: u16 = 0;
            if reality_line_count > 0 {
                h += 1; // blank line before reality
                h += reality_line_count;
            }
            h += 1; // reality rule
            h
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

        // === Render top zone (desire anchor) ===
        self.render_desire_zone(frame, top_zone, w, &cols, parent, &desire_lines,
            has_breadcrumb, has_deadline, deadline_label.unwrap_or(""));

        // === Render bottom zone (reality anchor) ===
        self.render_reality_zone(frame, bottom_zone, w, parent, reality_age_str);

        // === Render middle zone ===
        if middle_zone.height == 0 {
            return;
        }

        // V7: measure inline focus detail height to reserve space
        let focus_detail_height: usize = if self.deck_zoom == ZoomLevel::Focus {
            if let Some(ref detail) = self.focused_detail {
                let ch = detail.children.len();
                let rl = if detail.actual.is_empty() { 0 } else {
                    word_wrap(&detail.actual, w).len() + 1 // +1 for blank line before reality
                };
                ch + rl + 1 // children + reality + closing separator
            } else { 0 }
        } else { 0 };

        // Compute space-aware expansion for held/accumulated
        let middle_start = middle_zone.y;
        let middle_end = middle_zone.y + middle_zone.height;
        let middle_lines = middle_zone.height as usize;
        frontier.compute_expansion(middle_lines.saturating_sub(focus_detail_height));

        let cursor_idx = self.deck_cursor.index;
        let focused_sibling = if self.deck_zoom == ZoomLevel::Focus {
            self.focused_detail.as_ref().map(|d| d.sibling_index)
        } else {
            None
        };

        // === Bottom-up pass: accumulated items (gravity toward reality) ===
        // Render from middle_end upward. We compute positions first, then render.

        let mut acc_top = middle_end; // will be decremented as we place accumulated items

        if !frontier.accumulated.is_empty() {
            let shown = frontier.show_accumulated.min(frontier.accumulated.len());
            let remaining = frontier.accumulated.len() - shown;

            // Summary line (rendered closest to reality, at the bottom of accumulated)
            if remaining > 0 {
                acc_top -= 1;
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Accumulated;

                let remaining_resolved = frontier.accumulated[shown..].iter()
                    .filter(|&&idx| self.siblings[idx].status == TensionStatus::Resolved)
                    .count();
                let remaining_released = frontier.accumulated[shown..].iter()
                    .filter(|&&idx| self.siblings[idx].status == TensionStatus::Released)
                    .count();

                let mut parts = Vec::new();
                if remaining_resolved > 0 {
                    parts.push(format!("\u{2713} {} more resolved", remaining_resolved));
                }
                if remaining_released > 0 {
                    parts.push(format!("~ {} more released", remaining_released));
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
                if frontier.accumulated[..shown].contains(&fi) {
                    focus_detail_height as u16
                } else { 0 }
            } else { 0 };

            for i in (0..shown).rev() {
                if acc_top <= middle_start { break; }
                // Reserve space for focus detail if this is the focused item
                let sibling_idx = frontier.accumulated[i];
                if focused_sibling == Some(sibling_idx) {
                    // Place focus detail first (above the item in render order)
                    if let Some(ref detail) = self.focused_detail {
                        let dl = self.render_inline_focus(
                            frame, area.x, acc_top.saturating_sub(focus_detail_height as u16),
                            acc_top, w, &cols, detail, 0
                        );
                        acc_top = acc_top.saturating_sub(dl);
                    }
                }
                if acc_top <= middle_start { break; }
                acc_top -= 1;
                let entry = &self.siblings[sibling_idx];
                let glyph = match entry.status {
                    TensionStatus::Resolved => "\u{2713}",  // ✓
                    TensionStatus::Released => "~",
                    _ => "\u{00B7}",
                };
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::AccumulatedItem(sibling_idx);
                self.render_child_line(frame, area.x, acc_top, w, &cols, entry, glyph, is_selected, false, 0);
            }
        }

        // The top-down section must not overlap with the accumulated section
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
            // Merge route + held into one summary line above NOW (Q28)
            let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::RouteSummary
                || frontier.cursor_target(cursor_idx) == CursorTarget::Held;
            let text = format!(
                "\u{25B8} {} route \u{00B7} {} held",
                frontier.route.len(),
                frontier.held.len()
            );
            self.render_indicator_line(frame, area.x, my, w, &cols, &text, is_selected, STYLES.dim, 0);
            my += 1;

            // Overdue and next still render even when unified
            for &sibling_idx in &frontier.overdue {
                if my >= top_limit { break; }
                let entry = &self.siblings[sibling_idx];
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Overdue(sibling_idx);
                self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{00B7}", is_selected, true, 0);
                my += 1;
            }
            if let Some(next_idx) = frontier.next {
                if my < top_limit {
                    let entry = &self.siblings[next_idx];
                    let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Next(next_idx);
                    self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{00B7}", is_selected, false, 0);
                    my += 1;
                }
            }
        } else {
            // --- Route zone (gradual: show N individually, summary for rest) ---
            for i in 0..shown_route {
                if my >= top_limit { break; }
                let sibling_idx = frontier.route[i];
                let entry = &self.siblings[sibling_idx];
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Route(sibling_idx);
                let glyph = status_glyph(entry.status);
                self.render_child_line(frame, area.x, my, w, &cols, entry, glyph, is_selected, false, 0);
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
                self.render_child_line(frame, area.x, my, w, &cols, entry, glyph, is_selected, false, 0);
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
                let text = match next_deadline {
                    Some(dl) => format!("\u{25B2} {} remaining \u{00B7} next {}", count, dl),
                    None => format!("\u{25B2} {} remaining", count),
                };
                self.render_indicator_line(frame, area.x, my, w, &cols, &text, is_selected, STYLES.dim, 0);
                my += 1;
            }

            // --- Overdue steps (part of the action sequence, above the separator) ---
            for &sibling_idx in &frontier.overdue {
                if my >= top_limit { break; }
                let entry = &self.siblings[sibling_idx];
                let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Overdue(sibling_idx);
                self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{00B7}", is_selected, true, 0);
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
                    self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{00B7}", is_selected, false, 0);
                    my += 1;
                    if focused_sibling == Some(next_idx) {
                        if let Some(ref detail) = self.focused_detail {
                            my += self.render_inline_focus(frame, area.x, my, top_limit, w, &cols, detail, 0);
                        }
                    }
                }
            }

            // --- Console boundary: separates ordered (route+overdue+next) from unordered (held+input) ---
            let has_unordered = !frontier.held.is_empty() || !frontier.accumulated.is_empty();
            let has_ordered = !frontier.route.is_empty() || !frontier.overdue.is_empty() || frontier.next.is_some();
            let show_separator = match self.deck_config.chrome {
                ChromeMode::Quiet => false,
                ChromeMode::Adaptive => has_unordered && has_ordered,
                ChromeMode::Structured => has_ordered || has_unordered,
            };
            if show_separator && my < top_limit {
                let boundary = glyphs::LIGHT_RULE.to_string().repeat(w);
                render_line(frame, area.x, my, area.width, &[
                    (boundary, STYLES.dim),
                ]);
                my += 1;
            }

            // --- Held items (gradual: show N individually, summary for rest) ---
            if !frontier.held.is_empty() && my < top_limit {
                // Individual items
                for i in 0..shown_held {
                    if my >= top_limit { break; }
                    let sibling_idx = frontier.held[i];
                    let entry = &self.siblings[sibling_idx];
                    let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::HeldItem(sibling_idx);
                    self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{00B7}", is_selected, false, HELD_INDENT);
                    my += 1;
                    if focused_sibling == Some(sibling_idx) {
                        if let Some(ref detail) = self.focused_detail {
                            my += self.render_inline_focus(frame, area.x, my, top_limit, w, &cols, detail, HELD_INDENT);
                        }
                    }
                }
                // Summary for remaining
                if held_remaining > 0 && my < top_limit {
                    let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::Held;
                    let text = if shown_held == 0 {
                        format!("\u{00B7} {} held", frontier.held.len())
                    } else {
                        format!("\u{00B7} {} more held", held_remaining)
                    };
                    self.render_indicator_line(frame, area.x, my, w, &cols, &text, is_selected, STYLES.dim, HELD_INDENT);
                    my += 1;
                }
            }
        }

        // --- Input point ---
        if my < top_limit {
            let is_selected = frontier.cursor_target(cursor_idx) == CursorTarget::InputPoint;
            let style = if is_selected {
                Style::new().fg(CLR_DIM).bg(CLR_SELECTED_BG)
            } else {
                STYLES.dim
            };
            let prefix_len = cols.left + GUTTER;
            let content = "+ ___";
            let pad_right = w.saturating_sub(prefix_len + content.len());
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
    ) {
        let is_done = entry.status == TensionStatus::Resolved
            || entry.status == TensionStatus::Released;

        let base_style = if is_selected {
            STYLES.selected
        } else if is_overdue {
            STYLES.amber
        } else if is_done {
            STYLES.dim
        } else {
            STYLES.text
        };

        let glyph_style = if is_overdue && !is_selected {
            STYLES.amber
        } else {
            base_style
        };

        // Left column: deadline
        let left_str = entry.horizon_label.as_deref().unwrap_or("");
        let left_padded = format!("{:<width$}", left_str, width = cols.left);

        // Right sub-columns: [id] [→] [age]
        let id_num = entry.short_code
            .map(|sc| format!("{:0>width$}", sc, width = cols.id_width))
            .unwrap_or_else(|| entry.id[..cols.id_width.min(entry.id.len())].to_string());

        let arrow = if entry.child_count > 0 { "\u{2192}" } else { " " };

        let age_str = format!("{:>width$}", entry.created_age, width = cols.age_width);

        let right_str = format!("{} {} {}", id_num, arrow, age_str);

        // Main column: glyph + text — maximum budget
        let glyph_w = 2; // glyph + space
        let right_w = right_str.chars().count();
        let text_budget = w
            .saturating_sub(cols.left + GUTTER + extra_indent + glyph_w + GUTTER + right_w);
        let main_text = truncate_str(&entry.desired, text_budget);

        // Build the line
        let mut spans: Vec<Span> = Vec::new();

        let left_style = if is_selected { base_style } else if is_overdue { STYLES.amber } else { STYLES.dim };
        let right_style = if is_selected { base_style } else { STYLES.dim };

        spans.push(Span::styled(left_padded, left_style));
        spans.push(Span::styled(" ".repeat(GUTTER + extra_indent), base_style));
        spans.push(Span::styled(format!("{} ", glyph), glyph_style));
        spans.push(Span::styled(&main_text, base_style));

        // Pad between text and right sub-columns
        let used: usize = cols.left + GUTTER + extra_indent + glyph_w + main_text.chars().count();
        let gap = w.saturating_sub(used + right_w);
        spans.push(Span::styled(" ".repeat(gap), base_style));
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

            spans.push((line_text.clone(), STYLES.text_bold));

            if i == 0 {
                let text_used = if has_deadline { cols.left + GUTTER } else { 0 } + line_text.chars().count();
                let gap = w.saturating_sub(text_used + desire_right_w);
                if gap >= GUTTER {
                    spans.push((" ".repeat(gap), Style::new()));
                    spans.push((desire_right.clone(), STYLES.dim));
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

        // Reality text with word wrap via Paragraph
        let text_zone = zones[1];
        if text_zone.height == 0 { return; }

        // Build text with age suffix inline
        // Build a single Line with reality text + age suffix; Paragraph wraps it
        let mut spans = vec![Span::styled(&parent.actual, STYLES.dim)];
        if !reality_age.is_empty() {
            spans.push(Span::styled(" \u{00B7} ", STYLES.dim));
            spans.push(Span::styled(reality_age, STYLES.dim));
        }
        let reality_text = Text::from(Line::from_spans(spans));

        Paragraph::new(reality_text)
            .wrap(WrapMode::Word)
            .render(text_zone, frame);
    }

    /// Render inline focus detail below a child line (V7).
    /// Shows children (with right-column annotations) and reality.
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

        // Children indent further than the parent item
        let child_indent = parent_indent + HELD_INDENT;

        // Children rendered with render_child_line (indented deeper than parent)
        for child in &detail.children {
            if y >= limit_y { break; }
            let glyph = status_glyph(child.status);
            self.render_child_line(frame, x, y, w, cols, child, glyph, false, false, child_indent);
            y += 1;
        }

        // Reality (dim, word-wrapped) with blank line before if there are children
        if !detail.actual.is_empty() && y < limit_y {
            if !detail.children.is_empty() && y < limit_y {
                y += 1; // blank line between children and reality
            }
            let avail = limit_y.saturating_sub(y);
            if avail > 0 {
                let indent_str = " ".repeat(cols.left + GUTTER + child_indent);
                let text_w = w.saturating_sub(cols.left + GUTTER + child_indent);
                let reality_lines = word_wrap(&detail.actual, text_w);
                for line_text in &reality_lines {
                    if y >= limit_y { break; }
                    render_line(frame, x, y, w as u16, &[
                        (indent_str.clone(), STYLES.dim),
                        (line_text.clone(), STYLES.dim),
                    ]);
                    y += 1;
                }
            }
        }

        y - start_y
    }

    /// Compute frontier with maximum expansion for navigation.
    /// The render path does the precise space-aware expansion.
    pub fn frontier_for_navigation(&self) -> Frontier {
        let mut frontier = Frontier::compute(&self.siblings, self.trajectory_mode, self.epoch_boundary);
        // Show all items for navigation — render will compress if needed
        frontier.show_route = frontier.route.len();
        frontier.show_held = frontier.held.len();
        frontier.show_accumulated = frontier.accumulated.len();
        frontier
    }

    /// Handle pitch up (k / Up) in deck mode.
    pub fn deck_pitch_up(&mut self) {
        let frontier = self.frontier_for_navigation();
        self.deck_cursor.pitch_up(frontier.selectable_count());
    }

    /// Handle pitch down (j / Down) in deck mode.
    pub fn deck_pitch_down(&mut self) {
        let frontier = self.frontier_for_navigation();
        self.deck_cursor.pitch_down(frontier.selectable_count());
    }

    /// Reset deck cursor to default position after data reload.
    pub fn deck_cursor_reset(&mut self) {
        let frontier = self.frontier_for_navigation();
        self.deck_cursor.index = frontier.default_cursor();
    }

    /// Get the sibling index the deck cursor currently points to (if any).
    pub fn deck_selected_sibling_index(&self) -> Option<usize> {
        let frontier = self.frontier_for_navigation();
        frontier.cursor_target(self.deck_cursor.index).sibling_index()
    }

    /// Render the deck bottom bar using ftui StatusLine.
    pub fn render_deck_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        let content = self.deck_content_area(Rect::new(area.x, area.y, area.width, area.height + 10));
        let bar_area = Rect::new(content.x, area.y, content.width, 1);

        // Build dynamic text, then borrow for StatusLine
        let events_text = if self.parent_mutation_count > 0 {
            format!("\u{2193} {} prior events", self.parent_mutation_count)
        } else {
            String::new()
        };

        let mut bar = StatusLine::new().style(STYLES.lever);
        if !events_text.is_empty() {
            bar = bar.left(StatusItem::text(&events_text));
        }
        bar = bar.right(StatusItem::text("? help"));

        bar.render(bar_area, frame);
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
        TensionStatus::Active => "\u{00B7}",     // · subtle bullet
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
