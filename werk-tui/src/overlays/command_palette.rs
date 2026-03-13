use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_command_palette(&self, area: Rect, frame: &mut Frame<'_>) {
        let palette = match &self.command_palette {
            Some(p) => p,
            None => return,
        };

        let filtered = palette.filtered_actions();
        let visible_count = filtered.len().min(14);
        let overlay_height = (visible_count as u16) + 3;
        let overlay_width = 50u16.min(area.width.saturating_sub(4));
        let x = (area.width.saturating_sub(overlay_width)) / 2;
        let y = 2u16.min(area.height.saturating_sub(overlay_height));
        let overlay_area = Rect::new(x, y, overlay_width, overlay_height);

        let separator = "\u{2500}".repeat(overlay_width as usize);
        let mut lines = vec![
            Line::from_spans([Span::styled(
                &separator,
                Style::new().fg(CLR_DIM_GRAY),
            )]),
            Line::from_spans([Span::styled(
                format!("  : {}\u{2588}", palette.query),
                Style::new().fg(CLR_CYAN).bold(),
            )]),
        ];

        for (i, action) in filtered.iter().enumerate().take(visible_count) {
            let is_selected = i == palette.selected;
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
                Span::styled(
                    format!("{:<12}", action.name),
                    style,
                ),
                Span::styled(action.description, desc_style),
            ]));
        }

        lines.push(Line::from_spans([Span::styled(
            &separator,
            Style::new().fg(CLR_DIM_GRAY),
        )]));

        let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
        let paragraph = Paragraph::new(Text::from_lines(lines)).style(bg_style);
        paragraph.render(overlay_area, frame);
    }
}
