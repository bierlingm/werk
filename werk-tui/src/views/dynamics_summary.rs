use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::block::Block;
use ftui::widgets::borders::BorderType;
use ftui::widgets::progress::MiniBar;
use ftui::widgets::status_line::{StatusLine, StatusItem};
use ftui::PackedRgba;
use crate::app::WerkApp;
use crate::theme::*;
use crate::types::UrgencyTier;

impl WerkApp {
    pub(crate) fn render_dynamics_title(&self, area: &Rect, frame: &mut Frame<'_>) {
        let count = self.tensions.iter().filter(|t| t.tier != UrgencyTier::Resolved).count();
        let left_text = format!(" Health  |  {} active tensions", count);
        let status = StatusLine::new()
            .left(StatusItem::text(&left_text))
            .style(Style::new().fg(CLR_LIGHT_GRAY).bold());
        status.render(*area, frame);
    }

    pub(crate) fn render_dynamics_body(&self, area: &Rect, frame: &mut Frame<'_>) {
        let active: Vec<_> = self.tensions.iter()
            .filter(|t| t.tier != UrgencyTier::Resolved)
            .collect();

        let total = active.len().max(1) as f64;

        // Phase distribution
        let germination = active.iter().filter(|t| t.phase == "G").count();
        let assimilation = active.iter().filter(|t| t.phase == "A").count();
        let completion = active.iter().filter(|t| t.phase == "C").count();
        let momentum = active.iter().filter(|t| t.phase == "M").count();

        // Movement distribution
        let advancing = active.iter().filter(|t| t.movement == "\u{2192}").count();
        let oscillating = active.iter().filter(|t| t.movement == "\u{2194}").count();
        let stagnant = active.iter().filter(|t| t.movement == "\u{25CB}").count();

        // Tier distribution
        let urgent = active.iter().filter(|t| t.tier == UrgencyTier::Urgent).count();
        let neglected = active.iter().filter(|t| t.tier == UrgencyTier::Neglected).count();

        let bar_w: u16 = 10;

        // Phase Distribution section
        let block = Block::bordered()
            .title(" Phase Distribution ")
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(CLR_DIM_GRAY));
        let section_h = 6u16; // 4 phases + 2 chrome
        if area.height >= section_h {
            let section = Rect::new(area.x, area.y, area.width, section_h);
            block.render(section, frame);
            let inner = Block::bordered().inner(section);

            let phase_data = [
                ("Germination ", germination, CLR_CYAN),
                ("Assimilation", assimilation, CLR_CYAN),
                ("Completion  ", completion, CLR_GREEN),
                ("Momentum    ", momentum, CLR_CYAN),
            ];
            render_distribution_rows(&phase_data, total, bar_w, inner, frame);
        }

        // Movement Ratios section
        let y2 = area.y + section_h;
        if y2 + 5 <= area.bottom() {
            let block2 = Block::bordered()
                .title(" Movement Ratios ")
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(CLR_DIM_GRAY));
            let section2 = Rect::new(area.x, y2, area.width, 5);
            block2.render(section2, frame);
            let inner2 = Block::bordered().inner(section2);

