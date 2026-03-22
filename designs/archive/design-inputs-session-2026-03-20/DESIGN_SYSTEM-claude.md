# Werk Operative Instrument TUI Design System
## A Complete ftui Implementation Guide

---

## I. Design Philosophy & Spatial Laws

### The Fundamental Direction
**Reality is ground (bottom/left), desire is sky (top/right).** This law governs every visual decision. The vertical axis encodes the fundamental direction of structural dynamics. Movement toward desire is always upward. Movement toward reality is always downward.

### Premium Terminal Aesthetic
The instrument fuses:
- **Old terminals**: Phosphor restraint, ruled surfaces, hard alignment, symbolic compression
- **Updated terminals**: Responsive grids, overlays, command surfaces, notification systems
- **Operator tools**: Information hierarchy that assumes taste, focus, and repeated use

### Progressive Disclosure Architecture
Three depth layers, each adding density without removing the previous layer:

1. **Field Scanning** (Depth 0): One line per tension, maximum scan rate
2. **Focused Gaze** (Depth 1): Inline expansion showing children, reality, gap
3. **Full Analysis** (Depth 2): Complete dynamics + history in dedicated view

---

## II. ftui Widget Mapping & Configuration

### Core Widget Specifications

| Domain Concept | ftui Widget | Configuration | Purpose |
|----------------|-------------|---------------|---------|
| **Tension Line** | `Paragraph` | Single line, styled spans | Basic display unit |
| **Gaze Card** | `Panel` | Rounded borders, internal layout | Inline expansion |
| **Field View** | `Flex::vertical()` | Dynamic constraints | Scrollable list |
| **Desire Header** | `Paragraph` | Multi-line, right-aligned annotations | Parent context |
| **Reality Footer** | `Paragraph` | Multi-line, temporal annotation | Current state |
| **Heavy Rule** | `Rule` | Heavy style (`━`) | Desire separator |
| **Light Rule** | `Rule` | Light style (`┄`) | Reality separator |
| **Dotted Separator** | Custom spans | `· · ·` pattern | Positioned/unpositioned boundary |
| **Trunk Segment** | `Paragraph` | Single `│` character | Structural path |
| **Status Line** | `StatusLine` | Left/right items | Context & navigation |
| **Alert List** | `Paragraph` | Numbered items | Actionable signals |
| **Edit Panel** | `Panel` | `TextInput` widgets | Inline editing |
| **Agent Output** | `Paragraph` | Scrollable text | Response display |
| **Mutation Cards** | `Panel` | Checkbox list | Structured proposals |

---

## III. Visual Grammar System

### Glyphs: Status + Phase Encoding

```rust
// Phase progression (active tensions)
◇  // Germination - hollow diamond, just forming
◆  // Assimilation - solid diamond, being worked
◈  // Completion - textured diamond, nearing resolution
◉  // Momentum - filled circle, radiating energy

// Terminal states
✦  // Resolved - six-pointed star, achievement
·  // Released - small dot, acknowledged absence
```

**Color encoding**: Glyph color represents tendency
- **Cyan** (`CLR_CYAN`): Advancing, forward motion
- **White** (`CLR_DEFAULT`): Stagnant, no movement
- **Amber** (`CLR_AMBER`): Oscillating, back-and-forth

### Rules: Structural Meaning

```rust
pub const HEAVY_RULE: char = '━';  // Desire - firm, anchored
pub const LIGHT_RULE: char = '┄';  // Reality - fluid, shifting
pub const DOTTED_PATTERN: &str = "· · ·";  // Choice boundary
pub const TRUNK: char = '│';  // Structural path
```

### Temporal Indicators: Six-Dot System

**With Horizon** (action window):
```
◌◌◦◌●◌  // ◦=now, ●=horizon end
```

**Without Horizon** (staleness):
```
◌◌◌◌◌◎  // ◎ position shows weeks since reality check
```

