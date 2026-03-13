use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::block::Block;
use ftui::widgets::borders::BorderType;
use ftui::widgets::paragraph::Paragraph;
use werk_shared::truncate;
use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_reflect_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        let Some(ref tension) = self.detail_tension else { return };
        let Some(ref textarea) = self.reflect_textarea else { return };

        // Centered overlay: 80% width, 60% height
        let w = (area.width as f64 * 0.8) as u16;
        let h = (area.height as f64 * 0.6) as u16;
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let overlay_area = Rect::new(x, y, w, h);

        let title = format!(" Reflect: {} ", truncate(&tension.desired, w.saturating_sub(14) as usize));
        let block = Block::bordered()
            .title(title.as_str())
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(CLR_YELLOW));
        let inner = block.inner(overlay_area);
        block.render(overlay_area, frame);

        // Help line at bottom
        let help_y = inner.bottom().saturating_sub(1);
        if help_y > inner.y {
            let content_area = Rect::new(inner.x, inner.y, inner.width, inner.height.saturating_sub(1));
            // Render the native TextArea widget
            textarea.render(content_area, frame);

            let help_area = Rect::new(inner.x, help_y, inner.width, 1);
            Paragraph::new(Line::from_spans([
                Span::styled(" Ctrl+S submit  Esc cancel  Enter newline", Style::new().fg(CLR_DIM_GRAY)),
            ])).render(help_area, frame);
        } else {
            textarea.render(inner, frame);
        }
    }
}
