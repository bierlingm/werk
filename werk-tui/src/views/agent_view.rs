use ftui::Frame;
use ftui::layout::{Constraint, Flex, Rect};
use ftui::text::{Line, Span, Text, WrapMode};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::StatefulWidget;
use ftui::widgets::block::Block;
use ftui::widgets::borders::BorderType;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::list::{List, ListItem};
use ftui::widgets::status_line::{StatusLine, StatusItem};

use sd_core::compute_urgency;
use chrono::Utc;
use werk_shared::truncate;

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_agent_title(&self, tension_id: &str, area: &Rect, frame: &mut Frame<'_>) {
        let desired = self
            .engine
            .store()
            .get_tension(tension_id)
            .ok()
            .flatten()
            .map(|t| truncate(&t.desired, area.width.saturating_sub(16) as usize).to_string())
            .unwrap_or_else(|| tension_id.chars().take(8).collect());

        let left_text = format!(" Agent: {}", desired);
        let right_text = if self.agent.running { "[running...]" } else { "" };
        let mut status = StatusLine::new()
            .left(StatusItem::text(&left_text))
            .style(Style::new().fg(CLR_CYAN).bold());
        if !right_text.is_empty() {
            status = status.right(StatusItem::text(right_text));
        }
        status.render(*area, frame);
    }

    pub(crate) fn render_agent_body(&self, area: &Rect, frame: &mut Frame<'_>) {
        let border_style = Style::new().fg(CLR_DIM_GRAY);

        if !self.agent.mutations.is_empty() {
            // Split area: top ~60% for response, bottom for mutations
            let mutations_height = (self.agent.mutations.len() as u16).saturating_add(2)
                .min(area.height.saturating_sub(4));
            let layout = Flex::vertical().constraints([
                Constraint::Fill,
                Constraint::Fixed(mutations_height),
            ]);
            let rects = layout.split(*area);

            self.render_response_block(&rects[0], frame, border_style);
            self.render_mutations_block(&rects[1], frame, border_style);
        } else {
            // No mutations — full area for response
            self.render_response_block(area, frame, border_style);
        }
    }

    fn render_response_block(&self, area: &Rect, frame: &mut Frame<'_>, border_style: Style) {
        let block = Block::bordered()
            .title(" Response ")
            .border_type(BorderType::Rounded)
            .border_style(border_style);
        block.render(*area, frame);
        let inner = block.inner(*area);

        let mut lines: Vec<Line> = Vec::new();

        if self.agent.running {
            lines.push(Line::from_spans([Span::styled(
                "Running agent...",
                Style::new().fg(CLR_YELLOW),
            )]));
        } else if let Some(response_text) = &self.agent.response_text {
            for line in response_text.lines() {
                lines.push(Line::from_spans([Span::styled(
                    line.to_string(),
                    Style::new().fg(CLR_LIGHT_GRAY),
                )]));
            }
        } else if !self.agent.output.is_empty() {
            for line in &self.agent.output {
                lines.push(Line::from_spans([Span::styled(
                    line.clone(),
                    Style::new().fg(CLR_LIGHT_GRAY),
                )]));
            }
        } else {
            lines.push(Line::from_spans([Span::styled(
                "No agent output yet. Press Esc to go back.",
                Style::new().fg(CLR_DIM_GRAY),
            )]));
        }

        let text = Text::from_lines(lines);
        let paragraph = Paragraph::new(text)
            .wrap(WrapMode::Word)
            .scroll((self.agent.scroll, 0));
        paragraph.render(inner, frame);
    }

    fn render_mutations_block(&self, area: &Rect, frame: &mut Frame<'_>, border_style: Style) {
        let title = format!(" Suggested Changes ({}) ", self.agent.mutations.len());
        let block = Block::bordered()
            .title(title.as_str())
            .border_type(BorderType::Rounded)
            .border_style(border_style);
        let inner = block.inner(*area);

        let items: Vec<ListItem> = self.agent.mutations.iter().enumerate().map(|(i, mutation)| {
            let is_selected = self
                .agent.mutation_selected
                .get(i)
                .copied()
                .unwrap_or(false);
            let check = if is_selected { "x" } else { " " };

            let summary = mutation.summary();
            let reasoning_budget = (inner.width as usize).saturating_sub(summary.len() + 12);
            let reasoning = mutation
                .reasoning()
                .filter(|_| reasoning_budget > 5)
                .map(|r| format!(" ({})", truncate(r, reasoning_budget)))
                .unwrap_or_default();

            let item_style = if is_selected {
                Style::new().fg(CLR_GREEN)
            } else {
                Style::new().fg(CLR_MID_GRAY)
            };

            let label = format!(" {}. [{}] {}{}", i + 1, check, summary, reasoning);
            ListItem::new(Text::from_spans([Span::styled(label, item_style)])).style(item_style)
        }).collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().fg(CLR_WHITE).bold())
            .highlight_symbol("> ");

        let mut state = ftui::widgets::list::ListState::default();
        if !self.agent.mutations.is_empty() {
            state.select(Some(self.agent.mutation_cursor));
        }
        StatefulWidget::render(&list, *area, frame, &mut state);
    }

    #[allow(dead_code)]
    pub(crate) fn render_agent_separator(&self, _area: &Rect, _frame: &mut Frame<'_>) {
        // No-op: separator is now replaced by Block borders
    }

    pub(crate) fn render_agent_context(&self, tension_id: &str, area: &Rect, frame: &mut Frame<'_>) {
        let block = Block::bordered()
            .title(" Context ")
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(CLR_DIM_GRAY));
        block.render(*area, frame);
        let inner = block.inner(*area);

        let mut lines: Vec<Line> = Vec::new();

        if let Ok(Some(tension)) = self.engine.store().get_tension(tension_id) {
            let now = Utc::now();
            let max_w = inner.width.saturating_sub(12) as usize;
            lines.push(Line::from_spans([
                Span::styled("Desired  ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(
                    truncate(&tension.desired, max_w),
                    Style::new().fg(CLR_LIGHT_GRAY),
                ),
            ]));
            lines.push(Line::from_spans([
                Span::styled("Actual   ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(
                    truncate(&tension.actual, max_w),
                    Style::new().fg(CLR_LIGHT_GRAY),
                ),
            ]));

            let urgency_str = compute_urgency(&tension, now)
                .map(|u| format!("{:.0}%", u.value * 100.0))
                .unwrap_or_else(|| "--".to_string());

            lines.push(Line::from_spans([
                Span::styled("Status   ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(tension.status.to_string(), Style::new().fg(CLR_LIGHT_GRAY)),
                Span::styled("    Urgency  ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(urgency_str, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
        } else {
            lines.push(Line::from_spans([Span::styled(
                "Tension not found",
                Style::new().fg(CLR_DIM_GRAY),
            )]));
        }

        let text = Text::from_lines(lines);
        let paragraph = Paragraph::new(text);
        paragraph.render(inner, frame);
    }

    pub(crate) fn render_agent_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = if self.agent.mutations.is_empty() {
            StatusLine::new()
                .separator("  ")
                .left(StatusItem::key_hint("Esc", "back"))
                .left(StatusItem::key_hint("q", "quit"))
                .left(StatusItem::key_hint("?", "help"))
                .style(Style::new().fg(CLR_MID_GRAY))
        } else {
            StatusLine::new()
                .separator("  ")
                .left(StatusItem::key_hint("j/k", "nav"))
                .left(StatusItem::key_hint("Enter", "toggle"))
                .left(StatusItem::key_hint("1-9", "toggle"))
                .left(StatusItem::key_hint("a", "apply selected"))
                .left(StatusItem::key_hint("Esc", "back"))
                .left(StatusItem::key_hint("q", "quit"))
                .style(Style::new().fg(CLR_MID_GRAY))
        };
        hints.render(*area, frame);
    }
}