**Implementation**:
```rust
const EMPTY: &str = "◌";     // Empty position
const NOW_MARKER: &str = "◦"; // Current position (open, moving)
const HORIZON_MARKER: &str = "●"; // Target (solid, fixed)
const STALE_MARKER: &str = "◎"; // Staleness indicator
```

### Alert System: Badges & Numbers

```rust
// Alert types with visual encoding
"⚠"  // Warning symbol (CLR_AMBER)
"!"  // Critical symbol (CLR_RED)
"1"  // Action number (CLR_CYAN)
```

---

## IV. Layout Architecture

### Responsive Width Thresholds

```rust
// Layout breakpoints
const NARROW: u16 = 80;   // Single column, minimal annotations
const MEDIUM: u16 = 104;  // Standard layout, full annotations
const WIDE: u16 = 140;    // Expanded margins, breathing room
```

### Constraint System

```rust
// Field view layout (descended)
Flex::vertical()
    .constraints([
        Constraint::Min(2),        // Desire header (word-wrapped)
        Constraint::Fixed(1),      // Heavy rule
        Constraint::Min(0),        // Children (dynamic)
        Constraint::Fixed(1),      // Dotted separator (if needed)
        Constraint::Min(0),        // Unpositioned children
        Constraint::Fixed(1),      // Light rule
        Constraint::Min(1),        // Reality footer
        Constraint::Min(0),        // Alerts (dynamic)
    ])
    .margin(Sides::horizontal(2))
```

### Progressive Element Heights

```rust
impl FieldElement {
    fn height(&self, width: u16) -> u16 {
        match self {
            FieldElement::TensionLine { selected, entry } => {
                if *selected && needs_wrapping(entry, width) {
                    word_wrap_lines(&entry.desired, width - PREFIX_WIDTH).len() as u16
                } else {
                    1
                }
            }
            FieldElement::GazeCard { index } => {
                2 +  // Panel borders
                1 +  // Tension line inside panel
                self.gaze_children_height() +
                self.gaze_reality_height(width) +
                if self.show_full_dynamics() {
                    self.dynamics_height()
                } else { 0 }
            }
            FieldElement::DesireHeader { lines } => lines.len() as u16,
            FieldElement::RealityFooter { lines } => lines.len() as u16,
            _ => 1
        }
    }
}
```

---

## V. Structural Dynamics Rendering

### Phase Visualization Strategy

Each phase maps to specific visual treatment:

```rust
fn render_phase_context(phase: CreativeCyclePhase, entry: &FieldEntry) -> Style {
    match phase {
        CreativeCyclePhase::Germination => {
            // Light treatment, encouraging formation
            Style::new().fg(CLR_DEFAULT)
        }
        CreativeCyclePhase::Assimilation => {
            // Solid treatment, work in progress
            Style::new().fg(CLR_DEFAULT).bold()
        }
        CreativeCyclePhase::Completion => {
            // Rich treatment, approaching resolution
            Style::new().fg(CLR_GREEN)
        }
        CreativeCyclePhase::Momentum => {
            // Energetic treatment, radiating progress
            Style::new().fg(CLR_CYAN).bold()
        }
    }
}
```

### Dynamic State Indicators

```rust
// Conflict visualization
fn render_conflict(conflict: &Conflict) -> Line {
    match conflict.pattern {
        ConflictPattern::CompetingTensions => {
            Line::from_spans([
                Span::styled("⚡ ", Style::new().fg(CLR_RED)),
                Span::styled("competing tensions", Style::new().fg(CLR_RED)),
            ])
        }
        ConflictPattern::AsymmetricActivity => {
            Line::from_spans([
                Span::styled("⚡ ", Style::new().fg(CLR_AMBER)),
                Span::styled("asymmetric activity", Style::new().fg(CLR_AMBER)),
            ])
        }
    }
}

// Oscillation visualization
fn render_oscillation(oscillation: &Oscillation) -> Line {
    let intensity_bar = "█".repeat((oscillation.magnitude * 8.0) as usize);
    Line::from_spans([
        Span::styled("↔ ", Style::new().fg(CLR_AMBER)),
        Span::styled("oscillating ", Style::new().fg(CLR_AMBER)),
        Span::styled(&intensity_bar, Style::new().fg(CLR_AMBER)),
    ])
}

// Resolution visualization
fn render_resolution(resolution: &Resolution) -> Line {
    let velocity_color = if resolution.velocity > resolution.required_velocity.unwrap_or(0.0) {
        CLR_CYAN
    } else {
        CLR_AMBER
    };

    Line::from_spans([
        Span::styled("→ ", Style::new().fg(velocity_color)),
        Span::styled(
            format!("resolving {:.2}x", resolution.velocity),
            Style::new().fg(velocity_color)
        ),
    ])
}
```

