# The Operative Instrument: TUI Design System

A holistic visual language for structural dynamics in the terminal.

---

## I. PHILOSOPHY

### The Operator's Console

The Operative Instrument is a **control surface**, not an application. It draws from three traditions:

1. **Process control rooms** (nuclear, industrial) — where every indicator earns its place because misreading it has consequences. Information density is high, but never decorative. Color means something. Position means something. Absence means something.

2. **Financial terminals** (Bloomberg, Reuters) — where operators spend 14 hours a day and the interface becomes an extension of cognition. Muscle memory matters. Spatial stability matters. The tool disappears into the act of using it.

3. **Unix instrument culture** (top, htop, tig, lazygit) — where the terminal is not a limitation but a medium with its own aesthetic integrity. Monospace grids. Character-cell precision. The beauty of constraint.

The Operative Instrument serves a practitioner of structural dynamics — someone who works with the generative tension between **what is** and **what is desired**. The interface must make structural reality legible. Not pretty. Not friendly. *Legible.*

### The Tension Axis

The single most important spatial concept: **reality is ground; desire is sky.**

In every visual composition — a tension line, a gaze card, the descended view — the vertical axis encodes the fundamental direction of structural dynamics:

```
    ┌─────────────────────────────┐
    │  DESIRED STATE              │  ↑ aspiration
    │                             │
    │         the gap             │  ← this IS the tension
    │                             │
    │  ACTUAL STATE (reality)     │  ↓ ground
    └─────────────────────────────┘
```

This is not metaphor. This is layout. Desire text appears above reality text. The parent's desire is the header; the parent's reality is the footer. Children fill the gap between — they ARE the structural tension made operational.

### Information Architecture: Three Depths

The interface operates at three depths of attention, each with distinct visual treatment:

| Depth | Name | What you see | ftui primitive |
|-------|------|-------------|----------------|
| 0 | **Field** | All siblings as single-line entries. Scanning. | Paragraph lines |
| 1 | **Gaze** | One tension expanded inline. Reading. | Panel (Rounded) |
| 2 | **Full Gaze** | Dynamics + history. Analyzing. | Panel sections + Layout grid |

Progressive disclosure is additive: Depth 1 contains Depth 0's line as its heading. Depth 2 contains Depth 1 and adds analytical sections below.

---

## II. THE PALETTE

Six colors. No exceptions. Every color carries semantic weight.

```
 CLR_DEFAULT   #DCDCDC   ░░  Active text, desire content, normal state
 CLR_DIM       #646464   ░░  Chrome, labels, resolved/released, separators
 CLR_CYAN      #50BED2   ░░  Agency — selection, gaze, operator action, agent
 CLR_AMBER     #C8AA3C   ░░  Attention — oscillation, neglect, staleness, drift
 CLR_RED       #DC5A5A   ░░  Conflict — structural conflict only, nothing else
 CLR_GREEN     #50BE78   ░░  Advancing — resolution velocity, convergence, momentum
```

### Color Rules

- **Cyan is the operator's color.** Anything the operator is currently touching — selected, editing, gazing — is cyan-accented. Cyan means "I am here, I am acting."
- **Amber is the system speaking.** Dynamics that surface concern — neglect, oscillation, compensating strategies — use amber. Amber means "the structure is telling you something."
- **Red is reserved for conflict.** Structural conflict between siblings. Nothing else gets red. If everything is red, nothing is red.
- **Green appears only on evidence of advancement.** Resolution velocity, gap convergence, advancing tendency. Green is earned, not assigned.
- **Default is the working color.** Active tensions, desire text, reality text in context. The majority of the screen should be default — the operator reads content, not chrome.
- **Dim is structure.** Borders, rules, labels, separators, resolved glyphs. Dim is the scaffolding. You don't read it; you orient by it.

### Background

```
 CLR_BG          #000000   Terminal black. No gray. No transparency.
 CLR_SELECTED_BG #23232A   Selection band — barely perceptible shift.
```

---

## III. TYPOGRAPHY & GLYPHS

### The Glyph System

Glyphs are **structural indicators**, not icons. Each encodes a specific dynamic state:

```
Phase Glyphs (lifecycle position):
  ◇  Germination    — open, forming, not yet engaged
  ◆  Assimilation   — filled, being worked, substance accumulating
  ◈  Completion     — internal structure visible, nearing closure
  ◉  Momentum       — dense center, energy concentrated and forward

Terminal Glyphs (lifecycle end):
  ✦  Resolved       — faceted, crystallized, tension closed by convergence
  ·  Released       — minimal, tension closed by release

The glyph IS the tension's phase. It replaces any "status" badge.
Glyphs carry tendency color: cyan (advancing), default (stagnant), amber (oscillating).
```

### Temporal Indicators

Six cells. Each cell is a temporal window position, not a week.

