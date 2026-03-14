use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::modal::{Modal, ModalPosition, ModalSizeConstraints};

use werk_shared::truncate;

use crate::app::{WerkApp, WhatIfAction};
use crate::theme::*;

impl WerkApp {
    pub(crate) fn render_what_if_overlay(&self, area: Rect, frame: &mut Frame<'_>) {
        let preview = match &self.what_if_preview {
            Some(p) => p,
            None => return,
        };

        let action_label = match preview.action {
            WhatIfAction::Resolve => "Resolve",
            WhatIfAction::Release => "Release",
        };

        let confirm_key = match preview.action {
            WhatIfAction::Resolve => "R",
            WhatIfAction::Release => "X",
        };

        let desired_display = truncate(&preview.tension_desired, 36);

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from_spans([Span::styled(
            format!(" {} '{}'?", action_label, desired_display),
            Style::new().fg(CLR_CYAN).bold(),
        )]));
        lines.push(Line::from(""));

        if preview.orphaned_children.is_empty() && preview.auto_resolved_parents.is_empty() {
            lines.push(Line::from_spans([Span::styled(
                "  No cascading effects.",
                Style::new().fg(CLR_MID_GRAY),
            )]));
        }

        if !preview.orphaned_children.is_empty() {
            lines.push(Line::from_spans([Span::styled(
                format!("  {} active child{} will continue:",
                    preview.children_count,
                    if preview.children_count == 1 { "" } else { "ren" }),
                Style::new().fg(CLR_YELLOW_SOFT),
            )]));
            for child in &preview.orphaned_children {
                lines.push(Line::from_spans([Span::styled(
                    format!("    \u{2022} {}", child),
                    Style::new().fg(CLR_LIGHT_GRAY),
                )]));
            }
        }

        if !preview.auto_resolved_parents.is_empty() {
            if !preview.orphaned_children.is_empty() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from_spans([Span::styled(
                "  Parent will auto-resolve:",
                Style::new().fg(CLR_GREEN),
            )]));
            for parent in &preview.auto_resolved_parents {
                lines.push(Line::from_spans([Span::styled(
                    format!("    \u{2713} {}", parent),
                    Style::new().fg(CLR_GREEN),
                )]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from_spans([Span::styled(
            format!("  {} to confirm  |  Esc to cancel", confirm_key),
            Style::new().fg(CLR_DIM_GRAY),
        )]));

        let line_count = lines.len() as u16;

        let bg_style = Style::new().fg(CLR_LIGHT_GRAY).bg(CLR_BG_DARK);
        let content = Paragraph::new(Text::from_lines(lines)).style(bg_style);
        let modal = Modal::new(content)
            .position(ModalPosition::Center)
            .size(
                ModalSizeConstraints::new()
                    .max_width(50)
                    .max_height(line_count.saturating_add(2)),
            );
        modal.render(area, frame);
    }
}
