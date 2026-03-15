use chrono::Utc;
use ftui::Frame;
use ftui::layout::Rect;
use ftui::text::{Line, Span, Text};
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::status_line::{StatusLine, StatusItem};

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::helpers::{render_bar, urgency_bar_color, render_subtle_divider};
use crate::views::dashboard::mini_sparkline;
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
            .style(STYLES.value_bold);
        if !right_text.is_empty() {
            status = status.right(StatusItem::text(&right_text));
        }
        status.render(*area, frame);
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
                        crumbs.push(Span::styled(" > ", STYLES.muted));
                    }
                    crumbs.push(Span::styled(
                        truncate(desired, 20).to_string(),
                        STYLES.label,
                    ));
                }
                lines.push(Line::from_spans(crumbs));
            }

            // Parent line
            if let Some(parent) = &self.detail.parent {
                lines.push(Line::from_spans([
                    Span::styled("Parent   ", STYLES.label),
                    Span::styled(
                        truncate(&parent.desired, 40).to_string(),
                        STYLES.accent,
                    ),
                ]));
            }

            lines.push(Line::from_spans([
                Span::styled("Desired  ", STYLES.label),
                Span::styled(&tension.desired, STYLES.value),
            ]));
            lines.push(Line::from_spans([
                Span::styled("Actual   ", STYLES.label),
                Span::styled(&tension.actual, STYLES.value),
            ]));
            lines.push(Line::from_spans([
                Span::styled("Status   ", STYLES.label),
                Span::styled(tension.status.to_string(), STYLES.value),
                Span::styled("       Created  ", STYLES.label),
                Span::styled(
                    werk_shared::relative_time(tension.created_at, now),
                    STYLES.value,
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
                Span::styled("Horizon  ", STYLES.label),
                Span::styled(horizon_str, STYLES.value),
            ]));
        }
        lines
    }

    /// Build the dynamics section lines with phase dots and movement-colored arrows (Phase 3a).
    fn build_dynamics_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        if let Some(dyn_display) = &self.detail.dynamics {
            // Phase with colored dot
            let phase_color = match dyn_display.phase.as_str() {
                "Germination" => WERK_THEME.phase_germination,
                "Assimilation" => WERK_THEME.phase_assimilation,
                "Completion" => WERK_THEME.phase_completion,
                "Momentum" => WERK_THEME.phase_momentum,
                _ => WERK_THEME.text_muted,
            };
            // Movement with colored arrow
            let movement_color = if dyn_display.movement.contains('\u{2192}') {
                WERK_THEME.advancing
            } else if dyn_display.movement.contains('\u{2194}') {
                WERK_THEME.oscillating
            } else {
                WERK_THEME.stagnant
            };

            lines.push(Line::from_spans([
                Span::styled("Phase       ", STYLES.label),
                Span::styled("\u{25CF} ", Style::new().fg(phase_color)),
                Span::styled(&dyn_display.phase, STYLES.value),
                Span::styled("      Movement    ", STYLES.label),
                Span::styled(&dyn_display.movement, Style::new().fg(movement_color)),
            ]));

            if let Some(mag) = dyn_display.magnitude {
                let bar = render_bar(mag, 10);
                lines.push(Line::from_spans([
                    Span::styled("Magnitude   ", STYLES.label),
                    Span::styled(bar, STYLES.accent),
                    Span::styled(format!(" {:.2}", mag), STYLES.value),
                ]));
            }

            if let Some(urg) = dyn_display.urgency {
                let bar = render_bar(urg.min(1.0), 10);
                let bar_color = urgency_bar_color(urg);
                lines.push(Line::from_spans([
                    Span::styled("Urgency     ", STYLES.label),
                    Span::styled(bar, Style::new().fg(bar_color)),
                    Span::styled(format!(" {:.0}%", (urg * 100.0).min(999.0)), STYLES.value),
                ]));
            }

            if let Some((ref text, color)) = dyn_display.forecast_line {
                lines.push(Line::from_spans([
                    Span::styled("Forecast    ", STYLES.label),
                    Span::styled(text, Style::new().fg(color)),
                ]));
            }

            if let Some(conflict) = &dyn_display.conflict {
                lines.push(Line::from_spans([
                    Span::styled("Conflict    ", STYLES.label),
                    Span::styled(conflict, STYLES.danger),
                ]));
            }

            if let Some(neglect) = &dyn_display.neglect {
                lines.push(Line::from_spans([
                    Span::styled("Neglect     ", STYLES.label),
                    Span::styled(neglect, STYLES.warn),
                ]));
            }

            // Oscillation mini-bar (Phase 3a)
            if let Some(v) = &dyn_display.oscillation {
                // Extract reversal count for mini-bar
                let reversals: usize = v.split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let osc_bar = if reversals > 0 {
                    let bars = ['\u{2582}', '\u{2583}', '\u{2585}', '\u{2587}'];
                    let idx = (reversals.min(4) - 1).min(3);
                    format!(" {}", bars[idx])
                } else {
                    String::new()
                };
                lines.push(Line::from_spans([
                    Span::styled("Oscillation ", STYLES.label),
                    Span::styled(v, STYLES.value),
                    Span::styled(osc_bar, STYLES.warn),
                ]));
            }

            {
                lines.push(Line::from(""));
                lines.push(Line::from_spans([Span::styled(
                    "Verbose Dynamics",
                    STYLES.muted,
                )]));

                if let Some(v) = &dyn_display.resolution {
                    lines.push(Line::from_spans([
                        Span::styled("Resolution          ", STYLES.label),
                        Span::styled(v, STYLES.value),
                    ]));
                }
                if let Some(v) = &dyn_display.orientation {
                    lines.push(Line::from_spans([
                        Span::styled("Orientation         ", STYLES.label),
                        Span::styled(v, STYLES.value),
                    ]));
                }
                if let Some(v) = &dyn_display.compensating_strategy {
                    lines.push(Line::from_spans([
                        Span::styled("Compensating Strat  ", STYLES.label),
                        Span::styled(v, STYLES.value),
                    ]));
                }
                if let Some(v) = &dyn_display.assimilation_depth {
                    lines.push(Line::from_spans([
                        Span::styled("Assimilation Depth  ", STYLES.label),
                        Span::styled(v, STYLES.value),
                    ]));
                }
                if let Some(v) = &dyn_display.horizon_drift {
                    lines.push(Line::from_spans([
                        Span::styled("Horizon Drift       ", STYLES.label),
                        Span::styled(v, STYLES.value),
                    ]));
                }
            }
        } else {
            lines.push(Line::from_spans([Span::styled(
                "No dynamics computed",
                STYLES.muted,
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
                        Span::styled("Trajectory  ", STYLES.label),
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
                            Span::styled(format!("{}    ", label), STYLES.label),
                            Span::styled(bar, STYLES.accent),
                            Span::styled(format!(" {:.2}", p.projected_gap), STYLES.value),
                        ]));
                    }

                    // Risk flags
                    if proj.oscillation_risk {
                        lines.push(Line::from_spans([Span::styled(
                            "  \u{26a0} Oscillation risk",
                            STYLES.warn,
                        )]));
                    }
                    if proj.neglect_risk {
                        lines.push(Line::from_spans([Span::styled(
                            "  \u{26a0} Neglect risk",
                            STYLES.warn,
                        )]));
                    }
                }
            }
        }
        lines
    }

    /// Build the history section lines with time-bucketed grouping (Phase 3b).
    fn build_history_lines(&self, width: usize) -> Vec<Line> {
        let mut lines = Vec::new();
        let budget = width.saturating_sub(16).max(10);

        // Time-bucketed grouping
        let mut last_bucket: Option<&str> = None;
        for m in &self.detail.mutations {
            // Determine time bucket from relative_time
            let bucket = if m.relative_time.contains("just now") || m.relative_time.contains("1m") || m.relative_time.contains("2m") || m.relative_time.contains("3m") || m.relative_time.contains("4m") || m.relative_time.contains("5m") {
                "just now"
            } else if m.relative_time.contains("h ago") || m.relative_time.ends_with("h") {
                "today"
            } else if m.relative_time.contains("1d") {
                "yesterday"
            } else if m.relative_time.contains("d ago") || m.relative_time.contains("d") {
                "this week"
            } else {
                "older"
            };

            // Insert divider between groups
            if last_bucket.is_some() && last_bucket != Some(bucket) {
                lines.push(render_subtle_divider(width.min(40)));
            }
            last_bucket = Some(bucket);

            let old_or_dash = m
                .old_value
                .as_deref()
                .map(|o| truncate(o, budget / 2).to_string())
                .unwrap_or_else(|| "\u{2014}".to_string());

            let (description, value_color) = match m.kind {
                MutationKind::Created => {
                    // Parse "desired='...';actual='...'" format to show just the desired value
                    let desired_display = if let Some(start) = m.new_value.find("desired='") {
                        let val_start = start + 9;
                        if let Some(end) = m.new_value[val_start..].find('\'') {
                            &m.new_value[val_start..val_start + end]
                        } else {
                            &m.new_value
                        }
                    } else {
                        &m.new_value
                    };
                    (
                        format!(
                            "Created \"{}\"",
                            truncate(desired_display, budget.saturating_sub(12))
                        ),
                        CLR_GREEN,
                    )
                }
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
                    STYLES.muted,
                ),
                Span::styled(description, Style::new().fg(value_color)),
            ]));
        }
        lines
    }

    /// Build the children section lines with inline sparklines (Phase 3c).
    fn build_children_lines(&self, width: usize) -> Vec<Line> {
        let mut lines = Vec::new();
        // Reserve space for short_id(8) + phase/movement(8) + sparkline(8) + padding
        let has_sparklines = width >= 40;
        let spark_width = if has_sparklines { 7 } else { 0 };
        let desired_budget = width.saturating_sub(18 + spark_width + 2).max(10);
        for child in &self.detail.children {
            let desired_trunc = truncate(&child.desired, desired_budget);
            let mut spans = vec![
                Span::styled(
                    format!("{}  ", child.short_id),
                    STYLES.muted,
                ),
                Span::styled(
                    format!("[{}] {} ", child.phase, child.movement),
                    STYLES.label,
                ),
                Span::styled(desired_trunc, STYLES.value),
            ];
            // Inline sparkline for child activity (Phase 3c)
            if has_sparklines && !child.activity.is_empty() {
                let spark = mini_sparkline(&child.activity, 7);
                spans.push(Span::styled(" ", Style::new()));
                spans.push(Span::styled(spark, STYLES.accent));
            }
            lines.push(Line::from_spans(spans));
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
        let content_width = area.width.saturating_sub(2) as usize;
        let mutations_count = self.detail.mutations.len();
        let children_start = 2 + mutations_count;

        // Build all lines into one scrollable list with section headers
        let mut all_lines: Vec<Line> = Vec::new();

        // --- Info section header ---
        let info_marker = if cursor == 0 { "\u{25b8}" } else { " " };
        let info_style = if cursor == 0 {
            Style::new().fg(WERK_THEME.border_active).bold()
        } else {
            Style::new().fg(WERK_THEME.border).bold()
        };
        all_lines.push(Line::from_spans([
            Span::styled(format!("{} ", info_marker), info_style),
            Span::styled("Info", info_style),
        ]));
        for line in self.build_info_lines() {
            // Indent content by 2 spaces
            let mut spans = vec![Span::styled("  ", Style::new())];
            spans.extend(line.spans().iter().cloned());
            all_lines.push(Line::from_spans(spans));
        }
        all_lines.push(Line::from(""));

        // --- Dynamics section header ---
        let dyn_marker = if cursor == 1 { "\u{25b8}" } else { " " };
        let dyn_style = if cursor == 1 {
            Style::new().fg(WERK_THEME.border_active).bold()
        } else {
            Style::new().fg(WERK_THEME.border).bold()
        };
        all_lines.push(Line::from_spans([
            Span::styled(format!("{} ", dyn_marker), dyn_style),
            Span::styled("Dynamics", dyn_style),
        ]));
        for line in self.build_dynamics_lines() {
            let mut spans = vec![Span::styled("  ", Style::new())];
            spans.extend(line.spans().iter().cloned());
            all_lines.push(Line::from_spans(spans));
        }
        all_lines.push(Line::from(""));

        // --- Trajectory section ---
        let trajectory_lines = self.build_trajectory_lines();
        if !trajectory_lines.is_empty() {
            all_lines.push(Line::from_spans([
                Span::styled("  ", Style::new()),
                Span::styled("Trajectory", Style::new().fg(WERK_THEME.border).bold()),
            ]));
            for line in trajectory_lines {
                let mut spans = vec![Span::styled("  ", Style::new())];
                spans.extend(line.spans().iter().cloned());
                all_lines.push(Line::from_spans(spans));
            }
            all_lines.push(Line::from(""));
        }

        // --- History section ---
        let has_history = !self.detail.mutations.is_empty();
        if has_history {
            let any_mutation_selected = cursor >= 2 && cursor < children_start;
            let hist_style = if any_mutation_selected {
                Style::new().fg(WERK_THEME.border_active).bold()
            } else {
                Style::new().fg(WERK_THEME.border).bold()
            };
            all_lines.push(Line::from_spans([
                Span::styled("  ", Style::new()),
                Span::styled(
                    format!("History ({})", self.detail.mutations.len()),
                    hist_style,
                ),
            ]));
            let history_lines = self.build_history_lines(content_width.saturating_sub(4));
            for (i, line) in history_lines.into_iter().enumerate() {
                let selected = any_mutation_selected && Some(i) == Some(cursor - 2);
                let marker = if selected { "\u{25b8} " } else { "  " };
                let marker_style = if selected { STYLES.accent_bold } else { Style::new() };
                let mut spans = vec![
                    Span::styled("  ", Style::new()),
                    Span::styled(marker, marker_style),
                ];
                spans.extend(line.spans().iter().cloned());
                all_lines.push(Line::from_spans(spans));
            }
            all_lines.push(Line::from(""));
        }

        // --- Children section ---
        let has_children = !self.detail.children.is_empty();
        if has_children {
            let any_child_selected = cursor >= children_start;
            let child_style = if any_child_selected {
                Style::new().fg(WERK_THEME.border_active).bold()
            } else {
                Style::new().fg(WERK_THEME.border).bold()
            };
            all_lines.push(Line::from_spans([
                Span::styled("  ", Style::new()),
                Span::styled(
                    format!("Children ({})", self.detail.children.len()),
                    child_style,
                ),
            ]));
            let children_lines = self.build_children_lines(content_width.saturating_sub(4));
            for (i, line) in children_lines.into_iter().enumerate() {
                let selected = any_child_selected && Some(i) == cursor.checked_sub(children_start);
                let marker = if selected { "\u{25b8} " } else { "  " };
                let marker_style = if selected { STYLES.accent_bold } else { Style::new() };
                let mut spans = vec![
                    Span::styled("  ", Style::new()),
                    Span::styled(marker, marker_style),
                ];
                spans.extend(line.spans().iter().cloned());
                all_lines.push(Line::from_spans(spans));
            }
        }

        // Calculate scroll offset to keep cursor-related content visible
        let scroll = self.detail.scroll as u16;
        let para = Paragraph::new(Text::from_lines(all_lines)).scroll((scroll, 0));
        para.render(*area, frame);
    }

    pub(crate) fn render_detail_body_responsive(&self, area: &Rect, frame: &mut Frame<'_>) {
        self.render_detail_body_inner(area, frame);
    }

    /// Adaptive detail hints based on width and selection state (Phase 5).
    pub(crate) fn render_detail_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let width = area.width as usize;
        let mut hints = StatusLine::new().separator("  ");

        // Essential hints (always shown)
        hints = hints
            .left(StatusItem::key_hint("Esc", "back"))
            .left(StatusItem::key_hint("j/k", "nav"));

        // State-aware hints (Phase 5b)
        let cursor = self.detail.cursor;
        let children_start = 2 + self.detail.mutations.len();
        if cursor >= children_start && !self.detail.children.is_empty() {
            hints = hints.left(StatusItem::key_hint("Enter", "open child"));
        } else {
            hints = hints.left(StatusItem::key_hint("Enter", "open"));
        }

        if width >= 60 {
            hints = hints
                .left(StatusItem::key_hint("r/d", "edit"))
                .left(StatusItem::key_hint("n", "note"))
                .left(StatusItem::key_hint("h", "horizon"))
                .left(StatusItem::key_hint("a", "add"));

            // Check if selected tension is resolved
            let is_resolved = self.detail.tension.as_ref()
                .map(|t| t.status == sd_core::TensionStatus::Resolved)
                .unwrap_or(false);
            if !is_resolved {
                hints = hints
                    .left(StatusItem::key_hint("R", "resolve"))
                    .left(StatusItem::key_hint("X", "release"));
            }
        }
        if width >= 100 {
            hints = hints
                .left(StatusItem::text("Del"))
                .left(StatusItem::key_hint("m", "move"))
                .left(StatusItem::key_hint("g", "agent"))
                .left(StatusItem::key_hint("w", "reflect"))
                .left(StatusItem::key_hint("T", "timeline"))
                .left(StatusItem::key_hint("D", "health"))
                .left(StatusItem::key_hint("L", "lever"));
        }
        hints = hints.left(StatusItem::key_hint("q/?", ""));
        hints = hints.style(STYLES.label);
        hints.render(*area, frame);
    }
}
