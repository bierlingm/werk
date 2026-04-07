//! Frontier zone rendering using ftui List widget.
//!
//! Extracts line-building logic from deck.rs into reusable functions.
//! Each frontier zone (route, overdue, next, held, accumulated) builds
//! a List widget from its items. The reconciliation algorithm in
//! render_deck() determines how many items each zone shows; this module
//! converts those items into ListItems.

use ftui::style::Style;
use ftui::text::{Line, Span};
use ftui::widgets::list::{List, ListItem, ListState};
use ftui::PackedRgba;

use sd_core::TensionStatus;

use crate::deck::{AccumulatedItem, ColumnLayout, CursorTarget, Frontier};
use crate::state::FieldEntry;
use crate::theme::InstrumentStyles;

/// Extra indent for held (unpositioned) items.
pub const HELD_INDENT: usize = 2;

// ---------------------------------------------------------------------------
// Status glyph
// ---------------------------------------------------------------------------

/// Return the glyph for a tension's status.
pub fn status_glyph(status: TensionStatus) -> &'static str {
    match status {
        TensionStatus::Active => "\u{25c6}",   // ◆
        TensionStatus::Resolved => "\u{2713}", // ✓
        TensionStatus::Released => "~",
    }
}

// ---------------------------------------------------------------------------
// Line builders — produce Line objects for each item type
// ---------------------------------------------------------------------------

/// Build a Line for a child tension in the 3-column layout.
///
/// This is the core rendering primitive for route, overdue, next, held,
/// and accumulated child items. The same layout logic that was in
/// `render_child_line()`, extracted for reuse with List widget.
pub fn build_child_line(
    entry: &FieldEntry,
    glyph: &str,
    is_selected: bool,
    is_overdue: bool,
    extra_indent: usize,
    glyph_color: Option<Style>,
    cols: &ColumnLayout,
    w: usize,
    styles: &InstrumentStyles,
) -> Line<'static> {
    let is_done = entry.status == TensionStatus::Resolved
        || entry.status == TensionStatus::Released;

    let base_style = if is_selected {
        styles.selected
    } else if is_overdue {
        if entry.temporal_urgency > 2.0 {
            Style::new().fg(PackedRgba::rgb(230, 190, 60)).bold()
        } else if entry.temporal_urgency > 1.3 {
            Style::new().fg(styles.clr_amber).bold()
        } else {
            styles.amber
        }
    } else if is_done {
        styles.dim
    } else {
        styles.text
    };

    let glyph_style = if is_selected || is_overdue {
        base_style
    } else {
        glyph_color.unwrap_or(base_style)
    };

    // Left column: deadline label
    let left_str = entry
        .horizon_label
        .as_deref()
        .unwrap_or("")
        .to_string();
    let left_padded = format!("{:<width$}", left_str, width = cols.left);

    // Right sub-columns: [id] [→] [age]
    let id_num = entry
        .short_code
        .map(|sc| format!("{:0>width$}", sc, width = cols.id_width))
        .unwrap_or_else(|| entry.id[..cols.id_width.min(entry.id.len())].to_string());

    let right_str = if cols.age_width > 0 {
        let arrow = if entry.child_count > 0 { "\u{2192}" } else { " " };
        let age_str = format!("{:>width$}", entry.created_age, width = cols.age_width);
        format!("{} {} {}", id_num, arrow, age_str)
    } else {
        id_num.clone()
    };

    // OVERDUE tag
    let overdue_tag = if is_overdue && !is_selected {
        "OVERDUE  "
    } else {
        ""
    };
    let overdue_tag_w = overdue_tag.chars().count();

    // Main column: glyph + text
    let glyph_w = 2; // glyph + space
    let right_w = right_str.chars().count();
    let text_budget = w.saturating_sub(
        cols.left + cols.gutter + extra_indent + glyph_w + overdue_tag_w + cols.gutter + right_w,
    );
    let main_text = truncate_str(&entry.desired, text_budget);

    // Build spans
    let left_style = if is_selected || is_overdue {
        base_style
    } else {
        styles.dim
    };
    let right_style = if is_selected { base_style } else { styles.dim };

    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled(left_padded, left_style));
    spans.push(Span::styled(" ".repeat(cols.gutter + extra_indent), base_style));
    spans.push(Span::styled(format!("{} ", glyph), glyph_style));
    spans.push(Span::styled(main_text.clone(), base_style));

    // Gap between text and OVERDUE tag / right columns
    let used = cols.left + cols.gutter + extra_indent + glyph_w + main_text.chars().count();
    let gap = w.saturating_sub(used + overdue_tag_w + right_w);
    spans.push(Span::styled(" ".repeat(gap), base_style));
    if !overdue_tag.is_empty() {
        spans.push(Span::styled(overdue_tag.to_string(), base_style));
    }
    spans.push(Span::styled(right_str, right_style));

    // Pad to full width for selection highlight
    let total_rendered: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if total_rendered < w {
        spans.push(Span::styled(" ".repeat(w - total_rendered), base_style));
    }

    Line::from_spans(spans)
}