### Urgency Visualization

```rust
// Temporal pressure color mapping
fn urgency_color(urgency: f64) -> PackedRgba {
    if urgency > 0.8 { CLR_RED }
    else if urgency > 0.5 { CLR_AMBER }
    else { CLR_CYAN }
}

// Magnitude bars (used in gaze cards)
fn render_magnitude_bar(magnitude: f64, width: usize) -> String {
    let filled = ((magnitude * width as f64).round() as usize).min(width);
    let empty = width - filled;
    format!("{}{}",
        "█".repeat(filled),
        "░".repeat(empty)
    )
}
```

---

## VI. Information Architecture

### Depth Layer Implementation

#### Depth 0: Scanning (Tension Lines)
```rust
// Single line, maximum information density
fn build_tension_line(entry: &FieldEntry, width: u16, selected: bool) -> Vec<Line> {
    let glyph = status_glyph(entry.status, entry.phase);
    let glyph_style = Style::new().fg(tendency_color(entry.tendency));

    let text_budget = width as usize - PREFIX_WIDTH - SUFFIX_WIDTH;
    let desired_text = if selected {
        word_wrap(&entry.desired, text_budget)
    } else {
        vec![truncate(&entry.desired, text_budget)]
    };

    let horizon_text = entry.horizon_label.as_deref().unwrap_or("");
    let indicator = &entry.temporal_indicator;

    desired_text.into_iter().enumerate().map(|(i, line)| {
        if i == 0 {
            // First line: glyph + text + horizon + indicator
            Line::from_spans([
                Span::styled(format!("{}{}  ", INDENT, glyph), glyph_style),
                Span::styled(line, base_style(selected)),
                Span::styled(format!("  {}", horizon_text), CLR_DIM),
                Span::styled(format!(" {}", indicator), urgency_color(entry.temporal_urgency)),
            ])
        } else {
            // Continuation line
            let prefix = if entry.position.is_some() && is_descended {
                format!("{}{}  ", INDENT, TRUNK)
            } else {
                format!("{}   ", INDENT)  // Space width of glyph
            };
            Line::from_spans([
                Span::styled(prefix, STYLES.dim),
                Span::styled(line, base_style(selected)),
            ])
        }
    }).collect()
}
```

#### Depth 1: Gaze Cards
```rust
fn render_gaze_card(entry: &FieldEntry, gaze_data: &GazeData, area: Rect, frame: &mut Frame) {
    let mut content = Vec::new();

    // Heading: tension line inside panel
    content.push(build_tension_line_for_panel(entry));

    // Light rule separator
    if has_content_below {
        content.push(Line::from(Span::styled(
            "─".repeat(area.width.saturating_sub(4) as usize),
            STYLES.dim
        )));
    }

    // Children preview (positioned first, then unpositioned)
    let positioned: Vec<_> = gaze_data.children.iter()
        .filter(|c| c.position.is_some()).collect();
    let unpositioned: Vec<_> = gaze_data.children.iter()
        .filter(|c| c.position.is_none()).collect();

    for child in positioned {
        content.push(build_child_preview_line(child, false));
    }

    if !positioned.is_empty() && !unpositioned.is_empty() {
        content.push(Line::from(Span::styled("· · ·", STYLES.dim)));
    }

    for child in unpositioned {
        content.push(build_child_preview_line(child, true));
    }

    // Reality section
    if !gaze_data.actual.is_empty() {
        content.push(Line::from(Span::styled("─".repeat(width), STYLES.dim)));
        let reality_lines = word_wrap(&gaze_data.actual, width);
        for line in reality_lines {
            content.push(Line::from(Span::styled(line, STYLES.dim)));
        }
    }

    let panel = Panel::new(Paragraph::new(Text::from_lines(content)))
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(CLR_DIM));

    panel.render(area, frame);
}
```

