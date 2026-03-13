use chrono::Utc;
use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::block::Block;
use ftui::widgets::borders::BorderType;

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::helpers::render_bar;
use crate::theme::*;
use crate::types::MutationKind;

impl WerkApp {
    pub(crate) fn render_detail_title(&self, area: &Rect, frame: &mut Frame<'_>) {
        let title = match &self.detail_tension {
            Some(t) => {
                let short_id: String = t.id.chars().take(8).collect();
                format!(
                    " {}  {}",
                    truncate(&t.desired, area.width.saturating_sub(12) as usize),
                    short_id,
                )
            }
            None => " Detail".to_string(),
        };
        let style = Style::new().fg(CLR_WHITE).bold();
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(&title, style)]));
        paragraph.render(*area, frame);
    }

    /// Build a Block with rounded borders and a title, using the dim gray border color.
    fn section_block(title: &str) -> Block<'_> {
        Block::bordered()
            .title(title)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(CLR_DIM_GRAY))
    }

    /// Build the info section lines (desired, actual, status, horizon).
    fn build_info_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        if let Some(tension) = &self.detail_tension {
            let now = Utc::now();

            // Breadcrumb (ancestor chain)
            if !self.detail_ancestors.is_empty() {
                let mut crumbs: Vec<Span> = Vec::new();
                for (i, (_, desired)) in self.detail_ancestors.iter().enumerate() {
                    if i > 0 {
                        crumbs.push(Span::styled(" > ", Style::new().fg(CLR_DIM_GRAY)));
                    }
                    crumbs.push(Span::styled(
                        truncate(desired, 20).to_string(),
                        Style::new().fg(CLR_MID_GRAY),
                    ));
                }
                lines.push(Line::from_spans(crumbs));
            }

            // Parent line
            if let Some(parent) = &self.detail_parent {
                lines.push(Line::from_spans([
                    Span::styled("Parent   ", Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(
                        truncate(&parent.desired, 40).to_string(),
                        Style::new().fg(CLR_CYAN),
                    ),
                ]));
            }

            lines.push(Line::from_spans([
                Span::styled("Desired  ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(&tension.desired, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
            lines.push(Line::from_spans([
                Span::styled("Actual   ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(&tension.actual, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
            lines.push(Line::from_spans([
                Span::styled("Status   ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(tension.status.to_string(), Style::new().fg(CLR_LIGHT_GRAY)),
                Span::styled("       Created  ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(
                    werk_shared::relative_time(tension.created_at, now),
                    Style::new().fg(CLR_LIGHT_GRAY),
                ),
            ]));
            let horizon_str = match &tension.horizon {
                Some(h) => {
                    let remaining = h.range_end().signed_duration_since(now).num_days();
                    if remaining < 0 {
                        format!("{}  ({}d past)", h, -remaining)
                    } else if remaining == 0 {
                        format!("{}  (today)", h)
                    } else {
                        format!("{}  ({}d remaining)", h, remaining)
                    }
                }
                None => "\u{2014}".to_string(),
            };
            lines.push(Line::from_spans([
                Span::styled("Horizon  ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(horizon_str, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
        }
        lines
    }

    /// Build the dynamics section lines.
    fn build_dynamics_lines(&self, suppress_verbose: bool) -> Vec<Line> {
        let mut lines = Vec::new();
        if let Some(dyn_display) = &self.detail_dynamics {
            lines.push(Line::from_spans([
                Span::styled("Phase       ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(&dyn_display.phase, Style::new().fg(CLR_LIGHT_GRAY)),
                Span::styled("        Movement    ", Style::new().fg(CLR_MID_GRAY)),
                Span::styled(&dyn_display.movement, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));

            if let Some(mag) = dyn_display.magnitude {
                let bar = render_bar(mag, 10);
                lines.push(Line::from_spans([
                    Span::styled("Magnitude   ", Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(bar, Style::new().fg(CLR_CYAN)),
                    Span::styled(format!(" {:.2}", mag), Style::new().fg(CLR_LIGHT_GRAY)),
                ]));
            }

            if let Some(urg) = dyn_display.urgency {
                let bar = render_bar(urg.min(1.0), 10);
                lines.push(Line::from_spans([
                    Span::styled("Urgency     ", Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(bar, Style::new().fg(
                        if urg > 0.75 { CLR_RED_SOFT } else { CLR_YELLOW_SOFT }
                    )),
                    Span::styled(format!(" {:.0}%", (urg * 100.0).min(999.0)), Style::new().fg(CLR_LIGHT_GRAY)),
                ]));
            }

            if let Some(conflict) = &dyn_display.conflict {
                lines.push(Line::from_spans([
                    Span::styled("Conflict    ", Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(conflict, Style::new().fg(CLR_RED_SOFT)),
                ]));
            }

            if let Some(neglect) = &dyn_display.neglect {
                lines.push(Line::from_spans([
                    Span::styled("Neglect     ", Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(neglect, Style::new().fg(CLR_YELLOW_SOFT)),
                ]));
            }

            if self.verbose && !suppress_verbose {
                lines.push(Line::from(""));
                lines.push(Line::from_spans([Span::styled(
                    "Verbose Dynamics",
                    Style::new().fg(CLR_DIM_GRAY),
                )]));

                if let Some(v) = &dyn_display.oscillation {
                    lines.push(Line::from_spans([
                        Span::styled("Oscillation         ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                    ]));
                }
                if let Some(v) = &dyn_display.resolution {
                    lines.push(Line::from_spans([
                        Span::styled("Resolution          ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                    ]));
                }
                if let Some(v) = &dyn_display.orientation {
                    lines.push(Line::from_spans([
                        Span::styled("Orientation         ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                    ]));
                }
                if let Some(v) = &dyn_display.compensating_strategy {
                    lines.push(Line::from_spans([
                        Span::styled("Compensating Strat  ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                    ]));
                }
                if let Some(v) = &dyn_display.assimilation_depth {
                    lines.push(Line::from_spans([
                        Span::styled("Assimilation Depth  ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                    ]));
                }
                if let Some(v) = &dyn_display.horizon_drift {
                    lines.push(Line::from_spans([
                        Span::styled("Horizon Drift       ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(v, Style::new().fg(CLR_LIGHT_GRAY)),
                    ]));
                }
            }
        } else {
            lines.push(Line::from_spans([Span::styled(
                "No dynamics computed",
                Style::new().fg(CLR_DIM_GRAY),
            )]));
        }
        lines
    }

    /// Build the history section lines.
    fn build_history_lines(&self, width: usize) -> Vec<Line> {
        let mut lines = Vec::new();
        // Reserve space for time(14) + spacing
        let budget = width.saturating_sub(16).max(10);
        for m in &self.detail_mutations {
            let old_or_dash = m
                .old_value
                .as_deref()
                .map(|o| truncate(o, budget / 2).to_string())
                .unwrap_or_else(|| "\u{2014}".to_string());

            let (description, value_color) = match m.kind {
                MutationKind::Created => (
                    format!(
                        "Created \u{2014} Desired: \"{}\"",
                        truncate(&m.new_value, budget.saturating_sub(22))
                    ),
                    CLR_GREEN,
                ),
                MutationKind::StatusChange => (
                    format!("Status: {} \u{2192} {}", old_or_dash, &m.new_value),
                    CLR_CYAN,
                ),
                MutationKind::ParentChange => {
                    let label = if let Some(ref lbl) = m.resolved_label {
                        format!("Parent: \u{2192} \"{}\"", lbl)
                    } else {
                        format!(
                            "Parent: \u{2192} {}",
                            truncate(&m.new_value, budget.saturating_sub(10))
                        )
                    };
                    (label, CLR_LIGHT_GRAY)
                }
                MutationKind::HorizonChange => (
                    format!("Horizon: {} \u{2192} {}", old_or_dash, &m.new_value),
                    CLR_YELLOW_SOFT,
                ),
                MutationKind::Note => (
                    format!(
                        "Note: \"{}\"",
                        truncate(&m.new_value, budget.saturating_sub(8))
                    ),
                    CLR_LIGHT_GRAY,
                ),
                MutationKind::FieldUpdate => {
                    let desc = match &m.old_value {
                        Some(old) => {
                            let half = budget / 2;
                            format!(
                                "{}: \"{}\" \u{2192} \"{}\"",
                                m.field,
                                truncate(old, half.max(8)),
                                truncate(&m.new_value, (budget - half).max(8))
                            )
                        }
                        None => format!(
                            "{}: \"{}\"",
                            m.field,
                            truncate(&m.new_value, budget.max(10))
                        ),
                    };
                    (desc, CLR_LIGHT_GRAY)
                }
            };

            lines.push(Line::from_spans([
                Span::styled(
                    format!("{:<14}", m.relative_time),
                    Style::new().fg(CLR_DIM_GRAY),
                ),
                Span::styled(description, Style::new().fg(value_color)),
            ]));
        }
        lines
    }

    /// Build the children section lines.
    fn build_children_lines(&self, width: usize) -> Vec<Line> {
        let mut lines = Vec::new();
        // Reserve space for short_id(8) + phase/movement(8) + padding
        let desired_budget = width.saturating_sub(18).max(10);
        for child in &self.detail_children {
            let desired_trunc = truncate(&child.desired, desired_budget);
            lines.push(Line::from_spans([
                Span::styled(
                    format!("{}  ", child.short_id),
                    Style::new().fg(CLR_DIM_GRAY),
                ),
                Span::styled(
                    format!("[{}] {} ", child.phase, child.movement),
                    Style::new().fg(CLR_MID_GRAY),
                ),
                Span::styled(desired_trunc, Style::new().fg(CLR_LIGHT_GRAY)),
            ]));
        }
        lines
    }

    pub(crate) fn render_detail_body_inner(&self, area: &Rect, frame: &mut Frame<'_>, suppress_verbose: bool) {
        if self.detail_tension.is_none() {
            let text = Text::from_lines(vec![Line::from("  No tension selected")]);
            let paragraph = Paragraph::new(text);
            paragraph.render(*area, frame);
            return;
        }

        // Build section content
        let info_lines = self.build_info_lines();
        let dynamics_lines = self.build_dynamics_lines(suppress_verbose);
        let content_width = area.width.saturating_sub(4) as usize; // subtract block chrome
        let history_lines = self.build_history_lines(content_width);
        let children_lines = self.build_children_lines(content_width);

        // Block chrome: bordered() = 2 rows top/bottom border + 2 rows padding = 4 vertical chrome
        // Actually Block::bordered() has Borders::ALL + padding(Sides::all(1))
        // So vertical chrome = 2 (borders) + 2 (padding) = 4
        let chrome_v: u16 = 4;

        // Calculate section heights (content lines + block chrome)
        let info_h = (info_lines.len() as u16).saturating_add(chrome_v);
        let dynamics_h = (dynamics_lines.len() as u16).saturating_add(chrome_v);

        let has_history = !self.detail_mutations.is_empty();
        let has_children = !self.detail_children.is_empty();

        // Calculate total fixed height needed
        let fixed_h = info_h.saturating_add(dynamics_h);
        let remaining = area.height.saturating_sub(fixed_h);

        // Distribute remaining space between history and children
        let (history_h, children_h) = if has_history && has_children {
            // Give history 2/3 of remaining, children 1/3
            let hist = remaining.saturating_mul(2) / 3;
            let chld = remaining.saturating_sub(hist);
            (hist, chld)
        } else if has_history {
            (remaining, 0u16)
        } else if has_children {
            (0u16, remaining)
        } else {
            (0u16, 0u16)
        };

        // Apply scroll to determine which section areas are visible.
        // We scroll the entire layout vertically using detail_scroll.
        let scroll = self.detail_scroll;

        // Stack sections vertically
        let mut y = area.y;
        let x = area.x;
        let w = area.width;

        // --- Info section ---
        if info_h > 0 && y.saturating_add(info_h) > area.y {
            let section_area = Rect::new(x, y, w, info_h.min(area.bottom().saturating_sub(y)));
            if section_area.height >= chrome_v {
                let block = Self::section_block(" Info ");
                let inner = block.inner(section_area);
                block.render(section_area, frame);
                let para = Paragraph::new(Text::from_lines(info_lines)).scroll((scroll, 0));
                para.render(inner, frame);
            }
            y = y.saturating_add(info_h);
        }

        if y >= area.bottom() {
            return;
        }

        // --- Dynamics section ---
        if dynamics_h > 0 {
            let avail_h = area.bottom().saturating_sub(y);
            let section_area = Rect::new(x, y, w, dynamics_h.min(avail_h));
            if section_area.height >= chrome_v {
                let block = Self::section_block(" Dynamics ");
                let inner = block.inner(section_area);
                block.render(section_area, frame);
                // Scroll: subtract info section lines from scroll offset
                let info_content_lines = self.build_info_lines().len() as u16;
                let dyn_scroll = scroll.saturating_sub(info_content_lines);
                let para = Paragraph::new(Text::from_lines(dynamics_lines)).scroll((dyn_scroll, 0));
                para.render(inner, frame);
            }
            y = y.saturating_add(dynamics_h);
        }

        if y >= area.bottom() {
            return;
        }

        // --- History section ---
        if has_history && history_h > 0 {
            let avail_h = area.bottom().saturating_sub(y);
            let section_h = history_h.min(avail_h);
            let section_area = Rect::new(x, y, w, section_h);
            if section_area.height >= chrome_v {
                let title = format!(" History ({}) ", self.detail_mutations.len());
                let block = Self::section_block(&title);
                let inner = block.inner(section_area);
                block.render(section_area, frame);
                // History gets the bulk of scrolling
                let info_content_lines = self.build_info_lines().len() as u16;
                let dyn_content_lines = self.build_dynamics_lines(suppress_verbose).len() as u16;
                let hist_scroll = scroll
                    .saturating_sub(info_content_lines)
                    .saturating_sub(dyn_content_lines);
                let para = Paragraph::new(Text::from_lines(history_lines)).scroll((hist_scroll, 0));
                para.render(inner, frame);
            }
            y = y.saturating_add(section_h);
        }

        if y >= area.bottom() {
            return;
        }

        // --- Children section ---
        if has_children && children_h > 0 {
            let avail_h = area.bottom().saturating_sub(y);
            let section_h = children_h.min(avail_h);
            let section_area = Rect::new(x, y, w, section_h);
            if section_area.height >= chrome_v {
                let title = format!(" Children ({}) ", self.detail_children.len());
                let block = Self::section_block(&title);
                let inner = block.inner(section_area);
                block.render(section_area, frame);
                let para = Paragraph::new(Text::from_lines(children_lines));
                para.render(inner, frame);
            }
        }
    }

    pub(crate) fn render_detail_body_responsive(&self, area: &Rect, frame: &mut Frame<'_>) {
        let suppress_verbose = area.height < 20;
        self.render_detail_body_inner(area, frame, suppress_verbose);
    }

    pub(crate) fn render_detail_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let verbose_label = if self.verbose { "v-" } else { "v+" };
        let hints = format!(
            " Esc back  j/k  {}  r/d edit  n note  h horizon  a add  R resolve  X release  Del  m move  g agent  w reflect  F focus  N graph  L lever  q/?",
            verbose_label,
        );
        let style = Style::new().fg(CLR_MID_GRAY);
        let paragraph = Paragraph::new(Text::from_spans([Span::styled(&hints, style)]));
        paragraph.render(*area, frame);
    }
}
