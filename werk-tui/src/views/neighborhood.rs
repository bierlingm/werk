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
            .left(StatusItem::key_hint("Esc", "back"))
            .left(StatusItem::key_hint("Enter", "detail"))
            .left(StatusItem::key_hint("r/d", "edit"))
            .left(StatusItem::key_hint("c/p", "child/parent"))
            .left(StatusItem::key_hint("F", "focus"))
            .left(StatusItem::key_hint("q/?", ""))
            .style(Style::new().fg(CLR_DIM_GRAY));
        hints.render(*area, frame);
    }

    fn find_tension_row(&self, id: &str) -> Option<&TensionRow> {
        self.tensions.iter().find(|r| r.id == id)
    }

    fn card_line(
        &self,
        tension: &Tension,
        width: usize,
    ) -> (String, String) {
        let row = self.find_tension_row(&tension.id);
        let phase = row.map(|r| r.phase.as_str()).unwrap_or("?");
        let urgency = row
            .and_then(|r| r.urgency)
            .map(|u| format!("{:.0}%", u * 100.0))
            .unwrap_or_else(|| "--".to_string());
        let horizon = row
            .map(|r| r.horizon_display.clone())
            .unwrap_or_default();
        let desired = truncate(&tension.desired, width.saturating_sub(8));
        let line1 = format!("[{}] \"{}\"", phase, desired);
        let line2 = format!("{}  {}", urgency, horizon);
        (line1, line2)
    }

    pub(crate) fn render_neighborhood(&self, area: &Rect, frame: &mut Frame<'_>) {
        let selected_id = match &self.neighborhood_tension_id {
            Some(id) => id.clone(),
            None => {
                Paragraph::new(Text::from_spans([Span::styled(
                    "  No tension selected. Press Esc to go back.",
                    Style::new().fg(CLR_MID_GRAY),
                )]))
                .render(*area, frame);
                return;
            }
        };

        let tensions = match self.engine.store().list_tensions() {
            Ok(t) => t,
            Err(_) => {
                Paragraph::new(Text::from_spans([Span::styled(
                    "  Error loading tensions.",
                    Style::new().fg(CLR_RED_SOFT),
                )]))
                .render(*area, frame);
                return;
            }
        };

        let forest = match Forest::from_tensions(tensions) {
            Ok(f) => f,
            Err(_) => {
                Paragraph::new(Text::from_spans([Span::styled(
                    "  Error building tension tree.",
                    Style::new().fg(CLR_RED_SOFT),
                )]))
                .render(*area, frame);
                return;
            }
        };

        let selected_node = match forest.find(&selected_id) {
            Some(n) => n,
            None => {
                Paragraph::new(Text::from_spans([Span::styled(
                    "  Tension not found in tree.",
                    Style::new().fg(CLR_MID_GRAY),
                )]))
                .render(*area, frame);
                return;
            }
        };

        // Gather neighborhood
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

        let w = area.width as usize;
        let card_w = (w / 2).max(20).min(w.saturating_sub(4));

        // Build layout: parent card, connector, center row, connector, children
        let card_h: u16 = 4;
        let conn_h: u16 = 1;
        let has_parent = parent.is_some();
        let has_children = !children.is_empty();

        let mut v_constraints: Vec<Constraint> = Vec::new();
        if has_parent {
            v_constraints.push(Constraint::Fixed(card_h));
            v_constraints.push(Constraint::Fixed(conn_h));
        }
        v_constraints.push(Constraint::Fixed(card_h)); // center row
        if has_children {
            v_constraints.push(Constraint::Fixed(conn_h));
            v_constraints.push(Constraint::Fixed(card_h));
        }
        v_constraints.push(Constraint::Fill);

        let v_rects = Flex::vertical().constraints(v_constraints).split(*area);
        let mut ri: usize = 0;

        // ---- Parent card ----
        if let Some(pt) = parent {
            let (l1, l2) = self.card_line(pt, card_w.saturating_sub(4));
            render_card(" Parent ", &l1, &l2, false, card_w, &v_rects[ri], frame);
            ri += 1;
            render_connector_line(&v_rects[ri], frame);
            ri += 1;
        }

        // ---- Center row: siblings + selected ----
        {
            let center_area = v_rects[ri];
            ri += 1;

            let total = 1 + siblings.len().min(4);
            let cw = (w / total).max(16).min(40);
            let sel_w = (cw + 4).min(w);

            let shown_left: Vec<&Tension> = siblings.iter().take(2).copied().collect();
            let shown_right: Vec<&Tension> = siblings.iter().skip(2).take(2).copied().collect();

            let mut h_constraints: Vec<Constraint> = vec![Constraint::Fill];
            for _ in &shown_left {
                h_constraints.push(Constraint::Fixed(cw as u16));
            }
            h_constraints.push(Constraint::Fixed(sel_w as u16));
            for _ in &shown_right {
                h_constraints.push(Constraint::Fixed(cw as u16));
            }
            h_constraints.push(Constraint::Fill);

            let h_rects = Flex::horizontal().constraints(h_constraints).split(center_area);
            let mut ci: usize = 1;

            for sib in &shown_left {
                let (l1, l2) = self.card_line(sib, cw.saturating_sub(4));
                render_card(" Sibling ", &l1, &l2, false, cw, &h_rects[ci], frame);
                ci += 1;
            }

            // Selected node — accent border
            let (l1, l2) = self.card_line(&selected_node.tension, sel_w.saturating_sub(4));
            render_card(" SELECTED ", &l1, &l2, true, sel_w, &h_rects[ci], frame);
            ci += 1;

            for sib in &shown_right {
                let (l1, l2) = self.card_line(sib, cw.saturating_sub(4));
                render_card(" Sibling ", &l1, &l2, false, cw, &h_rects[ci], frame);
                ci += 1;
            }

            if siblings.len() > 4 {
                let overflow = siblings.len() - 4;
                let hint = format!("+{} more", overflow);
                let trailing = h_rects[ci];
                if trailing.width > hint.len() as u16 {
                    Paragraph::new(Text::from_spans([Span::styled(
                        hint,
                        Style::new().fg(CLR_DIM_GRAY),
                    )]))
                    .render(
                        Rect::new(trailing.x, center_area.y, trailing.width, 1),
                        frame,
                    );
                }
            }
        }

        // ---- Children row ----
        if has_children {
            render_connector_line(&v_rects[ri], frame);
            ri += 1;

            let ch_area = v_rects[ri];
            let max_ch = (w / 16).max(2).min(children.len());
            let shown: Vec<&Tension> = children.iter().take(max_ch).copied().collect();
            let ch_w = (w / shown.len().max(1)).max(16).min(36);

            let mut ch_constraints: Vec<Constraint> = vec![Constraint::Fill];
            for _ in &shown {
                ch_constraints.push(Constraint::Fixed(ch_w as u16));
            }
            ch_constraints.push(Constraint::Fill);

            let ch_rects = Flex::horizontal().constraints(ch_constraints).split(ch_area);

            for (i, child) in shown.iter().enumerate() {
                let label = format!(" Child {} ", i + 1);
                let (l1, l2) = self.card_line(child, ch_w.saturating_sub(4));
                render_card(&label, &l1, &l2, false, ch_w, &ch_rects[i + 1], frame);
            }

            if children.len() > max_ch {
                let hint = format!("+{} more", children.len() - max_ch);
                let trailing = ch_rects[shown.len() + 1];
                if trailing.width > hint.len() as u16 {
                    Paragraph::new(Text::from_spans([Span::styled(
                        hint,
                        Style::new().fg(CLR_DIM_GRAY),
                    )]))
                    .render(
                        Rect::new(trailing.x, ch_area.y, trailing.width, 1),
                        frame,
                    );
                }
            }
        }
    }
}

/// Render a single card: bordered block with two content lines.
fn render_card(
    title: &str,
    line1: &str,
    line2: &str,
    selected: bool,
    _width: usize,
    area: &Rect,
    frame: &mut Frame<'_>,
) {
    let border_color = if selected { CLR_CYAN } else { CLR_DIM_GRAY };
    let text_color = if selected { CLR_WHITE } else { CLR_LIGHT_GRAY };
    let meta_color = if selected { CLR_YELLOW } else { CLR_YELLOW_SOFT };

    let block = Block::bordered()
        .title(title)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border_color));

    let text = Text::from_lines([
        Line::from_spans([Span::styled(line1.to_string(), Style::new().fg(text_color))]),
        Line::from_spans([Span::styled(line2.to_string(), Style::new().fg(meta_color))]),
    ]);

    Paragraph::new(text).block(block).render(*area, frame);
}

/// Render a centered "|" connector.
fn render_connector_line(area: &Rect, frame: &mut Frame<'_>) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let mid_x = area.x + area.width / 2;
    Paragraph::new(Text::from_spans([Span::styled(
        "\u{2502}",
        Style::new().fg(CLR_DIM_GRAY),
    )]))
    .render(Rect::new(mid_x, area.y, 1, 1), frame);
}