/// Build a Line for a note in the accumulated zone.
pub fn build_note_line(
    text: &str,
    age: &str,
    is_selected: bool,
    cols: &ColumnLayout,
    w: usize,
    styles: &InstrumentStyles,
) -> Line<'static> {
    let base_style = if is_selected {
        styles.selected
    } else {
        styles.dim
    };
    let glyph = "\u{203b}"; // ※
    let glyph_w = 2;
    let age_w = age.chars().count();
    let text_budget = w.saturating_sub(cols.left + cols.gutter + glyph_w + cols.gutter + age_w);
    let main_text = if text.chars().count() > text_budget {
        let t: String = text.chars().take(text_budget.saturating_sub(1)).collect();
        format!("{}\u{2026}", t)
    } else {
        text.to_string()
    };

    let used = cols.left + cols.gutter + glyph_w + main_text.chars().count();
    let gap = w.saturating_sub(used + age_w);

    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(format!("{:<width$}", "", width = cols.left), base_style),
        Span::styled(" ".repeat(cols.gutter), base_style),
        Span::styled(format!("{} ", glyph), base_style),
        Span::styled(main_text, base_style),
        Span::styled(" ".repeat(gap), base_style),
        Span::styled(age.to_string(), base_style),
    ];

    let total_rendered: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if total_rendered < w {
        spans.push(Span::styled(
            " ".repeat(w - total_rendered),
            base_style,
        ));
    }

    Line::from_spans(spans)
}

/// Build a Line for an indicator/summary (held summary, accumulated summary).
pub fn build_indicator_line(
    text: &str,
    is_selected: bool,
    extra_indent: usize,
    cols: &ColumnLayout,
    w: usize,
    styles: &InstrumentStyles,
) -> Line<'static> {
    let style = if is_selected {
        styles.selected
    } else {
        styles.dim
    };

    let prefix_len = cols.left + cols.gutter + extra_indent;
    let pad_right = w.saturating_sub(prefix_len + text.chars().count());
    Line::from_spans([
        Span::styled(" ".repeat(prefix_len), style),
        Span::styled(text.to_string(), style),
        Span::styled(" ".repeat(pad_right), style),
    ])
}

/// Build a Line for the input point (action surface).
pub fn build_input_line(
    is_selected: bool,
    is_empty_console: bool,
    cols: &ColumnLayout,
    w: usize,
    styles: &InstrumentStyles,
) -> Line<'static> {
    let content = if is_selected {
        if is_empty_console {
            "\u{25B8} a add first step \u{00B7} n note \u{00B7} ! desire \u{00B7} ? reality"
        } else {
            "\u{25B8} a add \u{00B7} n note \u{00B7} ! desire \u{00B7} ? reality"
        }
        .to_string()
    } else {
        "\u{25B8} ___".to_string()
    };

    let style = if is_selected {
        styles.selected
    } else {
        styles.dim
    };

    let prefix_len = cols.left + cols.gutter;
    let pad_right = w.saturating_sub(prefix_len + content.chars().count());
    Line::from_spans([
        Span::styled(" ".repeat(prefix_len), style),
        Span::styled(content, style),
        Span::styled(" ".repeat(pad_right), style),
    ])
}

// ---------------------------------------------------------------------------
// List widget builders — one List per zone
// ---------------------------------------------------------------------------