#### Depth 2: Full Analysis
Dedicated view with complete dynamics table and mutation history.

---

## VII. Interaction Patterns

### Navigation State Machine

```rust
#[derive(Debug, Clone)]
pub enum NavigationMode {
    Normal,
    Gazing { index: usize, full: bool },
    Editing { field: EditField },
    Creating,
    Confirming { action: ConfirmAction },
    Moving { tension_id: String },
    AgentSession { tension_id: String },
}

#[derive(Debug, Clone)]
pub enum EditField {
    Desire,
    Reality,
    Horizon,
}
```

### Input Event Mapping

```rust
// Primary navigation (always available)
fn handle_navigation_input(key: KeyCode, mode: &NavigationMode) -> Option<Msg> {
    match (key, mode) {
        (KeyCode::Char('j') | KeyCode::Down, _) => Some(Msg::MoveDown),
        (KeyCode::Char('k') | KeyCode::Up, _) => Some(Msg::MoveUp),
        (KeyCode::Char('l') | KeyCode::Enter, NavigationMode::Normal) => Some(Msg::Descend),
        (KeyCode::Char('h') | KeyCode::Backspace, _) => Some(Msg::Ascend),
        (KeyCode::Char(' '), NavigationMode::Normal) => Some(Msg::ToggleGaze),
        (KeyCode::Char(' '), NavigationMode::Gazing { full: false, .. }) => Some(Msg::GazeFullToggle),
        (KeyCode::Char('a'), NavigationMode::Normal) => Some(Msg::StartCreate),
        (KeyCode::Char('e'), NavigationMode::Normal) => Some(Msg::StartEdit),
        // ...
        _ => None
    }
}

// Mode-specific input (contextual)
fn handle_mode_input(key: KeyCode, mode: &NavigationMode) -> Option<Msg> {
    match (key, mode) {
        // Reordering mode
        (KeyCode::Char('J'), NavigationMode::Normal) => Some(Msg::StartReorder),
        (KeyCode::Char('j'), NavigationMode::Reordering { .. }) => Some(Msg::ReorderDown),
        (KeyCode::Char('k'), NavigationMode::Reordering { .. }) => Some(Msg::ReorderUp),
        (KeyCode::Enter, NavigationMode::Reordering { .. }) => Some(Msg::ReorderCommit),
        (KeyCode::Esc, NavigationMode::Reordering { .. }) => Some(Msg::ReorderCancel),

        // Alert navigation
        (KeyCode::Char(c), NavigationMode::Normal) if c.is_ascii_digit() => {
            let index = c.to_digit(10)? as usize;
            if index > 0 && index <= self.alerts.len() {
                Some(Msg::ActOnAlert { index: index - 1 })
            } else {
                None
            }
        }

        _ => None
    }
}
```

### Panel-Based Editing

```rust
fn render_edit_panel(field: EditField, current_value: &str, area: Rect, frame: &mut Frame) {
    let title = match field {
        EditField::Desire => "desire",
        EditField::Reality => "reality",
        EditField::Horizon => "horizon",
    };

    // Tab indicators
    let tab_line = Line::from_spans([
        tab_span("desire", field == EditField::Desire),
        Span::styled("  ", Style::new()),
        tab_span("reality", field == EditField::Reality),
        Span::styled("  ", Style::new()),
        tab_span("horizon", field == EditField::Horizon),
    ]);

    let content = Paragraph::new(Text::from_lines(vec![tab_line]));

    let panel = Panel::new(content)
        .title(title)
        .title_style(STYLES.cyan)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(CLR_DIM));

    // Render panel with input widget inside
    panel.render(area, frame);

    // Render TextInput in the content area
    let input_area = Rect::new(
        area.x + 1,
        area.y + 3, // Border + tab line + spacing
        area.width.saturating_sub(2),
        1
    );

    self.text_input.render(input_area, frame);
}

fn tab_span(label: &str, active: bool) -> Span {
    if active {
        Span::styled(format!("[{}]", label), STYLES.cyan)
    } else {
        Span::styled(format!(" {} ", label), STYLES.dim)
    }
}
```

