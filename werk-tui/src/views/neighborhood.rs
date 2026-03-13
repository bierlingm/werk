use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_neighborhood(&self, area: &Rect, frame: &mut Frame<'_>) {
        let text = "  Neighborhood view \u{2014} coming soon. Press Esc to go back.";
        let paragraph = Paragraph::new(Text::from_spans([
            Span::styled(text, Style::new().fg(CLR_MID_GRAY)),
        ]));
        paragraph.render(*area, frame);
    }
}
