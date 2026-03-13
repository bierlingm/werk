use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::theme::*;
use crate::types::MAX_VISIBLE_TOASTS;

impl WerkApp {
    pub(crate) fn render_toasts(&self, area: Rect, frame: &mut Frame<'_>) {
        if self.toasts.is_empty() {
            return;
        }

        let visible_toasts: Vec<_> = self
            .toasts
            .iter()
            .rev()
            .take(MAX_VISIBLE_TOASTS)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        for (i, toast) in visible_toasts.iter().enumerate() {
            let toast_width = (toast.message.len() as u16 + 4).min(area.width.saturating_sub(2));
            let x = area.width.saturating_sub(toast_width + 1);
            let y = 1 + (i as u16);

            if y >= area.height.saturating_sub(2) {
                break;
            }

            let toast_area = Rect::new(x, y, toast_width, 1);
            let border_color = toast.color();
            let content = format!(
                " {} ",
                truncate(&toast.message, toast_width.saturating_sub(2) as usize)
            );

            let style = Style::new().fg(border_color).bg(CLR_BG_DARK).bold();
            let paragraph = Paragraph::new(Text::from_spans([Span::styled(&content, style)]));
            paragraph.render(toast_area, frame);
        }
    }
}