/// Build a List widget for the route zone.
pub fn build_route_list<'a>(
    frontier: &Frontier,
    siblings: &[FieldEntry],
    shown_route: usize,
    active_target: CursorTarget,
    cols: &ColumnLayout,
    w: usize,
    styles: &InstrumentStyles,
) -> (List<'a>, ListState) {
    let mut items: Vec<ListItem<'a>> = Vec::new();
    let mut selected: Option<usize> = None;

    for i in 0..shown_route {
        let sibling_idx = frontier.route[i];
        let entry = &siblings[sibling_idx];
        let is_sel =
            active_target == CursorTarget::Route(sibling_idx);
        if is_sel {
            selected = Some(items.len());
        }
        let glyph = status_glyph(entry.status);
        let line = build_child_line(
            entry,
            glyph,
            is_sel,
            false,
            0,
            Some(styles.cyan),
            cols,
            w,
            styles,
        );
        items.push(ListItem::new(line));
    }

    // Route summary (remaining items)
    let route_remaining = frontier.route.len() - shown_route;
    if route_remaining == 1 {
        let sibling_idx = frontier.route[shown_route];
        let entry = &siblings[sibling_idx];
        let is_sel =
            active_target == CursorTarget::RouteSummary;
        if is_sel {
            selected = Some(items.len());
        }
        let glyph = status_glyph(entry.status);
        let line = build_child_line(
            entry,
            glyph,
            is_sel,
            false,
            0,
            Some(styles.cyan),
            cols,
            w,
            styles,
        );
        items.push(ListItem::new(line));
    } else if route_remaining > 1 {
        let is_sel =
            active_target == CursorTarget::RouteSummary;
        if is_sel {
            selected = Some(items.len());
        }
        let count = if shown_route == 0 {
            frontier.route.len()
        } else {
            route_remaining
        };
        let next_deadline = frontier.route[shown_route..]
            .iter()
            .filter_map(|&idx| siblings[idx].horizon_label.as_deref())
            .next();
        let label = if shown_route > 0 { "more" } else { "route steps" };
        let text = match next_deadline {
            Some(dl) => format!("\u{25B8} {} {} \u{00B7} next {}", count, label, dl),
            None => format!("\u{25B8} {} {}", count, label),
        };
        let line = build_indicator_line(&text, is_sel, 0, cols, w, styles);
        items.push(ListItem::new(line));
    }

    let list = List::new(items).highlight_style(styles.selected);
    let mut state = ListState::default();
    state.select(selected);
    (list, state)
}

