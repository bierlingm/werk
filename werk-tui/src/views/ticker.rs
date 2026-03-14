use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::theme::*;
use crate::types::UrgencyTier;

#[allow(dead_code)]
impl WerkApp {
    pub(crate) fn render_urgency_ticker(&self, area: &Rect, frame: &mut Frame<'_>) {
        // Get top 3 most urgent tensions
        let mut urgent: Vec<_> = self.tensions.iter()
            .filter(|t| t.urgency.is_some() && t.tier != UrgencyTier::Resolved)
            .collect();
        urgent.sort_by(|a, b| {
            b.urgency.unwrap_or(0.0).partial_cmp(&a.urgency.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut spans: Vec<Span> = vec![Span::styled(" ", Style::new())];
        for (i, t) in urgent.iter().take(3).enumerate() {
            let pct = t.urgency.unwrap_or(0.0);
            let color = if pct > 0.9 { CLR_RED } else if pct > 0.75 { CLR_YELLOW } else { CLR_MID_GRAY };
            if i > 0 {
                spans.push(Span::styled("  |  ", Style::new().fg(CLR_DIM_GRAY)));
            }
            spans.push(Span::styled(
                format!("[{}] {:.0}% {}", i + 1, pct * 100.0, truncate(&t.desired, 20)),
                Style::new().fg(color),
            ));
        }

        // Add total count
        let active_count = self.tensions.iter().filter(|t| t.tier != UrgencyTier::Resolved).count();
        spans.push(Span::styled(
            format!("  {} active", active_count),
            Style::new().fg(CLR_DIM_GRAY),
        ));

        let line = Line::from_spans(spans);
        Paragraph::new(Text::from_lines(vec![line])).render(*area, frame);
    }
}
