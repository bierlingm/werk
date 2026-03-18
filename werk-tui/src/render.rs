//! All rendering for the Operative Instrument.

use ftui::Frame;
use ftui::layout::Rect;
use ftui::style::Style;
use ftui::text::{Line, Span, Text};
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::status_line::{StatusLine, StatusItem};

use werk_shared::truncate;

use crate::app::InstrumentApp;
use crate::glyphs;
use crate::state::*;
use crate::theme::*;

/// Maximum content width. Wide terminals get margin.
const MAX_CONTENT_WIDTH: u16 = 104;
/// Left indent for all content.
const INDENT: &str = "  ";

impl InstrumentApp {
    /// Constrain area to max content width, centered horizontally on wide terminals.
    pub(crate) fn content_area(&self, area: Rect) -> Rect {
        let width = area.width.min(MAX_CONTENT_WIDTH);
        let x_offset = if area.width > MAX_CONTENT_WIDTH {
            (area.width - MAX_CONTENT_WIDTH) / 2
        } else {
            0
        };
        // Small top padding on tall terminals
        let top_pad = if area.height > 30 { 1 } else { 0 };
        Rect::new(
            area.x + x_offset,
            area.y + top_pad,
            width,
            area.height.saturating_sub(top_pad),
        )
    }

    // -----------------------------------------------------------------------
    // Empty state
    // -----------------------------------------------------------------------

