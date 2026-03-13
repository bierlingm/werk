use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use crate::app::WerkApp;
use crate::input::{InputMode, View};
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_help_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        let help_lines = self.context_help_lines();
        let line_count = help_lines.len() as u16;
        let help_width = 62u16.min(area.width.saturating_sub(4));
        let help_height = (line_count + 2).min(area.height.saturating_sub(4));
        let x = (area.width.saturating_sub(help_width)) / 2;
        let y = (area.height.saturating_sub(help_height)) / 2;
        let help_area = Rect::new(x, y, help_width, help_height);

        let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
        let paragraph = Paragraph::new(Text::from_lines(help_lines)).style(bg_style);
        paragraph.render(help_area, frame);
    }

    pub(crate) fn context_help_lines(&self) -> Vec<Line> {
        let mut lines = vec![
            Line::from_spans([Span::styled(
                " werk \u{2014} structural dynamics TUI",
                Style::new().bold(),
            )]),
            Line::from(""),
        ];

        if !matches!(self.input_mode, InputMode::Normal) {
            lines.push(Line::from_spans([Span::styled("  Input Mode", Style::new().fg(CLR_CYAN).bold())]));
            lines.push(Line::from("  Enter       Submit input"));
            lines.push(Line::from("  Esc         Cancel"));
            lines.push(Line::from("  Left/Right  Move cursor"));
            lines.push(Line::from("  Home/End    Jump to start/end"));
            lines.push(Line::from("  Backspace   Delete before cursor"));
            lines.push(Line::from(""));
            lines.push(Line::from("  q / Ctrl+C  Quit          ?  Toggle this help"));
            return lines;
        }

        match &self.active_view {
            View::Welcome => {
                lines.push(Line::from("  j/k         Select option"));
                lines.push(Line::from("  Enter       Confirm selection"));
                lines.push(Line::from("  q           Quit"));
            }
            View::Dashboard => {
                lines.push(Line::from_spans([Span::styled("  Dashboard", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  j/k         Move up/down"));
                lines.push(Line::from("  Enter       Open detail view"));
                lines.push(Line::from("  Esc         (no-op at top level)"));
                lines.push(Line::from("  1           Dashboard     2/t  Tree view"));
                lines.push(Line::from("  f           Cycle filter   v   Toggle verbose"));
                lines.push(Line::from("  /           Search tensions"));
                lines.push(Line::from("  :           Command palette"));
                lines.push(Line::from("  L           Show lever detail"));
                lines.push(Line::from(""));
                lines.push(Line::from_spans([Span::styled("  Editing", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  a           Add new tension"));
                lines.push(Line::from("  c           Create child of selected"));
                lines.push(Line::from("  p           Create parent of selected"));
                lines.push(Line::from("  r           Update reality (actual state)"));
                lines.push(Line::from("  d           Update desire"));
                lines.push(Line::from("  n           Add note"));
                lines.push(Line::from("  h           Set horizon"));
                lines.push(Line::from("  R           Resolve tension"));
                lines.push(Line::from("  X           Release tension"));
                lines.push(Line::from("  m           Move/reparent tension"));
            }
            View::Detail => {
                lines.push(Line::from_spans([Span::styled("  Detail View", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  j/k         Scroll up/down"));
                lines.push(Line::from("  Esc         Back to dashboard"));
                lines.push(Line::from("  v           Toggle verbose dynamics"));
                lines.push(Line::from("  /           Search tensions"));
                lines.push(Line::from("  :           Command palette"));
                lines.push(Line::from(""));
                lines.push(Line::from_spans([Span::styled("  Editing", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  r           Update reality"));
                lines.push(Line::from("  d           Update desire"));
                lines.push(Line::from("  n           Add note"));
                lines.push(Line::from("  h           Set horizon"));
                lines.push(Line::from("  a           Add sub-tension"));
                lines.push(Line::from("  c           Create child of current"));
                lines.push(Line::from("  p           Create parent of current"));
                lines.push(Line::from("  R           Resolve"));
                lines.push(Line::from("  X           Release"));
                lines.push(Line::from("  Del         Delete tension"));
                lines.push(Line::from("  m           Move/reparent"));
                lines.push(Line::from("  g           Open agent"));
            }
            View::TreeView => {
                lines.push(Line::from_spans([Span::styled("  Tree View", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  j/k         Navigate tree"));
                lines.push(Line::from("  Enter       Open detail view"));
                lines.push(Line::from("  Esc/1       Back to dashboard"));
                lines.push(Line::from("  f           Cycle filter"));
                lines.push(Line::from("  /           Search tensions"));
                lines.push(Line::from("  :           Command palette"));
                lines.push(Line::from(""));
                lines.push(Line::from_spans([Span::styled("  Editing", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  a           Add tension"));
                lines.push(Line::from("  c           Create child of selected"));
                lines.push(Line::from("  p           Create parent of selected"));
                lines.push(Line::from("  r/d/n/h     Edit selected tension"));
                lines.push(Line::from("  R/X/m       Resolve/Release/Move"));
            }
            View::Neighborhood => {
                lines.push(Line::from_spans([Span::styled("  Neighborhood View", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  Esc         Back to dashboard"));
                lines.push(Line::from("  :           Command palette"));
            }
            View::Timeline => {
                lines.push(Line::from_spans([Span::styled("  Timeline View", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  Esc         Back to dashboard"));
                lines.push(Line::from("  1           Dashboard"));
                lines.push(Line::from("  :           Command palette"));
            }
            View::Focus => {
                lines.push(Line::from_spans([Span::styled("  Focus Mode", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  j/k         Cycle through tensions"));
                lines.push(Line::from("  r           Update reality"));
                lines.push(Line::from("  d           Update desire"));
                lines.push(Line::from("  h           Set horizon"));
                lines.push(Line::from("  Esc         Back to dashboard"));
            }
            View::DynamicsSummary => {
                lines.push(Line::from_spans([Span::styled("  Dynamics Summary", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  Esc         Back to dashboard"));
                lines.push(Line::from("  :           Command palette"));
            }
            View::Agent(_) => {
                lines.push(Line::from_spans([Span::styled("  Agent View", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  j/k         Navigate mutations"));
                lines.push(Line::from("  Enter       Toggle mutation selection"));
                lines.push(Line::from("  1-9         Toggle mutation by number"));
                lines.push(Line::from("  a           Apply selected mutations"));
                lines.push(Line::from("  Esc         Back to detail view"));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from("  q / Ctrl+C  Quit          ?  Toggle this help"));
        lines
    }
}
