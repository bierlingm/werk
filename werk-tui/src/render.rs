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
use crate::theme::*;

/// Maximum content width. Wide terminals get margin.
const MAX_CONTENT_WIDTH: u16 = 104;
/// Left indent for all content.
const INDENT: &str = "  ";
/// Trunk line character for descended view.
const TRUNK: &str = "\u{2502}"; // │

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
    // Field view — rect-slicing architecture
    // -----------------------------------------------------------------------

    pub fn render_field(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        let w = (area.width as usize).saturating_sub(4);
        let is_descended = self.parent_tension.is_some();

        // Phase 1: Compute total height needed for each element
        let mut elements: Vec<FieldElement> = Vec::new();

        // Parent desire header (if descended)
        if let Some(ref parent) = self.parent_tension {
            let (inline_suffix, right_text) = self.build_desire_header_parts();
            let suffix_w = if inline_suffix.is_empty() { 0 } else { inline_suffix.chars().count() + 3 }; // " · age"
            let right_w = if right_text.is_empty() { 0 } else { right_text.chars().count() + 2 }; // "  right"
            // Text budget: room for both annotations (worst case: single line)
            let text_width = w.saturating_sub(suffix_w + right_w);
            let desire_lines = word_wrap(&parent.desired, text_width);
            elements.push(FieldElement::DesireHeader {
                lines: desire_lines,
                inline_suffix,
                right_text,
            });
            elements.push(FieldElement::HeavyRule);
        }

        // Empty hint when descended with no children
        if is_descended && self.siblings.is_empty() {
            elements.push(FieldElement::BlankLine);
        }

        // Children (tension lines)
        for (i, entry) in self.siblings.iter().enumerate() {
            let is_selected = i == self.vlist.cursor;
            let is_gazed = self.gaze.as_ref().map(|g| g.index == i).unwrap_or(false);
            let is_positioned = entry.position.is_some();
            let is_last_positioned = is_positioned && self.siblings.get(i + 1)
                .map(|next| next.position.is_none()).unwrap_or(true);

            // Trunk segment above positioned children (descended view only)
            if is_descended && is_positioned {
                if i == 0 || self.siblings.get(i.wrapping_sub(1)).map(|prev| prev.position.is_some()).unwrap_or(false) {
                    elements.push(FieldElement::TrunkSegment);
                }
            }

            if is_gazed {
                // Gazed: the tension line becomes the first line inside the GazeCard
                elements.push(FieldElement::GazeCard { index: i });
            } else {
                // Normal tension line
                elements.push(FieldElement::TensionLine {
                    index: i,
                    selected: is_selected,
                    gazed: false,
                });
            }

            // Dotted separator between positioned and unpositioned
            if is_descended && is_last_positioned {
                let has_unpositioned = self.siblings.iter().skip(i + 1).any(|s| s.position.is_none());
                if has_unpositioned {
                    elements.push(FieldElement::DottedSeparator);
                }
            }
        }

        // Reality footer (if descended)
        if let Some(ref parent) = self.parent_tension {
            if !parent.actual.is_empty() {
                elements.push(FieldElement::BlankLine);
                elements.push(FieldElement::LightRule);
                let right_text = self.parent_reality_age.as_deref().unwrap_or("").to_string();
                let annotation_w = if right_text.is_empty() { 0 } else { right_text.chars().count() + 4 };
                let text_width = w.saturating_sub(annotation_w);
                let reality_lines = word_wrap(&parent.actual, text_width);
                elements.push(FieldElement::RealityFooter {
                    lines: reality_lines,
                    right_text,
                });
            }
        }

        // Alerts below reality
        if !self.alerts.is_empty() {
            elements.push(FieldElement::BlankLine);
            for (i, alert) in self.alerts.iter().enumerate() {
                elements.push(FieldElement::AlertLine { index: i, alert: alert.clone() });
            }
        }

        // Phase 2: Compute height for each element and assign rects
        let mut y = area.y;
        let max_y = area.y + area.height;

        // Apply scroll offset
        let scroll = self.vlist.scroll_offset as u16;
        let mut skipped: u16 = 0;

        for elem in &elements {
            let h = self.element_height(elem, area.width);
            if skipped < scroll {
                let skip = (scroll - skipped).min(h);
                skipped += skip;
                if skip < h {
                    // Partial element visible
                    let visible_h = h - skip;
                    let elem_rect = Rect::new(area.x, y, area.width, visible_h.min(max_y.saturating_sub(y)));
                    if elem_rect.height > 0 && y < max_y {
                        self.render_element(elem, elem_rect, frame);
                    }
                    y += visible_h;
                }
                continue;
            }

            if y >= max_y {
                break;
            }

            let visible_h = h.min(max_y.saturating_sub(y));
            let elem_rect = Rect::new(area.x, y, area.width, visible_h);
            if visible_h > 0 {
                self.render_element(elem, elem_rect, frame);
            }
            y += h;
        }
    }

    /// Build desire header annotations as two parts:
    /// - inline_suffix: age ("3d ago") follows the text with " · "
    /// - right_text: horizon + temporal indicator ("Jun ◌◌◦◌●◌") stays right-aligned
    fn build_desire_header_parts(&self) -> (String, String) {
        let inline_suffix = self.parent_desire_age.clone().unwrap_or_default();

        let mut right_parts: Vec<String> = Vec::new();
        if let Some(ref hl) = self.parent_horizon_label {
            right_parts.push(hl.clone());
        }
        if !self.parent_temporal_indicator.is_empty() {
            right_parts.push(self.parent_temporal_indicator.clone());
        }
        let right_text = right_parts.join(" ");

        (inline_suffix, right_text)
    }

    /// Compute the height (in lines) of a field element.
    fn element_height(&self, elem: &FieldElement, width: u16) -> u16 {
        match elem {
            FieldElement::DesireHeader { lines, .. } => lines.len() as u16,
            FieldElement::HeavyRule | FieldElement::LightRule
            | FieldElement::TrunkSegment | FieldElement::DottedSeparator
            | FieldElement::BlankLine => 1,
            FieldElement::TensionLine { index, selected, .. } => {
                if let Some(entry) = self.siblings.get(*index) {
                    let suffix_w = self.tension_suffix_width(entry);
                    let prefix_w = INDENT.len() + 2 + 1; // glyph + space
                    let budget = (width as usize).saturating_sub(suffix_w + prefix_w).max(10);
                    if *selected && entry.desired.chars().count() > budget {
                        word_wrap(&entry.desired, budget).len() as u16
                    } else {
                        1
                    }
                } else {
                    1
                }
            }
            FieldElement::GazeCard { .. } => {
                // Heading line + children lines + rule + reality lines + border chrome
                let mut h: u16 = 1; // heading line (tension line inside panel)
                if let Some(ref data) = self.gaze_data {
                    // Separator between heading and children (if there are children or reality)
                    if !data.children.is_empty() || !data.actual.is_empty() {
                        h += 1; // light rule after heading
                    }
                    h += data.children.len() as u16; // child preview lines
                    // Dotted separator between positioned and unpositioned children
                    let has_positioned = data.children.iter().any(|c| c.position.is_some());
                    let has_unpositioned = data.children.iter().any(|c| c.position.is_none());
                    if has_positioned && has_unpositioned {
                        h += 1;
                    }
                    if !data.actual.is_empty() {
                        let w = (width as usize).saturating_sub(8); // panel padding
                        h += 1; // light rule before reality
                        h += word_wrap(&data.actual, w).len() as u16; // reality
                    }
                }
                if self.gaze.as_ref().map(|g| g.full).unwrap_or(false) {
                    if let Some(ref full) = self.full_gaze_data {
                        h += self.full_gaze_line_count(full, width);
                    }
                }
                h + 2 // panel top + bottom border
            }
            FieldElement::RealityFooter { lines, .. } => lines.len() as u16,
            FieldElement::AlertLine { .. } => 1,
        }
    }

    /// Render a single field element into its assigned rect.
    fn render_element(&self, elem: &FieldElement, rect: Rect, frame: &mut Frame<'_>) {
        let w = (rect.width as usize).saturating_sub(4);
        match elem {
            FieldElement::DesireHeader { lines, inline_suffix, right_text } => {
                let right_w = if right_text.is_empty() { 0 } else { right_text.chars().count() + 2 };
                let mut out: Vec<Line> = Vec::new();
                let last = lines.len().saturating_sub(1);
                for (i, line) in lines.iter().enumerate() {
                    let is_last = i == last;
                    let text = format!("{}{}", INDENT, line);
                    let mut text_len = text.chars().count();

                    let mut spans: Vec<Span> = Vec::new();
                    spans.push(Span::styled(&text, STYLES.text_bold));

                    // Inline suffix (age) follows the last line of text
                    if is_last && !inline_suffix.is_empty() {
                        let suffix_str = format!(" \u{00B7} {}", inline_suffix);
                        text_len += suffix_str.chars().count();
                        spans.push(Span::styled(suffix_str, STYLES.dim));
                    }

                    // Right-aligned horizon + indicator on the first line
                    if i == 0 && !right_text.is_empty() {
                        let total = w + INDENT.len();
                        let pad = total.saturating_sub(text_len + right_w);
                        spans.push(Span::styled(" ".repeat(pad), Style::new()));
                        spans.push(Span::styled(format!("  {}", right_text), STYLES.dim));
                    }

                    out.push(Line::from_spans(spans));
                }
                Paragraph::new(Text::from_lines(out)).render(rect, frame);
            }

            FieldElement::HeavyRule => {
                let rule_w = (rect.width as usize).saturating_sub(INDENT.len());
                let rule = glyphs::HEAVY_RULE.to_string().repeat(rule_w);
                Paragraph::new(Text::from(Line::from(Span::styled(
                    format!("{}{}", INDENT, rule),
                    STYLES.dim,
                )))).render(rect, frame);
            }

            FieldElement::LightRule => {
                let rule_w = (rect.width as usize).saturating_sub(INDENT.len());
                let rule = glyphs::LIGHT_RULE.to_string().repeat(rule_w);
                Paragraph::new(Text::from(Line::from(Span::styled(
                    format!("{}{}", INDENT, rule),
                    STYLES.dim,
                )))).render(rect, frame);
            }

            FieldElement::TrunkSegment => {
                Paragraph::new(Text::from(Line::from(Span::styled(
                    format!("{}{}", INDENT, TRUNK),
                    STYLES.dim,
                )))).render(rect, frame);
            }

            FieldElement::DottedSeparator => {
                let rule_w = (rect.width as usize).saturating_sub(INDENT.len());
                Paragraph::new(Text::from(Line::from(Span::styled(
                    format!("{}{}", INDENT, "\u{00B7} ".repeat(rule_w / 2)),
                    STYLES.dim,
                )))).render(rect, frame);
            }

            FieldElement::BlankLine => {
                // Render nothing — the rect itself is the blank line
            }

            FieldElement::TensionLine { index, selected, gazed } => {
                if let Some(entry) = self.siblings.get(*index) {
                    let is_descended = self.parent_tension.is_some();
                    let is_positioned = entry.position.is_some();
                    let lines = self.build_tension_lines(
                        entry, *selected, *gazed, is_descended, is_positioned, rect.width,
                    );
                    Paragraph::new(Text::from_lines(lines)).render(rect, frame);
                }
            }

            FieldElement::GazeCard { .. } => {
                self.render_gaze_panel(rect, frame);
            }

            FieldElement::RealityFooter { lines, right_text } => {
                let mut out: Vec<Line> = Vec::new();
                let last = lines.len().saturating_sub(1);
                for (i, line) in lines.iter().enumerate() {
                    if i == last && !right_text.is_empty() {
                        // Annotation follows text directly with dot separator
                        out.push(Line::from_spans([
                            Span::styled(format!("{}{}", INDENT, line), STYLES.dim),
                            Span::styled(format!(" \u{00B7} {}", right_text), STYLES.dim),
                        ]));
                    } else {
                        out.push(Line::from(Span::styled(
                            format!("{}{}", INDENT, line),
                            STYLES.dim,
                        )));
                    }
                }
                Paragraph::new(Text::from_lines(out)).render(rect, frame);
            }

            FieldElement::AlertLine { index, alert } => {
                let num = format!("{}", index + 1);
                // Prefix: "  1  ⚠ " = INDENT(2) + num + "  ⚠ "
                let prefix_w = INDENT.len() + 2 + num.len() + 4; // "  " + num + "  ⚠ "
                let available = w.saturating_sub(prefix_w);
                let full_text = format!("{} \u{2014} {}", alert.message, alert.action_hint);
                let alert_text = truncate(&full_text, available);
                let line = Line::from_spans([
                    Span::styled(format!("{}  ", INDENT), Style::new()),
                    Span::styled(format!("{}", num), STYLES.amber),
                    Span::styled(format!("  \u{26A0} {}", alert_text), STYLES.amber),
                ]);
                Paragraph::new(Text::from(line)).render(rect, frame);
            }
        }
    }

    /// Compute the right-side suffix width for a tension line.
    /// Always reserves at least 8 chars for horizon area even if no horizon is set,
    /// plus the indicator width, for visual breathing room.
    fn tension_suffix_width(&self, entry: &FieldEntry) -> usize {
        let horizon_str = entry.horizon_label.as_deref().unwrap_or("");
        let horizon_w = horizon_str.chars().count().max(6) + 2; // min 6 chars + spacing
        let indicator_width = entry.temporal_indicator.chars().count();
        horizon_w + indicator_width + 1
    }

    fn build_tension_lines(
        &self,
        entry: &FieldEntry,
        selected: bool,
        _gazed: bool,
        is_descended: bool,
        is_positioned: bool,
        width: u16,
    ) -> Vec<Line> {
        let w = width as usize;
        let glyph = glyphs::status_glyph(entry.status);
        let indicator = &entry.temporal_indicator;
        let indicator_width = indicator.chars().count();
        let is_reordering = matches!(self.input_mode, InputMode::Reordering { .. });
        let is_grabbed = is_reordering && selected;

        let is_done = entry.status == sd_core::TensionStatus::Resolved
            || entry.status == sd_core::TensionStatus::Released;

        // When selected, ALL spans get the selected bg so the highlight is full-width
        let base_style = if is_grabbed {
            STYLES.cyan
        } else if selected {
            STYLES.selected
        } else if is_done {
            STYLES.dim
        } else {
            STYLES.text
        };

        let glyph_style = if is_grabbed {
            Style::new().fg(CLR_CYAN)
        } else if selected {
            Style::new().fg(CLR_CYAN).bg(CLR_SELECTED_BG).bold()
        } else if is_done {
            STYLES.dim
        } else {
            Style::new().fg(CLR_DEFAULT)
        };

        let indicator_color = if entry.temporal_urgency > 0.8 {
            CLR_RED
        } else if entry.temporal_urgency > 0.5 {
            CLR_AMBER
        } else {
            CLR_CYAN
        };
        let indicator_style = if selected && !is_grabbed {
            Style::new().fg(indicator_color).bg(CLR_SELECTED_BG)
        } else {
            Style::new().fg(indicator_color)
        };

        // Right side: horizon + indicator. Reserve space even without horizon for breathing room.
        let horizon_str = entry.horizon_label.as_deref().unwrap_or("");
        let horizon_w = horizon_str.chars().count().max(6) + 2; // min gap even without horizon
        let suffix_w = horizon_w + indicator_width + 1;

        // Prefix: INDENT(2) + glyph(1) + space(1) = 4 chars
        let prefix_w = INDENT.len() + 2 + 1;
        let desired_budget = w.saturating_sub(prefix_w + suffix_w).max(10);

        let dim_style = if is_grabbed { STYLES.cyan } else if selected { STYLES.selected } else { STYLES.dim };
        // bg_style: used for spacing/indent areas when selected, to ensure full-width background
        let bg_style = if selected && !is_grabbed { STYLES.selected } else { Style::new() };

        // Build the glyph prefix
        let glyph_prefix = if is_grabbed {
            if is_descended && is_positioned {
                format!("{}\u{2261} ", INDENT)
            } else if is_descended && !is_positioned {
                format!("{}  \u{2261} ", INDENT)
            } else {
                format!("{}\u{2261} ", INDENT)
            }
        } else if is_descended && is_positioned {
            format!("{}{} ", INDENT, glyph)
        } else if is_descended && !is_positioned {
            format!("{}  {} ", INDENT, glyph)
        } else {
            format!("{}{} ", INDENT, glyph)
        };



        // Right-side parts: horizon gap + indicator
        let horizon_part = if !horizon_str.is_empty() {
            format!("{:>width$} ", horizon_str, width = horizon_w.saturating_sub(1))
        } else {
            " ".repeat(horizon_w)
        };
        let indicator_part = format!(" {}", indicator);

        // Build a single full-width string for each line, then style regions.
        // This avoids any char-vs-display-width discrepancies between spans.

        if selected && entry.desired.chars().count() > desired_budget {
            let wrapped = word_wrap(&entry.desired, desired_budget);
            let mut lines = Vec::new();
            for (i, line_text) in wrapped.iter().enumerate() {
                if i == 0 {
                    // Build the full line as one string, then split into styled regions
                    let text_part = format!("{:<width$}", line_text, width = desired_budget);
                    // Total so far: prefix + text + gap + suffix
                    let content_len = glyph_prefix.chars().count() + desired_budget;
                    let right_len = horizon_part.chars().count() + indicator_part.chars().count();
                    let gap = w.saturating_sub(content_len + right_len);

                    lines.push(Line::from_spans([
                        Span::styled(&glyph_prefix, glyph_style),
                        Span::styled(text_part, base_style),
                        Span::styled(" ".repeat(gap), bg_style),
                        Span::styled(&horizon_part, dim_style),
                        Span::styled(&indicator_part, indicator_style),
                    ]));
                } else {
                    // Continuation line: prefix + text, padded to full width
                    let prefix_str = if is_descended && is_positioned {
                        format!("{}{} ", INDENT, TRUNK)
                    } else {
                        " ".repeat(prefix_w)
                    };
                    let fill_w = w.saturating_sub(prefix_str.chars().count());
                    let full_line = format!("{}{:<width$}", prefix_str, line_text, width = fill_w);
                    let p_len = prefix_str.chars().count();
                    let p: String = full_line.chars().take(p_len).collect();
                    let rest: String = full_line.chars().skip(p_len).collect();
                    let trunk_style = if selected { bg_style } else { STYLES.dim };
                    if is_descended && is_positioned {
                        lines.push(Line::from_spans([
                            Span::styled(p, trunk_style),
                            Span::styled(rest, base_style),
                        ]));
                    } else {
                        lines.push(Line::from_spans([
                            Span::styled(p, bg_style),
                            Span::styled(rest, base_style),
                        ]));
                    }
                }
            }
            lines
        } else {
            // Single line
            let desired_trunc = truncate(&entry.desired, desired_budget);
            let text_part = format!("{:<width$}", desired_trunc, width = desired_budget);
            let content_len = glyph_prefix.chars().count() + desired_budget;
            let right_len = horizon_part.chars().count() + indicator_part.chars().count();
            let gap = w.saturating_sub(content_len + right_len);

            vec![Line::from_spans([
                Span::styled(&glyph_prefix, glyph_style),
                Span::styled(text_part, base_style),
                Span::styled(" ".repeat(gap), bg_style),
                Span::styled(&horizon_part, dim_style),
                Span::styled(&indicator_part, indicator_style),
            ])]
        }
    }

    /// Render the gaze card as a Panel with rounded borders.
    /// The tension line is rendered as the first content line inside the panel,
    /// making the card a cohesive extension of the tension.
    fn render_gaze_panel(&self, rect: Rect, frame: &mut Frame<'_>) {
        if rect.height < 3 {
            return; // not enough space for panel borders + content
        }

        let mut content_lines: Vec<Line> = Vec::new();
        let inner_w = (rect.width as usize).saturating_sub(6); // panel borders + padding

        // First line: the tension line itself (glyph + desire + horizon + indicator)
        if let Some(ref gaze) = self.gaze {
            if let Some(entry) = self.siblings.get(gaze.index) {
                let glyph = glyphs::status_glyph(entry.status);
                let indicator = &entry.temporal_indicator;
                let indicator_width = indicator.chars().count();

                let glyph_style = Style::new().fg(CLR_CYAN); // always cyan when gazed
                let indicator_color = if entry.temporal_urgency > 0.8 {
                    CLR_RED
                } else if entry.temporal_urgency > 0.5 {
                    CLR_AMBER
                } else {
                    CLR_CYAN
                };

                let horizon_str = entry.horizon_label.as_deref().unwrap_or("");
                let horizon_w = if horizon_str.is_empty() { 0 } else { horizon_str.chars().count() + 1 };
                let suffix_w = horizon_w + indicator_width + 1;
                let desired_budget = inner_w.saturating_sub(suffix_w + 3).max(10); // 3 = glyph + spaces
                let desired_trunc = truncate(&entry.desired, desired_budget);
                let name_padded = format!("{:<width$}", desired_trunc, width = desired_budget);

                let mut spans = vec![
                    Span::styled(format!("{} ", glyph), glyph_style),
                    Span::styled(name_padded, STYLES.text_bold),
                ];
                if !horizon_str.is_empty() {
                    spans.push(Span::styled(format!(" {}", horizon_str), STYLES.dim));
                }
                spans.push(Span::styled(format!(" {}", indicator), Style::new().fg(indicator_color)));
                content_lines.push(Line::from_spans(spans));
            }
        }

        // Children preview — the action steps
        if let Some(ref data) = self.gaze_data {
            let has_content = !data.children.is_empty() || !data.actual.is_empty();
            if has_content {
                let rule = glyphs::LIGHT_RULE.to_string().repeat(inner_w);
                content_lines.push(Line::from(Span::styled(rule, STYLES.dim)));
            }

            if !data.children.is_empty() {
                // Split children into positioned and unpositioned groups
                let positioned: Vec<_> = data.children.iter().filter(|c| c.position.is_some()).collect();
                let unpositioned: Vec<_> = data.children.iter().filter(|c| c.position.is_none()).collect();

                for child in &positioned {
                    let child_glyph = glyphs::status_glyph(child.status);
                    let child_budget = inner_w.saturating_sub(4);
                    content_lines.push(Line::from_spans([
                        Span::styled(format!("{} ", child_glyph), Style::new().fg(CLR_DEFAULT)),
                        Span::styled(
                            truncate(&child.desired, child_budget).to_string(),
                            STYLES.text,
                        ),
                    ]));
                }

                // Dotted separator between positioned and unpositioned
                if !positioned.is_empty() && !unpositioned.is_empty() {
                    let dots = "\u{00B7} ".repeat(inner_w / 2);
                    content_lines.push(Line::from(Span::styled(dots, STYLES.dim)));
                }

                for child in &unpositioned {
                    let child_glyph = glyphs::status_glyph(child.status);
                    let child_budget = inner_w.saturating_sub(6); // extra indent for unpositioned
                    content_lines.push(Line::from_spans([
                        Span::styled("  ", Style::new()), // extra indent
                        Span::styled(format!("{} ", child_glyph), Style::new().fg(CLR_DEFAULT)),
                        Span::styled(
                            truncate(&child.desired, child_budget).to_string(),
                            STYLES.dim,
                        ),
                    ]));
                }
            }

            // Reality — the structural ground
            if !data.actual.is_empty() {
                let rule = glyphs::LIGHT_RULE.to_string().repeat(inner_w);
                content_lines.push(Line::from(Span::styled(rule, STYLES.dim)));
                let actual_lines = word_wrap(&data.actual, inner_w);
                for line in &actual_lines {
                    content_lines.push(Line::from(Span::styled(line.as_str(), STYLES.dim)));
                }
            }
        }

        // Full gaze data (dynamics + history)
        if self.gaze.as_ref().map(|g| g.full).unwrap_or(false) {
            if let Some(ref full_data) = self.full_gaze_data {
                let full_lines = self.build_full_gaze_content(full_data, rect.width);
                content_lines.extend(full_lines);
            }
        }

        if content_lines.len() <= 1 {
            // Only the heading line — add a dim hint
            content_lines.push(Line::from(Span::styled("no children", STYLES.dim)));
        }

        let para = Paragraph::new(Text::from_lines(content_lines));
        let panel = Panel::new(para)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(CLR_DIM));

        // Indent the panel
        let panel_rect = Rect::new(
            rect.x + INDENT.len() as u16,
            rect.y,
            rect.width.saturating_sub(INDENT.len() as u16),
            rect.height,
        );
        panel.render(panel_rect, frame);
    }

    /// Build content lines for the full gaze (facts + history) — used inside Panel.
    fn build_full_gaze_content(&self, data: &FullGazeData, width: u16) -> Vec<Line> {
        let mut lines = Vec::new();
        let w = (width as usize).saturating_sub(8); // panel padding
        let rule = glyphs::LIGHT_RULE.to_string().repeat(w);

        lines.push(Line::from(Span::styled(rule, STYLES.dim)));

        // Build facts column
        let mut dyn_lines: Vec<(String, String, Style)> = Vec::new();
        if let Some(ref v) = data.urgency {
            dyn_lines.push(("urgency".to_string(), v.clone(), STYLES.text));
        }
        if let Some(ref v) = data.horizon_drift {
            dyn_lines.push(("drift".to_string(), v.clone(), STYLES.text));
        }
        if let Some(ref v) = data.closure {
            dyn_lines.push(("closure".to_string(), v.clone(), STYLES.text));
        }

        // Build history column
        let dyn_count = dyn_lines.len();
        let max_history = dyn_count.max(3);
        let history: Vec<&HistoryEntry> = if data.history.len() <= max_history {
            data.history.iter().collect()
        } else {
            let remaining = max_history.saturating_sub(1);
            let recent_start = data.history.len().saturating_sub(remaining);
            let mut result = vec![&data.history[0]];
            result.extend(&data.history[recent_start..]);
            result
        };

        // Layout: dynamics left, │ divider, history right
        let dyn_col_width = 30.min(w / 2);
        let hist_col_width = w.saturating_sub(dyn_col_width + 3);
        let row_count = dyn_count.max(history.len());

        for i in 0..row_count {
            let left = if i < dyn_lines.len() {
                let (ref label, ref value, style) = dyn_lines[i];
                vec![
                    Span::styled(format!("{:<13}", label), STYLES.label),
                    Span::styled(
                        format!("{:<width$}", value, width = dyn_col_width.saturating_sub(13)),
                        style,
                    ),
                ]
            } else {
                vec![Span::styled(" ".repeat(dyn_col_width), Style::new())]
            };

            let divider = Span::styled(" \u{2502} ", STYLES.dim);

            let right = if i < history.len() {
                let entry = history[i];
                if entry.relative_time.is_empty() && entry.description.is_empty() {
                    vec![Span::styled("\u{22EE}", STYLES.dim)]
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

    /// Count lines for full gaze content (for height calculation).
    fn full_gaze_line_count(&self, data: &FullGazeData, width: u16) -> u16 {
        let mut count: u16 = 1; // separator rule
        let mut dyn_count: u16 = 0;
        if data.urgency.is_some() { dyn_count += 1; }
        if data.horizon_drift.is_some() { dyn_count += 1; }
        if data.closure.is_some() { dyn_count += 1; }
        let dyn_count = dyn_count.max(1); // at least 1 row
        let hist_count = data.history.len().min((dyn_count as usize).max(3)) as u16;
        count += dyn_count.max(hist_count);
        let _ = width; // reserved for future use
        count
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

        let left_text = if self.use_deck {
            // Deck mode: minimal lever — tension ID only, breadcrumb is in the deck itself
            if let Some(ref parent) = self.parent_tension {
                let id = werk_shared::display_id(parent.short_code, &parent.id);
                format!(" {}", id)
            } else {
                " werk".to_string()
            }
        } else {
            // Old field view: full breadcrumb path
            let crumbs = &self.breadcrumb_cache;
            if crumbs.is_empty() {
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
            }
        };

        let mut right_parts: Vec<String> = Vec::new();

        // Show filter state when not default
        if !matches!(self.filter, crate::app::Filter::Active) {
            right_parts.push(format!("filter: {}", self.filter.label()));
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
            Span::styled("NAVIGATION", STYLES.text_bold),
        ]));
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled(&light_rule, STYLES.dim),
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
                Span::styled(format!("{:<12}", k1), STYLES.cyan),
                Span::styled(format!("{:<22}", d1), STYLES.text),
                Span::styled(format!("{:<12}", k2), STYLES.cyan),
                Span::styled(*d2, STYLES.text),
            ]));
        }

        lines.push(Line::from(""));

        // --- Acts section ---
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("ACTS", STYLES.text_bold),
        ]));
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled(&light_rule, STYLES.dim),
        ]));
        let act_keys: &[(&str, &str, &str, &str)] = &[
            ("a", "add tension", "e", "edit (desire/reality/horizon)"),
            ("n", "add note", "m", "move / reparent"),
            ("r", "resolve", "x", "release"),
            ("o", "reopen", "u", "undo last act"),
            ("y", "copy ID", "f", "filter"),
            ("q", "quit", "", ""),
        ];
        for (k1, d1, k2, d2) in act_keys {
            let mut spans = vec![
                Span::styled(&pad, Style::new()),
                Span::styled(format!("{:<4}", k1), STYLES.cyan),
                Span::styled(format!("{:<30}", d1), STYLES.text),
            ];
            if !k2.is_empty() {
                spans.push(Span::styled(format!("{:<4}", k2), STYLES.cyan));
                spans.push(Span::styled(*d2, STYLES.text));
            }
            lines.push(Line::from_spans(spans));
        }

        lines.push(Line::from(""));

        // --- Glyphs section ---
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("GLYPHS & COLORS", STYLES.text_bold),
        ]));
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled(&light_rule, STYLES.dim),
        ]));

        // Glyphs with their actual colors inline
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("\u{25C7} ", Style::new().fg(CLR_DEFAULT)),
            Span::styled("active        ", STYLES.dim),
            Span::styled("\u{2726} ", Style::new().fg(CLR_DIM)),
            Span::styled("resolved      ", STYLES.dim),
            Span::styled("\u{00B7} ", Style::new().fg(CLR_DIM)),
            Span::styled("released", STYLES.dim),
        ]));

        lines.push(Line::from(""));

        // Temporal indicator legend
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("\u{25CC}\u{25CC}\u{25E6}\u{25CC}\u{25CF}\u{25CC}", Style::new().fg(CLR_CYAN)),
            Span::styled("  temporal window: ", STYLES.dim),
            Span::styled("\u{25E6}", STYLES.cyan),
            Span::styled(" now  ", STYLES.dim),
            Span::styled("\u{25CF}", STYLES.cyan),
            Span::styled(" horizon end", STYLES.dim),
        ]));
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CE}", STYLES.dim),
            Span::styled("  staleness (no horizon set)", STYLES.dim),
        ]));
        lines.push(Line::from_spans([
            Span::styled(&pad, Style::new()),
            Span::styled("\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}", Style::new().fg(CLR_CYAN)),
            Span::styled(" comfortable  ", STYLES.dim),
            Span::styled("\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}", Style::new().fg(CLR_AMBER)),
            Span::styled(" approaching  ", STYLES.dim),
            Span::styled("\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}\u{25CC}", Style::new().fg(CLR_RED)),
            Span::styled(" overdue", STYLES.dim),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("{}press any key to close", pad),
            STYLES.dim,
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
        let area = self.content_area(*area);

        // Position the prompt inline — after the parent header and siblings,
        // right where the new tension will appear in the list.
        let header_lines: u16 = if self.parent_tension.is_some() { 3 } else { 0 };
        let sibling_lines = self.siblings.len() as u16;
        let prompt_y = area.y + header_lines + sibling_lines;
        let prompt_area = Rect::new(area.x, prompt_y, area.width, 4);
        crate::helpers::clear_area(frame, prompt_area);

        let (label, hint) = match step {
            AddStep::Name => ("name", ""),
            AddStep::Desire { .. } => ("desire", "  (Esc to skip)"),
            AddStep::Reality { .. } => ("reality", ""),
            AddStep::Horizon { .. } => ("horizon", "  e.g. 2026-W13 or tomorrow  (Esc to skip)"),
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

    pub fn render_confirm(&self, kind: &ConfirmKind, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        let cy = area.height / 2;
        let prompt_area = Rect::new(area.x, area.y + cy.saturating_sub(3), area.width, 6);
        crate::helpers::clear_area(frame, prompt_area);

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
    // Pathway palette — inline option set for structural signals
    // -----------------------------------------------------------------------

    pub fn render_pathway(&self, area: &Rect, frame: &mut Frame<'_>) {
        let pw = match &self.pathway_state {
            Some(pw) => pw,
            None => return,
        };
        let area = self.content_area(*area);

        // Compute height: 1 signal + 1 blank + options + 1 blank + 1 hint
        let option_count = pw.palette.options.len();
        let total_h = (3 + option_count + 2) as u16;
        let cy = area.height / 2;
        let top_y = area.y + cy.saturating_sub(total_h / 2);
        let prompt_area = Rect::new(area.x, top_y, area.width, total_h.min(area.height));
        crate::helpers::clear_area(frame, prompt_area);

        let mut lines: Vec<Line> = Vec::new();

        // Signal description with glyph
        lines.push(Line::from(""));
        lines.push(Line::from_spans([
            Span::styled(format!("{}  ", INDENT), Style::new()),
            Span::styled("\u{26A1} ", STYLES.amber), // ⚡
            Span::styled(&pw.palette.description, STYLES.amber),
        ]));
        lines.push(Line::from(""));

        // Options
        for (i, opt) in pw.palette.options.iter().enumerate() {
            let is_cursor = i == pw.cursor;
            let idx_style = if is_cursor { STYLES.selected } else { STYLES.cyan };
            let label_style = if is_cursor { STYLES.selected } else { STYLES.text };
            lines.push(Line::from_spans([
                Span::styled(format!("{}  ", INDENT), if is_cursor { STYLES.selected } else { Style::new() }),
                Span::styled(format!("[{}]", opt.index), idx_style),
                Span::styled(format!(" {}", opt.label), label_style),
                // Pad to full width for selection highlight
                if is_cursor {
                    let used = INDENT.len() + 2 + 3 + 1 + opt.label.len();
                    let pad = (area.width as usize).saturating_sub(used);
                    Span::styled(" ".repeat(pad), STYLES.selected)
                } else {
                    Span::styled("", Style::new())
                },
            ]));
        }

        // Hint line
        lines.push(Line::from(""));
        lines.push(Line::from_spans([
            Span::styled(format!("{}  ", INDENT), Style::new()),
            Span::styled("j/k", STYLES.cyan),
            Span::styled(" navigate  ", STYLES.dim),
            Span::styled("Enter", STYLES.cyan),
            Span::styled(" select  ", STYLES.dim),
            Span::styled("Esc", STYLES.cyan),
            Span::styled(" dismiss", STYLES.dim),
        ]));

        let para = Paragraph::new(Text::from_lines(lines));
        para.render(prompt_area, frame);
    }

    // -----------------------------------------------------------------------
    // Edit prompt — Panel card with field label
    // -----------------------------------------------------------------------

    pub fn render_edit_prompt(&self, field: &EditField, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        let panel_h: u16 = 5;
        let bottom_y = area.height.saturating_sub(panel_h + 1);
        let panel_x = area.x + INDENT.len() as u16;
        let panel_w = area.width.saturating_sub(INDENT.len() as u16);
        let prompt_area = Rect::new(panel_x, area.y + bottom_y, panel_w, panel_h);
        crate::helpers::clear_area(frame, Rect::new(area.x, area.y + bottom_y, area.width, panel_h + 1));

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
                tab_spans.push(Span::styled(format!("[{}]", name), STYLES.cyan));
            } else {
                tab_spans.push(Span::styled(format!(" {} ", name), STYLES.dim));
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
            .border_style(Style::new().fg(CLR_DIM))
            .title(label)
            .title_style(STYLES.cyan);
        panel.render(prompt_area, frame);

        // Render the TextInput widget in the input area within the panel
        // Panel border = 1 on each side, so inner area starts at +1,+1 and shrinks by 2
        let input_rect = Rect::new(
            panel_x + 1,
            prompt_area.y + 3, // border(1) + tab_line(1) + blank(1)
            panel_w.saturating_sub(2),
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
    // Search overlay
    // -----------------------------------------------------------------------

    pub fn render_search(&self, area: &Rect, frame: &mut Frame<'_>) {
        let area = self.content_area(*area);
        crate::helpers::clear_area(frame, area);

        let mut lines: Vec<Line> = Vec::new();

        // Search input
        let is_moving = matches!(self.input_mode, InputMode::Moving { .. });
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


    pub fn render_input_hints(&self, text: &str, area: &Rect, frame: &mut Frame<'_>) {
        let display = format!(" {}", text);
        let hints = StatusLine::new()
            .left(StatusItem::text(&display))
            .style(STYLES.dim);
        hints.render(*area, frame);
    }
}

/// Internal element types for rect-slicing layout.
#[derive(Clone)]
enum FieldElement {
    DesireHeader {
        lines: Vec<String>,
        inline_suffix: String,  // age ("3d ago") — follows last line of text
        right_text: String,     // horizon + indicator ("Jun ◌○○●◎") — right-aligned
    },
    HeavyRule,
    LightRule,
    TrunkSegment,
    DottedSeparator,
    BlankLine,
    TensionLine {
        index: usize,
        selected: bool,
        gazed: bool,
    },
    GazeCard {
        #[allow(dead_code)]
        index: usize,
    },
    RealityFooter {
        lines: Vec<String>,
        right_text: String,
    },
    AlertLine {
        index: usize,
        alert: Alert,
    },
}

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