    pub fn render_empty(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        let cy = area.height / 2;

        if area.height < 6 {
            return;
        }

        let w = area.width as usize;
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("{:^width$}", "\u{25C7}", width = w), // ◇ centered
                Style::new().fg(CLR_CYAN),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{:^width$}", "nothing here yet.", width = w),
                STYLES.dim,
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{:^width$}", "press  a  to name what matters.", width = w),
                STYLES.dim,
            )),
        ];

        // Center vertically
        let start_y = cy.saturating_sub(3);
        let text_area = Rect::new(area.x, area.y + start_y, area.width, area.height - start_y);
        let para = Paragraph::new(Text::from_lines(lines));
        para.render(text_area, frame);
    }

    // -----------------------------------------------------------------------
    // Field view (the main list)
    // -----------------------------------------------------------------------

    pub fn render_field(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        let mut lines: Vec<Line> = Vec::new();

        // Parent header if descended — desire + temporal annotations
        if let Some(ref _parent) = self.parent_tension {
            let w = (area.width as usize).saturating_sub(4);

            // Build right-side annotation: "Mar ◌◌◦◌●◌ · 3w ago"
            let mut right_parts: Vec<String> = Vec::new();
            if let Some(ref hl) = self.parent_horizon_label {
                right_parts.push(hl.clone());
            }
            if !self.parent_temporal_indicator.is_empty() {
                right_parts.push(self.parent_temporal_indicator.clone());
            }
            if let Some(ref age) = self.parent_desire_age {
                right_parts.push(format!("· {}", age));
            }
            let right_text = right_parts.join(" ");
            let right_w = right_text.chars().count();

            // Word-wrap desire, leaving room for annotation on first line
            let text_width = w.saturating_sub(right_w + 2);
            let desired_lines = word_wrap(&_parent.desired, text_width);

            for (i, line) in desired_lines.iter().enumerate() {
                if i == 0 && !right_text.is_empty() {
                    let padded = format!("{:<width$}", line, width = text_width);
                    lines.push(Line::from_spans([
                        Span::styled(format!("{}{}", INDENT, padded), STYLES.text_bold),
                        Span::styled(format!("  {}", right_text), STYLES.dim),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("{}{}", INDENT, line),
                        STYLES.text_bold,
                    )));
                }
            }

            // Heavy rule — firm, anchored
            lines.push(Line::from(Span::styled(
                format!(
                    "{}{}",
                    INDENT,
                    glyphs::HEAVY_RULE.to_string().repeat(w)
                ),
                STYLES.dim,
            )));
        }

        let is_descended = self.parent_tension.is_some();

        // Build tension lines with trunk line (descended) and gaze expansion
        for (i, entry) in self.siblings.iter().enumerate() {
            let is_selected = i == self.vlist.cursor;
            let is_gazed = self.gaze.as_ref().map(|g| g.index == i).unwrap_or(false);
            let is_positioned = entry.position.is_some();
            let is_last_positioned = is_positioned && self.siblings.get(i + 1)
                .map(|next| next.position.is_none()).unwrap_or(true);

            if is_descended && is_positioned {
                // Trunk line segment above this child (connecting from previous or header)
                if i == 0 || self.siblings.get(i.wrapping_sub(1)).map(|prev| prev.position.is_some()).unwrap_or(false) {
                    lines.push(Line::from(Span::styled(
                        format!("{}│", INDENT),
                        STYLES.dim,
                    )));
                }
            }

            // Tension line(s)
            lines.extend(self.build_tension_lines(entry, is_selected, is_gazed, area.width));

            // Gaze expansion (if this tension is gazed)
            if is_gazed {
                if let Some(ref gaze_data) = self.gaze_data {
                    let gaze_lines = self.build_gaze_lines(gaze_data, area.width);
                    lines.extend(gaze_lines);
                }
                if self.gaze.as_ref().map(|g| g.full).unwrap_or(false) {
                    if let Some(ref full_data) = self.full_gaze_data {
                        let full_lines = self.build_full_gaze_lines(full_data, area.width);
                        lines.extend(full_lines);
                    }
                }
            }

            // Dotted separator between positioned and unpositioned groups
            if is_descended && is_last_positioned {
                let has_unpositioned = self.siblings.iter().skip(i + 1).any(|s| s.position.is_none());
                if has_unpositioned {
                    let w = (area.width as usize).saturating_sub(4);
                    lines.push(Line::from(Span::styled(
                        format!("{}{}",INDENT, "· ".repeat(w / 2)),
                        STYLES.dim,
                    )));
                }
            }
        }

        // Parent reality footer with temporal annotation
        if let Some(ref parent) = self.parent_tension {
            if !parent.actual.is_empty() {
                let w = (area.width as usize).saturating_sub(4);

                // Light rule — fluid, shifting
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("{}{}", INDENT, glyphs::LIGHT_RULE.to_string().repeat(w)),
                    STYLES.dim,
                )));

                // Reality text with temporal annotation on first line
                let right_text = self.parent_reality_age.as_deref().unwrap_or("");
                let right_w = right_text.chars().count();
                let text_width = w.saturating_sub(right_w + 2);
                let reality_lines = word_wrap(&parent.actual, text_width);

                for (i, line) in reality_lines.iter().enumerate() {
                    if i == 0 && !right_text.is_empty() {
                        let padded = format!("{:<width$}", line, width = text_width);
                        lines.push(Line::from_spans([
                            Span::styled(format!("{}{}", INDENT, padded), STYLES.dim),
                            Span::styled(format!("  {}", right_text), STYLES.dim),
                        ]));
                    } else {
                        lines.push(Line::from(Span::styled(
                            format!("{}{}", INDENT, line),
                            STYLES.dim,
                        )));
                    }
                }
            }
        }

        // Apply scroll offset
        let scroll = self.vlist.scroll_offset.saturating_sub(0); // header not counted in vlist
        let para = Paragraph::new(Text::from_lines(lines)).scroll((scroll as u16, 0));
        para.render(area, frame);
    }

    fn build_tension_lines(
        &self,
        entry: &FieldEntry,
        selected: bool,
        gazed: bool,
        width: u16,
    ) -> Vec<Line> {
        let glyph = glyphs::status_glyph(entry.status, entry.phase);
        let indicator = &entry.temporal_indicator;
        let indicator_width = indicator.chars().count();

        // Glyph color based on tendency
        let tendency_color = match entry.tendency {
            sd_core::StructuralTendency::Advancing => CLR_CYAN,
            sd_core::StructuralTendency::Stagnant => CLR_DEFAULT,
            sd_core::StructuralTendency::Oscillating => CLR_AMBER,
        };

        let base_style = if selected {
            STYLES.selected
        } else if entry.status == sd_core::TensionStatus::Resolved
            || entry.status == sd_core::TensionStatus::Released
        {
            STYLES.dim
        } else {
            STYLES.text
        };

        let glyph_style = if gazed {
            Style::new().fg(CLR_CYAN)
        } else if entry.status == sd_core::TensionStatus::Resolved
            || entry.status == sd_core::TensionStatus::Released
        {
            STYLES.dim
        } else {
            Style::new().fg(tendency_color)
        };

        // Temporal indicator color based on urgency
        let indicator_color = if entry.temporal_urgency > 0.8 {
            CLR_RED
        } else if entry.temporal_urgency > 0.5 {
            CLR_AMBER
        } else {
            CLR_CYAN
        };
        let indicator_style = if selected {
            STYLES.selected
        } else {
            Style::new().fg(indicator_color)
        };

        // Right side: horizon label + temporal indicator
        let horizon_str = entry.horizon_label.as_deref().unwrap_or("");
        let horizon_w = if horizon_str.is_empty() { 0 } else { horizon_str.chars().count() + 1 };
        let suffix_w = horizon_w + indicator_width + 1; // horizon + space + indicator

        // Layout: INDENT + glyph + " " + text + " " + horizon + " " + indicator
        let prefix_w = INDENT.len() + 2 + 1; // indent + glyph + space
        let desired_budget = (width as usize).saturating_sub(prefix_w + suffix_w).max(10);

        let dim_style = if selected { STYLES.selected } else { STYLES.dim };

        if selected && entry.desired.chars().count() > desired_budget {
            // Focused + long text: word-wrap, first line gets glyph + suffix
            let wrapped = word_wrap(&entry.desired, desired_budget);
            let mut lines = Vec::new();
            for (i, line_text) in wrapped.iter().enumerate() {
                if i == 0 {
                    let padded = format!("{:<width$}", line_text, width = desired_budget);
                    let mut spans = vec![
                        Span::styled(INDENT, Style::new()),
                        Span::styled(format!("{} ", glyph), glyph_style),
                        Span::styled(padded, base_style),
                    ];
                    if !horizon_str.is_empty() {
                        spans.push(Span::styled(format!(" {}", horizon_str), dim_style));
                    }
                    spans.push(Span::styled(format!(" {}", indicator), indicator_style));
                    lines.push(Line::from_spans(spans));
                } else {
                    let indent_pad = " ".repeat(prefix_w);
                    lines.push(Line::from_spans([
                        Span::styled(indent_pad, Style::new()),
                        Span::styled(line_text.as_str(), base_style),
                    ]));
                }
            }
            lines
        } else {
            // Single line: truncate if needed
            let desired_trunc = truncate(&entry.desired, desired_budget);
            let name_padded = format!("{:<width$}", desired_trunc, width = desired_budget);

            let mut spans = vec![
                Span::styled(INDENT, Style::new()),
                Span::styled(format!("{} ", glyph), glyph_style),
                Span::styled(name_padded, base_style),
            ];
            if !horizon_str.is_empty() {
                spans.push(Span::styled(format!(" {}", horizon_str), dim_style));
            }
            spans.push(Span::styled(format!(" {}", indicator), indicator_style));
            vec![Line::from_spans(spans)]
        }
    }

    fn build_gaze_lines(&self, data: &GazeData, width: u16) -> Vec<Line> {
        let mut lines = Vec::new();
        let w = (width as usize).saturating_sub(4);

        // Children preview — the action steps
        if !data.children.is_empty() {
            for child in &data.children {
                let glyph = glyphs::status_glyph(child.status, child.phase);
                let child_budget = w.saturating_sub(6);
                lines.push(Line::from_spans([
                    Span::styled(format!("{}  {} ", INDENT, glyph), STYLES.dim),
                    Span::styled(
                        truncate(&child.desired, child_budget).to_string(),
                        STYLES.dim,
                    ),
                ]));
            }
        }

        // Reality — the structural ground
        let rule = glyphs::LIGHT_RULE.to_string().repeat(w);
        lines.push(Line::from(Span::styled(
            format!("{}{}", INDENT, rule),
            STYLES.dim,
        )));
        let actual_lines = word_wrap(&data.actual, w);
        for line in &actual_lines {
            lines.push(Line::from(Span::styled(
                format!("{}{}", INDENT, line),
                STYLES.dim,
            )));
        }

        lines
    }

    fn build_full_gaze_lines(&self, data: &crate::state::FullGazeData, width: u16) -> Vec<Line> {
        let mut lines = Vec::new();
        let w = (width as usize).saturating_sub(4);
        let rule = glyphs::LIGHT_RULE.to_string().repeat(w);

        // Dynamics + History side by side below reality
        lines.push(Line::from(Span::styled(
            format!("{}{}", INDENT, rule),
            STYLES.dim,
        )));

        // Build dynamics column
        let mut dyn_lines: Vec<(String, String, Style)> = Vec::new();
        dyn_lines.push(("phase".to_string(), data.phase.clone(), STYLES.text));
        dyn_lines.push(("tendency".to_string(), data.tendency.clone(), STYLES.text));
        if let Some(mag) = data.magnitude {
            let label = if mag > 0.7 { "large" } else if mag > 0.4 { "moderate" } else { "small" };
            dyn_lines.push(("magnitude".to_string(), label.to_string(), STYLES.text));
        }
        if let Some(ref v) = data.orientation {
            dyn_lines.push(("orientation".to_string(), v.clone(), STYLES.text));
        }
        if let Some(ref v) = data.conflict {
            dyn_lines.push(("conflict".to_string(), v.clone(), STYLES.red));
        }
        if let Some(ref v) = data.neglect {
            dyn_lines.push(("neglect".to_string(), v.clone(), STYLES.amber));
        }
        if let Some(ref v) = data.oscillation {
            dyn_lines.push(("oscillation".to_string(), v.clone(), STYLES.amber));
        }
        if let Some(ref v) = data.resolution {
            dyn_lines.push(("resolution".to_string(), v.clone(), STYLES.text));
        }
        if let Some(ref v) = data.compensating_strategy {
            dyn_lines.push(("strategy".to_string(), v.clone(), STYLES.amber));
        }
        if let Some(ref v) = data.assimilation {
            dyn_lines.push(("assimilation".to_string(), v.clone(), STYLES.text));
        }
        if let Some(ref v) = data.horizon_drift {
            dyn_lines.push(("drift".to_string(), v.clone(), STYLES.text));
        }

        // Build history column — show creation + most recent, cut middle if needed
        let dyn_count = dyn_lines.len();
        let max_history = dyn_count.max(3); // at least 3 history lines, match dynamics height
        let history: Vec<&crate::state::HistoryEntry> = if data.history.len() <= max_history {
            data.history.iter().collect()
        } else {
            // Keep first (creation) + most recent entries that fit
            let mut selected = vec![&data.history[0]]; // creation event
            let remaining = max_history.saturating_sub(2); // reserve 1 for creation, 1 for ellipsis
            let recent_start = data.history.len().saturating_sub(remaining);
            selected.push(&crate::state::HistoryEntry {
                relative_time: String::new(),
                description: String::new(),
            });
            // Can't push a temporary reference, so handle inline below
            let recent: Vec<&crate::state::HistoryEntry> = data.history[recent_start..].iter().collect();
            let mut result = vec![&data.history[0]];
            result.extend(recent);
            result
        };

        // Layout: dynamics left, │ divider, history right
        let dyn_col_width = 30.min(w / 2);
        let hist_col_width = w.saturating_sub(dyn_col_width + 3); // 3 = " │ "
        let row_count = dyn_count.max(history.len());

        for i in 0..row_count {
            let left = if i < dyn_lines.len() {
                let (ref label, ref value, style) = dyn_lines[i];
                vec![
                    Span::styled(format!("{}{:<13}", INDENT, label), STYLES.label),
                    Span::styled(
                        format!("{:<width$}", value, width = dyn_col_width.saturating_sub(13 + INDENT.len())),
                        style,
                    ),
                ]
            } else {
                vec![Span::styled(
                    " ".repeat(INDENT.len() + dyn_col_width.saturating_sub(INDENT.len())),
                    Style::new(),
                )]
            };

            let divider = Span::styled(" \u{2502} ", STYLES.dim); // │

            let right = if i < history.len() {
                let entry = history[i];
                if entry.relative_time.is_empty() && entry.description.is_empty() {
                    vec![Span::styled("\u{22EE}", STYLES.dim)] // ⋮ ellipsis
                } else {
                    let time_w = 12;
                    let desc_w = hist_col_width.saturating_sub(time_w + 1);
                    vec![
                        Span::styled(format!("{:<width$}", entry.relative_time, width = time_w), STYLES.dim),
                        Span::styled(truncate(&entry.description, desc_w).to_string(), STYLES.text),
                    ]
                }
            } else {
                vec![Span::styled("", Style::new())]
            };

            let mut spans = left;
            spans.push(divider);
            spans.extend(right);
            lines.push(Line::from_spans(spans));
        }

        lines
    }

    // -----------------------------------------------------------------------
    // Lever (bottom line)
    // -----------------------------------------------------------------------

    pub fn render_lever(&self, area: &Rect, frame: &mut Frame<'_>) {
        // Check for transient message first
        if let Some(ref msg) = self.transient {
            if !msg.is_expired() {
                let text = format!(" {}", msg.text);
                let status = StatusLine::new()
                    .left(StatusItem::text(&text))
                    .style(STYLES.cyan);
                status.render(*area, frame);
                return;
            }
        }

        let crumbs = &self.breadcrumb_cache;
        let left_text = if crumbs.is_empty() {
            " werk".to_string()
        } else {
            let path: String = crumbs
                .iter()
                .map(|(glyph, name)| {
                    let short = truncate(name, 20);
                    format!("{} {}", glyph, short)
                })
                .collect::<Vec<_>>()
                .join(" \u{203A} "); // ›
            format!(" {}", path)
        };

        let mut right_parts: Vec<String> = Vec::new();

        // Show filter state when not default
        if !matches!(self.filter, crate::app::Filter::Active) {
            right_parts.push(format!("filter: {}", self.filter.label()));
        }

        // Show insight count if any
        if self.pending_insight_count > 0 {
            right_parts.push(format!(
                "{} insight{}",
                self.pending_insight_count,
                if self.pending_insight_count == 1 { "" } else { "s" },
            ));
        }

        right_parts.push("? help".to_string());
        let right_text = format!("{} ", right_parts.join("  "));

        let status = StatusLine::new()
            .left(StatusItem::text(&left_text))
            .right(StatusItem::text(&right_text))
            .style(STYLES.lever);
        status.render(*area, frame);
    }

    // -----------------------------------------------------------------------
    // Hints (bottom-most line)
    // -----------------------------------------------------------------------

    pub fn render_hints(&self, _area: &Rect, _frame: &mut Frame<'_>) {
        // Help hint is now integrated into the lever bar
    }

    // -----------------------------------------------------------------------
    // Help overlay
    // -----------------------------------------------------------------------

    pub fn render_help(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        crate::helpers::clear_area(frame, area);

        // Two-column layout with precise alignment
        let col1 = [
            ("", "NAVIGATION", "", "ACTS"),
            ("", "", "", ""),
            ("j/k", "move up/down", "a", "add tension"),
            ("l/Enter", "descend", "e", "edit"),
            ("h/Bksp", "ascend", "n", "add note"),
            ("", "", "r", "resolve"),
            ("g", "jump to top", "x", "release"),
            ("G", "jump to bottom", "o", "reopen"),
            ("", "", "m", "move/reparent"),
            ("", "VIEWS", "@", "agent (one-shot)"),
            ("", "", "", ""),
            ("Space", "gaze (expand)", "u", "undo"),
            ("Tab", "full dynamics", "y", "copy ID"),
            ("/", "search", "f", "filter"),
            ("", "", "i", "insights"),
            ("", "", "q", "quit"),
        ];

        let start_y = area.height.saturating_sub(col1.len() as u16 + 3) / 2;
        let left_pad = (area.width as usize).saturating_sub(60) / 2;
        let pad = " ".repeat(left_pad);

        let mut lines: Vec<Line> = Vec::new();
        for (key1, desc1, key2, desc2) in &col1 {
            let mut spans = vec![Span::styled(&pad, Style::new())];
            if !key1.is_empty() {
                spans.push(Span::styled(format!("{:<10}", key1), STYLES.cyan));
            } else {
                spans.push(Span::styled("          ", Style::new()));
            }
            spans.push(Span::styled(format!("{:<18}", desc1), STYLES.text));
            if !key2.is_empty() {
                spans.push(Span::styled(format!("{:<4}", key2), STYLES.cyan));
            } else {
                spans.push(Span::styled("    ", Style::new()));
            }
            spans.push(Span::styled(*desc2, STYLES.text));
            lines.push(Line::from_spans(spans));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("{}        press any key to close", pad),
            STYLES.dim,
        )));

        let text_area = Rect::new(area.x, area.y + start_y, area.width, area.height - start_y);
        let para = Paragraph::new(Text::from_lines(lines));
        para.render(text_area, frame);
    }

    // -----------------------------------------------------------------------
    // Add prompt (inline)
    // -----------------------------------------------------------------------

    pub fn render_add_prompt(&self, step: &crate::state::AddStep, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);

        // Position the prompt inline — after the parent header and siblings,
        // right where the new tension will appear in the list.
        let header_lines: u16 = if self.parent_tension.is_some() { 3 } else { 0 };
        let sibling_lines = self.siblings.len() as u16;
        let prompt_y = area.y + header_lines + sibling_lines;
        let prompt_area = Rect::new(area.x, prompt_y, area.width, 4);
        crate::helpers::clear_area(frame, prompt_area);

        let (label, hint) = match step {
            crate::state::AddStep::Name => ("name", ""),
            crate::state::AddStep::Desire { .. } => ("desire", "  (Esc to skip)"),
            crate::state::AddStep::Reality { .. } => ("reality", ""),
            crate::state::AddStep::Horizon { .. } => ("horizon", "  e.g. 2026-W13 or tomorrow  (Esc to skip)"),
        };

        let lines = vec![
            Line::from(""),
            Line::from_spans([
                Span::styled(format!("{}{}: ", INDENT, label), STYLES.dim),
                Span::styled(&self.input_buffer, STYLES.text_bold),
                Span::styled("\u{2588}", STYLES.cyan), // cursor block
                Span::styled(hint, STYLES.dim),
            ]),
        ];

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(prompt_area, frame);
    }

    // -----------------------------------------------------------------------
    // Confirm dialog (inline)
    // -----------------------------------------------------------------------

    pub fn render_confirm(&self, kind: &crate::state::ConfirmKind, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        let cy = area.height / 2;
        let prompt_area = Rect::new(area.x, area.y + cy.saturating_sub(3), area.width, 6);
        crate::helpers::clear_area(frame, prompt_area);

        let (action, desired) = match kind {
            crate::state::ConfirmKind::Resolve { desired, .. } => ("resolve", desired.as_str()),
            crate::state::ConfirmKind::Release { desired, .. } => ("release", desired.as_str()),
        };

        let description = match kind {
            crate::state::ConfirmKind::Resolve { .. } => "desire met reality. the gap is closed.",
            crate::state::ConfirmKind::Release { .. } => "letting it go. acknowledging the gap without closing it.",
        };

        let short = truncate(desired, 40);
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("{}  {} \"{}\"?", INDENT, action, short),
                STYLES.text_bold,
            )),
            Line::from(Span::styled(
                format!("{}  {}", INDENT, description),
                STYLES.dim,
            )),
            Line::from(""),
            Line::from_spans([
                Span::styled(format!("{}  ", INDENT), Style::new()),
                Span::styled("y", STYLES.cyan),
                Span::styled(" confirm    ", STYLES.dim),
                Span::styled("n", STYLES.cyan),
                Span::styled(" cancel", STYLES.dim),
            ]),
        ];

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(prompt_area, frame);
    }

    // -----------------------------------------------------------------------
    // Input mode hints
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Search overlay
    // -----------------------------------------------------------------------

    pub fn render_search(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        crate::helpers::clear_area(frame, area);

        let mut lines: Vec<Line> = Vec::new();

        // Search input
        let is_moving = matches!(self.input_mode, crate::state::InputMode::Moving { .. });
        let prefix = if is_moving { "move to" } else { "/" };

        lines.push(Line::from_spans([
            Span::styled(format!("{}{}: ", INDENT, prefix), STYLES.label),
            Span::styled(&self.input_buffer, STYLES.text_bold),
            Span::styled("\u{2588}", STYLES.cyan),
        ]));
        lines.push(Line::from(""));

        // Results
        if let Some(ref search) = self.search_state {
            for (i, result) in search.results.iter().enumerate() {
                let is_selected = i == search.cursor;
                let style = if is_selected { STYLES.selected } else { STYLES.text };
                let dim = if is_selected { STYLES.text_bold } else { STYLES.dim };

                let selector = if is_selected { "\u{25B8}" } else { " " };

                if result.is_root_entry {
                    lines.push(Line::from_spans([
                        Span::styled(format!("{}{} ", INDENT, selector), style),
                        Span::styled("(root level)", dim),
                    ]));
                } else {
                    let desired_budget = (area.width as usize).saturating_sub(30).max(15);
                    lines.push(Line::from_spans([
                        Span::styled(format!("{}{} ", INDENT, selector), style),
                        Span::styled(
                            truncate(&result.desired, desired_budget).to_string(),
                            style,
                        ),
                        Span::styled(
                            format!("  {}", result.parent_path),
                            STYLES.dim,
                        ),
                    ]));
                }
            }

            if search.results.is_empty() && !self.input_buffer.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("{}  no matches", INDENT),
                    STYLES.dim,
                )));
            }
        }

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(area, frame);
    }

    // -----------------------------------------------------------------------
    // Edit prompt (inline)
    // -----------------------------------------------------------------------

    pub fn render_edit_prompt(&self, field: &crate::state::EditField, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        let bottom_y = area.height.saturating_sub(5);
        let prompt_area = Rect::new(area.x, area.y + bottom_y, area.width, 4);
        crate::helpers::clear_area(frame, prompt_area);

        let label = match field {
            crate::state::EditField::Desire => "desire",
            crate::state::EditField::Reality => "reality",
            crate::state::EditField::Horizon => "horizon",
        };

        let lines = vec![
            Line::from(""),
            Line::from_spans([
                Span::styled(format!("{}  {}: ", INDENT, label), STYLES.label),
                Span::styled(&self.input_buffer, STYLES.text_bold),
                Span::styled("\u{2588}", STYLES.cyan),
            ]),
            Line::from(Span::styled(
                format!("{}  Tab to switch field", INDENT),
                STYLES.dim,
            )),
        ];

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(prompt_area, frame);
    }

    // -----------------------------------------------------------------------
    // Note prompt (inline)
    // -----------------------------------------------------------------------

    pub fn render_note_prompt(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        let bottom_y = area.height.saturating_sub(4);
        let prompt_area = Rect::new(area.x, area.y + bottom_y, area.width, 3);
        crate::helpers::clear_area(frame, prompt_area);

        let lines = vec![
            Line::from(""),
            Line::from_spans([
                Span::styled(format!("{}  note: ", INDENT), STYLES.label),
                Span::styled(&self.input_buffer, STYLES.text_bold),
                Span::styled("\u{2588}", STYLES.cyan),
            ]),
        ];

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(prompt_area, frame);
    }

    // -----------------------------------------------------------------------
    // Input mode hints
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Agent prompt (inline)
    // -----------------------------------------------------------------------

    pub fn render_agent_prompt(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        let bottom_y = area.height.saturating_sub(5);
        let prompt_area = Rect::new(area.x, area.y + bottom_y, area.width, 4);
        crate::helpers::clear_area(frame, prompt_area);

        let lines = vec![
            Line::from(""),
            Line::from_spans([
                Span::styled(format!("{}@ ", INDENT), STYLES.cyan),
                Span::styled(&self.input_buffer, STYLES.text_bold),
                Span::styled("\u{2588}", STYLES.cyan),
            ]),
            Line::from(Span::styled(
                format!("{}  type ! for clipboard handoff", INDENT),
                STYLES.dim,
            )),
        ];

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(prompt_area, frame);
    }

    // -----------------------------------------------------------------------
    // Mutation review (full screen)
    // -----------------------------------------------------------------------

    pub fn render_mutation_review(&self, area: &Rect, frame: &mut Frame<'_>) {
        // Clear the FULL area (not just content_area) to prevent field bleeding through
        crate::helpers::clear_area(frame, *area);
        let area = self.content_area(*area);

        let mut lines: Vec<Line> = Vec::new();

        // Header
        lines.push(Line::from(Span::styled(
            format!("{}agent response", INDENT),
            STYLES.cyan,
        )));
        lines.push(Line::from(""));

        // Response text with manual word wrapping
        if let Some(ref text) = self.agent_response_text {
            let wrap_width = (area.width as usize).saturating_sub(4);
            for paragraph in text.split("\n\n") {
                for line in paragraph.lines() {
                    for wrapped in word_wrap(line, wrap_width) {
                        lines.push(Line::from(Span::styled(
                            format!("{}{}", INDENT, wrapped),
                            STYLES.text,
                        )));
                    }
                }
                lines.push(Line::from(""));
            }
        }

        // Mutations
        if !self.agent_mutations.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("{}suggested changes:", INDENT),
                STYLES.dim,
            )));
            lines.push(Line::from(""));

            for (i, mutation) in self.agent_mutations.iter().enumerate() {
                let is_selected = i == self.agent_mutation_cursor;
                let is_checked = self.agent_mutation_selected.get(i).copied().unwrap_or(false);
                let check = if is_checked { "x" } else { " " };
                let style = if is_selected { STYLES.selected } else { STYLES.text };

                let summary = mutation.summary();
                let cursor_char = if is_selected { "\u{25B8}" } else { " " };

                lines.push(Line::from_spans([
                    Span::styled(format!("{}{} [{}] ", INDENT, cursor_char, check), style),
                    Span::styled(summary, style),
                ]));

                if is_selected {
                    if let Some(reasoning) = mutation.reasoning() {
                        lines.push(Line::from(Span::styled(
                            format!("{}      {}", INDENT, truncate(reasoning, 70)),
                            STYLES.dim,
                        )));
                    }
                }
            }
        } else if self.agent_response_text.is_some() {
            lines.push(Line::from(Span::styled(
                format!("{}no structured mutations \u{2014} press Esc to close", INDENT),
                STYLES.dim,
            )));
        }

        let para = Paragraph::new(Text::from_lines(lines)).scroll((0, 0));
        para.render(area, frame);
    }

    pub fn render_insight_review(&self, area: &Rect, frame: &mut Frame<'_>) {
        crate::helpers::clear_area(frame, *area);
        let area = self.content_area(*area);
        let mut lines: Vec<Line> = Vec::new();
        let w = (area.width as usize).saturating_sub(4);
        let rule = glyphs::LIGHT_RULE.to_string().repeat(w);
        let wrap_width = w.saturating_sub(2);

        if self.pending_insights.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("{}no pending insights", INDENT),
                STYLES.dim,
            )));
            let para = Paragraph::new(Text::from_lines(lines));
            para.render(area, frame);
            return;
        }

        // Header
        lines.push(Line::from(Span::styled(
            format!(
                "{}the daimon noticed ({} insight{})",
                INDENT,
                self.pending_insights.len(),
                if self.pending_insights.len() == 1 { "" } else { "s" },
            ),
            STYLES.cyan,
        )));
        lines.push(Line::from(""));

        for (i, insight) in self.pending_insights.iter().enumerate() {
            let is_selected = i == self.insight_cursor;
            let cursor_char = if is_selected { "\u{25B8}" } else { " " };
            let style = if is_selected { STYLES.selected } else { STYLES.text };

            // Trigger as human-readable label
            let trigger_label = match insight.trigger.as_str() {
                "conflict_detected" => "conflict detected",
                "oscillation_spike" => "oscillation spike",
                "neglect_onset" => "neglect onset",
                "horizon_breach" => "horizon breached",
                "phase_transition" => "phase transition",
                "stagnation" => "stagnation",
                "resolution" => "resolved",
                other => other,
            };

            lines.push(Line::from_spans([
                Span::styled(format!("{}{} ", INDENT, cursor_char), style),
                Span::styled(trigger_label, if is_selected { STYLES.cyan } else { STYLES.amber }),
                Span::styled(
                    format!(" on \"{}\"", truncate(&insight.tension_desired, 35)),
                    style,
                ),
            ]));

            // Gaze-like expansion: only when Space is pressed (expanded)
            if insight.expanded {
                lines.push(Line::from(Span::styled(
                    format!("{}{}", INDENT, rule),
                    STYLES.dim,
                )));

                // Clean response text
                let clean_response = clean_agent_response(&insight.response);
                for line in clean_response.lines() {
                    if line.is_empty() {
                        lines.push(Line::from(""));
                    } else {
                        for wrapped in word_wrap(line, wrap_width) {
                            lines.push(Line::from(Span::styled(
                                format!("{}  {}", INDENT, wrapped),
                                STYLES.text,
                            )));
                        }
                    }
                }

                // Suggested mutations — clean, human-readable
                if insight.mutation_count > 0 {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("{}  suggested:", INDENT),
                        STYLES.cyan,
                    )));
                    lines.push(Line::from(""));

                    for mutation in parse_mutation_display(&insight.mutation_text) {
                        lines.push(Line::from_spans([
                            Span::styled(format!("{}    \u{25B8} ", INDENT), STYLES.cyan),
                            Span::styled(&mutation.action_label, STYLES.cyan),
                        ]));
                        for detail in &mutation.details {
                            lines.push(Line::from(Span::styled(
                                format!("{}      {}", INDENT, truncate(detail, wrap_width.saturating_sub(6))),
                                STYLES.text,
                            )));
                        }
                    }
                }

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("{}{}", INDENT, rule),
                    STYLES.dim,
                )));
            }
        }

        let para = Paragraph::new(Text::from_lines(lines)).scroll((0, 0));
        para.render(area, frame);
    }

    pub fn render_input_hints(&self, text: &str, area: &Rect, frame: &mut Frame<'_>) {
        let display = format!(" {}", text);
        let hints = StatusLine::new()
            .left(StatusItem::text(&display))
            .style(STYLES.dim);
        hints.render(*area, frame);
    }
}

