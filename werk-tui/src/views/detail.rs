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

    // -----------------------------------------------------------------------
    // Section: Gap — the core tension (desired vs actual)
    // -----------------------------------------------------------------------

    fn build_gap_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        if let Some(tension) = &self.detail.tension {
            lines.push(Line::from_spans([
                Span::styled("Desired  ", STYLES.label),
                Span::styled(&tension.desired, STYLES.value),
            ]));
            lines.push(Line::from_spans([
                Span::styled("Reality  ", STYLES.label),
                Span::styled(&tension.actual, STYLES.value),
            ]));
        }
        lines
    }

    // -----------------------------------------------------------------------
    // Section: State — operational summary
    // -----------------------------------------------------------------------

    fn build_state_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        if let Some(tension) = &self.detail.tension {
            let now = Utc::now();

            lines.push(Line::from_spans([
                Span::styled("Status   ", STYLES.label),
                Span::styled(tension.status.to_string(), STYLES.value),
                Span::styled("    Created  ", STYLES.label),
                Span::styled(
                    werk_shared::relative_time(tension.created_at, now),
                    STYLES.value,
                ),
            ]));

            if let Some(dyn_display) = &self.detail.dynamics {
                let phase_color = match dyn_display.phase.as_str() {
                    "Germination" => WERK_THEME.phase_germination,
                    "Assimilation" => WERK_THEME.phase_assimilation,
                    "Completion" => WERK_THEME.phase_completion,
                    "Momentum" => WERK_THEME.phase_momentum,
                    _ => WERK_THEME.text_muted,
                };
                let movement_color = if dyn_display.movement.contains('\u{2192}') {
                    WERK_THEME.advancing
                } else if dyn_display.movement.contains('\u{2194}') {
                    WERK_THEME.oscillating
                } else {
                    WERK_THEME.stagnant
                };

                lines.push(Line::from_spans([
                    Span::styled("Phase    ", STYLES.label),
                    Span::styled("\u{25CF} ", Style::new().fg(phase_color)),
                    Span::styled(&dyn_display.phase, STYLES.value),
                    Span::styled("  ", Style::new()),
                    Span::styled(&dyn_display.movement, Style::new().fg(movement_color)),
                ]));
            }

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

            if let Some(parent) = &self.detail.parent {
                if self.detail.ancestors.len() > 1 {
                    let mut crumbs: Vec<Span> = vec![Span::styled("Parent   ", STYLES.label)];
                    for (i, (_, desired)) in self.detail.ancestors.iter().enumerate() {
                        if i > 0 {
                            crumbs.push(Span::styled(" \u{203A} ", STYLES.muted));
                        }
                        crumbs.push(Span::styled(
                            truncate(desired, 18).to_string(),
                            if i == self.detail.ancestors.len() - 1 { STYLES.accent } else { STYLES.muted },
                        ));
                    }
                    lines.push(Line::from_spans(crumbs));
                } else {
                    lines.push(Line::from_spans([
                        Span::styled("Parent   ", STYLES.label),
                        Span::styled(
                            truncate(&parent.desired, 40).to_string(),
                            STYLES.accent,
                        ),
                    ]));
                }
            }
        }
        lines
    }

    // -----------------------------------------------------------------------
    // Section: Dynamics
    // -----------------------------------------------------------------------

    fn build_dynamics_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        let dyn_display = match &self.detail.dynamics {
            Some(d) => d,
            None => return lines,
        };

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

        // Trajectory + projections
        let tension_id = self.detail.tension.as_ref().map(|t| &t.id);
        if let (Some(tid), Some(ref fp)) = (tension_id, &self.field_projection) {
            if let Some((_, projs)) = fp.tension_projections.iter().find(|(id, _)| id == tid) {
                if let Some(proj) = projs.first() {
                    let (traj_label, traj_color) = match proj.trajectory {
                        sd_core::Trajectory::Resolving => ("\u{2193} Resolving", CLR_GREEN),
                        sd_core::Trajectory::Stalling => ("\u{2014} Stalling", CLR_DIM_GRAY),
                        sd_core::Trajectory::Drifting => ("~ Drifting", CLR_YELLOW),
                        sd_core::Trajectory::Oscillating => ("\u{21cc} Oscillating", CLR_RED_SOFT),
                    };
                    lines.push(Line::from_spans([
                        Span::styled("Trajectory  ", STYLES.label),
                        Span::styled(traj_label, Style::new().fg(traj_color)),
                    ]));

                    let gap_parts: Vec<String> = projs.iter().enumerate().map(|(i, p)| {
                        let label = match i { 0 => "+1w", 1 => "+1m", _ => "+3m" };
                        format!("{} {:.2}", label, p.projected_gap)
                    }).collect();
                    lines.push(Line::from_spans([
                        Span::styled("Gap Outlook ", STYLES.label),
                        Span::styled(gap_parts.join("  "), STYLES.value),
                    ]));

                    let mut risks = Vec::new();
                    if proj.oscillation_risk { risks.push("oscillation"); }
                    if proj.neglect_risk { risks.push("neglect"); }
                    if !risks.is_empty() {
                        lines.push(Line::from_spans([
                            Span::styled("            ", STYLES.label),
                            Span::styled(
                                format!("\u{26a0} {} risk", risks.join(", ")),
                                STYLES.warn,
                            ),
                        ]));
                    }
                }
            }
        }

        if let Some((ref text, color)) = dyn_display.forecast_line {
            lines.push(Line::from_spans([
                Span::styled("Forecast    ", STYLES.label),
                Span::styled(text, Style::new().fg(color)),
            ]));
        }

        // Signals
        let has_signals = dyn_display.conflict.is_some()
            || dyn_display.neglect.is_some()
            || dyn_display.oscillation.is_some()
            || dyn_display.horizon_drift.is_some()
            || dyn_display.compensating_strategy.is_some();

        if has_signals {
            lines.push(Line::from(""));
            lines.push(Line::from_spans([Span::styled("Signals", STYLES.muted)]));

            if let Some(conflict) = &dyn_display.conflict {
                lines.push(Line::from_spans([
                    Span::styled("  Conflict       ", STYLES.label),
                    Span::styled(conflict, STYLES.danger),
                ]));
            }
            if let Some(neglect) = &dyn_display.neglect {
                lines.push(Line::from_spans([
                    Span::styled("  Neglect        ", STYLES.label),
                    Span::styled(neglect, STYLES.warn),
                ]));
            }
            if let Some(v) = &dyn_display.oscillation {
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
                    Span::styled("  Oscillation    ", STYLES.label),
                    Span::styled(v, STYLES.value),
                    Span::styled(osc_bar, STYLES.warn),
                ]));
            }
            if let Some(v) = &dyn_display.horizon_drift {
                lines.push(Line::from_spans([
                    Span::styled("  Horizon Drift  ", STYLES.label),
                    Span::styled(v, STYLES.value),
                ]));
            }
            if let Some(v) = &dyn_display.compensating_strategy {
                lines.push(Line::from_spans([
                    Span::styled("  Strategy       ", STYLES.label),
                    Span::styled(v, STYLES.value),
                ]));
            }
        }

        // Analysis
        let has_analysis = dyn_display.resolution.is_some()
            || dyn_display.orientation.is_some()
            || dyn_display.assimilation_depth.is_some();

        if has_analysis {
            lines.push(Line::from(""));
            lines.push(Line::from_spans([Span::styled("Analysis", STYLES.muted)]));

            if let Some(v) = &dyn_display.resolution {
                lines.push(Line::from_spans([
                    Span::styled("  Resolution     ", STYLES.label),
                    Span::styled(v, STYLES.value),
                ]));
            }
            if let Some(v) = &dyn_display.orientation {
                lines.push(Line::from_spans([
                    Span::styled("  Orientation    ", STYLES.label),
                    Span::styled(v, STYLES.value),
                ]));
            }
            if let Some(v) = &dyn_display.assimilation_depth {
                lines.push(Line::from_spans([
                    Span::styled("  Assimilation   ", STYLES.label),
                    Span::styled(v, STYLES.value),
                ]));
            }
        }

        lines
    }

    // -----------------------------------------------------------------------
    // Section: History
    // -----------------------------------------------------------------------

    /// Returns (lines, mutation_index_per_line) where mutation_index_per_line[i]
    /// is Some(mutation_idx) for actual mutation lines, None for dividers.
    fn build_history_lines(&self, width: usize) -> (Vec<Line>, Vec<Option<usize>>) {
        let mut lines = Vec::new();
        let mut indices: Vec<Option<usize>> = Vec::new();
        let budget = width.saturating_sub(16).max(10);

        let mut last_bucket: Option<&str> = None;
        for (mut_idx, m) in self.detail.mutations.iter().enumerate() {
            let bucket = if m.relative_time.contains("just now") || m.relative_time.contains("min ago") {
                "just now"
            } else if m.relative_time.contains("hour") {
                "today"
            } else if m.relative_time == "1 day ago" {
                "yesterday"
            } else if m.relative_time.contains("day") {
                "this week"
            } else {
                "older"
            };

            if last_bucket.is_some() && last_bucket != Some(bucket) {
                lines.push(render_subtle_divider(width.min(40)));
                indices.push(None);
            }
            last_bucket = Some(bucket);

            let old_or_dash = m
                .old_value
                .as_deref()
                .map(|o| truncate(o, budget / 2).to_string())
                .unwrap_or_else(|| "\u{2014}".to_string());

            let (description, value_color) = match m.kind {
                MutationKind::Created => {
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
            indices.push(Some(mut_idx));

            // If this mutation is expanded, add detail lines
            if self.detail.expanded_mutation == Some(mut_idx) {
                if let Some(old) = &m.old_value {
                    lines.push(Line::from_spans([
                        Span::styled("              ", STYLES.muted),
                        Span::styled("was: ", STYLES.label),
                        Span::styled(old, STYLES.muted),
                    ]));
                    indices.push(None);
                }
                lines.push(Line::from_spans([
                    Span::styled("              ", STYLES.muted),
                    Span::styled("now: ", STYLES.label),
                    Span::styled(&m.new_value, STYLES.value),
                ]));
                indices.push(None);
                lines.push(Line::from_spans([
                    Span::styled("              ", STYLES.muted),
                    Span::styled(
                        format!("field: {}  kind: {:?}", m.field, m.kind),
                        STYLES.muted,
                    ),
                ]));
                indices.push(None);
            }
        }
        (lines, indices)
    }

    // -----------------------------------------------------------------------
    // Section: Children
    // -----------------------------------------------------------------------

    fn build_children_lines(&self, width: usize) -> Vec<Line> {
        let mut lines = Vec::new();
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
            if has_sparklines && !child.activity.is_empty() {
                let spark = mini_sparkline(&child.activity, 7);
                spans.push(Span::styled(" ", Style::new()));
                spans.push(Span::styled(spark, STYLES.accent));
            }
            lines.push(Line::from_spans(spans));
        }
        lines
    }

    // -----------------------------------------------------------------------
    // Main render
    // -----------------------------------------------------------------------

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
        let children_start = mutations_count;

        let mut all_lines: Vec<Line> = Vec::new();

        // --- Gap section (not selectable) ---
        all_lines.push(Line::from_spans([
            Span::styled("  ", Style::new()),
            Span::styled("Gap", Style::new().fg(WERK_THEME.border).bold()),
        ]));
        for line in self.build_gap_lines() {
            let mut spans = vec![Span::styled("  ", Style::new())];
            spans.extend(line.spans().iter().cloned());
            all_lines.push(Line::from_spans(spans));
        }
        all_lines.push(Line::from(""));

        // --- State section (not selectable) ---
        all_lines.push(Line::from_spans([
            Span::styled("  ", Style::new()),
            Span::styled("State", Style::new().fg(WERK_THEME.border).bold()),
        ]));
        for line in self.build_state_lines() {
            let mut spans = vec![Span::styled("  ", Style::new())];
            spans.extend(line.spans().iter().cloned());
            all_lines.push(Line::from_spans(spans));
        }
        all_lines.push(Line::from(""));

        // --- Dynamics section (not selectable) ---
        let dynamics_lines = self.build_dynamics_lines();
        if !dynamics_lines.is_empty() {
            all_lines.push(Line::from_spans([
                Span::styled("  ", Style::new()),
                Span::styled("Dynamics", Style::new().fg(WERK_THEME.border).bold()),
            ]));
            for line in dynamics_lines {
                let mut spans = vec![Span::styled("  ", Style::new())];
                spans.extend(line.spans().iter().cloned());
                all_lines.push(Line::from_spans(spans));
            }
            all_lines.push(Line::from(""));
        }

        // --- History section (selectable: cursor 0..mutations_count) ---
        let has_history = !self.detail.mutations.is_empty();
        if has_history {
            let any_mutation_selected = cursor < mutations_count;
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
            let (history_lines, history_indices) = self.build_history_lines(content_width.saturating_sub(4));
            let selected_mutation = if any_mutation_selected { Some(cursor) } else { None };
            for (i, line) in history_lines.into_iter().enumerate() {
                let is_selected = selected_mutation.is_some()
                    && history_indices.get(i).copied().flatten() == selected_mutation;
                let marker = if is_selected { "\u{25b8} " } else { "  " };
                let marker_style = if is_selected { STYLES.accent_bold } else { Style::new() };
                let mut spans = vec![
                    Span::styled("  ", Style::new()),
                    Span::styled(marker, marker_style),
                ];
                spans.extend(line.spans().iter().cloned());
                all_lines.push(Line::from_spans(spans));
            }
            all_lines.push(Line::from(""));
        }

        // --- Children section (selectable: cursor mutations_count..) ---
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

        let scroll = self.detail.scroll as u16;
        let para = Paragraph::new(Text::from_lines(all_lines)).scroll((scroll, 0));
        para.render(*area, frame);
    }

    pub(crate) fn render_detail_body_responsive(&self, area: &Rect, frame: &mut Frame<'_>) {
        self.render_detail_body_inner(area, frame);
    }

    pub(crate) fn render_detail_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = StatusLine::new()
            .separator("  ")
            .left(StatusItem::key_hint("Esc", "back"))
            .left(StatusItem::key_hint("?", "help"))
            .left(StatusItem::key_hint("Ctrl-/", "commands"))
            .style(STYLES.muted);
        hints.render(*area, frame);
    }
}
