//! Inspector overlay — dev tool for focus graph and frontier diagnostics.
//!
//! Activated via Ctrl+Shift+I. Shows focus graph state, zone counts,
//! layout regime, and frontier stats. Signal by exception: silent by default.

use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::{Frame, PackedRgba};

use crate::app::InstrumentApp;
use crate::layout::SizeRegime;

/// Render the inspector overlay if active.
pub fn render_inspector(app: &InstrumentApp, frame: &mut Frame<'_>, area: Rect) {
    if !app.show_inspector {
        return;
    }

    let s = &app.styles;

    // Collect diagnostic lines
    let mut lines: Vec<Line> = Vec::new();

    let title_style = ftui::style::Style::new().fg(s.clr_cyan);
    let dim_style = s.dim;

    lines.push(Line::from_spans(vec![
        Span::styled(" Inspector ", title_style),
    ]));

    // Focus state
    let active_id = app.focus_state.active;
    let target = app.focus_state.cursor_target();
    let node_count = app.focus_state.selectable_count();
    lines.push(Line::from_spans(vec![
        Span::styled(format!(" Focus: id={active_id} target={target:?}"), dim_style),
    ]));
    lines.push(Line::from_spans(vec![
        Span::styled(format!(" Nodes: {node_count} selectable"), dim_style),
    ]));

    // Frontier zone counts
    let f = &app.frontier;
    lines.push(Line::from_spans(vec![
        Span::styled(
            format!(" Frontier: route={} overdue={} held={} accum={}",
                f.route.len(), f.overdue.len(), f.held.len(), f.accumulated.len()),
            dim_style,
        ),
    ]));
    lines.push(Line::from_spans(vec![
        Span::styled(
            format!(" Show: route={} held={} accum={}",
                f.show_route, f.show_held, f.show_accumulated),
            dim_style,
        ),
    ]));

    // Layout
    let regime = match app.layout.regime {
        SizeRegime::Compact => "Compact",
        SizeRegime::Standard => "Standard",
        SizeRegime::Expansive => "Expansive",
    };
    lines.push(Line::from_spans(vec![
        Span::styled(format!(" Layout: {regime} | View: {:?}", app.view_orientation), dim_style),
    ]));
    lines.push(Line::from_spans(vec![
        Span::styled(format!(" Zoom: {:?} | Mode: {:?}", app.deck_zoom, app.input_mode), dim_style),
    ]));

    // Parent
    let parent_label = match &app.parent_id {
        Some(pid) => werk_shared::display_id(
            app.parent_tension.as_ref().and_then(|t| t.short_code),
            pid,
        ),
        None => "root".to_string(),
    };
    lines.push(Line::from_spans(vec![
        Span::styled(format!(" Parent: {parent_label} | Siblings: {}", app.siblings.len()), dim_style),
    ]));

    let height = lines.len() as u16;
    let width = 50u16.min(area.width.saturating_sub(2));

    if area.width < width + 2 || area.height < height + 2 {
        return;
    }

    // Position: bottom-right corner
    let x = area.x + area.width - width - 1;
    let y = area.y + area.height - height - 1;
    let overlay_area = Rect::new(x, y, width, height);

    // Clear background
    let bg_cell = ftui::Cell::from_char(' ')
        .with_fg(s.clr_dim)
        .with_bg(PackedRgba::BLACK);
    frame.buffer.fill(overlay_area, bg_cell);

    // Render text
    let text = Text::from_lines(lines);
    Paragraph::new(text).render(overlay_area, frame);
}