**With horizon (action window):**
```
  ◌◌◦◌◌●    ◦ = now (open, moving), ● = horizon end (solid, fixed)
  ◌◌◌◦◌●    now approaches horizon
  ◌◌◌◌◦●    now at horizon — urgency peaks
  ◌◌◌◌●◦    now past horizon — overdue
```

**Without horizon (staleness drift):**
```
  ◌◌◌◌◌◎    fresh — just checked
  ◌◌◌◎◌◌    drifting — weeks since check
  ◎◌◌◌◌◌    stale — attention needed
```

Color follows urgency: cyan (comfortable) → amber (approaching) → red (overdue/stale).

### Text Hierarchy

```
  STYLES.text_bold    — Desire text (the vision). White, bold. Demands reading.
  STYLES.text         — Active content, child previews, history descriptions.
  STYLES.dim          — Reality text, labels, chrome, resolved items. Recedes.
  STYLES.label        — Dynamics labels in full gaze. Dim, fixed-width column.
```

Reality text is always dim. This is deliberate: reality is ground, it recedes. Desire is bold — it pulls attention upward, toward aspiration. The visual weight difference between desire and reality IS the tension made visible.

---

## IV. STRUCTURAL ELEMENTS

### A. The Tension Line (Depth 0)

A single tension in the field. One line. Every element positioned by constraint, not character math.

```
  ◆ Build the authentication layer              Mar 20 ◌◌◦◌●◌
  ├─────────────────────────────────────────────────────────────┤
  │glyph│ desire text                      │horizon│ temporal  │
```

**ftui implementation:**

Use `Layout` with horizontal `Flex` and four constraints:
```rust
Flex::horizontal()
    .split(line_rect, &[
        Constraint::Fixed(4),          // INDENT(2) + glyph(1) + space(1)
        Constraint::Fill,              // desire text — takes remaining
        Constraint::FitContent,        // horizon label — natural width
        Constraint::Fixed(8),          // temporal indicator — always 6 dots + spacing
    ])
```

This eliminates ALL manual character-width arithmetic. The layout engine handles alignment. Unicode width issues disappear because each region gets its own rect — text is truncated to fit the rect, not a computed character budget.

**Glyph region** (Fixed 4):
- `Span::styled(format!("{}{} ", INDENT, glyph), glyph_style)`
- Glyph colored by tendency: cyan/default/amber

**Desire region** (Fill):
- `Paragraph::new(desire_text).style(base_style)`
- Paragraph handles its own truncation within the rect
- Selected: `base_style` has `bg(CLR_SELECTED_BG)`

**Horizon region** (FitContent):
- `Badge::new(&horizon_label).with_style(STYLES.dim)`
- Badge gives us: natural-width label with consistent padding
- No horizon → empty Badge or skip region

**Temporal region** (Fixed 8):
- `Span` with urgency-colored indicator
- Right-aligned within its cell

**Selection treatment:**

Selected line: `Panel` with `Borders::LEFT` only, `border_style(CLR_CYAN)`, `style(bg(CLR_SELECTED_BG))`.

```rust
// Unselected: raw line, no container
render_tension_line(line_rect, entry, frame);

// Selected: Panel wraps the line
Panel::new(tension_line_widget)
    .borders(Borders::LEFT)
    .border_type(BorderType::Heavy)    // ┃ thick left edge
    .border_style(Style::new().fg(CLR_CYAN))
    .style(Style::new().bg(CLR_SELECTED_BG))
    .render(line_rect, frame);
```

The heavy left border (┃) is the selection indicator. No background-width math. No span padding. Panel handles it.

**Descended view — trunk line:**

When viewing children of a parent, positioned children show a trunk line. Use `Borders::LEFT` Panel around the entire positioned section:

```rust
// The trunk is a Panel wrapping all positioned children
Panel::new(positioned_children_group)
    .borders(Borders::LEFT)
    .border_type(BorderType::Square)   // │ standard trunk
    .border_style(STYLES.dim)
    .padding(Sides::left(1))
    .render(positioned_section_rect, frame);
```

This gives us continuous trunk lines for free. No manual TrunkSegment elements. No gap logic. The border IS the trunk.

### B. The Gaze Card (Depth 1)

When the operator gazes at a tension (Space), it expands inline into a Panel.

```
  ╭──────────────────────────────────────────────────────────╮
  │ ◆ Build the authentication layer          Mar 20 ◌◌◦◌●◌ │
  │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
  │ ◇ Design token storage schema                           │
  │ ◆ Implement OAuth2 flow                                 │
  │ · · · · · · · · · · · · · · · · · · · · · · · · · · · · │
  │   ◇ Research session management                         │
  │   ◇ Write integration tests                             │
  │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
  │ Using JWT with refresh tokens. Redis for session store. │
  ╰──────────────────────────────────────────────────────────╯
```

**Structure (top to bottom = desire to reality):**

