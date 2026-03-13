use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_welcome_screen(&self, area: Rect, frame: &mut Frame<'_>) {
        let mut lines: Vec<Line> = vec![
            Line::from(""),
            Line::from(""),
            Line::from_spans([Span::styled(
                "  Welcome to werk",
                Style::new().fg(CLR_CYAN).bold(),
            )]),
            Line::from(""),
            Line::from_spans([Span::styled(
                "  werk is a structural dynamics tool for managing creative tensions.",
                Style::new().fg(CLR_LIGHT_GRAY),
            )]),
            Line::from_spans([Span::styled(
                "  No workspace was found. Where would you like to create one?",
                Style::new().fg(CLR_LIGHT_GRAY),
            )]),
            Line::from(""),
        ];

        let options = [
            ("Create workspace here (.werk/)", "Local to this directory"),
            ("Create globally (~/.werk/)", "Shared across all directories"),
        ];

        for (i, (label, desc)) in options.iter().enumerate() {
            let is_selected = i == self.welcome_selected;
            let marker = if is_selected { ">" } else { " " };
            let style = if is_selected {
                Style::new().fg(CLR_WHITE).bold()
            } else {
                Style::new().fg(CLR_LIGHT_GRAY)
            };
            let desc_style = if is_selected {
                Style::new().fg(CLR_MID_GRAY)
            } else {
                Style::new().fg(CLR_DIM_GRAY)
            };
            lines.push(Line::from_spans([
                Span::styled(format!("  {} ", marker), style),
                Span::styled(*label, style),
                Span::styled(format!("  {}", desc), desc_style),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from_spans([Span::styled(
            "  j/k to select, Enter to confirm, q to quit",
            Style::new().fg(CLR_DIM_GRAY),
        )]));

        let text = Text::from_lines(lines);
        let paragraph = Paragraph::new(text);
        paragraph.render(area, frame);
    }
}
