use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::StatefulWidget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::list::{List, ListItem, ListState};

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_welcome_screen(&self, area: Rect, frame: &mut Frame<'_>) {
        // Render the header text above the list
        let header_lines: Vec<Line> = vec![
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
        let header_h = header_lines.len() as u16;
        let header_area = Rect::new(area.x, area.y, area.width, header_h.min(area.height));
        Paragraph::new(Text::from_lines(header_lines)).render(header_area, frame);

        // Render options as a List widget
        let list_y = area.y + header_h;
        let list_h = area.height.saturating_sub(header_h + 2); // reserve space for footer
        if list_h > 0 {
            let list_area = Rect::new(area.x, list_y, area.width, list_h);

            let options = [
                ("Create workspace here (.werk/)", "Local to this directory"),
                ("Create globally (~/.werk/)", "Shared across all directories"),
            ];

            let items: Vec<ListItem> = options.iter().map(|(label, desc)| {
                let text = Text::from_spans([
                    Span::styled(*label, Style::new().fg(CLR_LIGHT_GRAY)),
                    Span::styled(format!("  {}", desc), Style::new().fg(CLR_DIM_GRAY)),
                ]);
                ListItem::new(text).style(Style::new().fg(CLR_LIGHT_GRAY))
            }).collect();

            let list = List::new(items)
                .highlight_style(Style::new().fg(CLR_WHITE).bold())
                .highlight_symbol("  > ");

            let mut state = ListState::default();
            state.select(Some(self.welcome_selected));
            StatefulWidget::render(&list, list_area, frame, &mut state);
        }

        // Footer hints
        let footer_y = list_y + list_h;
        if footer_y < area.bottom() {
            let footer_area = Rect::new(area.x, footer_y, area.width, area.bottom().saturating_sub(footer_y));
            Paragraph::new(Text::from_spans([Span::styled(
                "  j/k to select, Enter to confirm, q to quit",
                Style::new().fg(CLR_DIM_GRAY),
            )])).render(footer_area, frame);
        }
    }
}