1. **Heading line** — the tension line itself, inside the panel. Same layout as field line.
2. **Light Rule** — `Rule::new().border_type(BorderType::Square).style(STYLES.dim)` — separates vision from action.
3. **Positioned children** — with phase glyphs, tendency colors. These are the operational commitments.
4. **Dotted separator** — `Rule::new().title("backlog").title_style(STYLES.dim).style(STYLES.dim)` — or custom dotted rule.
5. **Unpositioned children** — indented 2 chars. These are acknowledged but not committed.
6. **Light Rule** — separates action from ground.
7. **Reality text** — dim. The current actual state. This is the ground the children stand on.

**ftui implementation:**

```rust
let content = Group::vertical(vec![
    Box::new(tension_heading_line),
    Box::new(Rule::new().style(STYLES.dim)),
    Box::new(children_list),
    Box::new(Rule::new().style(STYLES.dim)),
    Box::new(Paragraph::new(reality_text).style(STYLES.dim)),
]);

Panel::new(content)
    .border_type(BorderType::Rounded)
    .border_style(Style::new().fg(CLR_CYAN))   // cyan = operator is gazing
    .style(Style::new())
    .render(card_rect, frame);
```

**Key: the gaze card border is cyan**, not dim. Dim borders are structural chrome. Cyan borders mean the operator's attention is here. This is the single strongest visual signal on screen.

**Empty gaze card** (no children, no reality):

```
  ╭──────────────────────────────────────────────────────────╮
  │ ◇ Some new tension with nothing yet                     │
  ╰──────────────────────────────────────────────────────────╯
```

Just the heading. No "no children" text. Emptiness speaks for itself.

### C. The Full Gaze (Depth 2)

Tab from quick gaze adds analytical sections below the card content. The card grows downward (toward reality, toward ground, toward the structural substrate).

```
  ╭──────────────────────────────────────────────────────────╮
  │ ◆ Build the authentication layer          Mar 20 ◌◌◦◌●◌ │
  │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
  │ ◇ Design token storage schema                           │
  │ ◆ Implement OAuth2 flow                                 │
  │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
  │ Using JWT with refresh tokens.                          │
  │ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
  │                                                         │
  │ DYNAMICS                  │ HISTORY                     │
  │ phase       assimilation  │ 2h ago    updated reality   │
  │ tendency    advancing     │ 1d ago    added child       │
  │ magnitude   ████░░░░ .72  │ 3d ago    set horizon       │
  │ conflict    —             │ 1w ago    created           │
  │ neglect     —             │           ⋮                 │
  │ oscillation —             │ 3w ago    initial desire    │
  │ drift       stable        │                             │
  │                                                         │
  ╰──────────────────────────────────────────────────────────╯
```

**The heavy rule (━) separates operational content from analytical content.** Above the heavy rule: what you act on (children, reality). Below: what the structure tells you (dynamics, history).

**Two-column layout via ftui Layout widget:**

```rust
let dynamics_history = Layout::horizontal(
    vec![
        Box::new(dynamics_column),
        Box::new(Paragraph::new(" │ ")),  // divider
        Box::new(history_column),
    ],
    vec![
        Constraint::Ratio(1, 2),    // dynamics gets half
        Constraint::Fixed(3),        // divider
        Constraint::Ratio(1, 2),    // history gets half
    ],
);
```

**Dynamics column:**

Each dynamic rendered as `label(13w) + value`:

```rust
// Magnitude gets a MiniBar
Line::from_spans([
    Span::styled("magnitude    ", STYLES.label),
    // MiniBar::new(0.72, 8) renders ████░░░░
    Span::styled(mini_bar_string, STYLES.text),
    Span::styled(" .72", STYLES.dim),
]);

// Conflict/neglect/oscillation: absent = "—" (dim), present = colored value
Line::from_spans([
    Span::styled("conflict     ", STYLES.label),
    Span::styled("siblings competing", STYLES.red),  // or "—" in dim
]);
```

**History column:**

Reverse chronological. Time left-aligned (fixed 10w), description fills remainder:

```
  2h ago    updated reality
  1d ago    added child "Design token storage"
  3d ago    set horizon Mar 20
```

When history is longer than dynamics, show most recent + first entry with `⋮` gap:

```
  2h ago    updated reality
  1d ago    added child
            ⋮
  3w ago    created
```

### D. The Descended View (Parent Context)

When descended into a tension's children, the parent's desire and reality frame the entire view — desire as header, reality as footer. Children live in the gap between.

```
  Build the authentication layer                · 3d ago         Mar 20 ◌◌◦◌●◌
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  │
  │ ◇ Design token storage schema                                    ◌◌◌◌◌◎
  │ ◆ Implement OAuth2 flow                              Mar 15 ◌◌◦◌●◌
  │ ◈ Write migration scripts                            Mar 18 ◌◌◌◦●◌
  │
  · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · ·
      ◇ Research session management                                  ◌◌◌◌◌◎
      ◇ Write integration tests                                     ◌◌◌◌◌◎

  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  Using JWT with refresh tokens. Redis for session store.         · 2h ago
```

