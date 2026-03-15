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
use crate::helpers::sparkline_block_color;
use crate::theme::*;
use crate::types::UrgencyTier;

fn trajectory_char(trajectory: &Option<sd_core::Trajectory>) -> &'static str {
    match trajectory {
        Some(sd_core::Trajectory::Resolving) => "\u{2193}",
        Some(sd_core::Trajectory::Stalling) => "\u{2014}",
        Some(sd_core::Trajectory::Drifting) => "~",
        Some(sd_core::Trajectory::Oscillating) => "\u{21cc}",
        None => " ",
    }
}

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
    // Pad if data is shorter than width
    let pad = width.saturating_sub(s.chars().count());
    format!("{}{}", " ".repeat(pad), s)
}

#[allow(dead_code)]
fn colored_sparkline_spans(data: &[f64], width: usize) -> Vec<Span<'_>> {
    let blocks = [' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
    if data.is_empty() {
        return vec![Span::styled(" ".repeat(width), Style::new())];
    }
    let max = data.iter().cloned().fold(0.0f64, f64::max).max(1.0);
    let pad = width.saturating_sub(data.len());
    let mut spans = Vec::new();
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), Style::new()));
    }
    for &v in data.iter().take(width) {
        let idx = ((v / max) * 8.0).round().min(8.0) as usize;
        let color = sparkline_block_color(v, max);
        spans.push(Span::styled(
            blocks[idx].to_string(),
            Style::new().fg(color),
        ));
    }
    spans
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

        // Build rows with tier section headers inserted when the tier changes.
        let mut rows: Vec<Row> = Vec::new();
        let mut current_tier: Option<UrgencyTier> = None;
        let mut headers_before_selected: usize = 0;
        let selected_tension_idx = self.selected();

        // Progressive disclosure breakpoints (Phase 2a)
        let num_cols = if width < 40 {
            3  // selector + phase + desired
        } else if width < 60 {
            7  // + movement, traj, horizon, urgency%
        } else if width < 80 {
            8  // + urgency bar
        } else if width < 100 {
            9  // + sparkline
        } else {
            10 // + urgency bar (wider)
        };

        // Column header row
        {
            let header_style = Style::new().fg(CLR_MID_GRAY).bold();
            let header_cells: Vec<String> = if width < 40 {
                vec!["".to_string(), "".to_string(), "Tension".to_string()]
            } else if width < 60 {
                vec!["".to_string(), "".to_string(), "".to_string(), "".to_string(),
                     "Tension".to_string(), "Horizon".to_string(), "Urg".to_string()]
            } else if width >= 80 {
                vec!["".to_string(), "".to_string(), "".to_string(), "".to_string(),
                     "Tension".to_string(), "Activity".to_string(), "Horizon".to_string(),
                     "Urgency".to_string(), "".to_string()]
            } else {
                vec!["".to_string(), "".to_string(), "".to_string(), "".to_string(),
                     "Tension".to_string(), "Horizon".to_string(),
                     "Urgency".to_string(), "".to_string()]
            };
            rows.push(Row::new(header_cells).style(header_style));
            // Header counts toward offset before selected
            headers_before_selected += 1;
        }

        let mut tension_idx: usize = 0;
        for row in &visible {
            // Insert tier header with badge styling (Phase 1c)
            if current_tier != Some(row.tier) {
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
                if num_cols >= 3 {
                    cells[1] = String::new(); // selector column
                    cells[2] = header_text.to_string();
                } else if num_cols >= 2 {
                    cells[1] = header_text.to_string();
                } else {
                    cells[0] = header_text.to_string();
                }
                rows.push(Row::new(cells).style(header_style));
                if tension_idx <= selected_tension_idx {
                    headers_before_selected += 1;
                }
            }

            // Build the tension data row
            let tier_style = match row.tier {
                UrgencyTier::Urgent => Style::new().fg(CLR_RED_SOFT).bold(),
                UrgencyTier::Active => Style::new().fg(CLR_WHITE),
                UrgencyTier::Neglected => Style::new().fg(CLR_YELLOW_SOFT),
                UrgencyTier::Resolved => Style::new().fg(CLR_DIM_GRAY),
            };

            // Selection indicator (Phase 2b)
            let selector = if tension_idx == selected_tension_idx {
                "\u{25b8} "
            } else {
                "  "
            };

            let phase_str = format!("[{}]", row.phase);
            let urgency_pct = match row.urgency {
                Some(u) => format!("{:>3.0}%", (u * 100.0).min(999.0)),
                None => "  --".to_string(),
            };

            if width < 40 {
                let desired_width = width.saturating_sub(10).max(5);
                let desired_trunc = truncate(&row.desired, desired_width);
                rows.push(
                    Row::new(vec![
                        selector.to_string(),
                        phase_str,
                        desired_trunc.to_string(),
                    ])
                    .style(tier_style),
                );
            } else if width < 60 {
                let traj = trajectory_char(&row.trajectory);
                let fixed_width = 2 + 4 + 2 + 2 + 11 + 5;
                let desired_width = width.saturating_sub(fixed_width).max(10);
                let desired_trunc = truncate(&row.desired, desired_width);
                rows.push(
                    Row::new(vec![
                        selector.to_string(),
                        phase_str,
                        row.movement.clone(),
                        traj.to_string(),
                        desired_trunc.to_string(),
                        format!("{:>10}", row.horizon_display),
                        urgency_pct,
                    ])
                    .style(tier_style),
                );
            } else {
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
                let traj = trajectory_char(&row.trajectory);
                if width >= 80 {
                    let spark = mini_sparkline(&row.activity, 7);
                    rows.push(
                        Row::new(vec![
                            selector.to_string(),
                            phase_str,
                            row.movement.clone(),
                            traj.to_string(),
                            row.desired.clone(),
                            spark,
                            format!("{:>11}", row.horizon_display),
                            urgency_bar,
                            urgency_pct,
                        ])
                        .style(tier_style),
                    );
                } else {
                    rows.push(
                        Row::new(vec![
                            selector.to_string(),
                            phase_str,
                            row.movement.clone(),
                            traj.to_string(),
                            row.desired.clone(),
                            format!("{:>11}", row.horizon_display),
                            urgency_bar,
                            urgency_pct,
                        ])
                        .style(tier_style),
                    );
                }
            }
            tension_idx += 1;
        }

        let widths: Vec<Constraint> = if width < 40 {
            vec![
                Constraint::Fixed(2),  // selector
                Constraint::Fixed(4),
                Constraint::Fill,
            ]
        } else if width < 60 {
            vec![
                Constraint::Fixed(2),  // selector
                Constraint::Fixed(4),
                Constraint::Fixed(2),
                Constraint::Fixed(2),
                Constraint::Fill,
                Constraint::Fixed(11),
                Constraint::Fixed(5),
            ]
        } else if width >= 80 {
            vec![
                Constraint::Fixed(2),  // selector
                Constraint::Fixed(4),
                Constraint::Fixed(2),
                Constraint::Fixed(2),
                Constraint::Fill,
                Constraint::Fixed(8),
                Constraint::Fixed(12),
                Constraint::Fixed(7),
                Constraint::Fixed(5),
            ]
        } else {
            vec![
                Constraint::Fixed(2),  // selector
                Constraint::Fixed(4),
                Constraint::Fixed(2),
                Constraint::Fixed(2),
                Constraint::Fill,
                Constraint::Fixed(12),
                Constraint::Fixed(7),
                Constraint::Fixed(5),
            ]
        };

        // Selection highlight with subtle background (Phase 2b)
        let table = Table::new(rows, widths)
            .highlight_style(Style::new().fg(CLR_WHITE).bg(WERK_THEME.highlight).bold())
            .column_spacing(1);

        let adjusted_index = selected_tension_idx + headers_before_selected;
        let mut state = self.dashboard_state.borrow().clone();
        state.select(Some(adjusted_index));
        StatefulWidget::render(&table, *area, frame, &mut state);
        // Write back offset so scrolling works correctly
        self.dashboard_state.borrow_mut().offset = state.offset;
    }

    pub(crate) fn render_dashboard_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let width = area.width as usize;
        let filter_hint = format!("f[{}]", self.filter.label());

        // Adaptive hints based on width (Phase 5a)
        let mut hints = StatusLine::new().separator("  ");

        // Essential hints (always shown)
        hints = hints
            .left(StatusItem::key_hint("j/k", ""))
            .left(StatusItem::key_hint("Enter", "detail"))
            .left(StatusItem::key_hint("Tab", "tree"))
            .left(StatusItem::text(&filter_hint))
            .left(StatusItem::key_hint("a", "add"));

        if width >= 60 {
            hints = hints
                .left(StatusItem::key_hint("c/p", "child/parent"))
                .left(StatusItem::key_hint("r/d", "edit"))
                .left(StatusItem::key_hint("w", "reflect"));
        }
        if width >= 100 {
            hints = hints
                .left(StatusItem::key_hint("</>", "split"))
                .left(StatusItem::key_hint("T", "timeline"))
                .left(StatusItem::key_hint("D", "health"))
                .left(StatusItem::key_hint("L", "lever"));
        }
        hints = hints.left(StatusItem::key_hint("q/?", ""));
        hints = hints.style(STYLES.label);
        hints.render(*area, frame);
    }
}
