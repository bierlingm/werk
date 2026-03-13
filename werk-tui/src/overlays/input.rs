use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::StatefulWidget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::input::TextInput;
use ftui::widgets::list::{List, ListItem};
use werk_shared::truncate;

use crate::app::WerkApp;
use crate::input::InputMode;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_input_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        match &self.input_mode {
            InputMode::Normal => {
                if let Some(toast) = &self.status_toast {
                    let toast_area = Rect::new(0, area.height.saturating_sub(1), area.width, 1);
                    let style = Style::new().fg(CLR_YELLOW).bold();
                    let paragraph =
                        Paragraph::new(Text::from_spans([Span::styled(
                            format!(" {} ", toast),
                            style,
                        )]));
                    paragraph.render(toast_area, frame);
                }
            }
            InputMode::TextInput(_) => {
                if self.input_overlay.is_some() {
                    let overlay_height = 3u16;
                    let y = area.height.saturating_sub(overlay_height);
                    let w = area.width as usize;

                    let separator = "\u{2500}".repeat(w);
                    let prompt_raw = format!("  {}", self.input_overlay.as_ref().unwrap().prompt);

                    // Render separator and prompt lines
                    let separator_area = Rect::new(0, y, area.width, 1);
                    let sep_paragraph = Paragraph::new(Text::from_spans([Span::styled(
                        separator,
                        Style::new().fg(CLR_DIM_GRAY).bg(CLR_BG_DARK),
                    )]));
                    sep_paragraph.render(separator_area, frame);

                    let prompt_area = Rect::new(0, y + 1, area.width, 1);
                    let prompt_paragraph = Paragraph::new(Text::from_spans([Span::styled(
                        format!("{:<width$}", prompt_raw, width = w),
                        Style::new().fg(CLR_CYAN).bold().bg(CLR_BG_DARK),
                    )]));
                    prompt_paragraph.render(prompt_area, frame);

                    // Render TextInput widget on the input line
                    let input_area = Rect::new(2, y + 2, area.width.saturating_sub(2), 1);

                    // Fill the input line background
                    let input_bg_area = Rect::new(0, y + 2, area.width, 1);
                    let bg_fill = Paragraph::new(Text::from_spans([Span::styled(
                        " ".repeat(w),
                        Style::new().bg(CLR_BG_DARK),
                    )]));
                    bg_fill.render(input_bg_area, frame);

                    // Render the "> " prefix
                    let prefix_paragraph = Paragraph::new(Text::from_spans([Span::styled(
                        "> ",
                        Style::new().fg(CLR_WHITE).bg(CLR_BG_DARK),
                    )]));
                    let prefix_area = Rect::new(0, y + 2, 2, 1);
                    prefix_paragraph.render(prefix_area, frame);

                    let input_widget = TextInput::new()
                        .with_value(self.text_input_widget.value())
                        .with_style(Style::new().fg(CLR_WHITE).bg(CLR_BG_DARK))
                        .with_cursor_style(Style::new().fg(CLR_CYAN).bg(CLR_BG_DARK).reverse())
                        .with_focused(true);
                    input_widget.render(input_area, frame);
                }
            }
            InputMode::Confirm(_) => {
                if self.input_overlay.is_some() {
                    let overlay_height = 2u16;
                    let y = area.height.saturating_sub(overlay_height);
                    let overlay_area = Rect::new(0, y, area.width, overlay_height);
                    let w = area.width as usize;

                    let separator = "\u{2500}".repeat(w);
                    let prompt_raw = format!("  {}", self.input_overlay.as_ref().unwrap().prompt);

                    let lines = vec![
                        Line::from_spans([Span::styled(
                            separator,
                            Style::new().fg(CLR_DIM_GRAY).bg(CLR_BG_DARK),
                        )]),
                        Line::from_spans([Span::styled(
                            format!("{:<width$}", prompt_raw, width = w),
                            Style::new().fg(CLR_YELLOW).bold().bg(CLR_BG_DARK),
                        )]),
                    ];

                    let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
                    let paragraph =
                        Paragraph::new(Text::from_lines(lines)).style(bg_style);
                    paragraph.render(overlay_area, frame);
                }
            }
            InputMode::MovePicker(state) => {
                let visible_count = state.candidates.len().min(10);
                let overlay_height = (visible_count as u16) + 2;
                let y = area.height.saturating_sub(overlay_height);
                let w = area.width as usize;

                // Render separator
                let separator = "\u{2500}".repeat(w);
                let sep_area = Rect::new(0, y, area.width, 1);
                let sep_paragraph = Paragraph::new(Text::from_spans([Span::styled(
                    separator,
                    Style::new().fg(CLR_DIM_GRAY).bg(CLR_BG_DARK),
                )]));
                sep_paragraph.render(sep_area, frame);

                // Render prompt
                if let Some(overlay) = &self.input_overlay {
                    let prompt_raw = format!("  {}", overlay.prompt);
                    let prompt_area = Rect::new(0, y + 1, area.width, 1);
                    let prompt_paragraph = Paragraph::new(Text::from_spans([Span::styled(
                        format!("{:<width$}", prompt_raw, width = w),
                        Style::new().fg(CLR_CYAN).bold().bg(CLR_BG_DARK),
                    )]));
                    prompt_paragraph.render(prompt_area, frame);
                }

                // Build list items from candidates
                let items: Vec<ListItem<'_>> = state
                    .candidates
                    .iter()
                    .map(|(_, label)| {
                        ListItem::new(format!("  {}", truncate(label, w.saturating_sub(6))))
                            .style(Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK))
                    })
                    .collect();

                let list = List::new(items)
                    .style(Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK))
                    .highlight_style(Style::new().fg(CLR_WHITE).bold().bg(CLR_BG_DARK))
                    .highlight_symbol("> ");

                let list_area = Rect::new(0, y + 2, area.width, visible_count as u16);

                // Use the move_picker_state from self (via RefCell for interior mutability)
                let mut list_state = self.move_picker_state.borrow_mut();
                StatefulWidget::render(&list, list_area, frame, &mut list_state);
            }
            InputMode::Reflect => {}
        }
    }
}