**The spatial metaphor is literal:**
- **Top** (desire header): where we're going. Bold, prominent, present tense.
- **Heavy rule**: the commitment boundary. Below this, we're in operational space.
- **Trunk line** (│): the positioned children are structurally committed. The trunk connects them to the parent's desire.
- **Dotted separator**: the boundary between committed and uncommitted.
- **Indented children**: unpositioned items. Present but not sequenced.
- **Light rule**: transition back to ground.
- **Bottom** (reality footer): where we are. Dim, grounding, honest.

**The trunk line is a Panel border:**

```rust
// Positioned children section wrapped in left-border Panel
Panel::new(positioned_list)
    .borders(Borders::LEFT)
    .border_type(BorderType::Square)    // │
    .border_style(STYLES.dim)
    .padding(Sides::new(0, 0, 0, 1))   // 1 cell left padding after border
    .render(positioned_rect, frame);
```

No manual trunk segment insertion. No gap tracking. The border IS the structural connection.

**Desire header layout:**

```rust
Flex::horizontal().split(header_rect, &[
    Constraint::Fill,               // desire text (bold)
    Constraint::FitContent,         // " · 3d ago" (dim, inline age)
    Constraint::Fixed(2),           // gap
    Constraint::FitContent,         // horizon label
    Constraint::Fixed(8),           // temporal indicator
])
```

**Reality footer layout:**

```rust
Flex::horizontal().split(footer_rect, &[
    Constraint::Fill,               // reality text (dim)
    Constraint::FitContent,         // " · 2h ago" (dim, inline age)
])
```

### E. The Lever (Status Bar)

The bottom bar. Always visible. The operator's instrument panel.

```
  ◆ Build auth › ◇ Token storage                    filter: all  2 insights  ? help
```

**ftui implementation: `StatusLine` with three regions:**

```rust
StatusLine::new()
    .left(StatusItem::Text(&breadcrumb_path))
    .right(StatusItem::KeyHint { key: "?", action: "help" })
    .right(StatusItem::Text(&insight_count))
    .right(StatusItem::Text(&filter_label))
    .separator("  ")
    .style(STYLES.lever)
    .render(lever_rect, frame);
```

**During agent activity:**

```rust
StatusLine::new()
    .left(StatusItem::Spinner(frame_counter))
    .left(StatusItem::Text(" thinking..."))
    .style(STYLES.cyan)
```

The `Spinner` gives us the braille animation (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏) — a subtle sign of computational life. No manual animation frames.

**Transient messages** replace the lever content for 3 seconds:

```rust
StatusLine::new()
    .left(StatusItem::Text(&format!(" {}", transient.text)))
    .style(STYLES.cyan)
```

Alternatively, use the **Toast** system for transient messages that don't displace the lever:

```rust
Toast::new(ToastContent::Text(message))
    .position(ToastPosition::BottomCenter)
    .timeout_ms(3000)
    .style(toast_style)
```

This keeps the lever stable while showing feedback. The operator never loses orientation.

### F. Alerts

Alerts are structural signals, not notifications. They surface when the dynamics computation detects actionable conditions.

**Current approach** (text lines at bottom): loses them in scroll, no visual distinction from content.

**Design system approach**: Alerts as `Badge` widgets in a dedicated alert bar above the lever.

```
  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  1 ⚠ neglect 3w — check reality    2 ⚠ horizon past 5d — extend or close
  ◆ Build auth › ◇ Token storage                                    ? help
```

Each alert is a `Badge` with amber styling and a numeric prefix for keyboard access (press 1-9):

```rust
Badge::new(&format!("{} {} — {}", num, alert.message, alert.action_hint))
    .with_style(STYLES.amber)
    .with_padding(1, 1)
```

Conflict alerts get red styling:

```rust
Badge::new(&format!("{} {} — {}", num, alert.message, alert.action_hint))
    .with_style(STYLES.red)
    .with_padding(1, 1)
```

**Layout**: Alert bar is a fixed-height region above the lever, only present when alerts exist. Use `Flex::vertical()` to split the screen:

```rust
Flex::vertical().split(terminal_rect, &[
    Constraint::Fill,           // content area (field, gaze, etc.)
    Constraint::FitContent,     // alert bar (0 or 1 row)
    Constraint::Fixed(1),       // lever (always 1 row)
])
```

Alerts become **persistent, non-scrolling, always-visible**. The operator sees structural warnings at all times.

### G. Input Surfaces

All text input uses ftui's `TextInput` widget (single-line) or `TextArea` (multi-line). No manual cursor rendering with `█`.

**Add tension flow** — Panel at insertion point:

```
  ╭ name ──────────────────────────────────────────────────╮
  │ ▏                                                      │
  ╰────────────────────────────────────────────────────────╯
```