---

## VIII. Agent Integration Interface

### Session Visual Structure

```rust
// Agent session transforms the view
fn render_agent_session(tension: &Tension, session: &AgentSession, area: Rect, frame: &mut Frame) {
    let layout = Flex::vertical()
        .constraints([
            Constraint::Min(8),    // Context section
            Constraint::Fixed(1),  // Heavy rule separator
            Constraint::Min(4),    // Conversation area
        ])
        .split(area);

    // Top: Tension context (structured for agent consumption)
    render_agent_context(tension, layout[0], frame);

    // Separator
    render_heavy_rule(layout[1], frame);

    // Bottom: Conversation + proposals
    render_agent_conversation(session, layout[2], frame);
}

fn render_agent_context(tension: &Tension, area: Rect, frame: &mut Frame) {
    let mut lines = vec![
        Line::from(Span::styled(
            format!("◆ {}", tension.desired),
            Style::new().fg(CLR_CYAN).bold()
        )),
        Line::from(""),
        Line::from_spans([
            Span::styled("desire  ", STYLES.label),
            Span::styled(&tension.desired, STYLES.text),
        ]),
        Line::from_spans([
            Span::styled("reality ", STYLES.label),
            Span::styled(&tension.actual, STYLES.text),
        ]),
    ];

    // Add dynamics summary, children count, recent activity
    // Structured for both human reading and agent context

    Paragraph::new(Text::from_lines(lines)).render(area, frame);
}
```

### Mutation Proposal Cards

```rust
fn render_mutation_cards(mutations: &[AgentMutation], selected: &[bool], area: Rect, frame: &mut Frame) {
    let mut cards = Vec::new();

    for (i, mutation) in mutations.iter().enumerate() {
        let is_selected = selected.get(i).copied().unwrap_or(false);
        let cursor_char = if i == self.mutation_cursor { "▸" } else { " " };
        let check_char = if is_selected { "✓" } else { " " };

        let card_content = vec![
            Line::from_spans([
                Span::styled(format!("{} [{}] ", cursor_char, check_char), STYLES.cyan),
                Span::styled(mutation.action_label(), STYLES.text_bold),
            ]),
            Line::from_spans([
                Span::styled("    ", Style::new()),
                Span::styled(mutation.description(), STYLES.text),
            ]),
        ];

        if i == self.mutation_cursor && mutation.has_reasoning() {
            let reasoning_lines = word_wrap(mutation.reasoning(), width.saturating_sub(8));
            for line in reasoning_lines {
                cards.extend(vec![Line::from_spans([
                    Span::styled("      ", Style::new()),
                    Span::styled(line, STYLES.dim),
                ])]);
            }
        }

        cards.extend(card_content);
        cards.push(Line::from(""));
    }

    Paragraph::new(Text::from_lines(cards))
        .scroll((self.mutation_scroll, 0))
        .render(area, frame);
}
```

---

## IX. Responsive Behavior

### Width-Based Adaptations

