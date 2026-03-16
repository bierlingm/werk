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
            .style(STYLES.status_bar);
        status.render(*area, frame);
    }

    pub(crate) fn render_tree_body(&self, area: &Rect, frame: &mut Frame<'_>) {
        if self.tree_items.is_empty() {
            let msg = Paragraph::new(Text::from_spans([Span::styled(
                "  No tensions yet. Press `a` to create your first.",
                STYLES.label,
            )]));
            msg.render(*area, frame);
            return;
        }

        let items: Vec<ListItem> = self
            .tree_items
            .iter()
            .map(|item| {
                let connector_width = item.connector.chars().count();
                let desired_width = (area.width as usize)
                    .saturating_sub(connector_width + 2 + 4);
                let desired_trunc = truncate(&item.desired, desired_width.max(10));

                let item_style = match item.tier {
                    UrgencyTier::Urgent => Style::new().fg(CLR_RED_SOFT),
                    UrgencyTier::Active => Style::new().fg(CLR_LIGHT_GRAY),
                    UrgencyTier::Neglected => Style::new().fg(CLR_YELLOW_SOFT),
                    UrgencyTier::Resolved => Style::new().fg(CLR_DIM_GRAY),
                };

                let indicator = format!("{}{} ", item.phase, item.movement);

                let text = Text::from_spans([
                    Span::styled("  ", item_style),
                    Span::styled(&item.connector, STYLES.muted),
                    Span::styled(indicator, item_style),
                    Span::styled(desired_trunc.to_string(), item_style),
                ]);

                ListItem::new(text).style(item_style)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::new().fg(CLR_WHITE).bg(WERK_THEME.highlight).bold());

        let mut state = self.tree_state.borrow_mut();
        StatefulWidget::render(&list, *area, frame, &mut state);
    }

    pub(crate) fn render_tree_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = StatusLine::new()
            .separator("  ")
            .left(StatusItem::key_hint("Esc/Tab", "back"))
            .left(StatusItem::key_hint("?", "help"))
            .left(StatusItem::key_hint("Ctrl-/", "commands"))
            .style(STYLES.muted);
        hints.render(*area, frame);
    }
}