```rust
Panel::new(TextInput::new().with_placeholder("what matters?"))
    .border_type(BorderType::Rounded)
    .border_style(Style::new().fg(CLR_CYAN))
    .title("name")
    .title_style(STYLES.cyan)
    .render(input_rect, frame);
```

TextInput handles:
- Cursor rendering and positioning
- Selection highlighting
- Grapheme-aware editing (Ctrl+W, Ctrl+A, etc.)
- Unicode width for all display math

**Edit tension flow** — Panel at bottom with field tabs:

```
  ╭ desire ────────────────────────────────────────────────╮
  │ [desire]  reality   horizon                            │
  │                                                        │
  │ Build the authentication layer▏                        │
  ╰────────────────────────────────────────────────────────╯
```

Tab cycles active field. Each field label is a `Badge`:

```rust
// Active field tab
Badge::new("desire").with_style(STYLES.cyan).with_padding(0, 0)

// Inactive field tab
Badge::new("reality").with_style(STYLES.dim).with_padding(0, 0)
```

**Confirm dialog** — Modal widget:

```rust
Modal::new(confirm_content)
    .position(ModalPosition::Centered)
    .size(ModalSizeConstraints::Bounded {
        min: (40, 6),
        max: (60, 8),
    })
    .backdrop(BackdropConfig { opacity: 0.3, clickable: false })
```

This gives us a proper centered overlay with backdrop dimming, rather than clearing a rect and hoping.

**Search overlay** — full-screen Panel:

```
  ╭ / ─────────────────────────────────────────────────────╮
  │ auth▏                                                  │
  │                                                        │
  │ ▸ Build the authentication layer       root            │
  │   Token storage schema                 › Build auth    │
  │   OAuth2 flow                          › Build auth    │
  ╰────────────────────────────────────────────────────────╯
```

Search results as a `List` widget with selection highlighting. Parent path as dim right-aligned text. The `▸` selector comes from `List`'s built-in selection indicator.

### H. The Help Surface

Full-screen overlay using `Modal` with a Panel inside.

```
  ╭──────────────────────────────────────────────────────────────────╮
  │                                                                  │
  │  NAVIGATION                                                      │
  │  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
  │  j/k          move up/down           g/G          top/bottom     │
  │  l/Enter      descend                h/Bksp       ascend         │
  │  Shift+J/K    reorder                Space         gaze           │
  │  /            search                 1-9           act on alert   │
  │                                                                  │
  │  ACTS                                                            │
  │  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
  │  a   add tension       e   edit         n   note                 │
  │  r   resolve           x   release      o   reopen               │
  │  ...                                                             │
  │                                                                  │
  │  READING THE FIELD                                               │
  │  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
  │  ◇ germination   ◆ assimilation   ◈ completion   ◉ momentum     │
  │  ✦ resolved      · released                                     │
  │                                                                  │
  │  ◆ advancing     ◆ stagnant      ◆ oscillating                  │
  │                                                                  │
  │  ◌◌◦◌●◌ temporal window    ◦ now  ● horizon end                 │
  │  ◌◌◌◌◌◎ staleness (no horizon)                                  │
  │  ◌◌◌◌◌◌ comfortable  ◌◌◌◌◌◌ approaching  ◌◌◌◌◌◌ overdue       │
  │                                                                  │
  │  press any key to close                                          │
  │                                                                  │
  ╰──────────────────────────────────────────────────────────────────╯
```

Section headers use `Rule::new().title("NAVIGATION").title_alignment(Alignment::Left)` — the rule IS the section header. No separate heading line + rule.

Key hints rendered with `StatusItem::KeyHint` layout logic — key in cyan, description in default.

### I. Mutation Review (Agent Response)

When the agent returns, the review surface replaces the field.

```
  ╭──────────────────────────────────────────────────────────────────╮
  │ agent response                                                   │
  │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
  │ The authentication layer shows good structural progression.      │
  │ Token storage and OAuth2 are advancing. Consider setting         │
  │ horizons on the unpositioned children to prevent drift.          │
  │ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
  │ suggested changes                                                │
  │                                                                  │
  │ ▸ [x] set horizon on "Research session management"    Mar 25     │
  │   [x] set horizon on "Write integration tests"       Mar 28     │
  │   [ ] add note on "Build auth"                                   │
  │                                                                  │
  ╰──────────────────────────────────────────────────────────────────╯
```

Heavy rule separates agent prose (reading) from actionable mutations (doing). Mutations as a `List` with checkbox selection.

### J. Insight Review (Daimon)

Insights from the background watch daemon. Same Panel language, progressive expand.

```
  ╭──────────────────────────────────────────────────────────────────╮
  │ the daimon noticed (3 insights)                                  │
  │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
  │ ▸ neglect onset on "Build the authentication layer"              │
  │   oscillation spike on "Revenue model"                           │
  │   horizon breached on "Q1 planning"                              │
  ╰──────────────────────────────────────────────────────────────────╯
```

Space on an insight expands it inline (same as gaze — progressive disclosure, not modal):

