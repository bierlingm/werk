use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::modal::{Modal, ModalPosition, ModalSizeConstraints};

use crate::app::WerkApp;
use crate::input::{InputMode, View};
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_help_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        let help_lines = self.context_help_lines();
        let line_count = help_lines.len() as u16;

        let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
        let content = Paragraph::new(Text::from_lines(help_lines)).style(bg_style);

        let modal = Modal::new(content)
            .position(ModalPosition::Center)
            .size(
                ModalSizeConstraints::new()
                    .max_width(62)
                    .max_height(line_count.saturating_add(2)),
            );
        modal.render(area, frame);
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
                lines.push(Line::from("  Tab         Switch view (Dashboard/Tree)"));
                lines.push(Line::from("  f           Cycle filter"));
                lines.push(Line::from("  /           Search tensions"));
                lines.push(Line::from("  :           Command palette"));
                lines.push(Line::from("  T           Toggle timeline panel"));
                lines.push(Line::from("  D           Toggle health overlay"));
                lines.push(Line::from("  L           Show lever detail"));
                lines.push(Line::from(""));
                lines.push(Line::from_spans([Span::styled("  Editing", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  a           Quick-add tension (desired [+2w] [| actual])"));
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
                lines.push(Line::from("  T           Toggle timeline panel"));
                lines.push(Line::from("  D           Toggle health overlay"));
                lines.push(Line::from("  L           Show lever detail"));
                lines.push(Line::from(""));
                lines.push(Line::from("  /           Search tensions"));
                lines.push(Line::from("  :           Command palette"));
                lines.push(Line::from(""));
                lines.push(Line::from_spans([Span::styled("  Editing", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  r           Update reality"));
                lines.push(Line::from("  d           Update desire"));
                lines.push(Line::from("  n           Add note"));
                lines.push(Line::from("  h           Set horizon"));
                lines.push(Line::from("  a           Quick-add sub-tension (desired [+2w] [| actual])"));
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
                lines.push(Line::from("  Tab/Esc     Back to dashboard"));
                lines.push(Line::from("  f           Cycle filter"));
                lines.push(Line::from("  /           Search tensions"));
                lines.push(Line::from("  :           Command palette"));
                lines.push(Line::from(""));
                lines.push(Line::from_spans([Span::styled("  Editing", Style::new().fg(CLR_CYAN).bold())]));
                lines.push(Line::from("  a           Quick-add tension (desired [+2w] [| actual])"));
                lines.push(Line::from("  c           Create child of selected"));
                lines.push(Line::from("  p           Create parent of selected"));
                lines.push(Line::from("  r/d/n/h     Edit selected tension"));
                lines.push(Line::from("  R/X/m       Resolve/Release/Move"));
            }
            // Legacy views (absorbed into primary views)
            View::Neighborhood | View::Timeline | View::Focus | View::DynamicsSummary => {
                lines.push(Line::from_spans([Span::styled("  Dashboard", Style::new().fg(CLR_CYAN).bold())]));
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
