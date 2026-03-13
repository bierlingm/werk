use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_lever_detail_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        let lever = match &self.lever {
            Some(l) => l,
            None => {
                // No lever — show a short message
                let msg = " No active tensions to compute lever. ";
                let w = (msg.len() as u16 + 4).min(area.width.saturating_sub(4));
                let h = 3u16.min(area.height.saturating_sub(4));
                let x = (area.width.saturating_sub(w)) / 2;
                let y = (area.height.saturating_sub(h)) / 2;
                let r = Rect::new(x, y, w, h);
                let style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
                Paragraph::new(Text::from(msg)).style(style).render(r, frame);
                return;
            }
        };

        // Build the overlay lines
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from_spans([Span::styled(
            " Lever Detail",
            Style::new().fg(CLR_CYAN).bold(),
        )]));
        lines.push(Line::from(""));

        // Tension info
        lines.push(Line::from_spans([
            Span::styled("  Tension: ", Style::new().fg(CLR_MID_GRAY)),
            Span::styled(
                truncate(&lever.tension_desired, 50),
                Style::new().fg(CLR_WHITE),
            ),
        ]));
        lines.push(Line::from_spans([
            Span::styled("  Action:  ", Style::new().fg(CLR_MID_GRAY)),
            Span::styled(lever.action.label(), Style::new().fg(CLR_CYAN).bold()),
        ]));
        lines.push(Line::from_spans([
            Span::styled("  Score:   ", Style::new().fg(CLR_MID_GRAY)),
            Span::styled(
                format!("{:.1}%", lever.score * 100.0),
                Style::new().fg(CLR_YELLOW),
            ),
        ]));
        lines.push(Line::from(""));

        // Reasoning
        lines.push(Line::from_spans([
            Span::styled("  ", Style::new()),
            Span::styled(&lever.reasoning, Style::new().fg(CLR_LIGHT_GRAY).italic()),
        ]));
        lines.push(Line::from(""));

        // Breakdown header
        lines.push(Line::from_spans([Span::styled(
            "  Score Breakdown",
            Style::new().fg(CLR_CYAN).bold(),
        )]));

        let b = &lever.breakdown;
        let components = [
            ("  Temporal pressure", b.temporal_pressure, 0.15),
            ("  Gap magnitude    ", b.gap_magnitude, 0.15),
            ("  Combined pressure", b.combined_pressure, 0.10),
            ("  Stuck energy     ", b.stuck_energy, 0.10),
            ("  Sibling imbalance", b.sibling_imbalance, 0.10),
            ("  Workaround dur.  ", b.workaround_duration, 0.05),
            ("  Stalled potential", b.stalled_potential, 0.10),
            ("  Cascade potential", b.cascade_potential, 0.10),
            ("  Falling behind   ", b.falling_behind, 0.05),
            ("  Systemic blocker ", b.systemic_blocker, 0.05),
            ("  Horizon integrity", b.horizon_integrity, 0.05),
        ];

        for (label, value, weight) in &components {
            let bar_len = (*value * 10.0).round() as usize;
            let bar: String = "\u{2588}".repeat(bar_len);
            let empty: String = "\u{2591}".repeat(10 - bar_len);
            let color = if *value > 0.7 {
                CLR_RED_SOFT
            } else if *value > 0.3 {
                CLR_YELLOW
            } else {
                CLR_GREEN
            };
            lines.push(Line::from_spans([
                Span::styled(*label, Style::new().fg(CLR_MID_GRAY)),
                Span::styled(
                    format!(" ({:.0}%) ", weight * 100.0),
                    Style::new().fg(CLR_DIM_GRAY),
                ),
                Span::styled(bar, Style::new().fg(color)),
                Span::styled(empty, Style::new().fg(CLR_DIM_GRAY)),
                Span::styled(format!(" {:.0}%", value * 100.0), Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
        }

        // Cascade info
        if !lever.cascade_tensions.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from_spans([Span::styled(
                format!("  Cascade ({} downstream)", lever.cascade_count),
                Style::new().fg(CLR_CYAN).bold(),
            )]));
            for (_, desired) in lever.cascade_tensions.iter().take(5) {
                lines.push(Line::from_spans([
                    Span::styled("    \u{2514} ", Style::new().fg(CLR_DIM_GRAY)),
                    Span::styled(
                        truncate(desired, 45),
                        Style::new().fg(CLR_LIGHT_GRAY),
                    ),
                ]));
            }
            if lever.cascade_tensions.len() > 5 {
                lines.push(Line::from_spans([Span::styled(
                    format!("    ... and {} more", lever.cascade_tensions.len() - 5),
                    Style::new().fg(CLR_DIM_GRAY),
                )]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from_spans([Span::styled(
            "  Press L or Esc to close",
            Style::new().fg(CLR_DIM_GRAY),
        )]));

        // Size and position
        let line_count = lines.len() as u16;
        let overlay_width = 65u16.min(area.width.saturating_sub(4));
        let overlay_height = (line_count + 2).min(area.height.saturating_sub(2));
        let x = (area.width.saturating_sub(overlay_width)) / 2;
        let y = (area.height.saturating_sub(overlay_height)) / 2;
        let overlay_area = Rect::new(x, y, overlay_width, overlay_height);

        // Pad lines to full width to create solid background
        let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
        let paragraph = Paragraph::new(Text::from_lines(lines)).style(bg_style);
        paragraph.render(overlay_area, frame);
    }
}