```
  ╭──────────────────────────────────────────────────────────────────╮
  │ the daimon noticed (3 insights)                                  │
  │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │
  │ ▸ neglect onset on "Build the authentication layer"              │
  │   ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
  │   No reality check in 4 weeks. The structural gap between       │
  │   desired and actual is growing stale...                         │
  │                                                                  │
  │   ▸ update reality                                               │
  │     "Currently using basic JWT, no refresh tokens yet"           │
  │   ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  │
  │   oscillation spike on "Revenue model"                           │
  │   horizon breached on "Q1 planning"                              │
  ╰──────────────────────────────────────────────────────────────────╯
```

---

## V. WIDGET MAPPING

Complete mapping from domain concept to ftui widget:

| Domain Concept | ftui Widget | Configuration |
|---|---|---|
| Tension line (unselected) | `Paragraph` in `Flex` layout | Horizontal, 4 constraints |
| Tension line (selected) | `Panel` + `Paragraph` | `Borders::LEFT`, `Heavy`, cyan, bg fill |
| Gaze card | `Panel` + composite content | `Rounded`, cyan border |
| Full gaze dynamics/history | `Layout` (2-column grid) | `Ratio(1,2)` + `Fixed(3)` + `Ratio(1,2)` |
| Magnitude bar | `MiniBar` | Custom chars `█` `░`, 8 width |
| Activity sparkline | `Sparkline` | 6-12 data points, gradient cyan→amber |
| Temporal indicator | `Span` sequence | Manual (6 glyphs, colored by urgency) |
| Phase glyph | `Span` | Colored by tendency |
| Horizon label | `Badge` | Dim style, natural width |
| Alert | `Badge` | Amber or red, numbered |
| Alert bar | `Flex` row of `Badge`s | Fixed height, above lever |
| Lever (status bar) | `StatusLine` | 3 regions, breadcrumbs left, hints right |
| Spinner (agent active) | `StatusLine` with `Spinner` | Braille animation |
| Transient message | `Toast` | BottomCenter, 3s timeout |
| Desire header | `Paragraph` in `Flex` | Bold, with age suffix and horizon |
| Reality footer | `Paragraph` in `Flex` | Dim, with age suffix |
| Heavy rule | `Rule` | `BorderType::Heavy`, dim |
| Light rule | `Rule` | `BorderType::Square`, dim |
| Dotted separator | `Rule` with custom `BorderSet` | Dotted char `·` |
| Section header | `Rule` with title | Title left-aligned |
| Trunk line | `Panel` `Borders::LEFT` | Wraps positioned children section |
| Text input (single-line) | `TextInput` in `Panel` | Rounded, cyan border, placeholder |
| Text input (multi-line) | `TextArea` in `Panel` | With soft wrap |
| Field tab labels | `Badge` | Cyan (active) / dim (inactive) |
| Confirm dialog | `Modal` + `Panel` | Centered, backdrop, bounded size |
| Search results | `List` in `Panel` | Selection, truncation |
| Help overlay | `Modal` + `Panel` | Full-screen, `Rule` section headers |
| Mutation review | `Panel` + `List` | Checkboxes, heavy rule divider |
| Insight review | `Panel` + `List` | Progressive expand (like gaze) |
| Empty state | `Paragraph` | Centered, dim, with ◇ glyph |
| Breadcrumbs | `StatusItem::Text` | Glyph + truncated name, `›` separator |
| Reorder grab handle | `Badge` | `≡` glyph, cyan |
| Key hints | `StatusItem::KeyHint` | Key cyan, action default |
| Content area | `Flex::vertical` | `Fill` (content) + `FitContent` (alerts) + `Fixed(1)` (lever) |

---

## VI. LAYOUT ARCHITECTURE

### Screen Decomposition

The terminal is divided vertically into three fixed regions:

```
  ┌─────────────────────────────────────────────┐
  │                                             │
  │              CONTENT AREA                   │  Constraint::Fill
  │         (field, gaze, overlays)             │
  │                                             │
  │                                             │
  ├─────────────────────────────────────────────┤
  │ 1 ⚠ neglect  2 ⚠ horizon past              │  Constraint::FitContent (0 or 1 row)
  ├─────────────────────────────────────────────┤
  │ ◆ Build auth › ◇ Token       ? help        │  Constraint::Fixed(1)
  └─────────────────────────────────────────────┘
```

```rust
let regions = Flex::vertical().split(terminal_rect, &[
    Constraint::Fill,           // content
    Constraint::FitContent,     // alert bar (collapses to 0 when empty)
    Constraint::Fixed(1),       // lever
]);
```

### Content Area Width Constraint

Content is centered and max-width constrained on wide terminals:

```rust
fn content_area(terminal: Rect) -> Rect {
    let max_w = 104;
    let width = terminal.width.min(max_w);
    let x = (terminal.width - width) / 2;
    Rect::new(x, terminal.y, width, terminal.height)
}
```

