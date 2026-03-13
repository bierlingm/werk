use ftui::Frame;
use ftui::layout::{Constraint, Flex, Rect};
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::block::Block;
use ftui::widgets::borders::BorderType;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::status_line::{StatusLine, StatusItem};

use sd_core::{Forest, Tension};
use werk_shared::truncate;

use crate::app::WerkApp;
use crate::theme::*;
use crate::types::TensionRow;

// ---------------------------------------------------------------------------
// Card rendering helper
// ---------------------------------------------------------------------------

/// Build a bordered card widget for a single tension node.
/// Uses pre-computed TensionRow data when available, falls back to raw tension.
fn tension_card<'a>(
    tension: &Tension,
    row: Option<&TensionRow>,
    label: &'a str,
    selected: bool,
    card_width: usize,
) -> Paragraph<'a> {
    let border_color = if selected { CLR_CYAN } else { CLR_DIM_GRAY };
    let text_color = if selected { CLR_WHITE } else { CLR_LIGHT_GRAY };

    let phase_str = row.map(|r| r.phase.as_str()).unwrap_or("?");

    let urgency_str = row
        .and_then(|r| r.urgency)
        .map(|u| format!("{:.0}%", u * 100.0))
        .unwrap_or_else(|| "--".to_string());

    let horizon_str = row
        .map(|r| r.horizon_display.clone())
        .unwrap_or_else(|| "\u{2014}".to_string());

    // Inner width = card_width minus 2 for borders
    let inner = card_width.saturating_sub(2);
    let desired = truncate(&tension.desired, inner.saturating_sub(5));

    let line1 = Line::from_spans([
        Span::styled(
            format!("[{}] ", phase_str),
            Style::new().fg(CLR_MID_GRAY),
        ),
        Span::styled(
            format!("\"{}\"", desired),
            Style::new().fg(text_color),
        ),
    ]);

    let line2 = Line::from_spans([
        Span::styled(urgency_str, Style::new().fg(CLR_YELLOW_SOFT)),
        Span::styled("  ", Style::new()),
        Span::styled(horizon_str, Style::new().fg(CLR_MID_GRAY)),
    ]);

    let block = Block::bordered()
        .title(label)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border_color));

    Paragraph::new(Text::from_lines([line1, line2])).block(block)
}

// ---------------------------------------------------------------------------
// Connector helper
// ---------------------------------------------------------------------------

/// Render a centered vertical connector "|" in the given area.
fn render_connector(area: &Rect, frame: &mut Frame<'_>) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let mid_x = area.x + area.width / 2;
    let connector_area = Rect {
        x: mid_x,
        y: area.y,
        width: 1,
        height: 1,
    };
    let text = Paragraph::new(Text::from_spans([
        Span::styled("|", Style::new().fg(CLR_DIM_GRAY)),
    ]));
    text.render(connector_area, frame);
}

// ---------------------------------------------------------------------------
// Neighborhood view implementation
// ---------------------------------------------------------------------------

