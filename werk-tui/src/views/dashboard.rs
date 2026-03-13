use ftui::Frame;
use ftui::layout::{Constraint, Rect};
use ftui::text::{Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::StatefulWidget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::table::{Row, Table};

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::theme::*;
use crate::types::UrgencyTier;

fn mini_sparkline(data: &[f64], width: usize) -> String {
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
    // Pad if data is shorter than width
    let pad = width.saturating_sub(s.chars().count());
    format!("{}{}", " ".repeat(pad), s)
}

impl WerkApp {
    pub(crate) fn render_title_bar(&self, area: &Rect, frame: &mut Frame<'_>) {
        let filter_label = if self.filter != crate::types::Filter::Active {
            format!("  [{}]", self.filter.label())
        } else {
            String::new()
        };
        let status = format!(
            " werk  |  {} active  {} urgent  {} neglected  {} resolved  {} released{}",
            self.total_active,
            self.total_urgent,
            self.total_neglected,
            self.total_resolved,
            self.total_released,
            filter_label,
        );
        let style = Style::new().fg(CLR_LIGHT_GRAY).bold();
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(&status, style)]));
        paragraph.render(*area, frame);
    }

    pub(crate) fn render_tension_list(&self, area: &Rect, frame: &mut Frame<'_>) {
        let visible = self.visible_tensions();
        if visible.is_empty() {
            let message = if self.search_query.is_some() {
                "  No matching tensions. Press Esc to clear search, f to change filter."
            } else if self.tensions.is_empty() {
                "  No tensions yet. Press `a` to create your first."
            } else {
                "  No matching tensions. Press `f` to change filter."
            };
            let msg = Paragraph::new(Text::from_spans([Span::styled(
                message,
                Style::new().fg(CLR_MID_GRAY),
            )]));
            msg.render(*area, frame);
            return;
        }

        let width = area.width as usize;

        let rows: Vec<Row> = visible
            .iter()
            .map(|row| {
                let tier_style = match row.tier {
                    UrgencyTier::Urgent => Style::new().fg(CLR_RED_SOFT),
                    UrgencyTier::Active => Style::new().fg(CLR_LIGHT_GRAY),
                    UrgencyTier::Neglected => Style::new().fg(CLR_YELLOW_SOFT),
                    UrgencyTier::Resolved => Style::new().fg(CLR_DIM_GRAY),
                };

                let phase_str = format!("[{}]", row.phase);
                let urgency_pct = match row.urgency {
                    Some(u) => format!("{:>3.0}%", (u * 100.0).min(999.0)),
                    None => "  --".to_string(),
                };

                if width < 40 {
                    // Very narrow: phase + desired only
                    let desired_width = width.saturating_sub(8).max(5);
                    let desired_trunc = truncate(&row.desired, desired_width);
                    Row::new(vec![
                        phase_str,
                        desired_trunc.to_string(),
                    ])
                    .style(tier_style)
                } else if width < 60 {
                    // Narrow: phase + movement + desired + horizon + urgency%
                    let fixed_width = 4 + 2 + 12 + 5;
                    let desired_width = width.saturating_sub(fixed_width).max(10);
                    let desired_trunc = truncate(&row.desired, desired_width);
                    Row::new(vec![
                        phase_str,
                        row.movement.clone(),
                        desired_trunc.to_string(),
                        format!("{:>11}", row.horizon_display),
                        urgency_pct,
                    ])
                    .style(tier_style)
                } else {
                    // Full width: all columns
                    let urgency_bar = match row.urgency {
                        Some(u) => {
                            let filled = ((u * 6.0).round() as usize).min(6);
                            let empty = 6 - filled;
                            format!(
                                "{}{}",
                                "\u{2588}".repeat(filled),
                                "\u{2591}".repeat(empty),
                            )
                        }
                        None => "------".to_string(),
                    };
                    if width >= 80 {
                        let spark = mini_sparkline(&row.activity, 7);
                        Row::new(vec![
                            phase_str,
                            row.movement.clone(),
                            row.desired.clone(),
                            spark,
                            format!("{:>11}", row.horizon_display),
                            urgency_bar,
                            urgency_pct,
                        ])
                        .style(tier_style)
                    } else {
                        Row::new(vec![
                            phase_str,
                            row.movement.clone(),
                            row.desired.clone(),
                            format!("{:>11}", row.horizon_display),
                            urgency_bar,
                            urgency_pct,
                        ])
                        .style(tier_style)
                    }
                }
            })
            .collect();

        let widths: Vec<Constraint> = if width < 40 {
            vec![
                Constraint::Fixed(4),
                Constraint::Fill,
            ]
        } else if width < 60 {
            vec![
                Constraint::Fixed(4),
                Constraint::Fixed(2),
                Constraint::Fill,
                Constraint::Fixed(12),
                Constraint::Fixed(5),
            ]
        } else if width >= 80 {
            vec![
                Constraint::Fixed(4),
                Constraint::Fixed(2),
                Constraint::Fill,
                Constraint::Fixed(8),
                Constraint::Fixed(12),
                Constraint::Fixed(7),
                Constraint::Fixed(5),
            ]
        } else {
            vec![
                Constraint::Fixed(4),
                Constraint::Fixed(2),
                Constraint::Fill,
                Constraint::Fixed(12),
                Constraint::Fixed(7),
                Constraint::Fixed(5),
            ]
        };

        let table = Table::new(rows, widths)
            .highlight_style(Style::new().fg(CLR_WHITE).bold())
            .column_spacing(1);

        let mut state = self.dashboard_state.borrow_mut();
        StatefulWidget::render(&table, *area, frame, &mut state);
    }

    pub(crate) fn render_dashboard_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = format!(
            " j/k  Enter detail  t tree  f[{}]  a add  c/p child/parent  r/d edit  w reflect  F focus  T timeline  D health  N graph  L lever  q/?",
            self.filter.label()
        );
        let style = Style::new().fg(CLR_MID_GRAY);
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(&hints, style)]));
        paragraph.render(*area, frame);
    }
}
