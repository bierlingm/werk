use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::modal::{Modal, ModalPosition, ModalSizeConstraints};
use ftui::widgets::progress::MiniBar;

use crate::app::WerkApp;
use crate::theme::*;
use crate::types::UrgencyTier;

impl WerkApp {
    pub(crate) fn render_health_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
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

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from_spans([Span::styled(
            format!(" Health  |  {} active tensions", active.len()),
            Style::new().fg(CLR_CYAN).bold(),
        )]));
        lines.push(Line::from(""));

        // Phase Distribution
        lines.push(Line::from_spans([Span::styled(
            " Phase Distribution",
            Style::new().fg(CLR_LIGHT_GRAY).bold(),
        )]));
        for (label, count, color) in [
            ("  Germination ", germination, CLR_CYAN),
            ("  Assimilation", assimilation, CLR_CYAN),
            ("  Completion  ", completion, CLR_GREEN),
            ("  Momentum    ", momentum, CLR_CYAN),
        ] {
            let bar = MiniBar::new(count as f64 / total, 10);
            let bar_str = bar.render_string();
            lines.push(Line::from_spans([
                Span::styled(label, Style::new().fg(CLR_MID_GRAY)),
                Span::styled(" ", Style::new()),
                Span::styled(bar_str, Style::new().fg(color)),
                Span::styled(format!(" {}", count), Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
        }

        lines.push(Line::from(""));

        // Movement Ratios
        lines.push(Line::from_spans([Span::styled(
            " Movement Ratios",
            Style::new().fg(CLR_LIGHT_GRAY).bold(),
        )]));
        for (label, count, color) in [
            ("  Advancing   ", advancing, CLR_GREEN),
            ("  Oscillating ", oscillating, CLR_YELLOW),
            ("  Stagnant    ", stagnant, CLR_MID_GRAY),
        ] {
            let bar = MiniBar::new(count as f64 / total, 10);
            let bar_str = bar.render_string();
            lines.push(Line::from_spans([
                Span::styled(label, Style::new().fg(CLR_MID_GRAY)),
                Span::styled(" ", Style::new()),
                Span::styled(bar_str, Style::new().fg(color)),
                Span::styled(format!(" {}", count), Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
        }

        lines.push(Line::from(""));

        // Alerts
        lines.push(Line::from_spans([Span::styled(
            " Alerts",
            Style::new().fg(CLR_LIGHT_GRAY).bold(),
        )]));
        let mut has_alerts = false;
        if urgent > 0 {
            lines.push(Line::from_spans([Span::styled(
                format!("  ! {} urgent tensions need attention", urgent),
                Style::new().fg(CLR_RED_SOFT),
            )]));
            has_alerts = true;
        }
        if neglected > 0 {
            lines.push(Line::from_spans([Span::styled(
                format!("  ! {} tensions are neglected", neglected),
                Style::new().fg(CLR_YELLOW_SOFT),
            )]));
            has_alerts = true;
        }
        if stagnant > 0 {
            lines.push(Line::from_spans([Span::styled(
                format!("  ~ {} tensions are stagnant", stagnant),
                Style::new().fg(CLR_MID_GRAY),
            )]));
            has_alerts = true;
        }
        if !has_alerts {
            lines.push(Line::from_spans([Span::styled(
                "  No alerts. All systems healthy.",
                Style::new().fg(CLR_GREEN),
            )]));
        }

        // System-wide activity sparkline
        let all_activity: Vec<f64> = (0..7).map(|day| {
            active.iter().map(|t| t.activity.get(day).copied().unwrap_or(0.0)).sum()
        }).collect();
        let sparkline = mini_sparkline_overlay(&all_activity);
        lines.push(Line::from(""));
        lines.push(Line::from_spans([
            Span::styled("  Activity (7d): ", Style::new().fg(CLR_MID_GRAY)),
            Span::styled(sparkline, Style::new().fg(CLR_CYAN)),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from_spans([Span::styled(
            "  Press D or Esc to close",
            Style::new().fg(CLR_DIM_GRAY),
        )]));

        let line_count = lines.len() as u16;

        let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
        let content = Paragraph::new(Text::from_lines(lines)).style(bg_style);
        let modal = Modal::new(content)
            .position(ModalPosition::Center)
            .size(
                ModalSizeConstraints::new()
                    .max_width(55)
                    .max_height(line_count.saturating_add(2)),
            );
        modal.render(area, frame);
    }
}

fn mini_sparkline_overlay(data: &[f64]) -> String {
    let blocks = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
    let max = data.iter().cloned().fold(0.0f64, f64::max).max(1.0);
    data.iter()
        .map(|&v| {
            let idx = ((v / max) * 7.0).round().min(7.0) as usize;
            blocks[idx]
        })
        .collect()
}
