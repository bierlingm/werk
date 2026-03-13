use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::block::Block;
use ftui::widgets::borders::BorderType;
use chrono::Utc;
use crate::app::WerkApp;
use crate::theme::*;
use crate::helpers::render_bar;

impl WerkApp {
    pub(crate) fn render_focus(&self, area: Rect, frame: &mut Frame<'_>) {
        let Some(tension) = &self.detail_tension else {
            Paragraph::new("  No tension selected. Press Esc to go back.")
                .render(area, frame);
            return;
        };

        let block = Block::bordered()
            .title(" Focus ")
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(CLR_CYAN));
        let inner = block.inner(area);
        block.render(area, frame);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        // DESIRED
        lines.push(Line::from_spans([
            Span::styled("  DESIRED", Style::new().fg(CLR_MID_GRAY).bold()),
        ]));
        lines.push(Line::from_spans([
            Span::styled(format!("  {}", &tension.desired), Style::new().fg(CLR_WHITE)),
        ]));
        lines.push(Line::from(""));

        // Gap visualization
        if let Some(dyn_display) = &self.detail_dynamics {
            if let Some(mag) = dyn_display.magnitude {
                let bar = render_bar(mag, 20);
                let gap_line = format!("  {} gap (magnitude: {:.2}) {}", "\u{2500}".repeat(3), mag, "\u{2500}".repeat(3));
                lines.push(Line::from_spans([
                    Span::styled(gap_line, Style::new().fg(CLR_DIM_GRAY)),
                ]));
                lines.push(Line::from_spans([
                    Span::styled(format!("  {}", bar), Style::new().fg(CLR_CYAN)),
                ]));
            }
        }
        lines.push(Line::from(""));

        // ACTUAL
        lines.push(Line::from_spans([
            Span::styled("  ACTUAL", Style::new().fg(CLR_MID_GRAY).bold()),
        ]));
        lines.push(Line::from_spans([
            Span::styled(format!("  {}", &tension.actual), Style::new().fg(CLR_LIGHT_GRAY)),
        ]));
        lines.push(Line::from(""));

        // HORIZON
        let now = Utc::now();
        let horizon_str = match &tension.horizon {
            Some(h) => {
                let remaining = h.range_end().signed_duration_since(now).num_days();
                if remaining < 0 {
                    format!("{} ({}d past)", h, -remaining)
                } else if remaining == 0 {
                    format!("{} (today)", h)
                } else {
                    format!("{} ({}d remaining)", h, remaining)
                }
            }
            None => "\u{2014} no horizon set".to_string(),
        };
        let horizon_line = format!("  {} horizon: {} {}", "\u{2500}".repeat(3), horizon_str, "\u{2500}".repeat(3));
        lines.push(Line::from_spans([
            Span::styled(horizon_line, Style::new().fg(CLR_DIM_GRAY)),
        ]));
        lines.push(Line::from(""));

        // NEXT ACTION (from Lever if available, else heuristic)
        lines.push(Line::from_spans([
            Span::styled("  NEXT ACTION", Style::new().fg(CLR_MID_GRAY).bold()),
        ]));
        let action_text = if let Some(ref lever) = self.lever {
            if lever.tension_id == tension.id {
                format!("  {} \u{2014} {}", lever.action.label(), lever.reasoning)
            } else {
                "  Update reality \u{2014} check current progress".to_string()
            }
        } else {
            "  Update reality \u{2014} check current progress".to_string()
        };
        lines.push(Line::from_spans([
            Span::styled(action_text, Style::new().fg(CLR_CYAN)),
        ]));
        lines.push(Line::from(""));

        // Action keys
        lines.push(Line::from_spans([
            Span::styled("  r: update reality   d: update desire   h: set horizon   Esc: back   j/k: cycle", Style::new().fg(CLR_DIM_GRAY)),
        ]));

        let text = Text::from_lines(lines);
        Paragraph::new(text).render(inner, frame);
    }
}
