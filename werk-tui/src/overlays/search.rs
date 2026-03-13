use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::input::TextInput;
use ftui::widgets::paragraph::Paragraph;

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_search_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        // Render " / " prefix
        let prefix_area = Rect::new(0, 0, 3, 1);
        let prefix = Paragraph::new(Text::from_spans([Span::styled(
            " / ",
            Style::new().fg(CLR_CYAN).bold().bg(CLR_BG_DARK),
        )]));
        prefix.render(prefix_area, frame);

        // Render TextInput widget for the search field
        let input_area = Rect::new(3, 0, area.width.saturating_sub(3), 1);
        let input_widget = TextInput::new()
            .with_value(self.search_input_widget.value())
            .with_style(Style::new().fg(CLR_CYAN).bold().bg(CLR_BG_DARK))
            .with_cursor_style(Style::new().fg(CLR_CYAN).bg(CLR_BG_DARK).reverse())
            .with_focused(true);
        input_widget.render(input_area, frame);
    }
}