### Descended View Decomposition

```rust
let regions = Flex::vertical().split(content_rect, &[
    Constraint::FitContent,     // desire header (1-3 lines)
    Constraint::Fixed(1),       // heavy rule
    Constraint::Fill,           // children (scrollable)
    Constraint::Fixed(1),       // light rule (if reality exists)
    Constraint::FitContent,     // reality footer (1-3 lines)
]);
```

### Tension Line Decomposition

```rust
let cells = Flex::horizontal().split(line_rect, &[
    Constraint::Fixed(4),       // indent + glyph + space
    Constraint::Fill,           // desire text
    Constraint::FitContent,     // horizon badge
    Constraint::Fixed(8),       // temporal indicator
]);
```

---

## VII. INTERACTION PATTERNS

### Selection Language

| State | Visual | Border | Background | Glyph Color |
|-------|--------|--------|------------|-------------|
| Unselected | Clean line | None | Terminal bg | Tendency color |
| Selected | Left-accent line | `LEFT` Heavy, cyan | `CLR_SELECTED_BG` | Tendency color, bold |
| Gazed (quick) | Rounded card | `ALL` Rounded, cyan | Terminal bg | Cyan (override) |
| Gazed (full) | Extended card | `ALL` Rounded, cyan | Terminal bg | Cyan (override) |
| Grabbed (reorder) | Left-accent line | `LEFT` Heavy, cyan | Terminal bg | Cyan (override) |

**Key principle**: selection is a **left-edge accent**. The eye scans down the left margin. The cyan heavy bar catches it. No full-width background wrestling.

When grabbed for reordering, the `≡` grab handle replaces the phase glyph:

```rust
Badge::new("≡").with_style(STYLES.cyan)
```

### Progressive Disclosure

```
  Normal     →  Space  →  Tab    →  l/Enter
  (scan)        (gaze)    (full)    (descend)

  1 line        card      card+     new field
                          dynamics
```

Each step adds information without removing context. Gaze doesn't navigate — it reveals. Only `l`/`Enter` navigates (changes the field).

### Directional Semantics

| Direction | Meaning | Keys |
|-----------|---------|------|
| **Down** (j) | Toward ground, toward next sibling | Scanning |
| **Up** (k) | Toward vision, toward previous sibling | Scanning |
| **Right** (l) | Into depth, into children, more detail | Descending |
| **Left** (h) | Out of depth, toward parent, less detail | Ascending |
| **Space** | Expand in place (gaze) | Revealing |
| **Tab** | Deepen in place (full gaze) | Analyzing |

Right = into the structure. Left = out of the structure. Down = through siblings. Up = through siblings. This maps to tree navigation where depth = horizontal, breadth = vertical.

---

## VIII. SPECIAL STATES

### Empty State

```
  ╭──────────────────────────────────────────────────────────╮
  │                                                          │
  │                          ◇                               │
  │                                                          │
  │                  nothing here yet.                       │
  │                                                          │
  │              press  a  to name what matters.             │
  │                                                          │
  ╰──────────────────────────────────────────────────────────╯
```

The Panel wraps the empty state, giving it presence. The `◇` germination glyph signals: this is a space waiting for its first tension. The invitation is specific: "name what matters" — not "create a task."

### Descended Empty State

When inside a tension that has no children:

```
  Build the authentication layer                              Mar 20 ◌◌◦◌●◌
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

                          ◇

                  no children yet.

              press  a  to decompose.

  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  Using JWT with refresh tokens.                                    · 2h ago
```

"decompose" — not "add." The structural dynamics language: you decompose a tension into sub-tensions.

### Filter State

When filter is set to "All", resolved and released tensions appear dim with their terminal glyphs:

```
  ◆ Build the authentication layer              Mar 20 ◌◌◦◌●◌
  ✦ Set up CI pipeline                                              ← dim, ✦ resolved
  · Research competitor auth                                        ← dim, · released
  ◇ Design API rate limiting                             ◌◌◌◌◌◎
```

No separate visual treatment needed — the glyphs and dim style communicate terminal state.

---

## IX. DYNAMICS VISUALIZATION

### Magnitude: MiniBar

```rust
MiniBar::new(magnitude, 8)
    .filled_char('█')
    .empty_char('░')
    .colors(MiniBarColors {
        high: CLR_GREEN,      // large gap = green (high tension = energy)
        mid: CLR_DEFAULT,
        low: CLR_DIM,         // small gap = dim (approaching resolution)
        critical: CLR_AMBER,  // if oscillating
    })
```

### Activity History: Sparkline

Replace the manual dot trail with actual `Sparkline`:

```rust
Sparkline::new(&weekly_mutation_counts)  // [0.0, 1.0, 3.0, 0.0, 2.0, 1.0]
    .gradient(CLR_DIM, CLR_CYAN)         // quiet → active
    .bounds(0.0, 5.0)                    // normalize
    .render(sparkline_rect, frame);      // 12w × 1h
```

