use chrono::Utc;
use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::block::Block;
use ftui::widgets::borders::BorderType;
use ftui::widgets::status_line::{StatusLine, StatusItem};

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::helpers::render_bar;
use crate::theme::*;
use crate::types::MutationKind;

impl WerkApp {
    pub(crate) fn render_detail_title(&self, area: &Rect, frame: &mut Frame<'_>) {
        let (left_text, right_text) = match &self.detail.tension {
            Some(t) => {
                let short_id: String = t.id.chars().take(8).collect();
                let desired = truncate(&t.desired, area.width.saturating_sub(12) as usize).to_string();
                (format!(" {}", desired), short_id)
            }
            None => (" Detail".to_string(), String::new()),
        };
        let mut status = StatusLine::new()
            .left(StatusItem::text(&left_text))
            .style(Style::new().fg(CLR_WHITE).bold());
        if !right_text.is_empty() {
            status = status.right(StatusItem::text(&right_text));
        }
        status.render(*area, frame);
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
        if let Some(tension) = &self.detail.tension {
            let now = Utc::now();

            // Breadcrumb (ancestor chain)
            if !self.detail.ancestors.is_empty() {
                let mut crumbs: Vec<Span> = Vec::new();
                for (i, (_, desired)) in self.detail.ancestors.iter().enumerate() {
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
            if let Some(parent) = &self.detail.parent {
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
    fn build_dynamics_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        if let Some(dyn_display) = &self.detail.dynamics {
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

            if let Some((ref text, color)) = dyn_display.forecast_line {
                lines.push(Line::from_spans([
                    Span::styled("Forecast    ", Style::new().fg(CLR_MID_GRAY)),
                    Span::styled(text, Style::new().fg(color)),
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

            {
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

    /// Build the trajectory section lines from cached field projection.
    fn build_trajectory_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        let tension_id = match &self.detail.tension {
            Some(t) => &t.id,
            None => return lines,
        };

        if let Some(ref fp) = self.field_projection {
            if let Some((_, projs)) = fp.tension_projections.iter().find(|(id, _)| id == tension_id) {
                if let Some(proj) = projs.first() {
                    let traj_label = match proj.trajectory {
                        sd_core::Trajectory::Resolving => "\u{2193} Resolving",
                        sd_core::Trajectory::Stalling => "\u{2014} Stalling",
                        sd_core::Trajectory::Drifting => "~ Drifting",
                        sd_core::Trajectory::Oscillating => "\u{21cc} Oscillating",
                    };
                    lines.push(Line::from_spans([
                        Span::styled("Trajectory  ", Style::new().fg(CLR_MID_GRAY)),
                        Span::styled(traj_label, Style::new().fg(match proj.trajectory {
                            sd_core::Trajectory::Resolving => CLR_GREEN,
                            sd_core::Trajectory::Stalling => CLR_DIM_GRAY,
                            sd_core::Trajectory::Drifting => CLR_YELLOW,
                            sd_core::Trajectory::Oscillating => CLR_RED_SOFT,
                        })),
                    ]));

                    // Gap progression bars
                    for (i, p) in projs.iter().enumerate() {
                        let label = match i { 0 => "Gap +1w ", 1 => "Gap +1m ", _ => "Gap +3m " };
                        let bar = render_bar(p.projected_gap, 10);
                        lines.push(Line::from_spans([
                            Span::styled(format!("{}    ", label), Style::new().fg(CLR_MID_GRAY)),
                            Span::styled(bar, Style::new().fg(CLR_CYAN)),
                            Span::styled(format!(" {:.2}", p.projected_gap), Style::new().fg(CLR_LIGHT_GRAY)),
                        ]));
                    }

                    // Risk flags
                    if proj.oscillation_risk {
                        lines.push(Line::from_spans([Span::styled(
                            "  \u{26a0} Oscillation risk",
                            Style::new().fg(CLR_YELLOW),
                        )]));
                    }
                    if proj.neglect_risk {
                        lines.push(Line::from_spans([Span::styled(
                            "  \u{26a0} Neglect risk",
                            Style::new().fg(CLR_YELLOW),
                        )]));
                    }
                }
            }
        }
        lines
    }

    /// Build the history section lines.
    fn build_history_lines(&self, width: usize) -> Vec<Line> {
        let mut lines = Vec::new();
        // Reserve space for time(14) + spacing
        let budget = width.saturating_sub(16).max(10);
        for m in &self.detail.mutations {
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
        for child in &self.detail.children {
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

    pub(crate) fn render_detail_body_inner(&self, area: &Rect, frame: &mut Frame<'_>) {
        if self.detail.tension.is_none() {
            let text = Text::from_lines(vec![Line::from("  No tension selected")]);
            let paragraph = Paragraph::new(text);
            paragraph.render(*area, frame);
            return;
        }

        let cursor = self.detail.cursor;
        let highlight_style = Style::new().fg(CLR_WHITE).bold();

        // Build section content
        let info_lines = self.build_info_lines();
        let dynamics_lines = self.build_dynamics_lines();
        let trajectory_lines = self.build_trajectory_lines();
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
        let trajectory_h = if trajectory_lines.is_empty() {
            0u16
        } else {
            (trajectory_lines.len() as u16).saturating_add(chrome_v)
        };

        let has_history = !self.detail.mutations.is_empty();
        let has_children = !self.detail.children.is_empty();

        // Calculate total fixed height needed
        let fixed_h = info_h.saturating_add(dynamics_h).saturating_add(trajectory_h);
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

        // Cursor-to-section mapping:
        // cursor 0 = Info section
        // cursor 1 = Dynamics section
        // cursor 2..2+mutations.len() = individual mutation lines
        // cursor 2+mutations.len()..end = individual children
        let mutations_count = self.detail.mutations.len();
        let children_start = 2 + mutations_count;

        // Determine which section block should be highlighted based on cursor
        let info_selected = cursor == 0;
        let dynamics_selected = cursor == 1;

        // Stack sections vertically
        let mut y = area.y;
        let x = area.x;
        let w = area.width;

        // --- Info section ---
        if info_h > 0 && y.saturating_add(info_h) > area.y {
            let section_area = Rect::new(x, y, w, info_h.min(area.bottom().saturating_sub(y)));
            if section_area.height >= chrome_v {
                let block = if info_selected {
                    Self::section_block(" Info ").border_style(Style::new().fg(CLR_CYAN))
                } else {
                    Self::section_block(" Info ")
                };
                let inner = block.inner(section_area);
                block.render(section_area, frame);
                let para = Paragraph::new(Text::from_lines(info_lines));
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
                let block = if dynamics_selected {
                    Self::section_block(" Dynamics ").border_style(Style::new().fg(CLR_CYAN))
                } else {
                    Self::section_block(" Dynamics ")
                };
                let inner = block.inner(section_area);
                block.render(section_area, frame);
                let para = Paragraph::new(Text::from_lines(dynamics_lines));
                para.render(inner, frame);
            }
            y = y.saturating_add(dynamics_h);
        }

        if y >= area.bottom() {
            return;
        }

        // --- Trajectory section ---
        if trajectory_h > 0 {
            let avail_h = area.bottom().saturating_sub(y);
            let section_area = Rect::new(x, y, w, trajectory_h.min(avail_h));
            if section_area.height >= chrome_v {
                let block = Self::section_block(" Trajectory ");
                let inner = block.inner(section_area);
                block.render(section_area, frame);
                let para = Paragraph::new(Text::from_lines(trajectory_lines));
                para.render(inner, frame);
            }
            y = y.saturating_add(trajectory_h);
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
                let title = format!(" History ({}) ", self.detail.mutations.len());
                // Highlight the block border if any mutation item is selected
                let any_mutation_selected = cursor >= 2 && cursor < children_start;
                let block = if any_mutation_selected {
                    Self::section_block(&title).border_style(Style::new().fg(CLR_CYAN))
                } else {
                    Self::section_block(&title)
                };
                let inner = block.inner(section_area);
                block.render(section_area, frame);

                // Highlight the specific mutation line that the cursor is on
                let selected_mutation_idx = if any_mutation_selected {
                    Some(cursor - 2)
                } else {
                    None
                };
                let highlighted_lines: Vec<Line> = history_lines
                    .into_iter()
                    .enumerate()
                    .map(|(i, line)| {
                        if Some(i) == selected_mutation_idx {
                            Line::from_spans(
                                std::iter::once(Span::styled("\u{25b6} ", highlight_style))
                                    .chain(line.spans().iter().cloned())
                                    .collect::<Vec<_>>(),
                            )
                        } else {
                            Line::from_spans(
                                std::iter::once(Span::styled("  ", Style::new()))
                                    .chain(line.spans().iter().cloned())
                                    .collect::<Vec<_>>(),
                            )
                        }
                    })
                    .collect();

                // Auto-scroll history to keep selected mutation visible
                let hist_scroll = if let Some(sel) = selected_mutation_idx {
                    let visible_h = inner.height as usize;
                    if sel >= visible_h {
                        (sel - visible_h + 1) as u16
                    } else {
                        0
                    }
                } else {
                    0
                };

                let para = Paragraph::new(Text::from_lines(highlighted_lines)).scroll((hist_scroll, 0));
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
                let title = format!(" Children ({}) ", self.detail.children.len());
                let any_child_selected = cursor >= children_start;
                let block = if any_child_selected {
                    Self::section_block(&title).border_style(Style::new().fg(CLR_CYAN))
                } else {
                    Self::section_block(&title)
                };
                let inner = block.inner(section_area);
                block.render(section_area, frame);

                // Highlight the specific child line that the cursor is on
                let selected_child_idx = if any_child_selected {
                    Some(cursor - children_start)
                } else {
                    None
                };
                let highlighted_lines: Vec<Line> = children_lines
                    .into_iter()
                    .enumerate()
                    .map(|(i, line)| {
                        if Some(i) == selected_child_idx {
                            Line::from_spans(
                                std::iter::once(Span::styled("\u{25b6} ", highlight_style))
                                    .chain(line.spans().iter().cloned())
                                    .collect::<Vec<_>>(),
                            )
                        } else {
                            Line::from_spans(
                                std::iter::once(Span::styled("  ", Style::new()))
                                    .chain(line.spans().iter().cloned())
                                    .collect::<Vec<_>>(),
                            )
                        }
                    })
                    .collect();

                // Auto-scroll children to keep selected child visible
                let child_scroll = if let Some(sel) = selected_child_idx {
                    let visible_h = inner.height as usize;
                    if sel >= visible_h {
                        (sel - visible_h + 1) as u16
                    } else {
                        0
                    }
                } else {
                    0
                };

                let para = Paragraph::new(Text::from_lines(highlighted_lines)).scroll((child_scroll, 0));
                para.render(inner, frame);
            }
        }
    }

    pub(crate) fn render_detail_body_responsive(&self, area: &Rect, frame: &mut Frame<'_>) {
        self.render_detail_body_inner(area, frame);
    }

    pub(crate) fn render_detail_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = StatusLine::new()
            .separator("  ")
            .left(StatusItem::key_hint("Esc", "back"))
            .left(StatusItem::key_hint("j/k", "nav"))
            .left(StatusItem::key_hint("Enter", "open"))
            .left(StatusItem::key_hint("r/d", "edit"))
            .left(StatusItem::key_hint("n", "note"))
            .left(StatusItem::key_hint("h", "horizon"))
            .left(StatusItem::key_hint("a", "add"))
            .left(StatusItem::key_hint("R", "resolve"))
            .left(StatusItem::key_hint("X", "release"))
            .left(StatusItem::text("Del"))
            .left(StatusItem::key_hint("m", "move"))
            .left(StatusItem::key_hint("g", "agent"))
            .left(StatusItem::key_hint("w", "reflect"))
            .left(StatusItem::key_hint("T", "timeline"))
            .left(StatusItem::key_hint("D", "health"))
            .left(StatusItem::key_hint("L", "lever"))
            .left(StatusItem::key_hint("q/?", ""))
            .style(Style::new().fg(CLR_MID_GRAY));
        hints.render(*area, frame);
    }
}
