use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::status_line::{StatusLine, StatusItem};
use werk_shared::truncate;
use crate::app::WerkApp;
use crate::theme::*;
use crate::types::UrgencyTier;

impl WerkApp {
    pub(crate) fn render_timeline_title(&self, area: &Rect, frame: &mut Frame<'_>) {
        let count = self.tensions.iter()
            .filter(|t| t.tier != UrgencyTier::Resolved && !t.horizon_display.contains('\u{2014}'))
            .count();
        let left_text = format!(" Timeline  |  {} tensions with horizons", count);
        let status = StatusLine::new()
            .left(StatusItem::text(&left_text))
            .style(Style::new().fg(CLR_LIGHT_GRAY).bold());
        status.render(*area, frame);
    }

    pub(crate) fn render_timeline_body(&self, area: &Rect, frame: &mut Frame<'_>) {
        let mut lines: Vec<Line> = Vec::new();

        // Filter tensions that have horizons and are not resolved
        let mut timeline_tensions: Vec<_> = self.tensions.iter()
            .filter(|t| t.tier != UrgencyTier::Resolved && !t.horizon_display.contains('\u{2014}'))
            .collect();

        if timeline_tensions.is_empty() {
            lines.push(Line::from_spans([Span::styled(
                "  No tensions with horizons. Set horizons with 'h' in detail view.",
                Style::new().fg(CLR_MID_GRAY),
            )]));
            let text = Text::from_lines(lines);
            Paragraph::new(text).render(*area, frame);
            return;
        }

        // Sort by urgency (most urgent first)
        timeline_tensions.sort_by(|a, b| {
            b.urgency.unwrap_or(0.0).partial_cmp(&a.urgency.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Build a simple text-based timeline
        // Label column: first 20 chars of desired
        // Bar column: remaining width
        let label_w = 22;
        let bar_w = (area.width as usize).saturating_sub(label_w + 4);

        // Header with time markers
        let header_label = format!("{:>label_w$}", "");
        let today_marker = format!("{:<bar_w$}", "today");
        lines.push(Line::from_spans([
            Span::styled(header_label, Style::new().fg(CLR_DIM_GRAY)),
            Span::styled("  | ", Style::new().fg(CLR_DIM_GRAY)),
            Span::styled(today_marker, Style::new().fg(CLR_DIM_GRAY)),
        ]));

        for t in timeline_tensions.iter().take(area.height.saturating_sub(2) as usize) {
            let label = format!("  {:<width$}", truncate(&t.desired, label_w - 2), width = label_w - 2);

            // Build a simple bar based on urgency
            let urgency = t.urgency.unwrap_or(0.0);
            let bar_total = bar_w.saturating_sub(t.horizon_display.len() + 2);
            let bar_filled = ((1.0 - urgency) * bar_total as f64).round().max(1.0) as usize;
            let bar_remaining = bar_total.saturating_sub(bar_filled);

            let tier_color = tier_color(t.tier);

            let bar = format!(
                "{}{}  {}",
                "\u{2588}".repeat(bar_filled.min(bar_total)),
                "\u{2591}".repeat(bar_remaining),
                t.horizon_display,
            );

            lines.push(Line::from_spans([
                Span::styled(label, Style::new().fg(CLR_LIGHT_GRAY)),
                Span::styled("  | ", Style::new().fg(CLR_DIM_GRAY)),
                Span::styled(bar, Style::new().fg(tier_color)),
            ]));
        }

        let text = Text::from_lines(lines);
        Paragraph::new(text).render(*area, frame);
    }

    pub(crate) fn render_timeline_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = StatusLine::new()
            .separator("  ")
            .left(StatusItem::key_hint("Esc", "back"))
            .left(StatusItem::key_hint("1", "dashboard"))
            .left(StatusItem::key_hint("q", "quit"))
            .left(StatusItem::key_hint("?", "help"))
            .style(Style::new().fg(CLR_MID_GRAY));
        hints.render(*area, frame);
    }
}