This gives us `▁▃█▁▅▃` — a real sparkline showing mutation activity over time. Far more information-dense than dots.

Consider using this in the full gaze dynamics section alongside or instead of the temporal indicator for a richer activity signal.

### Resolution Velocity

In full gaze, when resolution data exists:

```
  resolution   ▃▅▆▇ closing              ← sparkline of gap reduction over time
               velocity sufficient ✓       ← or: insufficient ⚠
```

The sparkline shows gap measurements over time. Upward = gap increasing (bad). Downward = gap decreasing (good). Invert the data so the visual reads correctly: higher bars = more progress.

---

## X. DESIGN TOKENS

Collected constants for the design system:

```rust
// Layout
const MAX_CONTENT_WIDTH: u16 = 104;
const INDENT: u16 = 2;                    // left margin
const GLYPH_CELL_WIDTH: u16 = 4;          // indent + glyph + space
const TEMPORAL_CELL_WIDTH: u16 = 8;       // 6 dots + 2 spacing
const TRUNK_PADDING_LEFT: u16 = 1;        // space after trunk border

// Border types by semantic role
const BORDER_STRUCTURAL: BorderType = BorderType::Square;    // trunk, dividers
const BORDER_CONTAINER: BorderType = BorderType::Rounded;    // gaze card, input panel
const BORDER_ACCENT: BorderType = BorderType::Heavy;         // selection indicator
const BORDER_DIVISION: BorderType = BorderType::Heavy;       // operational ↔ analytical

// Rule types by semantic role
// Light rule (┄): within a container, between related sections
// Heavy rule (━): between operational and analytical, or parent header and children
// Dotted rule (· ·): between positioned and unpositioned (commitment boundary)

// Timing
const TRANSIENT_DURATION_MS: u64 = 3000;
const TOAST_DURATION_MS: u64 = 3000;

// Content limits
const BREADCRUMB_MAX_NAME_WIDTH: usize = 20;
const HORIZON_BADGE_MIN_WIDTH: usize = 6;
const CHILDREN_PREVIEW_MAX: usize = 8;
const HISTORY_DISPLAY_MAX: usize = 12;
const DYNAMICS_LABEL_WIDTH: usize = 13;
```

---

## XI. ANTI-PATTERNS

Things this design system explicitly avoids:

1. **No manual character-width arithmetic.** All layout through `Flex`/`Layout` constraints. Unicode width handled by ftui's `display_width()` and widget internals.

2. **No background-band span padding.** Selection via `Panel::borders(LEFT)` + `.style(bg)`. The Panel handles its own geometry.

3. **No manual trunk segment insertion.** Trunk via `Panel::borders(LEFT)` wrapping the positioned section. Continuous by construction.

4. **No manual cursor rendering** (█ blocks). TextInput handles cursor display, blinking, and positioning.

5. **No manual rule repetition** (`repeat(w)`). `Rule` widget fills its rect automatically.

6. **No `chars().count()` for layout math.** ftui's text primitives handle display width internally.

7. **No clearing rects for overlays.** `Modal` with backdrop handles occlusion. Panel clips its children.

8. **No mixed concerns in render functions.** Each `FieldElement` maps to one widget composition. Render function = widget tree construction.

---

## XII. MIGRATION STRATEGY

### Phase 1: Foundation
- Add `unicode-width` crate (ftui uses it internally but render.rs needs it for `word_wrap`)
- Replace manual `Rule` rendering with `Rule` widget
- Replace manual StatusLine construction with proper `StatusItem` usage

### Phase 2: Layout Engine
- Replace character-math tension line layout with `Flex::horizontal()` + constraints
- Replace desire header / reality footer char math with `Flex` layout
- Introduce `content_area` → `alert_bar` → `lever` vertical split

### Phase 3: Selection & Trunk
- Replace background-band spans with `Panel::borders(LEFT)` selection
- Replace TrunkSegment elements with `Panel::borders(LEFT)` around positioned section
- Remove all `CLR_SELECTED_BG` span arithmetic

### Phase 4: Input Surfaces
- Replace manual cursor (█) with `TextInput` widget throughout
- Replace manual modal clearing with `Modal` widget for confirms
- Use `Panel` with `TextInput` for all input flows

### Phase 5: Polish
- `Badge` for horizon labels and alerts
- `Sparkline` for activity visualization in full gaze
- `MiniBar` for magnitude display
- `Toast` for transient messages
- Alert bar with `Badge` widgets above lever

### Phase 6: Refinement
- `StatusLine::Spinner` for agent activity
- `StatusItem::KeyHint` for help integration
- Custom `BorderSet` for dotted separator rule
- Animation on Toast/Modal if terminal supports it

---

*This document is the source of truth for all visual decisions in the Operative Instrument. When in doubt, return to the tension axis: reality is ground, desire is sky, and the gap between them is where the operator works.*