/// Build a route list segment: items from `route_start` to `route_start + route_count`,
/// with optional summary appended for remaining items beyond `shown_route`.
pub fn build_route_list_segment<'a>(
    frontier: &Frontier,
    siblings: &[FieldEntry],
    route_start: usize,
    route_count: usize,
    shown_route: usize,
    include_summary: bool,
    active_target: CursorTarget,
    cols: &ColumnLayout,
    w: usize,
    styles: &InstrumentStyles,
) -> (List<'a>, ListState) {
    let mut items: Vec<ListItem<'a>> = Vec::new();
    let mut selected: Option<usize> = None;
    let route_end = (route_start + route_count).min(frontier.route.len());

    for i in route_start..route_end {
        let sibling_idx = frontier.route[i];
        let entry = &siblings[sibling_idx];
        let is_sel = active_target == CursorTarget::Route(sibling_idx);
        if is_sel { selected = Some(items.len()); }
        let glyph = status_glyph(entry.status);
        let line = build_child_line(entry, glyph, is_sel, false, 0, Some(styles.cyan), cols, w, styles);
        items.push(ListItem::new(line));
    }

    if include_summary {
        let route_remaining = frontier.route.len() - shown_route;
        if route_remaining == 1 {
            let sibling_idx = frontier.route[shown_route];
            let entry = &siblings[sibling_idx];
            let is_sel = active_target == CursorTarget::RouteSummary;
            if is_sel { selected = Some(items.len()); }
            let glyph = status_glyph(entry.status);
            let line = build_child_line(entry, glyph, is_sel, false, 0, Some(styles.cyan), cols, w, styles);
            items.push(ListItem::new(line));
        } else if route_remaining > 1 {
            let is_sel = active_target == CursorTarget::RouteSummary;
            if is_sel { selected = Some(items.len()); }
            let count = if shown_route == 0 { frontier.route.len() } else { route_remaining };
            let next_deadline = frontier.route[shown_route..]
                .iter()
                .filter_map(|&idx| siblings[idx].horizon_label.as_deref())
                .next();
            let label = if shown_route > 0 { "more" } else { "route steps" };
            let text = match next_deadline {
                Some(dl) => format!("\u{25B8} {} {} \u{00B7} next {}", count, label, dl),
                None => format!("\u{25B8} {} {}", count, label),
            };
            let line = build_indicator_line(&text, is_sel, 0, cols, w, styles);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items).highlight_style(styles.selected);
    let mut state = ListState::default();
    state.select(selected);
    (list, state)
}

/// Build a held list segment: items from `held_start` to `held_start + held_count`,
/// with optional summary appended for remaining items beyond `shown_held`.
pub fn build_held_list_segment<'a>(
    frontier: &Frontier,
    siblings: &[FieldEntry],
    held_start: usize,
    held_count: usize,
    shown_held: usize,
    include_summary: bool,
    active_target: CursorTarget,
    cols: &ColumnLayout,
    w: usize,
    styles: &InstrumentStyles,
) -> (List<'a>, ListState) {
    let mut items: Vec<ListItem<'a>> = Vec::new();
    let mut selected: Option<usize> = None;
    let held_end = (held_start + held_count).min(frontier.held.len());

    for i in held_start..held_end {
        let sibling_idx = frontier.held[i];
        let entry = &siblings[sibling_idx];
        let is_sel = active_target == CursorTarget::HeldItem(sibling_idx);
        if is_sel { selected = Some(items.len()); }
        let line = build_child_line(entry, "\u{2727}", is_sel, false, HELD_INDENT, Some(styles.subdued), cols, w, styles);
        items.push(ListItem::new(line));
    }

    if include_summary {
        let held_remaining = frontier.held.len() - shown_held;
        if held_remaining == 1 {
            let sibling_idx = frontier.held[shown_held];
            let entry = &siblings[sibling_idx];
            let is_sel = active_target == CursorTarget::Held;
            if is_sel { selected = Some(items.len()); }
            let line = build_child_line(entry, "\u{2727}", is_sel, false, HELD_INDENT, Some(styles.subdued), cols, w, styles);
            items.push(ListItem::new(line));
        } else if held_remaining > 1 {
            let is_sel = active_target == CursorTarget::Held;
            if is_sel { selected = Some(items.len()); }
            let text = if shown_held == 0 {
                format!("\u{2727} {} held", frontier.held.len())
            } else {
                format!("\u{2727} {} more held", held_remaining)
            };
            let line = build_indicator_line(&text, is_sel, HELD_INDENT, cols, w, styles);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items).highlight_style(styles.selected);
    let mut state = ListState::default();
    state.select(selected);
    (list, state)
}

/// Build a List widget for the overdue zone.
pub fn build_overdue_list<'a>(
    frontier: &Frontier,
    siblings: &[FieldEntry],
    active_target: CursorTarget,
    cols: &ColumnLayout,
    w: usize,
    styles: &InstrumentStyles,
) -> (List<'a>, ListState) {
    let mut items: Vec<ListItem<'a>> = Vec::new();
    let mut selected: Option<usize> = None;

    for &sibling_idx in &frontier.overdue {
        let entry = &siblings[sibling_idx];
        let is_sel =
            active_target == CursorTarget::Overdue(sibling_idx);
        if is_sel {
            selected = Some(items.len());
        }
        let line = build_child_line(
            entry,
            "\u{25c6}",
            is_sel,
            true,
            0,
            Some(styles.cyan),
            cols,
            w,
            styles,
        );
        items.push(ListItem::new(line));
    }

    let list = List::new(items).highlight_style(styles.selected);
    let mut state = ListState::default();
    state.select(selected);
    (list, state)
}

/// Build a List widget for the held zone.
pub fn build_held_list<'a>(
    frontier: &Frontier,
    siblings: &[FieldEntry],
    shown_held: usize,
    active_target: CursorTarget,
    cols: &ColumnLayout,
    w: usize,
    styles: &InstrumentStyles,
) -> (List<'a>, ListState) {
    let mut items: Vec<ListItem<'a>> = Vec::new();
    let mut selected: Option<usize> = None;

    // Individual items
    for i in 0..shown_held {
        let sibling_idx = frontier.held[i];
        let entry = &siblings[sibling_idx];
        let is_sel = active_target == CursorTarget::HeldItem(sibling_idx);
        if is_sel {
            selected = Some(items.len());
        }
        let line = build_child_line(
            entry,
            "\u{2727}",
            is_sel,
            false,
            HELD_INDENT,
            Some(styles.subdued),
            cols,
            w,
            styles,
        );
        items.push(ListItem::new(line));
    }

    // Remaining: show individually if 1, else summary
    let held_remaining = frontier.held.len() - shown_held;
    if held_remaining == 1 {
        let sibling_idx = frontier.held[shown_held];
        let entry = &siblings[sibling_idx];
        let is_sel =
            active_target == CursorTarget::Held;
        if is_sel {
            selected = Some(items.len());
        }
        let line = build_child_line(
            entry,
            "\u{2727}",
            is_sel,
            false,
            HELD_INDENT,
            Some(styles.subdued),
            cols,
            w,
            styles,
        );
        items.push(ListItem::new(line));
    } else if held_remaining > 1 {
        let is_sel =
            active_target == CursorTarget::Held;
        if is_sel {
            selected = Some(items.len());
        }
        let text = if shown_held == 0 {
            format!("\u{2727} {} held", frontier.held.len())
        } else {
            format!("\u{2727} {} more held", held_remaining)
        };
        let line = build_indicator_line(&text, is_sel, HELD_INDENT, cols, w, styles);
        items.push(ListItem::new(line));
    }

    let list = List::new(items).highlight_style(styles.selected);
    let mut state = ListState::default();
    state.select(selected);
    (list, state)
}

