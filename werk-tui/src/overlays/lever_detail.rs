use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::modal::{Modal, ModalPosition, ModalSizeConstraints};
use ftui::widgets::progress::MiniBar;

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_lever_detail_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        let lever = match &self.lever {
            Some(l) => l,
            None => {
                // No lever — show a short message in a Modal
                let msg = Paragraph::new(Text::from(" No active tensions to compute lever. "))
                    .style(Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK));
                let modal = Modal::new(msg)
                    .position(ModalPosition::Center)
                    .size(ModalSizeConstraints::new().max_width(45).max_height(3));
                modal.render(area, frame);
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

        // Build breakdown lines using MiniBar::render_string() for inline bar text
        for (label, value, weight) in &components {
            let bar = MiniBar::new(*value, 10);
            let bar_str = bar.render_string();
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
                Span::styled(bar_str, Style::new().fg(color)),
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

        let line_count = lines.len() as u16;

        // Wrap content in a Modal for proper overlay positioning and backdrop
        let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
        let content = Paragraph::new(Text::from_lines(lines)).style(bg_style);
        let modal = Modal::new(content)
            .position(ModalPosition::Center)
            .size(
                ModalSizeConstraints::new()
                    .max_width(65)
                    .max_height(line_count.saturating_add(2)),
            );
        modal.render(area, frame);
    }
}