```rust
impl ResponsiveLayout {
    fn compute_layout(width: u16) -> LayoutConfig {
        match width {
            0..=79 => LayoutConfig {
                max_content_width: width,
                horizon_label_width: 0, // Hide horizons
                temporal_indicator_dots: 4, // Shorter indicators
                tension_prefix: "  ",
                show_right_annotations: false,
            },
            80..=103 => LayoutConfig {
                max_content_width: width,
                horizon_label_width: 8,
                temporal_indicator_dots: 6,
                tension_prefix: "  ",
                show_right_annotations: true,
            },
            104.. => LayoutConfig {
                max_content_width: 104, // Max width constraint
                horizon_label_width: 12,
                temporal_indicator_dots: 6,
                tension_prefix: "  ",
                show_right_annotations: true,
            },
        }
    }

    fn content_area(&self, total_area: Rect) -> Rect {
        let config = Self::compute_layout(total_area.width);
        let width = total_area.width.min(config.max_content_width);
        let x_offset = if total_area.width > config.max_content_width {
            (total_area.width - config.max_content_width) / 2
        } else {
            0
        };

        Rect::new(
            total_area.x + x_offset,
            total_area.y,
            width,
            total_area.height
        )
    }
}
```

### Height-Based Scrolling

```rust
// Virtual list implementation for field view
impl VirtualList {
    fn compute_visible_range(&self, viewport_height: u16) -> Range<usize> {
        let start = self.scroll_offset;
        let mut height_used = 0u16;
        let mut end = start;

        for i in start..self.item_count {
            let item_height = self.get_item_height(i);
            if height_used + item_height > viewport_height {
                break;
            }
            height_used += item_height;
            end = i + 1;
        }

        start..end.min(self.item_count)
    }

    fn ensure_cursor_visible(&mut self, viewport_height: u16) {
        let cursor_height = self.get_item_height(self.cursor);
        let cursor_top = self.get_item_top(self.cursor);
        let cursor_bottom = cursor_top + cursor_height;

        if cursor_top < self.scroll_offset {
            self.scroll_offset = cursor_top;
        } else if cursor_bottom > self.scroll_offset + viewport_height {
            self.scroll_offset = cursor_bottom.saturating_sub(viewport_height);
        }
    }
}
```

---

## X. Complete Widget Implementation Guide

### Primary Rendering Function

```rust
fn render_field(&self, area: Rect, frame: &mut Frame) {
    let area = self.content_area(area);
    let is_descended = self.parent_tension.is_some();

    // Phase 1: Build element list
    let elements = self.build_field_elements(area.width);

    // Phase 2: Assign rectangles and render
    let mut y = area.y;
    let scroll_offset = self.vlist.scroll_offset;
    let mut rendered_height = 0u16;

    for (i, element) in elements.iter().enumerate() {
        if rendered_height < scroll_offset {
            rendered_height += element.height(area.width);
            continue;
        }

        let element_height = element.height(area.width);
        let visible_height = element_height.min(
            (area.y + area.height).saturating_sub(y)
        );

        if visible_height > 0 {
            let element_rect = Rect::new(area.x, y, area.width, visible_height);
            self.render_element(element, element_rect, frame);
            y += visible_height;
        }

        if y >= area.y + area.height {
            break;
        }

        rendered_height += element_height;
    }
}

fn build_field_elements(&self, width: u16) -> Vec<FieldElement> {
    let mut elements = Vec::new();

    // Desire header (descended view)
    if let Some(ref parent) = self.parent_tension {
        elements.push(FieldElement::DesireHeader {
            lines: word_wrap(&parent.desired, width.saturating_sub(20) as usize),
            right_annotations: self.build_desire_annotations(),
        });
        elements.push(FieldElement::HeavyRule);
    }

    // Children (tension lines or gaze cards)
    for (i, entry) in self.siblings.iter().enumerate() {
        let is_gazed = self.gaze.as_ref().map(|g| g.index == i).unwrap_or(false);
        let is_selected = i == self.vlist.cursor;
        let is_positioned = entry.position.is_some();

        // Trunk segment for positioned children in descended view
        if self.parent_tension.is_some() && is_positioned && should_show_trunk(i, &self.siblings) {
            elements.push(FieldElement::TrunkSegment);
        }

        if is_gazed {
            elements.push(FieldElement::GazeCard { index: i });
        } else {
            elements.push(FieldElement::TensionLine {
                index: i,
                selected: is_selected,
                positioned: is_positioned,
            });
        }

        // Dotted separator between positioned/unpositioned
        if should_show_dotted_separator(i, &self.siblings) {
            elements.push(FieldElement::DottedSeparator);
        }
    }

    // Reality footer (descended view)
    if let Some(ref parent) = self.parent_tension {
        if !parent.actual.is_empty() {
            elements.push(FieldElement::LightRule);
            elements.push(FieldElement::RealityFooter {
                lines: word_wrap(&parent.actual, width.saturating_sub(20) as usize),
                right_annotations: self.build_reality_annotations(),
            });
        }
    }

    // Alerts
    if !self.alerts.is_empty() {
        elements.push(FieldElement::BlankLine);
        for (i, alert) in self.alerts.iter().enumerate() {
            elements.push(FieldElement::Alert {
                index: i,
                alert: alert.clone(),
            });
        }
    }

    elements
}
```

