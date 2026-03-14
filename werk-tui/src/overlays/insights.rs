use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::Text;
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::modal::{Modal, ModalPosition, ModalSizeConstraints};

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_insights_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        let line_count = self.insights_lines.len() as u16;

        let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
        let content = Paragraph::new(Text::from_lines(self.insights_lines.clone())).style(bg_style);
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
