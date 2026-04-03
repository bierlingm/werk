//! All rendering for the Operative Instrument.

use ftui::Frame;
use ftui::layout::Rect;
use ftui::style::Style;
use ftui::text::{Line, Span, Text};
use ftui::widgets::Widget;
use ftui::widgets::borders::BorderType;
use ftui::widgets::panel::Panel;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::status_line::{StatusLine, StatusItem};

use werk_shared::truncate;

use crate::app::InstrumentApp;
use crate::glyphs;
use crate::state::*;

// Content centering moved to layout.rs — LayoutState.content_area().

/// Left indent for all content.
const INDENT: &str = "  ";
impl InstrumentApp {

    // -----------------------------------------------------------------------
    // Empty state
    // -----------------------------------------------------------------------

    pub fn render_empty(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.layout.content_area(*area);
        let cy = area.height / 2;

        if area.height < 6 {
            return;
        }

        let w = area.width as usize;
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("{:^width$}", "\u{25C7}", width = w), // ◇ centered
                Style::new().fg(self.styles.clr_cyan),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{:^width$}", "nothing here yet.", width = w),
                self.styles.dim,
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{:^width$}", "press  a  to name what matters.", width = w),
                self.styles.dim,
            )),
        ];

        // Center vertically
        let start_y = cy.saturating_sub(3);
        let text_area = Rect::new(area.x, area.y + start_y, area.width, area.height - start_y);
        let para = Paragraph::new(Text::from_lines(lines));
        para.render(text_area, frame);
    }



    // -----------------------------------------------------------------------
    // Help overlay
    // -----------------------------------------------------------------------

    pub fn render_help(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.layout.content_area(*area);
        crate::helpers::clear_area_styled(frame, area, self.styles.clr_dim);

        let w = area.width as usize;
        let content_w = w.min(72);
        let left_pad = (w.saturating_sub(content_w)) / 2;
        let pad = " ".repeat(left_pad);
        let rule_w = content_w.saturating_sub(2);
        let light_rule = glyphs::LIGHT_RULE.to_string().repeat(rule_w);

        let mut lines: Vec<Line> = Vec::new();

        // --- Navigation section ---
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("NAVIGATION", self.styles.text_bold),
        ]));
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled(&light_rule, self.styles.dim),
        ]));
        let nav_keys: &[(&str, &str, &str, &str)] = &[
            ("j/k", "move up/down", "g/G", "jump to top/bottom"),
            ("l/Enter", "descend into", "h/Bksp", "ascend out"),
            ("Shift+J/K", "reorder position", "Space", "gaze (peek)"),
            ("/", "search", "1-9", "act on alert"),
        ];
        for (k1, d1, k2, d2) in nav_keys {
            lines.push(Line::from_spans([
                Span::styled(&pad, Style::new()),
                Span::styled(format!("{:<12}", k1), self.styles.cyan),
                Span::styled(format!("{:<22}", d1), self.styles.text),
                Span::styled(format!("{:<12}", k2), self.styles.cyan),
                Span::styled(*d2, self.styles.text),
            ]));
        }

        lines.push(Line::from(""));

        // --- Acts section ---
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("ACTS", self.styles.text_bold),
        ]));
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled(&light_rule, self.styles.dim),
        ]));
        let act_keys: &[(&str, &str, &str, &str)] = &[
            ("a", "add tension", "e", "edit (desire/reality/horizon)"),
            ("n", "add note", "m", "move / reparent"),
            ("r", "resolve", "x", "release"),
            ("o", "reopen", "u/U", "undo / redo"),
            ("y", "copy ID", "f", "filter"),
            ("q", "quit", "", ""),
        ];
        for (k1, d1, k2, d2) in act_keys {
            let mut spans = vec![
                Span::styled(&pad, Style::new()),
                Span::styled(format!("{:<4}", k1), self.styles.cyan),
                Span::styled(format!("{:<30}", d1), self.styles.text),
            ];
            if !k2.is_empty() {
                spans.push(Span::styled(format!("{:<4}", k2), self.styles.cyan));
                spans.push(Span::styled(*d2, self.styles.text));
            }
            lines.push(Line::from_spans(spans));
        }

        lines.push(Line::from(""));

        // --- Glyphs section ---
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("GLYPHS & COLORS", self.styles.text_bold),
        ]));
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled(&light_rule, self.styles.dim),
        ]));

        // Glyphs with their actual colors inline
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("\u{25C7} ", Style::new().fg(self.styles.clr_text)),
            Span::styled("active        ", self.styles.dim),
            Span::styled("\u{2726} ", Style::new().fg(self.styles.clr_dim)),
            Span::styled("resolved      ", self.styles.dim),
            Span::styled("\u{00B7} ", Style::new().fg(self.styles.clr_dim)),
            Span::styled("released", self.styles.dim),
        ]));

        lines.push(Line::from(""));

        // Temporal indicator legend
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("\u{25CC}\u{25CC}\u{25E6}\u{25CC}\u{25CF}\u{25CC}", Style::new().fg(self.styles.clr_cyan)),
            Span::styled("  temporal window: ", self.styles.dim),
            Span::styled("\u{25E6}", self.styles.cyan),
            Span::styled(" now  ", self.styles.dim),
            Span::styled("\u{25CF}", self.styles.cyan),
            Span::styled(" horizon end", self.styles.dim),
        ]));
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CE}", self.styles.dim),
            Span::styled("  staleness (no horizon set)", self.styles.dim),
        ]));
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}", Style::new().fg(self.styles.clr_cyan)),
            Span::styled(" comfortable  ", self.styles.dim),
            Span::styled("\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}", Style::new().fg(self.styles.clr_amber)),
            Span::styled(" approaching  ", self.styles.dim),
            Span::styled("\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}", Style::new().fg(self.styles.clr_red)),
            Span::styled(" overdue", self.styles.dim),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("{}press any key to close", pad),
            self.styles.dim,
        )));

        // Center vertically
        let total_lines = lines.len() as u16;
        let start_y = area.height.saturating_sub(total_lines) / 2;
        let text_area = Rect::new(area.x, area.y + start_y, area.width, area.height - start_y);
        let para = Paragraph::new(Text::from_lines(lines));
        para.render(text_area, frame);
    }

    // -----------------------------------------------------------------------
    // Add prompt (inline)
    // -----------------------------------------------------------------------

    pub fn render_add_prompt(&self, step: &AddStep, area: &Rect, frame: &mut Frame<'_>) {
        // Backdrop dims the field — preserving spatial context
        crate::modal::render_backdrop(frame, *area, &self.styles);

        let area = self.layout.content_area(*area);

        // Centered modal for the add prompt
        let modal_area = crate::modal::center_modal(area, 60.min(area.width), 4);

        let (label, hint) = match step {
            AddStep::Desire => ("desire", "  (Enter to create, Tab for more)"),
            AddStep::Reality { .. } => ("reality", "  (Enter to create, Tab for more)"),
            AddStep::Horizon { .. } => ("horizon", "  e.g. 2026-W13 or tomorrow"),
        };

        let lines = vec![
            Line::from(""),
            Line::from_spans([
                Span::styled(format!(" {}: ", label), self.styles.dim),
                Span::styled(&self.input_buffer, self.styles.text_bold),
                Span::styled("\u{2588}", self.styles.cyan), // cursor block
                Span::styled(hint, self.styles.dim),
            ]),
        ];

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(modal_area, frame);
    }

    // -----------------------------------------------------------------------
    // Confirm dialog (inline)
    // -----------------------------------------------------------------------

    pub fn render_confirm(&self, kind: &ConfirmKind, area: &Rect, frame: &mut Frame<'_>) {
        // Backdrop dims the field
        crate::modal::render_backdrop(frame, *area, &self.styles);

        let area = self.layout.content_area(*area);
        let prompt_area = crate::modal::center_modal(area, 60.min(area.width), 6);

        let (action, desired) = match kind {
            ConfirmKind::Resolve { desired, .. } => ("resolve", desired.as_str()),
            ConfirmKind::Release { desired, .. } => ("release", desired.as_str()),
        };

        let description = match kind {
            ConfirmKind::Resolve { .. } => "desire met reality. the gap is closed.",
            ConfirmKind::Release { .. } => "letting it go. acknowledging the gap without closing it.",
        };

        let short = truncate(desired, 40);
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("{}  {} \"{}\"?", INDENT, action, short),
                self.styles.text_bold,
            )),
            Line::from(Span::styled(
                format!("{}  {}", INDENT, description),
                self.styles.dim,
            )),
            Line::from(""),
            Line::from_spans([
                Span::styled(format!("{}  ", INDENT), Style::new()),
                Span::styled("y", self.styles.cyan),
                Span::styled(" confirm    ", self.styles.dim),
                Span::styled("n", self.styles.cyan),
                Span::styled(" cancel", self.styles.dim),
            ]),
        ];

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(prompt_area, frame);
    }

    // -----------------------------------------------------------------------
    // Pathway palette — inline option set for structural signals
    // -----------------------------------------------------------------------

    pub fn render_pathway(&self, area: &Rect, frame: &mut Frame<'_>) {
        let pw = match &self.pathway_state {
            Some(pw) => pw,
            None => return,
        };
        // Backdrop dims the field
        crate::modal::render_backdrop(frame, *area, &self.styles);

        let area = self.layout.content_area(*area);

        // Compute height: 1 signal + 1 blank + options + 1 blank + 1 hint
        let option_count = pw.palette.options.len();
        let total_h = (3 + option_count + 2) as u16;
        let prompt_area = crate::modal::center_modal(area, 60.min(area.width), total_h);

        let mut lines: Vec<Line> = Vec::new();

        // Signal description with glyph
        lines.push(Line::from(""));
        lines.push(Line::from_spans([
            Span::styled(format!("{}  ", INDENT), Style::new()),
            Span::styled("\u{26A1} ", self.styles.amber), // ⚡
            Span::styled(&pw.palette.description, self.styles.amber),
        ]));
        lines.push(Line::from(""));

        // Options
        for (i, opt) in pw.palette.options.iter().enumerate() {
            let is_cursor = i == pw.cursor;
            let idx_style = if is_cursor { self.styles.selected } else { self.styles.cyan };
            let label_style = if is_cursor { self.styles.selected } else { self.styles.text };
            lines.push(Line::from_spans([
                Span::styled(format!("{}  ", INDENT), if is_cursor { self.styles.selected } else { Style::new() }),
                Span::styled(format!("[{}]", opt.index), idx_style),
                Span::styled(format!(" {}", opt.label), label_style),
                // Pad to full width for selection highlight
                if is_cursor {
                    let used = INDENT.len() + 2 + 3 + 1 + opt.label.len();
                    let pad = (area.width as usize).saturating_sub(used);
                    Span::styled(" ".repeat(pad), self.styles.selected)
                } else {
                    Span::styled("", Style::new())
                },
            ]));
        }

        // Hint line
        lines.push(Line::from(""));
        lines.push(Line::from_spans([
            Span::styled(format!("{}  ", INDENT), Style::new()),
            Span::styled("j/k", self.styles.cyan),
            Span::styled(" navigate  ", self.styles.dim),
            Span::styled("Enter", self.styles.cyan),
            Span::styled(" select  ", self.styles.dim),
            Span::styled("Esc", self.styles.cyan),
            Span::styled(" dismiss", self.styles.dim),
        ]));

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(prompt_area, frame);
    }

    // -----------------------------------------------------------------------
    // Edit prompt — Panel card with field label
    // -----------------------------------------------------------------------

    pub fn render_edit_prompt(&self, field: &EditField, area: &Rect, frame: &mut Frame<'_>) {
        // Backdrop dims the field
        crate::modal::render_backdrop(frame, *area, &self.styles);

        let area = self.layout.content_area(*area);
        let panel_h: u16 = 5;
        let panel_w = area.width.saturating_sub(INDENT.len() as u16 * 2);
        let prompt_area = crate::modal::center_modal(area, panel_w, panel_h);

        let label = match field {
            EditField::Desire => "desire",
            EditField::Reality => "reality",
            EditField::Horizon => "horizon",
        };

        let field_labels = [
            ("desire", EditField::Desire),
            ("reality", EditField::Reality),
            ("horizon", EditField::Horizon),
        ];

        // Build tab bar showing which field is active
        let mut tab_spans: Vec<Span> = Vec::new();
        for (name, f) in &field_labels {
            let is_active = std::mem::discriminant(field) == std::mem::discriminant(f);
            if is_active {
                tab_spans.push(Span::styled(format!("[{}]", name), self.styles.cyan));
            } else {
                tab_spans.push(Span::styled(format!(" {} ", name), self.styles.dim));
            }
            tab_spans.push(Span::styled(" ", Style::new()));
        }

        // Render the panel border + tab bar as content
        let tab_line = Line::from_spans(tab_spans);
        let content_lines = vec![
            tab_line,
            Line::from(""),
        ];
        let para = Paragraph::new(Text::from_lines(content_lines));
        let panel = Panel::new(para)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(self.styles.clr_dim))
            .title(label)
            .title_style(self.styles.cyan);
        panel.render(prompt_area, frame);

        // Render the TextInput widget in the input area within the panel
        // Panel border = 1 on each side, so inner area starts at +1,+1 and shrinks by 2
        let input_rect = Rect::new(
            prompt_area.x + 1,
            prompt_area.y + 3, // border(1) + tab_line(1) + blank(1)
            prompt_area.width.saturating_sub(2),
            1,
        );
        self.text_input.render(input_rect, frame);

        // Show the cursor at the TextInput's position
        if self.text_input.focused() {
            let (cx, cy) = self.text_input.cursor_position(input_rect);
            frame.set_cursor_visible(true);
            frame.set_cursor(Some((cx, cy)));
        }
    }

    // -----------------------------------------------------------------------
    // Note prompt (inline)
    // -----------------------------------------------------------------------

    pub fn render_note_prompt(&self, area: &Rect, frame: &mut Frame<'_>) {
        // Backdrop dims the field
        crate::modal::render_backdrop(frame, *area, &self.styles);

        let area = self.layout.content_area(*area);
        let prompt_area = crate::modal::center_modal(area, 60.min(area.width), 3);

        let label_text = format!("{}  note: ", INDENT);
        let label_w = label_text.len() as u16;
        let lines = vec![
            Line::from(""),
            Line::from_spans([
                Span::styled(&label_text, self.styles.label),
            ]),
        ];

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(prompt_area, frame);

        // Render TextInput widget after the label
        let input_rect = Rect::new(
            prompt_area.x + label_w,
            prompt_area.y + 1, // after the blank line
            prompt_area.width.saturating_sub(label_w),
            1,
        );
        self.text_input.render(input_rect, frame);

        if self.text_input.focused() {
            let (cx, cy) = self.text_input.cursor_position(input_rect);
            frame.set_cursor_visible(true);
            frame.set_cursor(Some((cx, cy)));
        }
    }

    // -----------------------------------------------------------------------
    // Search overlay
    // -----------------------------------------------------------------------

    pub fn render_search(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.layout.content_area(*area);
        crate::helpers::clear_area_styled(frame, area, self.styles.clr_dim);

        let mut lines: Vec<Line> = Vec::new();

        // Search input
        let is_moving = matches!(self.input_mode, InputMode::Moving { .. });
        let prefix = if is_moving { "move to" } else { "/" };

        lines.push(Line::from_spans([
            Span::styled(format!("{}{}: ", INDENT, prefix), self.styles.label),
            Span::styled(&self.input_buffer, self.styles.text_bold),
            Span::styled("\u{2588}", self.styles.cyan),
        ]));
        lines.push(Line::from(""));

        // Results
        if let Some(ref search) = self.search_state {
            for (i, result) in search.results.iter().enumerate() {
                let is_selected = i == search.cursor;
                let style = if is_selected { self.styles.selected } else { self.styles.text };
                let dim = if is_selected { self.styles.text_bold } else { self.styles.dim };

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
                            self.styles.dim,
                        ),
                    ]));
                }
            }

            if search.results.is_empty() && !self.input_buffer.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("{}  no matches", INDENT),
                    self.styles.dim,
                )));
            }
        }

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(area, frame);
    }


    pub fn render_input_hints(&self, text: &str, area: &Rect, frame: &mut Frame<'_>) {
        let display = format!(" {}", text);
        let hints = StatusLine::new()
            .left(StatusItem::text(&display))
            .style(self.styles.dim);
        hints.render(*area, frame);
    }
}