/// Clean an agent response: strip --- YAML blocks, session_id lines, and trailing whitespace.
fn clean_agent_response(text: &str) -> String {
    let mut result = String::new();
    let mut in_yaml_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        // Skip session_id lines
        if trimmed.starts_with("session_id:") {
            continue;
        }

        // Track --- blocks and skip them
        if trimmed == "---" {
            in_yaml_block = !in_yaml_block;
            continue;
        }
        if in_yaml_block {
            continue;
        }

        // Skip lines that look like raw YAML mutation data
        if trimmed.starts_with("- action:") || trimmed.starts_with("tension_id:")
            || trimmed.starts_with("text:") && result.contains("action:")
            || trimmed.starts_with("response: |")
            || trimmed.starts_with("mutations:")
        {
            continue;
        }

        if !result.is_empty() || !trimmed.is_empty() {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Trim trailing blank lines
    result.trim_end().to_string()
}

/// A parsed mutation for clean display.
struct MutationDisplay {
    action_label: String,
    details: Vec<String>,
}

/// Parse raw mutation text into human-readable display items.
fn parse_mutation_display(raw: &str) -> Vec<MutationDisplay> {
    let mut mutations = Vec::new();
    let mut current: Option<MutationDisplay> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('\u{25B8}') || trimmed.starts_with("- action:") {
            // New mutation
            if let Some(m) = current.take() {
                mutations.push(m);
            }
            let action = trimmed
                .trim_start_matches('\u{25B8}')
                .trim_start_matches("- action:")
                .trim();
            let label = match action {
                "add_note" => "add note".to_string(),
                "update_actual" => "update reality".to_string(),
                "update_desired" => "update desire".to_string(),
                "create_child" => "create child".to_string(),
                "update_status" => "change status".to_string(),
                "set_horizon" => "set horizon".to_string(),
                "move_tension" => "move".to_string(),
                "create_parent" => "create parent".to_string(),
                other => other.to_string(),
            };
            current = Some(MutationDisplay {
                action_label: label,
                details: Vec::new(),
            });
        } else if let Some(ref mut m) = current {
            // Detail line — clean up field names
            let cleaned = trimmed
                .trim_start_matches("tension_id:")
                .trim_start_matches("parent_id:")
                .trim_start_matches("new_value:")
                .trim_start_matches("new_status:")
                .trim();
            if trimmed.starts_with("tension_id:") || trimmed.starts_with("parent_id:") {
                // Skip IDs — not useful to the human
                continue;
            }
            if trimmed.starts_with("text:") {
                let text = trimmed.trim_start_matches("text:").trim().trim_matches('\'').trim_matches('"');
                m.details.push(format!("\"{}\"", text));
            } else if trimmed.starts_with("new_value:") || trimmed.starts_with("desired:") || trimmed.starts_with("actual:") {
                let label = if trimmed.starts_with("new_value:") { "" }
                    else if trimmed.starts_with("desired:") { "desired: " }
                    else { "actual: " };
                let val = cleaned.trim_matches('\'').trim_matches('"');
                m.details.push(format!("{}\"{}\""  , label, val));
            } else if trimmed.starts_with("reasoning:") {
                let reason = trimmed.trim_start_matches("reasoning:").trim().trim_matches('\'').trim_matches('"');
                if !reason.is_empty() {
                    m.details.push(format!("because: {}", reason));
                }
            } else if trimmed.starts_with("horizon:") {
                let h = trimmed.trim_start_matches("horizon:").trim().trim_matches('\'').trim_matches('"');
                m.details.push(format!("horizon: {}", h));
            } else if !trimmed.is_empty() {
                m.details.push(trimmed.to_string());
            }
        }
    }
    if let Some(m) = current {
        mutations.push(m);
    }
    mutations
}

/// Simple word wrapping: break a line into chunks of at most `width` characters at word boundaries.
fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if text.len() <= width {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
