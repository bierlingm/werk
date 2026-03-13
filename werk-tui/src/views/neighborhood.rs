use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::StatefulWidget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::list::{List, ListItem};
use ftui::widgets::status_line::{StatusLine, StatusItem};

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_neighborhood_title(&self, area: &Rect, frame: &mut Frame<'_>) {
        let title = match &self.neighborhood_tension_id {
            Some(id) => {
                let desired = self
                    .engine
                    .store()
                    .get_tension(id)
                    .ok()
                    .flatten()
                    .map(|t| {
                        truncate(&t.desired, area.width.saturating_sub(24) as usize).to_string()
                    })
                    .unwrap_or_else(|| id.chars().take(8).collect());
                format!(" Neighborhood: {}", desired)
            }
            None => " Neighborhood".to_string(),
        };

        let status = StatusLine::new()
            .left(StatusItem::text(&title))
            .style(Style::new().fg(CLR_LIGHT_GRAY).bold());
        status.render(*area, frame);
    }

    pub(crate) fn render_neighborhood_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = StatusLine::new()
            .separator("  ")
            .left(StatusItem::key_hint("j/k", "navigate"))
            .left(StatusItem::key_hint("Enter", "select/detail"))
            .left(StatusItem::key_hint("Esc", "back"))
            .left(StatusItem::key_hint("r/d", "edit"))
            .left(StatusItem::key_hint("c/p", "child/parent"))
            .left(StatusItem::key_hint("F", "focus"))
            .left(StatusItem::key_hint("q/?", ""))
            .style(Style::new().fg(CLR_DIM_GRAY));
        hints.render(*area, frame);
    }

    pub(crate) fn render_neighborhood(&self, area: &Rect, frame: &mut Frame<'_>) {
        if self.neighborhood_items.is_empty() {
            Paragraph::new(Text::from_spans([Span::styled(
                "  No tension selected. Press Esc to go back.",
                Style::new().fg(CLR_MID_GRAY),
            )]))
            .render(*area, frame);
            return;
        }

        let w = area.width as usize;

        let items: Vec<ListItem> = self
            .neighborhood_items
            .iter()
            .map(|(id, role)| {
                let row = self.tensions.iter().find(|r| r.id == *id);
                let phase = row.map(|r| r.phase.as_str()).unwrap_or("?");
                let urgency = row
                    .and_then(|r| r.urgency)
                    .map(|u| format!("{:.0}%", u * 100.0))
                    .unwrap_or_else(|| "--".to_string());
                let horizon = row
                    .map(|r| r.horizon_display.as_str())
                    .unwrap_or("");
                let desired_text = row
                    .map(|r| r.desired.as_str())
                    .or_else(|| {
                        self.engine
                            .store()
                            .get_tension(id)
                            .ok()
                            .flatten()
                            .map(|_| "")
                    })
                    .unwrap_or("");
                // Fall back to store lookup for desired
                let desired_from_store;
                let desired = if !desired_text.is_empty() {
                    desired_text
                } else {
                    desired_from_store = self
                        .engine
                        .store()
                        .get_tension(id)
                        .ok()
                        .flatten()
                        .map(|t| t.desired.clone())
                        .unwrap_or_default();
                    &desired_from_store
                };

                let is_center = role == "SELECTED";

                let (prefix, role_style) = if is_center {
                    (" \u{25b6} ", Style::new().fg(CLR_CYAN).bold())
                } else {
                    ("   ", Style::new().fg(CLR_MID_GRAY))
                };

                let label_w = role.len() + 2;
                let desired_w = w.saturating_sub(label_w + 24);
                let desired_trunc = truncate(desired, desired_w);

                let text_style = if is_center {
                    Style::new().fg(CLR_WHITE)
                } else {
                    Style::new().fg(CLR_LIGHT_GRAY)
                };

                let text = Text::from_spans([
                    Span::styled(prefix, role_style),
                    Span::styled(format!("{:<10}", role), role_style),
                    Span::styled(format!("[{}] ", phase), Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(format!("{:<w$}", desired_trunc, w = desired_w), text_style),
                    Span::styled(format!("  {:>4}", urgency), Style::new().fg(CLR_YELLOW_SOFT)),
                    Span::styled(format!("  {:>11}", horizon), Style::new().fg(CLR_MID_GRAY)),
                ]);

                ListItem::new(text)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::new().fg(CLR_WHITE).bold());

        let mut state = self.neighborhood_state.borrow_mut();
        StatefulWidget::render(&list, *area, frame, &mut state);
    }
}