/// Build a List widget for the accumulated zone.
///
/// Items are in display order (most recent first, rendered bottom-up in the
/// original code). The caller is responsible for anchoring the list rect
/// appropriately.
pub fn build_accumulated_list<'a>(
    frontier: &Frontier,
    siblings: &[FieldEntry],
    shown: usize,
    active_target: CursorTarget,
    cols: &ColumnLayout,
    w: usize,
    styles: &InstrumentStyles,
) -> (List<'a>, ListState) {
    let mut items: Vec<ListItem<'a>> = Vec::new();
    let mut selected: Option<usize> = None;

    // Individual accumulated items (in order, 0..shown)
    for i in 0..shown {
        let item = &frontier.accumulated[i];
        match item {
            AccumulatedItem::Child(sibling_idx) => {
                let entry = &siblings[*sibling_idx];
                let is_sel = active_target == CursorTarget::AccumulatedItem(*sibling_idx);
                if is_sel {
                    selected = Some(items.len());
                }
                let glyph = match entry.status {
                    TensionStatus::Resolved => "\u{2713}",
                    TensionStatus::Released => "~",
                    _ => "\u{25c6}",
                };
                let line = build_child_line(
                    entry, glyph, is_sel, false, 0, None, cols, w, styles,
                );
                items.push(ListItem::new(line));
            }
            AccumulatedItem::Note { text, age, .. } => {
                let is_sel = active_target == CursorTarget::NoteItem(i);
                if is_sel {
                    selected = Some(items.len());
                }
                let line = build_note_line(text, age, is_sel, cols, w, styles);
                items.push(ListItem::new(line));
            }
        }
    }

    // Summary for remaining items
    let remaining = frontier.accumulated.len() - shown;
    if remaining == 1 {
        let is_sel =
            active_target == CursorTarget::Accumulated;
        if is_sel {
            selected = Some(items.len());
        }
        match &frontier.accumulated[shown] {
            AccumulatedItem::Child(sibling_idx) => {
                let entry = &siblings[*sibling_idx];
                let glyph = status_glyph(entry.status);
                let line = build_child_line(
                    entry, glyph, is_sel, false, 0, None, cols, w, styles,
                );
                items.push(ListItem::new(line));
            }
            AccumulatedItem::Note { text, age, .. } => {
                let line = build_note_line(text, age, is_sel, cols, w, styles);
                items.push(ListItem::new(line));
            }
        }
    } else if remaining > 1 {
        let is_sel =
            active_target == CursorTarget::Accumulated;
        if is_sel {
            selected = Some(items.len());
        }

        let remaining_resolved = frontier.accumulated[shown..]
            .iter()
            .filter(|item| {
                matches!(item, AccumulatedItem::Child(idx) if siblings[*idx].status == TensionStatus::Resolved)
            })
            .count();
        let remaining_released = frontier.accumulated[shown..]
            .iter()
            .filter(|item| {
                matches!(item, AccumulatedItem::Child(idx) if siblings[*idx].status == TensionStatus::Released)
            })
            .count();
        let remaining_notes = frontier.accumulated[shown..]
            .iter()
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
        let line = build_indicator_line(&acc_text, is_sel, 0, cols, w, styles);
        items.push(ListItem::new(line));
    }

    let list = List::new(items).highlight_style(styles.selected);
    let mut state = ListState::default();
    state.select(selected);
    (list, state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Truncate a string to fit within `max` characters, adding ellipsis if needed.
pub fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else if max == 0 {
        String::new()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}\u{2026}", t)
    }
}
