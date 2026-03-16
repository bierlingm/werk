use ftui::Frame;
use ftui::layout::{Constraint, Rect};
use ftui::text::{Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::StatefulWidget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::table::{Row, Table};
use ftui::widgets::status_line::{StatusLine, StatusItem};

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::helpers::activity_trail;
use crate::theme::*;
use crate::types::UrgencyTier;

pub fn mini_sparkline(data: &[f64], width: usize) -> String {
    let blocks = [' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
    if data.is_empty() {
        return " ".repeat(width);
    }
    let max = data.iter().cloned().fold(0.0f64, f64::max).max(1.0);
    let s: String = data
        .iter()
        .take(width)
        .map(|&v| {
            let idx = ((v / max) * 8.0).round().min(8.0) as usize;
            blocks[idx]
        })
        .collect();
    let pad = width.saturating_sub(s.chars().count());
    format!("{}{}", " ".repeat(pad), s)
}

impl WerkApp {
    pub(crate) fn render_status_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        let filter_label = if self.filter != crate::types::Filter::Active {
            format!("  [{}]", self.filter.label())
        } else {
            String::new()
        };

        // Left side: app name + counts (only non-zero)
        let mut left_spans: Vec<Span> = vec![
            Span::styled(
                format!(" werk  {} active", self.total_active),
                STYLES.status_bar,
            ),
        ];
        if self.total_urgent > 0 {
            left_spans.push(Span::styled(
                format!("  {}\u{25b2}", self.total_urgent),
                Style::new().fg(CLR_RED_SOFT).bold(),
            ));
        }
        if self.total_neglected > 0 {
            left_spans.push(Span::styled(
                format!("  {}\u{26a0}", self.total_neglected),
                Style::new().fg(CLR_YELLOW_SOFT).bold(),
            ));
        }
        let snoozed = self.snoozed_count();
        if snoozed > 0 {
            left_spans.push(Span::styled(
                format!("  {} snoozed", snoozed),
                STYLES.status_bar,
            ));
        }
        if !filter_label.is_empty() {
            left_spans.push(Span::styled(filter_label, STYLES.status_bar));
        }

        // Show active search filter indicator
        if let Some(ref q) = self.search.query {
            let visible = self.visible_tensions().len();
            let total = self.tensions.len();
            left_spans.push(Span::styled(
                format!("  /\"{}\" ({}/{})", q, visible, total),
                STYLES.status_bar,
            ));
        }

        // Flatten spans into a single string for StatusItem (StatusLine takes &str)
        let left: String = left_spans.iter().map(|s| s.content.to_string()).collect();

        // Right side: top 2 urgent tensions
        let mut urgent: Vec<_> = self.tensions.iter()
            .filter(|t| t.urgency.is_some() && t.tier != UrgencyTier::Resolved)
            .collect();
        urgent.sort_by(|a, b|
            b.urgency.unwrap_or(0.0).partial_cmp(&a.urgency.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        );

        let right_text: String = urgent.iter().take(2).enumerate().map(|(i, t)| {
            let pct = (t.urgency.unwrap_or(0.0) * 100.0).min(999.0) as u32;
            let name = truncate(&t.desired, 15);
            if i > 0 { format!("  {}% {}", pct, name) } else { format!("{}% {}", pct, name) }
        }).collect();

        let status = StatusLine::new()
            .left(StatusItem::text(&left))
            .right(StatusItem::text(&right_text))
            .style(STYLES.status_bar);
        status.render(*area, frame);
    }

    pub(crate) fn render_tension_list(&self, area: &Rect, frame: &mut Frame<'_>) {
        let visible = self.visible_tensions();
        if visible.is_empty() {
            let message = if self.search.query.is_some() {
                "  No matching tensions. Press Esc to clear search, f to change filter."
            } else if self.tensions.is_empty() {
                "  No tensions yet. Press `a` to create your first."
            } else {
                "  No matching tensions. Press `f` to change filter."
            };
            let msg = Paragraph::new(Text::from_spans([Span::styled(
                message,
                STYLES.label,
            )]));
            msg.render(*area, frame);
            return;
        }

        let width = area.width as usize;
        let show_trail = width >= 50;

        // Build rows with tier section headers inserted when the tier changes.
        let mut rows: Vec<Row> = Vec::new();
        let mut current_tier: Option<UrgencyTier> = None;
        let mut headers_before_selected: usize = 0;
        let selected_tension_idx = self.selected();

        let num_cols = if show_trail { 4 } else { 3 };

        let mut tension_idx: usize = 0;
        for row in &visible {
            // Insert tier header when tier changes — with blank line before for breathing room
            if current_tier != Some(row.tier) {
                // Add blank separator between tiers (not before the first one)
                if current_tier.is_some() {
                    rows.push(Row::new(vec![String::new(); num_cols]));
                    if tension_idx <= selected_tension_idx {
                        headers_before_selected += 1;
                    }
                }
                current_tier = Some(row.tier);
                let (header_text, header_style) = match row.tier {
                    UrgencyTier::Urgent => (
                        "\u{25b2} URGENT",
                        Style::new().fg(CLR_WHITE).bg(CLR_RED_SOFT).bold(),
                    ),
                    UrgencyTier::Active => (
                        "\u{25cf} ACTIVE",
                        Style::new().fg(CLR_LIGHT_GRAY).bold(),
                    ),
                    UrgencyTier::Neglected => (
                        "\u{26a0} NEGLECTED",
                        Style::new().fg(CLR_BG_DARK).bg(CLR_YELLOW_SOFT).bold(),
                    ),
                    UrgencyTier::Resolved => (
                        "\u{2713} RESOLVED",
                        Style::new().fg(CLR_DIM_GRAY).bold(),
                    ),
                };
                let mut cells = vec![String::new(); num_cols];
                cells[1] = header_text.to_string();
                rows.push(Row::new(cells).style(header_style));
                if tension_idx <= selected_tension_idx {
                    headers_before_selected += 1;
                }
            }

            // Build the tension data row — clean: selector, phase+movement, desired, horizon
            let tier_style = match row.tier {
                UrgencyTier::Urgent => Style::new().fg(CLR_RED_SOFT).bold(),
                UrgencyTier::Active => Style::new().fg(CLR_WHITE),
                UrgencyTier::Neglected => Style::new().fg(CLR_YELLOW_SOFT),
                UrgencyTier::Resolved => Style::new().fg(CLR_DIM_GRAY),
            };

            let selector = if tension_idx == selected_tension_idx {
                "\u{25b8}"
            } else {
                " "
            };

            // Phase glyph + movement: e.g. "◇→" or "◆↔"
            let indicator = format!("{}{}", row.phase, row.movement);

            if show_trail {
                let trail = activity_trail(&row.activity, 8);
                rows.push(
                    Row::new(vec![
                        selector.to_string(),
                        indicator,
                        row.desired.clone(),
                        trail,
                    ])
                    .style(tier_style),
                );
            } else {
                rows.push(
                    Row::new(vec![
                        selector.to_string(),
                        indicator,
                        row.desired.clone(),
                    ])
                    .style(tier_style),
                );
            }
            tension_idx += 1;
        }

        let widths: Vec<Constraint> = if show_trail {
            vec![
                Constraint::Fixed(2),  // selector
                Constraint::Fixed(3),  // phase glyph + movement
                Constraint::Fill,      // desired — gets ALL remaining space
                Constraint::Fixed(9),  // activity trail (8 dots + padding)
            ]
        } else {
            vec![
                Constraint::Fixed(2),  // selector
                Constraint::Fixed(3),  // phase glyph + movement
                Constraint::Fill,      // desired
            ]
        };

        let table = Table::new(rows, widths)
            .highlight_style(Style::new().fg(CLR_WHITE).bg(WERK_THEME.highlight).bold())
            .column_spacing(1);

        let adjusted_index = selected_tension_idx + headers_before_selected;
        let mut state = self.dashboard_state.borrow().clone();
        state.select(Some(adjusted_index));
        StatefulWidget::render(&table, *area, frame, &mut state);
        self.dashboard_state.borrow_mut().offset = state.offset;
    }

    pub(crate) fn render_dashboard_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = StatusLine::new()
            .separator("  ")
            .left(StatusItem::key_hint("?", "help"))
            .left(StatusItem::key_hint("Ctrl-/", "commands"))
            .style(STYLES.muted);
        hints.render(*area, frame);
    }
}