### Element Rendering Dispatch

```rust
fn render_element(&self, element: &FieldElement, rect: Rect, frame: &mut Frame) {
    match element {
        FieldElement::TensionLine { index, selected, positioned } => {
            let entry = &self.siblings[*index];
            let lines = self.build_tension_line(entry, rect.width, *selected, *positioned);
            Paragraph::new(Text::from_lines(lines)).render(rect, frame);
        }

        FieldElement::GazeCard { index } => {
            if let Some(gaze_data) = &self.gaze_data {
                self.render_gaze_card(*index, gaze_data, rect, frame);
            }
        }

        FieldElement::DesireHeader { lines, right_annotations } => {
            let content = self.build_header_content(lines, right_annotations, rect.width);
            Paragraph::new(Text::from_lines(content)).render(rect, frame);
        }

        FieldElement::RealityFooter { lines, right_annotations } => {
            let content = self.build_footer_content(lines, right_annotations, rect.width);
            Paragraph::new(Text::from_lines(content)).render(rect, frame);
        }

        FieldElement::HeavyRule => {
            let rule = HEAVY_RULE.to_string().repeat(rect.width.saturating_sub(4) as usize);
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", rule),
                STYLES.dim
            ))).render(rect, frame);
        }

        FieldElement::LightRule => {
            let rule = LIGHT_RULE.to_string().repeat(rect.width.saturating_sub(4) as usize);
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", rule),
                STYLES.dim
            ))).render(rect, frame);
        }

        FieldElement::TrunkSegment => {
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", TRUNK),
                STYLES.dim
            ))).render(rect, frame);
        }

        FieldElement::DottedSeparator => {
            let dots = "· ".repeat(rect.width.saturating_sub(4) as usize / 2);
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", dots),
                STYLES.dim
            ))).render(rect, frame);
        }

        FieldElement::Alert { index, alert } => {
            let line = Line::from_spans([
                Span::styled(format!("  {}  ", index + 1), STYLES.amber),
                Span::styled("⚠ ", STYLES.amber),
                Span::styled(&alert.message, STYLES.amber),
                Span::styled(" — ", STYLES.dim),
                Span::styled(&alert.action_hint, STYLES.text),
            ]);
            Paragraph::new(line).render(rect, frame);
        }

        FieldElement::BlankLine => {
            // Render nothing - the rect space itself is the blank line
        }
    }
}
```

---

## XI. Quality Assurance & Polish

### Performance Considerations

```rust
// Pre-compute expensive calculations
impl InstrumentApp {
    fn cache_computed_state(&mut self) {
        // Cache dynamics for all visible tensions
        for entry in &mut self.siblings {
            if entry.computed_dynamics.is_none() {
                entry.computed_dynamics = self.engine
                    .compute_full_dynamics_for_tension(&entry.id);
            }
        }

        // Cache temporal indicators
        let now = chrono::Utc::now();
        for entry in &mut self.siblings {
            if entry.temporal_indicator.is_empty() {
                let (indicator, urgency) = glyphs::temporal_indicator(
                    entry.last_reality_update,
                    entry.horizon.as_ref().map(|h| h.range_end()),
                    now
                );
                entry.temporal_indicator = indicator;
                entry.temporal_urgency = urgency;
            }
        }
    }
}
```

