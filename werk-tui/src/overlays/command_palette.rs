use ftui::Frame;
use ftui::layout::Rect;
use ftui::widgets::Widget;

use crate::app::WerkApp;

impl WerkApp {
    pub(crate) fn render_command_palette(&self, area: Rect, frame: &mut Frame<'_>) {
        if !self.command_palette.is_visible() {
            return;
        }
        // The native CommandPalette widget handles its own layout, styling,
        // fuzzy filtering, selection highlight, and match highlighting.
        self.command_palette.render(area, frame);
    }
}
