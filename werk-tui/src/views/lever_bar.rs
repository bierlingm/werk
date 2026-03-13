use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_lever_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        if let Some(ref lever) = self.lever {
            let text = format!(
                " \u{25B6} Lever: {} on \"{}\"",
                lever.action.label(),
                truncate(&lever.tension_desired, area.width.saturating_sub(30) as usize),
            );
            let style = Style::new().fg(CLR_CYAN);
            Paragraph::new(Text::from_spans([Span::styled(&text, style)])).render(*area, frame);
        }
    }
}