            let mvmt_data = [
                ("Advancing   ", advancing, CLR_GREEN),
                ("Oscillating ", oscillating, CLR_YELLOW),
                ("Stagnant    ", stagnant, CLR_MID_GRAY),
            ];
            render_distribution_rows(&mvmt_data, total, bar_w, inner2, frame);
        }

        // Alerts section
        let y3 = y2 + 5;
        let remaining = area.bottom().saturating_sub(y3);
        if remaining >= 4 {
            let block3 = Block::bordered()
                .title(" Alerts ")
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(CLR_DIM_GRAY));
            let section3 = Rect::new(area.x, y3, area.width, remaining);
            block3.render(section3, frame);
            let inner3 = Block::bordered().inner(section3);

            let mut alert_lines: Vec<Line> = Vec::new();
            if urgent > 0 {
                alert_lines.push(Line::from_spans([Span::styled(
                    format!("  ! {} urgent tensions need attention", urgent),
                    Style::new().fg(CLR_RED_SOFT),
                )]));
            }
            if neglected > 0 {
                alert_lines.push(Line::from_spans([Span::styled(
                    format!("  ! {} tensions are neglected", neglected),
                    Style::new().fg(CLR_YELLOW_SOFT),
                )]));
            }
            if stagnant > 0 {
                alert_lines.push(Line::from_spans([Span::styled(
                    format!("  ~ {} tensions are stagnant", stagnant),
                    Style::new().fg(CLR_MID_GRAY),
                )]));
            }
            if alert_lines.is_empty() {
                alert_lines.push(Line::from_spans([Span::styled(
                    "  No alerts. All systems healthy.",
                    Style::new().fg(CLR_GREEN),
                )]));
            }

            // System-wide activity sparkline
            let all_activity: Vec<f64> = (0..7).map(|day| {
                active.iter().map(|t| t.activity.get(day).copied().unwrap_or(0.0)).sum()
            }).collect();
            let sparkline = mini_sparkline(&all_activity, 30);
            alert_lines.push(Line::from(""));
            alert_lines.push(Line::from_spans([
                Span::styled("  Activity (7d): ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(sparkline, Style::new().fg(CLR_CYAN)),
            ]));

            Paragraph::new(Text::from_lines(alert_lines)).render(inner3, frame);
        }
    }

    pub(crate) fn render_dynamics_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = StatusLine::new()
            .separator("  ")
            .left(StatusItem::key_hint("Esc", "back"))
            .left(StatusItem::key_hint("1", "dashboard"))
            .left(StatusItem::key_hint("q", "quit"))
            .left(StatusItem::key_hint("?", "help"))
            .style(Style::new().fg(CLR_MID_GRAY));
        hints.render(*area, frame);
    }
}

/// Render distribution rows using MiniBar widgets for each entry.
/// Each row: "  Label  [===bar===] count"
fn render_distribution_rows(
    data: &[(&str, usize, PackedRgba)],
    total: f64,
    bar_w: u16,
    inner: ftui::layout::Rect,
    frame: &mut ftui::Frame<'_>,
) {
    use ftui::widgets::progress::MiniBarColors;

    let label_col_w: u16 = 16; // "  Label       " width
    let count_col_w: u16 = 5;  // " NNN" width
    let bar_actual = bar_w.min(inner.width.saturating_sub(label_col_w + count_col_w));

    for (i, (label, count, color)) in data.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.bottom() {
            break;
        }

        // Render label as a Span via Paragraph on a 1-row rect
        let label_area = ftui::layout::Rect::new(inner.x, y, label_col_w.min(inner.width), 1);
        let label_text = format!("  {} ", label);
        let label_para = Paragraph::new(Text::from_spans([
            Span::styled(&label_text, Style::new().fg(CLR_MID_GRAY)),
        ]));
        label_para.render(label_area, frame);

        // Render MiniBar
        let bar_x = inner.x + label_col_w;
        if bar_actual > 0 && bar_x < inner.right() {
            let bar_area = ftui::layout::Rect::new(bar_x, y, bar_actual, 1);
            let ratio = *count as f64 / total;
            // Use uniform color for this bar by setting all thresholds to use the same color
            let uniform_colors = MiniBarColors::new(*color, *color, *color, *color);
            let mini_bar = MiniBar::new(ratio, bar_actual)
                .colors(uniform_colors);
            mini_bar.render(bar_area, frame);
        }

        // Render count
        let count_x = inner.x + label_col_w + bar_actual;
        if count_x < inner.right() {
            let count_area = ftui::layout::Rect::new(count_x, y, inner.right().saturating_sub(count_x), 1);
            let count_text = format!(" {}", count);
            let count_para = Paragraph::new(Text::from_spans([
                Span::styled(&count_text, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
            count_para.render(count_area, frame);
        }
    }
}

fn mini_sparkline(data: &[f64], width: usize) -> String {
    let blocks = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
    let max = data.iter().cloned().fold(0.0f64, f64::max).max(1.0);
    data.iter()
        .take(width)
        .map(|&v| {
            let idx = ((v / max) * 7.0).round().min(7.0) as usize;
            blocks[idx]
        })
        .collect()
}
