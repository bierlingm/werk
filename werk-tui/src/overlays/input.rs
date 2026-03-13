use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

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
                if let Some(overlay) = &self.input_overlay {
                    let overlay_height = 3u16;
                    let y = area.height.saturating_sub(overlay_height);
                    let overlay_area = Rect::new(0, y, area.width, overlay_height);
                    let w = area.width as usize;

                    let separator = "\u{2500}".repeat(w);
                    let prefix = "  > ";
                    let input_raw = format!("{}{}", prefix, overlay.buffer);

                    let prompt_raw = format!("  {}", overlay.prompt);

                    let cursor_x = (prefix.len() + overlay.buffer[..overlay.cursor.min(overlay.buffer.len())]
                        .chars()
                        .count()) as u16;
                    let cursor_y = y + 2;
                    frame.set_cursor_visible(true);
                    frame.set_cursor(Some((cursor_x, cursor_y)));

                    let lines = vec![
                        Line::from_spans([Span::styled(
                            separator,
                            Style::new().fg(CLR_DIM_GRAY).bg(CLR_BG_DARK),
                        )]),
                        Line::from_spans([Span::styled(
                            format!("{:<width$}", prompt_raw, width = w),
                            Style::new().fg(CLR_CYAN).bold().bg(CLR_BG_DARK),
                        )]),
                        Line::from_spans([Span::styled(
                            format!("{:<width$}", input_raw, width = w),
                            Style::new().fg(CLR_WHITE).bg(CLR_BG_DARK),
                        )]),
                    ];

                    let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
                    let paragraph =
                        Paragraph::new(Text::from_lines(lines)).style(bg_style);
                    paragraph.render(overlay_area, frame);
                }
            }
            InputMode::Confirm(_) => {
                if let Some(overlay) = &self.input_overlay {
                    let overlay_height = 2u16;
                    let y = area.height.saturating_sub(overlay_height);
                    let overlay_area = Rect::new(0, y, area.width, overlay_height);
                    let w = area.width as usize;

                    let separator = "\u{2500}".repeat(w);
                    let prompt_raw = format!("  {}", overlay.prompt);

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
                let overlay_area = Rect::new(0, y, area.width, overlay_height);
                let w = area.width as usize;

                let separator = "\u{2500}".repeat(w);
                let mut lines = vec![Line::from_spans([Span::styled(
                    separator,
                    Style::new().fg(CLR_DIM_GRAY).bg(CLR_BG_DARK),
                )])];

                if let Some(overlay) = &self.input_overlay {
                    let prompt_raw = format!("  {}", overlay.prompt);
                    lines.push(Line::from_spans([Span::styled(
                        format!("{:<width$}", prompt_raw, width = w),
                        Style::new().fg(CLR_CYAN).bold().bg(CLR_BG_DARK),
                    )]));
                }

                let scroll_offset = if state.selected >= visible_count {
                    state.selected - visible_count + 1
                } else {
                    0
                };

                for (i, (_, label)) in state
                    .candidates
                    .iter()
                    .enumerate()
                    .skip(scroll_offset)
                    .take(visible_count)
                {
                    let is_selected = i == state.selected;
                    let marker = if is_selected { ">" } else { " " };
                    let style = if is_selected {
                        Style::new().fg(CLR_WHITE).bold().bg(CLR_BG_DARK)
                    } else {
                        Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK)
                    };
                    let row_raw = format!("  {} {}", marker, truncate(label, w.saturating_sub(6)));
                    lines.push(Line::from_spans([Span::styled(
                        format!("{:<width$}", row_raw, width = w),
                        style,
                    )]));
                }

                let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
                let paragraph =
                    Paragraph::new(Text::from_lines(lines)).style(bg_style);
                paragraph.render(overlay_area, frame);
            }
            InputMode::Reflect => {}
        }
    }
}
