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
    fn content_area(&self, area: Rect) -> Rect {
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
        let cx = area.width / 2;
        let cy = area.height / 2;

        if area.height < 6 {
            return;
        }

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("{:>width$}", "\u{25C7}", width = cx as usize), // ◇
                Style::new().fg(CLR_CYAN),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{:>width$}", "nothing here yet.", width = cx as usize + 8),
                STYLES.dim,
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{:>width$}", "press  a  to name what matters.", width = cx as usize + 15),
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

        // Parent header if descended
        if let Some(ref parent) = self.parent_tension {
            // Use cached phase from FieldEntry if available, else default
            let phase = self.parent_phase;
            let glyph = glyphs::status_glyph(parent.status, phase);

            lines.push(Line::from_spans([
                Span::styled(INDENT, Style::new()),
                Span::styled(
                    format!("{} {}", glyph, &parent.desired),
                    STYLES.text_bold,
                ),
            ]));
            lines.push(Line::from(Span::styled(
                format!(
                    "{}{}",
                    INDENT,
                    glyphs::RULE.to_string().repeat((area.width as usize).saturating_sub(4))
                ),
                STYLES.dim,
            )));
            lines.push(Line::from(""));
        }

        let header_lines = lines.len();

        // Build tension lines with Gaze expansion
        for (i, entry) in self.siblings.iter().enumerate() {
            let is_selected = i == self.vlist.cursor;
            let is_gazed = self.gaze.as_ref().map(|g| g.index == i).unwrap_or(false);

            // Tension line
            lines.push(self.build_tension_line(entry, is_selected, is_gazed, area.width));

            // Gaze expansion (if this tension is gazed)
            if is_gazed {
                if let Some(ref gaze_data) = self.gaze_data {
                    let gaze_lines = self.build_gaze_lines(gaze_data, area.width);
                    lines.extend(gaze_lines);
                }
                // Full gaze: dynamics + history (when Tab is pressed)
                if self.gaze.as_ref().map(|g| g.full).unwrap_or(false) {
                    if let Some(ref full_data) = self.full_gaze_data {
                        let full_lines = self.build_full_gaze_lines(full_data, area.width);
                        lines.extend(full_lines);
                    }
                }
            }
        }

        // Apply scroll offset
        let scroll = self.vlist.scroll_offset.saturating_sub(0); // header not counted in vlist
        let para = Paragraph::new(Text::from_lines(lines)).scroll((scroll as u16, 0));
        para.render(area, frame);
    }

    fn build_tension_line(
        &self,
        entry: &FieldEntry,
        selected: bool,
        gazed: bool,
        width: u16,
    ) -> Line {
        let glyph = glyphs::status_glyph(entry.status, entry.phase);
        let trail = glyphs::trail(&entry.activity, 8);
        let trail_width = trail.chars().count();

        // Budget for the desired text
        let fixed = 2 + 2 + 1 + trail_width + 2; // indent + glyph + space + trail + padding
        let desired_budget = (width as usize).saturating_sub(fixed).max(10);
        let desired_trunc = truncate(&entry.desired, desired_budget);

        // Right-align the trail
        let name_width = desired_budget;
        let name_padded = format!("{:<width$}", desired_trunc, width = name_width);

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
            base_style
        };

        // Children indicator: ▸ if has children, space if not (aligned)
        let children_marker = if entry.has_children { "\u{25B8}" } else { " " };

        // Extend selection background across the full line including trail
        let trail_style = if selected { STYLES.selected } else { STYLES.dim };

        Line::from_spans([
            Span::styled(INDENT, base_style),
            Span::styled(format!("{} ", glyph), glyph_style),
            Span::styled(name_padded, base_style),
            Span::styled(format!("{} {} ", children_marker, trail), trail_style),
        ])
    }

    fn build_gaze_lines(&self, data: &GazeData, width: u16) -> Vec<Line> {
        let mut lines = Vec::new();
        let w = (width as usize).saturating_sub(4);
        let rule = glyphs::LIGHT_RULE.to_string().repeat(w);

        // Light separator
        lines.push(Line::from(Span::styled(
            format!("{}{}", INDENT, rule),
            STYLES.dim,
        )));

        // ID (dim, for CLI reference)
        if let Some(entry) = self.gaze.as_ref().and_then(|g| self.siblings.get(g.index)) {
            lines.push(Line::from_spans([
                Span::styled(format!("{}id       ", INDENT), STYLES.dim),
                Span::styled(&entry.id, STYLES.dim),
            ]));
        }

        // Desire — show full text, word-wrapped
        let label_width = 9; // "desire   " or "reality  "
        let text_width = w.saturating_sub(label_width + 2);
        let desired_lines = word_wrap(&data.desired, text_width);
        for (i, line) in desired_lines.iter().enumerate() {
            let label = if i == 0 { "desire   " } else { "         " };
            lines.push(Line::from_spans([
                Span::styled(format!("{}{}", INDENT, label), STYLES.label),
                Span::styled(line.as_str(), STYLES.text),
            ]));
        }

        // Reality — show full text, word-wrapped
        let actual_lines = word_wrap(&data.actual, text_width);
        for (i, line) in actual_lines.iter().enumerate() {
            let label = if i == 0 { "reality  " } else { "         " };
            lines.push(Line::from_spans([
                Span::styled(format!("{}{}", INDENT, label), STYLES.label),
                Span::styled(line.as_str(), STYLES.text),
            ]));
        }

        // Horizon (if set)
        if let Some(ref horizon) = data.horizon {
            lines.push(Line::from_spans([
                Span::styled(format!("{}horizon  ", INDENT), STYLES.label),
                Span::styled(horizon, STYLES.text),
            ]));
        }

        // Children preview
        if !data.children.is_empty() {
            lines.push(Line::from(""));
            for child in &data.children {
                let glyph = glyphs::status_glyph(child.status, child.phase);
                let child_budget = w.saturating_sub(8);
                lines.push(Line::from_spans([
                    Span::styled(format!("{}  {} ", INDENT, glyph), STYLES.dim),
                    Span::styled(
                        truncate(&child.desired, child_budget).to_string(),
                        STYLES.dim,
                    ),
                ]));
            }
        }

        // Gap bar
        if let Some(mag) = data.magnitude {
            lines.push(Line::from(""));
            let bar = glyphs::gap_bar(mag, 16);
            let label = if mag > 0.7 {
                "large"
            } else if mag > 0.4 {
                "moderate"
            } else {
                "small"
            };
            lines.push(Line::from_spans([
                Span::styled(format!("{}gap      ", INDENT), STYLES.label),
                Span::styled(bar, STYLES.cyan),
                Span::styled(format!("  {}", label), STYLES.dim),
            ]));
        }

        // Conflict (only if present)
        if let Some(ref conflict) = data.conflict {
            lines.push(Line::from_spans([
                Span::styled(format!("{}conflict ", INDENT), STYLES.label),
                Span::styled(conflict, STYLES.red),
            ]));
        }

        // Neglect (only if present)
        if let Some(ref neglect) = data.neglect {
            lines.push(Line::from_spans([
                Span::styled(format!("{}neglect  ", INDENT), STYLES.label),
                Span::styled(neglect, STYLES.amber),
            ]));
        }

        // Oscillation (only if present)
        if let Some(ref osc) = data.oscillation {
            lines.push(Line::from_spans([
                Span::styled(format!("{}pattern  ", INDENT), STYLES.label),
                Span::styled(osc, STYLES.amber),
            ]));
        }

        // Light separator
        lines.push(Line::from(Span::styled(
            format!("{}{}", INDENT, rule),
            STYLES.dim,
        )));

        lines
    }

    fn build_full_gaze_lines(&self, data: &crate::state::FullGazeData, width: u16) -> Vec<Line> {
        let mut lines = Vec::new();
        let w = (width as usize).saturating_sub(4);
        let rule_str: String = glyphs::RULE.to_string().repeat(w.min(20));

        // Dynamics section
        lines.push(Line::from(""));
        lines.push(Line::from_spans([
            Span::styled(format!("{}", INDENT), Style::new()),
            Span::styled(format!("{} dynamics {}", glyphs::RULE, rule_str), STYLES.dim),
        ]));

        // Always show phase + tendency
        lines.push(Line::from_spans([
            Span::styled(format!("{}phase        ", INDENT), STYLES.label),
            Span::styled(&data.phase, STYLES.text),
        ]));
        lines.push(Line::from_spans([
            Span::styled(format!("{}tendency     ", INDENT), STYLES.label),
            Span::styled(&data.tendency, STYLES.text),
        ]));
        if let Some(mag) = data.magnitude {
            let label = if mag > 0.7 { "large" } else if mag > 0.4 { "moderate" } else { "small" };
            lines.push(Line::from_spans([
                Span::styled(format!("{}magnitude    ", INDENT), STYLES.label),
                Span::styled(label, STYLES.text),
            ]));
        }
        if let Some(ref v) = data.orientation {
            lines.push(Line::from_spans([
                Span::styled(format!("{}orientation  ", INDENT), STYLES.label),
                Span::styled(v, STYLES.text),
            ]));
        }
        if let Some(ref v) = data.conflict {
            lines.push(Line::from_spans([
                Span::styled(format!("{}conflict     ", INDENT), STYLES.label),
                Span::styled(v, STYLES.red),
            ]));
        }
        if let Some(ref v) = data.neglect {
            lines.push(Line::from_spans([
                Span::styled(format!("{}neglect      ", INDENT), STYLES.label),
                Span::styled(v, STYLES.amber),
            ]));
        }
        if let Some(ref v) = data.oscillation {
            lines.push(Line::from_spans([
                Span::styled(format!("{}oscillation  ", INDENT), STYLES.label),
                Span::styled(v, STYLES.amber),
            ]));
        }
        if let Some(ref v) = data.resolution {
            lines.push(Line::from_spans([
                Span::styled(format!("{}resolution   ", INDENT), STYLES.label),
                Span::styled(v, STYLES.text),
            ]));
        }
        if let Some(ref v) = data.compensating_strategy {
            lines.push(Line::from_spans([
                Span::styled(format!("{}strategy     ", INDENT), STYLES.label),
                Span::styled(v, STYLES.amber),
            ]));
        }
        if let Some(ref v) = data.assimilation {
            lines.push(Line::from_spans([
                Span::styled(format!("{}assimilation ", INDENT), STYLES.label),
                Span::styled(v, STYLES.text),
            ]));
        }
        if let Some(ref v) = data.horizon_drift {
            lines.push(Line::from_spans([
                Span::styled(format!("{}drift        ", INDENT), STYLES.label),
                Span::styled(v, STYLES.text),
            ]));
        }

        // History section
        if !data.history.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from_spans([
                Span::styled(format!("{}", INDENT), Style::new()),
                Span::styled(format!("{} history {}", glyphs::RULE, rule_str), STYLES.dim),
            ]));
            for entry in &data.history {
                lines.push(Line::from_spans([
                    Span::styled(format!("{}{:<14}", INDENT, entry.relative_time), STYLES.dim),
                    Span::styled(&entry.description, STYLES.text),
                ]));
            }
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

        let right_text = if self.pending_insight_count > 0 {
            format!(
                "{} insight{} waiting ",
                self.pending_insight_count,
                if self.pending_insight_count == 1 { "" } else { "s" },
            )
        } else if self.siblings.is_empty() {
            if self.parent_id.is_none() {
                format!("{} tensions ", self.total_count)
            } else {
                "no children ".to_string()
            }
        } else {
            format!(
                "{} of {} ",
                self.vlist.cursor + 1,
                self.siblings.len(),
            )
        };

        let status = StatusLine::new()
            .left(StatusItem::text(&left_text))
            .right(StatusItem::text(&right_text))
            .style(STYLES.lever);
        status.render(*area, frame);
    }

    // -----------------------------------------------------------------------
    // Hints (bottom-most line)
    // -----------------------------------------------------------------------

    pub fn render_hints(&self, area: &Rect, frame: &mut Frame<'_>) {
        let hints = StatusLine::new()
            .separator("  ")
            .left(StatusItem::key_hint("?", "help"))
            .style(STYLES.dim);
        hints.render(*area, frame);
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
        let bottom_y = area.height.saturating_sub(5);
        let prompt_area = Rect::new(area.x, area.y + bottom_y, area.width, 4);
        crate::helpers::clear_area(frame, prompt_area);

        let (label, hint) = match step {
            crate::state::AddStep::Name => ("name", ""),
            crate::state::AddStep::Desire { .. } => ("desire", "  (Esc to skip)"),
            crate::state::AddStep::Reality { .. } => ("reality", "  (Esc to skip)"),
            crate::state::AddStep::Horizon { .. } => ("horizon", "  e.g. 2026-W13 or 2026-03-20  (Esc to skip)"),
        };

        let lines = vec![
            Line::from(""),
            Line::from_spans([
                Span::styled(format!("{}  \u{2514}\u{2500} {}: ", INDENT, label), STYLES.dim),
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