impl WerkApp {
    pub(crate) fn render_neighborhood_title(&self, area: &Rect, frame: &mut Frame<'_>) {
        let title = match self.selected_tension_id() {
            Some(id) => {
                let desired = self
                    .engine
                    .store()
                    .get_tension(&id)
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
            .left(StatusItem::key_hint("Esc", "back"))
            .left(StatusItem::key_hint("Enter", "detail"))
            .left(StatusItem::key_hint("F", "focus"))
            .left(StatusItem::key_hint("c/p", "child/parent"))
            .left(StatusItem::key_hint("r/d", "edit"))
            .left(StatusItem::key_hint("q/?", ""))
            .style(Style::new().fg(CLR_DIM_GRAY));
        hints.render(*area, frame);
    }

    /// Look up the pre-computed TensionRow for a given tension ID.
    fn find_tension_row(&self, id: &str) -> Option<&TensionRow> {
        self.tensions.iter().find(|r| r.id == id)
    }

    pub(crate) fn render_neighborhood(&self, area: &Rect, frame: &mut Frame<'_>) {
        // Get the selected tension
        let selected_id = match self.selected_tension_id() {
            Some(id) => id,
            None => {
                let msg = "  No tension selected. Press Esc to go back to the dashboard.";
                Paragraph::new(Text::from_spans([
                    Span::styled(msg, Style::new().fg(CLR_MID_GRAY)),
                ]))
                .render(*area, frame);
                return;
            }
        };

        // Build a forest from all tensions
        let tensions = match self.engine.store().list_tensions() {
            Ok(t) => t,
            Err(_) => {
                Paragraph::new(Text::from_spans([
                    Span::styled(
                        "  Error loading tensions.",
                        Style::new().fg(CLR_RED_SOFT),
                    ),
                ]))
                .render(*area, frame);
                return;
            }
        };

        let forest = match Forest::from_tensions(tensions) {
            Ok(f) => f,
            Err(_) => {
                Paragraph::new(Text::from_spans([
                    Span::styled(
                        "  Error building tension tree.",
                        Style::new().fg(CLR_RED_SOFT),
                    ),
                ]))
                .render(*area, frame);
                return;
            }
        };

        let selected_node = match forest.find(&selected_id) {
            Some(n) => n,
            None => {
                Paragraph::new(Text::from_spans([
                    Span::styled(
                        "  Selected tension not found in tree.",
                        Style::new().fg(CLR_MID_GRAY),
                    ),
                ]))
                .render(*area, frame);
                return;
            }
        };

        // Gather neighborhood data
        let parent: Option<&Tension> = selected_node
            .tension
            .parent_id
            .as_ref()
            .and_then(|pid| forest.find(pid))
            .map(|n| &n.tension);

        let siblings: Vec<&Tension> = forest
            .siblings(&selected_id)
            .unwrap_or_default()
            .into_iter()
            .map(|n| &n.tension)
            .collect();

        let children: Vec<&Tension> = forest
            .children(&selected_id)
            .unwrap_or_default()
            .into_iter()
            .map(|n| &n.tension)
            .collect();

        // ---------------------------------------------------------------------------
        // Layout: 3 vertical rows -- parent, selected+siblings, children
        // Each card is 4 lines tall (2 border + 2 content).
        // Connector lines are 1 line tall.
        // ---------------------------------------------------------------------------

        let card_height: u16 = 4;
        let connector_height: u16 = 1;

        let has_parent = parent.is_some();
        let has_children = !children.is_empty();

        let mut v_constraints: Vec<Constraint> = Vec::new();
        if has_parent {
            v_constraints.push(Constraint::Fixed(card_height)); // parent card
            v_constraints.push(Constraint::Fixed(connector_height)); // connector
        }
        v_constraints.push(Constraint::Fixed(card_height)); // selected row
        if has_children {
            v_constraints.push(Constraint::Fixed(connector_height)); // connector
            v_constraints.push(Constraint::Fixed(card_height)); // children row
        }
        v_constraints.push(Constraint::Fill); // remaining space

        let v_layout = Flex::vertical().constraints(v_constraints);
        let v_rects = v_layout.split(*area);

        let full_width = area.width as usize;
        let mut row_idx: usize = 0;

        // ---- Parent row ----
        if let Some(parent_tension) = parent {
            let parent_card_width = (full_width / 2).max(20).min(full_width);
            let parent_area = v_rects[row_idx];
            row_idx += 1;

            // Center the parent card horizontally
            let h_layout = Flex::horizontal().constraints([
                Constraint::Fill,
                Constraint::Fixed(parent_card_width as u16),
                Constraint::Fill,
            ]);
            let h_rects = h_layout.split(parent_area);

            let row_data = self.find_tension_row(&parent_tension.id);
            let card = tension_card(
                parent_tension,
                row_data,
                " Parent ",
                false,
                parent_card_width,
            );
            card.render(h_rects[1], frame);

            // Connector below parent
            render_connector(&v_rects[row_idx], frame);
            row_idx += 1;
        }

        // ---- Selected + siblings row ----
        let sibling_row_area = v_rects[row_idx];
        row_idx += 1;

        // Limit visible siblings to avoid overflow
        let max_siblings_per_side = 2;
        let left_siblings: Vec<&Tension> =
            siblings.iter().take(max_siblings_per_side).copied().collect();
        let right_siblings: Vec<&Tension> = siblings
            .iter()
            .skip(max_siblings_per_side)
            .take(max_siblings_per_side)
            .copied()
            .collect();

        let total_cards = 1 + left_siblings.len() + right_siblings.len();
        let card_width = (full_width / total_cards).max(16).min(40);

        let mut h_constraints: Vec<Constraint> = Vec::new();
        h_constraints.push(Constraint::Fill); // left margin
        for _ in &left_siblings {
            h_constraints.push(Constraint::Fixed(card_width as u16));
        }
        // Selected card is slightly wider
        let selected_width = (card_width + 4).min(full_width);
        h_constraints.push(Constraint::Fixed(selected_width as u16));
        for _ in &right_siblings {
            h_constraints.push(Constraint::Fixed(card_width as u16));
        }
        h_constraints.push(Constraint::Fill); // right margin

        let h_layout = Flex::horizontal().constraints(h_constraints);
        let h_rects = h_layout.split(sibling_row_area);

        let mut col_idx: usize = 1; // skip left fill

        // Left siblings
        for sib in &left_siblings {
            let row_data = self.find_tension_row(&sib.id);
            let card = tension_card(sib, row_data, " Sibling ", false, card_width);
            card.render(h_rects[col_idx], frame);
            col_idx += 1;
        }

        // Selected node
        {
            let row_data = self.find_tension_row(&selected_node.tension.id);
            let card = tension_card(
                &selected_node.tension,
                row_data,
                " SELECTED ",
                true,
                selected_width,
            );
            card.render(h_rects[col_idx], frame);
            col_idx += 1;
        }

        // Right siblings
        for sib in &right_siblings {
            let row_data = self.find_tension_row(&sib.id);
            let card = tension_card(sib, row_data, " Sibling ", false, card_width);
            card.render(h_rects[col_idx], frame);
            col_idx += 1;
        }

        // Overflow indicator for siblings
        let overflow = siblings.len().saturating_sub(max_siblings_per_side * 2);
        if overflow > 0 {
            let hint = format!("+{} more siblings", overflow);
            let trailing_rect = h_rects[col_idx]; // the trailing Fill
            if trailing_rect.width >= hint.len() as u16 + 1 {
                let hint_area = Rect {
                    x: trailing_rect.x,
                    y: sibling_row_area.y,
                    width: hint.len() as u16 + 1,
                    height: 1,
                };
                Paragraph::new(Text::from_spans([
                    Span::styled(hint, Style::new().fg(CLR_DIM_GRAY)),
                ]))
                .render(hint_area, frame);
            }
        }

        // ---- Children row ----
        if !children.is_empty() {
            // Connector above children
            render_connector(&v_rects[row_idx], frame);
            row_idx += 1;

            let children_area = v_rects[row_idx];

            let max_children = (full_width / 16).max(2).min(children.len());
            let shown_children: Vec<&Tension> =
                children.iter().take(max_children).copied().collect();
            let child_card_width =
                (full_width / shown_children.len().max(1)).max(16).min(36);

            let mut ch_constraints: Vec<Constraint> = Vec::new();
            ch_constraints.push(Constraint::Fill);
            for _ in &shown_children {
                ch_constraints.push(Constraint::Fixed(child_card_width as u16));
            }
            ch_constraints.push(Constraint::Fill);

            let ch_layout = Flex::horizontal().constraints(ch_constraints);
            let ch_rects = ch_layout.split(children_area);

            for (i, child) in shown_children.iter().enumerate() {
                let label = format!(" Child {} ", i + 1);
                let row_data = self.find_tension_row(&child.id);
                let card =
                    tension_card(child, row_data, &label, false, child_card_width);
                card.render(ch_rects[i + 1], frame);
            }

            // Overflow hint for children
            let child_overflow = children.len().saturating_sub(max_children);
            if child_overflow > 0 {
                let hint = format!("+{} more", child_overflow);
                let trailing = ch_rects[shown_children.len() + 1];
                if trailing.width >= hint.len() as u16 + 1 {
                    let hint_area = Rect {
                        x: trailing.x,
                        y: children_area.y,
                        width: hint.len() as u16 + 1,
                        height: 1,
                    };
                    Paragraph::new(Text::from_spans([
                        Span::styled(hint, Style::new().fg(CLR_DIM_GRAY)),
                    ]))
                    .render(hint_area, frame);
                }
            }
        }
    }
}
