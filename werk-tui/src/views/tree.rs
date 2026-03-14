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
use crate::types::UrgencyTier;

impl WerkApp {
    pub(crate) fn render_tree_title(&self, area: &Rect, frame: &mut Frame<'_>) {
        let left_text = format!(
            " Tree  |  {} tensions  {} roots",
            self.tree_items.len(),
            self.tree_items.iter().filter(|i| i.depth == 0).count(),
        );
        let status = StatusLine::new()
            .left(StatusItem::text(&left_text))
            .style(Style::new().fg(CLR_LIGHT_GRAY).bold());
        status.render(*area, frame);
    }

    pub(crate) fn render_tree_body(&self, area: &Rect, frame: &mut Frame<'_>) {
        if self.tree_items.is_empty() {
            let msg = Paragraph::new(Text::from_spans([Span::styled(
                "  No tensions yet. Press `a` to create your first.",
                Style::new().fg(CLR_MID_GRAY),
            )]));
            msg.render(*area, frame);
            return;
        }

        let items: Vec<ListItem> = self
            .tree_items
            .iter()
            .map(|item| {
                let urgency_str = match item.urgency {
                    Some(u) => format!("{:>3.0}%", (u * 100.0).min(999.0)),
                    None => "  --".to_string(),
                };

                let desired_width = (area.width as usize)
                    .saturating_sub(item.connector.chars().count() + 2 + 4 + 4 + 12 + 5);
                let desired_trunc = truncate(&item.desired, desired_width.max(10));

                let item_style = match item.tier {
                    UrgencyTier::Urgent => Style::new().fg(CLR_RED_SOFT),
                    UrgencyTier::Active => Style::new().fg(CLR_LIGHT_GRAY),
                    UrgencyTier::Neglected => Style::new().fg(CLR_YELLOW_SOFT),
                    UrgencyTier::Resolved => Style::new().fg(CLR_DIM_GRAY),
                };

                let text = Text::from_spans([
                    Span::styled("  ", item_style),
                    Span::styled(&item.connector, Style::new().fg(CLR_DIM_GRAY)),
                    Span::styled(format!("[{}] {} ", item.phase, item.movement), item_style),
                    Span::styled(
                        format!("{:<width$} ", desired_trunc, width = desired_width),
                        item_style,
                    ),
                    Span::styled(format!("{:>11} ", item.horizon_display), item_style),
                    Span::styled(urgency_str, item_style),
                ]);

                ListItem::new(text).style(item_style)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::new().fg(CLR_WHITE).bold())
;

        let mut state = self.tree_state.borrow_mut();
        StatefulWidget::render(&list, *area, frame, &mut state);
    }

    pub(crate) fn render_tree_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = StatusLine::new()
            .separator("  ")
            .left(StatusItem::key_hint("j/k", "navigate"))
            .left(StatusItem::key_hint("Enter", "detail"))
            .left(StatusItem::key_hint("Tab", "dashboard"))
            .left(StatusItem::key_hint("f", "filter"))
            .left(StatusItem::key_hint("q", "quit"))
            .left(StatusItem::key_hint("?", "help"))
            .style(Style::new().fg(CLR_MID_GRAY));
        hints.render(*area, frame);
    }
}