### Accessibility & Graceful Degradation

```rust
// Color-blind friendly alternative indicators
fn render_tension_with_accessibility(entry: &FieldEntry) -> Vec<Span> {
    let mut spans = vec![
        Span::styled(
            format!("{}  ", glyphs::status_glyph(entry.status, entry.phase)),
            glyph_style(entry)
        )
    ];

    // Add text-based tendency indicator for color-blind users
    if entry.tendency == StructuralTendency::Oscillating {
        spans.insert(1, Span::styled("↔ ", STYLES.dim));
    } else if entry.tendency == StructuralTendency::Advancing {
        spans.insert(1, Span::styled("→ ", STYLES.dim));
    }

    spans
}

// Terminal capability detection
fn detect_capabilities() -> TerminalCapabilities {
    let colorterm = env::var("COLORTERM").unwrap_or_default();
    let term = env::var("TERM").unwrap_or_default();

    TerminalCapabilities {
        true_color: colorterm.contains("truecolor") || colorterm.contains("24bit"),
        unicode_support: !term.contains("xterm"),
        wide_chars: true, // Most modern terminals
    }
}
```

### Error Handling & Edge Cases

```rust
// Robust rendering with fallbacks
impl InstrumentApp {
    fn render_with_fallbacks(&self, area: Rect, frame: &mut Frame) {
        if area.width < 40 || area.height < 5 {
            // Terminal too small - show minimal message
            self.render_minimal_view(area, frame);
            return;
        }

        match self.load_tensions() {
            Ok(_) => self.render_field(area, frame),
            Err(_) => self.render_error_state(area, frame),
        }
    }

    fn render_minimal_view(&self, area: Rect, frame: &mut Frame) {
        let content = if area.width < 20 {
            "werk"
        } else {
            "terminal too small"
        };

        let centered_area = center_rect(content.len() as u16, 1, area);
        Paragraph::new(content)
            .alignment(Alignment::Center)
            .render(centered_area, frame);
    }
}
```

---

## XII. Summary & Implementation Checklist

### Core Rendering Pipeline

1. **Element Assembly** → Build `FieldElement` list from current state
2. **Height Calculation** → Compute dynamic heights for scrolling
3. **Rect Assignment** → Slice viewport into element rectangles
4. **Widget Rendering** → Dispatch to appropriate ftui widgets
5. **Style Application** → Apply semantic colors and emphasis

### Widget Mapping Verification

- [x] **Tension Lines** → `Paragraph` with styled spans
- [x] **Gaze Cards** → `Panel` with internal layout
- [x] **Rules & Separators** → Custom span patterns
- [x] **Edit Panels** → `Panel` + `TextInput`
- [x] **Status Line** → `StatusLine` with left/right items
- [x] **Alert System** → `Paragraph` with numbered actions

### Interaction Model Complete

- [x] **Navigation** → j/k/h/l with state preservation
- [x] **Progressive Disclosure** → Space toggles, Enter descends
- [x] **Inline Actions** → Edit panels, creation flows
- [x] **Alert Response** → Number keys for direct action
- [x] **Agent Integration** → Session mode with proposal cards

### Visual Grammar Implemented

- [x] **Directional Law** → Reality=ground, Desire=sky enforced
- [x] **Phase Glyphs** → ◇◆◈◉ with semantic color coding
- [x] **Temporal Indicators** → Six-dot system with urgency colors
- [x] **Structural Rules** → Heavy (desire) vs Light (reality)
- [x] **Alert Badges** → ⚠ with action numbers

This design system provides a complete, implementable mapping from structural dynamics concepts to ftui widget primitives, maintaining the premium terminal aesthetic while making the instrument's depth progressively discoverable. The operator should feel that the tool knows what is foreground and background, which direction reality and desire run, and that tension, drift, collision, and progress are legible without verbosity.

The instrument rewards repeated use through consistent spatial metaphors and grows with the practitioner's sophistication without overwhelming newcomers. Every rendering decision serves the core purpose: making structural dynamics feel native to computation.