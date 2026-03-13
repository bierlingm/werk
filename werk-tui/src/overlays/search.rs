use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_search_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        let overlay_area = Rect::new(0, 0, area.width, 1);
        let search_display = format!(
            " / {}\u{2588}",
            self.search_buffer,
        );
        let style = Style::new().fg(CLR_CYAN).bold().bg(CLR_BG_DARK);
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(&search_display, style)]));
        paragraph.render(overlay_area, frame);
    }
}
