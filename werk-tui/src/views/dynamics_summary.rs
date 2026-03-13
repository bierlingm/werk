use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::block::Block;
use ftui::widgets::borders::BorderType;
use crate::app::WerkApp;
use crate::theme::*;
use crate::types::UrgencyTier;

impl WerkApp {
    pub(crate) fn render_dynamics_title(&self, area: &Rect, frame: &mut Frame<'_>) {
        let count = self.tensions.iter().filter(|t| t.tier != UrgencyTier::Resolved).count();
        let title = format!(" Health  |  {} active tensions", count);
        let style = Style::new().fg(CLR_LIGHT_GRAY).bold();
        Paragraph::new(Text::from_spans([Span::styled(&title, style)])).render(*area, frame);
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

        let bar_w = 10;

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

            let phase_lines = vec![
                format_distribution_line("Germination ", germination, total, bar_w, CLR_CYAN),
                format_distribution_line("Assimilation", assimilation, total, bar_w, CLR_CYAN),
                format_distribution_line("Completion  ", completion, total, bar_w, CLR_GREEN),
                format_distribution_line("Momentum    ", momentum, total, bar_w, CLR_CYAN),
            ];
            Paragraph::new(Text::from_lines(phase_lines)).render(inner, frame);
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

            let mvmt_lines = vec![
                format_distribution_line("Advancing   ", advancing, total, bar_w, CLR_GREEN),
                format_distribution_line("Oscillating ", oscillating, total, bar_w, CLR_YELLOW),
                format_distribution_line("Stagnant    ", stagnant, total, bar_w, CLR_MID_GRAY),
            ];
            Paragraph::new(Text::from_lines(mvmt_lines)).render(inner2, frame);
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
        let hints = " Esc back  1 dashboard  q quit  ? help";
        Paragraph::new(Text::from_spans([Span::styled(hints, Style::new().fg(CLR_MID_GRAY))]))
            .render(*area, frame);
    }
}

fn format_distribution_line(label: &str, count: usize, total: f64, bar_w: usize, color: ftui::PackedRgba) -> Line {
    let ratio = count as f64 / total;
    let filled = (ratio * bar_w as f64).round() as usize;
    let empty = bar_w.saturating_sub(filled);
    let bar = format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty));
    Line::from_spans([
        Span::styled(format!("  {} ", label), Style::new().fg(CLR_MID_GRAY)),
        Span::styled(bar, Style::new().fg(color)),
        Span::styled(format!(" {}", count), Style::new().fg(CLR_LIGHT_GRAY)),
    ])
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
