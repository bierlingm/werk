use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::status_line::{StatusLine, StatusItem};

use sd_core::{Forest, Tension};
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
            .left(StatusItem::key_hint("Esc", "back"))
            .left(StatusItem::key_hint("Enter", "detail"))
            .left(StatusItem::key_hint("r/d", "edit"))
            .left(StatusItem::key_hint("c/p", "child/parent"))
            .left(StatusItem::key_hint("F", "focus"))
            .left(StatusItem::key_hint("q/?", ""))
            .style(Style::new().fg(CLR_DIM_GRAY));
        hints.render(*area, frame);
    }

    fn tension_label(&self, tension: &Tension, max_w: usize) -> String {
        let row = self.tensions.iter().find(|r| r.id == tension.id);
        let phase = row.map(|r| r.phase.as_str()).unwrap_or("?");
        let urgency = row
            .and_then(|r| r.urgency)
            .map(|u| format!("{:.0}%", u * 100.0))
            .unwrap_or_else(|| "--".to_string());
        let horizon = row
            .map(|r| r.horizon_display.as_str())
            .unwrap_or("");
        let desired = truncate(&tension.desired, max_w.saturating_sub(20));
        format!("[{}] \"{}\"  {}  {}", phase, desired, urgency, horizon)
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
            Err(_) => return,
        };

        let forest = match Forest::from_tensions(tensions) {
            Ok(f) => f,
            Err(_) => return,
        };

        let selected_node = match forest.find(&selected_id) {
            Some(n) => n,
            None => return,
        };

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
        let mut lines: Vec<Line> = Vec::new();
        let dim = Style::new().fg(CLR_DIM_GRAY);
        let label_style = Style::new().fg(CLR_MID_GRAY);

        // ---- Parent ----
        if let Some(pt) = parent {
            lines.push(Line::from_spans([
                Span::styled("  Parent    ", label_style),
                Span::styled(self.tension_label(pt, w.saturating_sub(14)), Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
            lines.push(Line::from_spans([
                Span::styled(format!("  {}", "\u{2502}"), dim),
            ]));
        }

        // ---- Selected ----
        lines.push(Line::from_spans([
            Span::styled(" \u{25b6} SELECTED  ", Style::new().fg(CLR_CYAN).bold()),
            Span::styled(
                self.tension_label(&selected_node.tension, w.saturating_sub(14)),
                Style::new().fg(CLR_WHITE).bold(),
            ),
        ]));

        // ---- Siblings ----
        if !siblings.is_empty() {
            lines.push(Line::from_spans([Span::styled("", dim)]));
            lines.push(Line::from_spans([
                Span::styled(
                    format!("  Siblings ({})", siblings.len()),
                    label_style,
                ),
            ]));
            for sib in siblings.iter().take(8) {
                lines.push(Line::from_spans([
                    Span::styled("    \u{251c} ", dim),
                    Span::styled(self.tension_label(sib, w.saturating_sub(8)), Style::new().fg(CLR_LIGHT_GRAY)),
                ]));
            }
            if siblings.len() > 8 {
                lines.push(Line::from_spans([
                    Span::styled(format!("    +{} more", siblings.len() - 8), dim),
                ]));
            }
        }

        // ---- Children ----
        if !children.is_empty() {
            lines.push(Line::from_spans([Span::styled("", dim)]));
            lines.push(Line::from_spans([
                Span::styled(
                    format!("  Children ({})", children.len()),
                    label_style,
                ),
            ]));
            for child in children.iter().take(8) {
                lines.push(Line::from_spans([
                    Span::styled("    \u{251c} ", dim),
                    Span::styled(self.tension_label(child, w.saturating_sub(8)), Style::new().fg(CLR_LIGHT_GRAY)),
                ]));
            }
            if children.len() > 8 {
                lines.push(Line::from_spans([
                    Span::styled(format!("    +{} more", children.len() - 8), dim),
                ]));
            }
        }

        // No relatives at all
        if parent.is_none() && siblings.is_empty() && children.is_empty() {
            lines.push(Line::from_spans([Span::styled("", dim)]));
            lines.push(Line::from_spans([Span::styled(
                "  No parent, siblings, or children.",
                dim,
            )]));
        }

        let text = Text::from_lines(lines);
        Paragraph::new(text).render(*area, frame);
    }
}
