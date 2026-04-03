# TUI Reimagined: Implementation Plan

**Date:** 2026-03-31
**Status:** Builder's document. Derived from `tui-reimagined.md` (vision), `werk-conceptual-foundation.md` (sacred core), `frankensuite-evaluation.md` (ftui deep dive).
**Parent tension:** #145. Phases: #162, #163, #164, #165, #166.

---

## Cross-Phase Foundations

These concerns span all five phases. They are defined once here and referenced throughout.

### The Color/Style Bridge

ftui's `Theme` system uses `ftui::Color` (an enum with `Rgb(Rgb { r, g, b })`, `Ansi256(u8)`, and named variants). The rendering pipeline uses `ftui::PackedRgba` for `Style` fg/bg. The bridge is:

```rust
fn color_to_packed(color: ftui::Color) -> PackedRgba {
    match color {
        ftui::Color::Rgb(rgb) => PackedRgba::rgb(rgb.r, rgb.g, rgb.b),
        ftui::Color::Ansi256(idx) => {
            // Convert ANSI 256 to approximate RGB
            let (r, g, b) = ansi256_to_rgb(idx);
            PackedRgba::rgb(r, g, b)
        }
        // Named colors map to their standard RGB values
        ftui::Color::Reset => PackedRgba::TRANSPARENT,
        _ => PackedRgba::rgb(220, 220, 220), // fallback
    }
}
```

This function lives in the new `werk-tui/src/theme.rs` as `resolve_color()`. Every place that currently reads a `CLR_*` constant will instead call `RESOLVED_THEME.primary()` (or similar), which returns `PackedRgba` after resolving through AdaptiveColor.

The `ResolvedTheme` struct (produced once at startup, re-produced on terminal background change) holds all 18 semantic slots pre-resolved to `PackedRgba`. This is the hot-path type -- no per-frame resolution.

### State Ownership Evolution

The current `InstrumentApp` struct (52 fields, ~1,100 lines) owns everything. The migration strategy is **wrap, don't scatter**:

```
Phase 1: InstrumentApp gains `theme: ResolvedTheme` field (replaces static STYLES)
Phase 2: InstrumentApp gains `layout: LayoutState` sub-struct (pane proportions, breakpoint)
Phase 3: InstrumentApp gains `interaction: InteractionState` sub-struct (modal stack, toast queue, undo)
Phase 4: Frontier zones become ftui widget state (ListState, TreeState)
Phase 5: InstrumentApp gains `workspace: WorkspaceSnapshot` for persistence
```

Each sub-struct is introduced in the phase that needs it. InstrumentApp remains the single Model impl. The sub-structs are owned fields, not separate state stores. This preserves the current architecture's simplicity while partitioning state logically.

### The deck.rs Incremental Strategy

`deck.rs` is 2,066 lines containing:
- `ColumnLayout` (lines ~50-157): column width computation
- `Frontier` + `CursorTarget` (lines ~160-600): frontier classification and cursor mapping
- `DeckCursor` (lines ~600-700): cursor position tracking
- `DeckConfig` (lines ~100-122): configuration
- `ZoomLevel` + `FocusedDetail` (lines ~700-900): zoom state
- `render_deck()` and sub-renderers (lines ~900-2066): the rendering pipeline

The migration cuts deck.rs along these seams:

| Phase | What migrates out of deck.rs | Where it goes |
|-------|------------------------------|---------------|
| #162 | Nothing. deck.rs gets the new theme wired in. | theme.rs (new ResolvedTheme) |
| #163 | `ColumnLayout`, `render_deck()` top-level layout | New `deck_layout.rs` using PaneLayout/Flex |
| #164 | Input-mode rendering (add/edit/confirm/pathway prompts) | Modals in render.rs, no longer overlay hacks |
| #165 | `Frontier` zone renderers (route/overdue/held/accumulated) | New `deck_zones.rs` using List widget |
| #166 | `DeckCursor` + `CursorTarget` | Replaced by FocusGraph; deleted |

Each phase leaves deck.rs compilable and functional. The old code co-exists with the new code until the new code proves correct, then the old code is deleted.

### The Transition Guarantee

After every phase, `cargo run --bin werk` produces a working TUI. The strategy:
1. New code is added alongside old code (new modules, new fields on InstrumentApp).
2. A feature flag or runtime toggle selects new vs. old rendering path where needed.
3. Old code is deleted only after the new code is verified.
4. Tests (ProgramSimulator-based after Phase 4) run both paths during transition.

---

## Phase 1: Rendering Foundation (#162)

**Deadline:** 2026-05
**Essence:** Replace the hardcoded 6-color palette with ftui's AdaptiveColor theme system. Re-enable Bayesian diff rendering. The TUI looks the same on dark terminals but now works on light terminals too.

### 1. Theory of Closure

Done means:
- `theme.rs` is rewritten: AdaptiveColor-based theme with 18 semantic slots, resolving to `PackedRgba`.
- `lib.rs` detects dark/light terminal background and resolves the theme once at startup.
- Every `CLR_*` constant reference across `deck.rs`, `render.rs`, `survey.rs`, `helpers.rs` is replaced with resolved theme lookups.
- `STYLES` static is replaced with a runtime-resolved `Styles` struct owned by `InstrumentApp`.
- Bayesian diff is re-enabled in `lib.rs` with correct configuration (the "all white" bug is fixed by the theme providing proper defaults, not by disabling diff).
- The TUI renders correctly on both dark and light terminal backgrounds.
- `helpers.rs` `clear_area_styled()` uses theme background instead of hardcoded `CLR_DIM`.

Files created:
- None (theme.rs is rewritten in place).

Files modified:
- `werk-tui/src/theme.rs` — full rewrite
- `werk-tui/src/lib.rs` — dark mode detection, diff config change
- `werk-tui/src/app.rs` — add `theme: ResolvedTheme` field, remove `STYLES` references
- `werk-tui/src/deck.rs` — replace `CLR_*` and `STYLES.*` with `self.theme.*`
- `werk-tui/src/render.rs` — same replacement
- `werk-tui/src/survey.rs` — same replacement
- `werk-tui/src/helpers.rs` — use theme background
- `werk-tui/Cargo.toml` — ensure ftui features include theme support

Files deleted:
- None in this phase.

What the TUI looks like after Phase 1: Visually identical on dark terminals. Now also correct on light terminals (amber is darker orange, text respects terminal fg, backgrounds adapt). The palette is richer (18 semantic slots vs 6 hardcoded colors) but used with the same restraint.

### 2. Dependencies

None. This phase has no prerequisites beyond the current codebase.

Can overlap with: Nothing yet. This is the foundation. Phases 2-5 depend on it.

### 3. Detailed Implementation Steps

**Step 1: Rewrite `theme.rs`**

Replace the current file (72 lines of hardcoded constants) with:

```rust
//! AdaptiveColor theme for the Operative Instrument.

use ftui::{Theme, ThemeBuilder, AdaptiveColor, ResolvedTheme, Color, PackedRgba};
use ftui::style::Style;

/// Resolve an ftui Color to a PackedRgba for rendering.
pub fn resolve_color(color: Color) -> PackedRgba {
    match color {
        Color::Rgb(rgb) => PackedRgba::rgb(rgb.r, rgb.g, rgb.b),
        Color::Reset => PackedRgba::TRANSPARENT,
        // Other variants get reasonable defaults
        _ => PackedRgba::rgb(220, 220, 220),
    }
}

/// Build the instrument's theme.
pub fn instrument_theme() -> Theme {
    Theme::builder()
        // Monochrome foundation
        .text(AdaptiveColor::adaptive(
            Color::Rgb(ftui::Rgb { r: 40, g: 40, b: 40 }),   // light terminal
            Color::Rgb(ftui::Rgb { r: 220, g: 220, b: 220 }), // dark terminal
        ))
        .text_muted(AdaptiveColor::adaptive(
            Color::Rgb(ftui::Rgb { r: 120, g: 120, b: 120 }),
            Color::Rgb(ftui::Rgb { r: 100, g: 100, b: 100 }),
        ))
        .text_subtle(AdaptiveColor::adaptive(
            Color::Rgb(ftui::Rgb { r: 160, g: 160, b: 160 }),
            Color::Rgb(ftui::Rgb { r: 160, g: 160, b: 160 }),
        ))
        // Accent — cyan
        .accent(AdaptiveColor::adaptive(
            Color::Rgb(ftui::Rgb { r: 30, g: 140, b: 170 }),
            Color::Rgb(ftui::Rgb { r: 80, g: 190, b: 210 }),
        ))
        // Exception colors
        .warning(AdaptiveColor::adaptive(
            Color::Rgb(ftui::Rgb { r: 170, g: 120, b: 20 }),  // darker amber on light
            Color::Rgb(ftui::Rgb { r: 200, g: 170, b: 60 }),  // warm amber on dark
        ))
        .error(AdaptiveColor::adaptive(
            Color::Rgb(ftui::Rgb { r: 180, g: 50, b: 50 }),
            Color::Rgb(ftui::Rgb { r: 220, g: 90, b: 90 }),
        ))
        .success(AdaptiveColor::adaptive(
            Color::Rgb(ftui::Rgb { r: 40, g: 140, b: 80 }),
            Color::Rgb(ftui::Rgb { r: 80, g: 190, b: 120 }),
        ))
        // Surfaces
        .background(AdaptiveColor::adaptive(
            Color::Rgb(ftui::Rgb { r: 255, g: 255, b: 255 }),
            Color::Rgb(ftui::Rgb { r: 0, g: 0, b: 0 }),
        ))
        .surface(AdaptiveColor::adaptive(
            Color::Rgb(ftui::Rgb { r: 245, g: 245, b: 248 }),
            Color::Rgb(ftui::Rgb { r: 35, g: 35, b: 42 }),
        ))
        .border(AdaptiveColor::adaptive(
            Color::Rgb(ftui::Rgb { r: 200, g: 200, b: 210 }),
            Color::Rgb(ftui::Rgb { r: 60, g: 60, b: 70 }),
        ))
        .build()
}

/// Pre-resolved styles for rendering. All colors are PackedRgba.
pub struct InstrumentStyles {
    pub text: Style,
    pub text_bold: Style,
    pub subdued: Style,
    pub dim: Style,
    pub amber: Style,
    pub red: Style,
    pub cyan: Style,
    pub green: Style,
    pub selected: Style,
    pub label: Style,
    pub lever: Style,
}

impl InstrumentStyles {
    /// Resolve all styles from a theme for the detected terminal mode.
    pub fn resolve(resolved: &ResolvedTheme) -> Self {
        let text_fg = resolve_color(resolved.text);
        let dim_fg = resolve_color(resolved.text_muted);
        let accent_fg = resolve_color(resolved.accent);
        let warn_fg = resolve_color(resolved.warning);
        let err_fg = resolve_color(resolved.error);
        let ok_fg = resolve_color(resolved.success);
        let surface_bg = resolve_color(resolved.surface);
        let subdued_fg = resolve_color(resolved.text_subtle);

        Self {
            text: Style::new().fg(text_fg),
            text_bold: Style::new().fg(PackedRgba::rgb(255, 255, 255)).bold(),
            subdued: Style::new().fg(subdued_fg),
            dim: Style::new().fg(dim_fg),
            amber: Style::new().fg(warn_fg),
            red: Style::new().fg(err_fg),
            cyan: Style::new().fg(accent_fg),
            green: Style::new().fg(ok_fg),
            selected: Style::new()
                .fg(PackedRgba::rgb(255, 255, 255))
                .bg(surface_bg),
            label: Style::new().fg(dim_fg),
            lever: Style::new().fg(dim_fg),
        }
    }
}
```

**Step 2: Add theme to InstrumentApp**

In `app.rs`, add these fields:

```rust
pub struct InstrumentApp {
    // ... existing fields ...
    pub is_dark: bool,
    pub styles: crate::theme::InstrumentStyles,
}
```

In `InstrumentApp::new()`, initialize:

```rust
let theme = crate::theme::instrument_theme();
let is_dark = Theme::detect_dark_mode();
let resolved = theme.resolve(is_dark);
let styles = crate::theme::InstrumentStyles::resolve(&resolved);
```

In `InstrumentApp::new_empty()`, same initialization.

**Step 3: Replace all `STYLES.*` references**

This is a mechanical find-and-replace across four files. The pattern:

| Old | New |
|-----|-----|
| `STYLES.text` | `self.styles.text` |
| `STYLES.dim` | `self.styles.dim` |
| `STYLES.amber` | `self.styles.amber` |
| `STYLES.red` | `self.styles.red` |
| `STYLES.cyan` | `self.styles.cyan` |
| `STYLES.green` | `self.styles.green` |
| `STYLES.text_bold` | `self.styles.text_bold` |
| `STYLES.subdued` | `self.styles.subdued` |
| `STYLES.selected` | `self.styles.selected` |
| `STYLES.label` | `self.styles.label` |
| `STYLES.lever` | `self.styles.lever` |

Also replace bare `CLR_*` constant uses:

| Old | New |
|-----|-----|
| `CLR_DEFAULT` | `self.styles.text.fg.unwrap()` (or store as separate `PackedRgba` fields) |
| `CLR_DIM` | `self.styles.dim.fg.unwrap()` |
| `CLR_CYAN` | `self.styles.cyan.fg.unwrap()` |
| `CLR_AMBER` | `self.styles.amber.fg.unwrap()` |
| `CLR_RED` | `self.styles.red.fg.unwrap()` |
| `CLR_GREEN` | `self.styles.green.fg.unwrap()` |
| `CLR_SELECTED_BG` | `self.styles.selected.bg.unwrap()` |
| `CLR_BG` | resolved from theme background |

Where `&self` is not available (standalone functions that take `Style` args), pass the style as a parameter. This affects:
- `helpers::clear_area_styled()` — add a `bg: PackedRgba` parameter
- Survey rendering functions that currently import `STYLES` directly

**Step 4: Fix `clear_area_styled` to use theme**

```rust
pub fn clear_area_styled(frame: &mut ftui::Frame<'_>, area: ftui::layout::Rect, dim_fg: PackedRgba) {
    let cell = ftui::Cell::from_char(' ')
        .with_fg(dim_fg)
        .with_bg(ftui::PackedRgba::TRANSPARENT);
    frame.buffer.fill(area, cell);
}
```

All call sites pass `self.styles.dim.fg.unwrap()`.

**Step 5: Re-enable Bayesian diff**

In `lib.rs`, change the diff config:

```rust
let diff_config = RuntimeDiffConfig::default()
    .with_bayesian_enabled(true)
    .with_dirty_rows_enabled(true);
```

The "all white" bug (documented in the comment at line 91-105 of `lib.rs`) was caused by `Cell::default()` having `fg=WHITE`. With the theme-based `clear_area_styled` now filling all cells with theme-appropriate defaults, un-written cells no longer flash white. The Bayesian diff can safely skip unchanged cells because every cell is initialized to a valid theme color.

**Investigation needed:** Test this on multiple terminals (iTerm2, WezTerm, Ghostty, Terminal.app) before committing. If the bug resurfaces, keep Bayesian disabled and file a detailed report. The fallback is to keep `with_bayesian_enabled(false)` — the theme migration is still valuable without it.

**Step 6: Remove dead constants from theme.rs**

After all references are updated, delete the old `CLR_*` constants, `Styles` struct, and `STYLES` static. The file should contain only the new `instrument_theme()`, `resolve_color()`, and `InstrumentStyles`.

### 4. ftui API Specifics

| Type | Import | Constructor | State |
|------|--------|-------------|-------|
| `Theme` | `ftui::Theme` | `Theme::builder()...build()` | Immutable. Built once. |
| `ThemeBuilder` | `ftui::ThemeBuilder` | Chained `.text()`, `.accent()`, etc. | Builder pattern, consumed by `.build()`. |
| `AdaptiveColor` | `ftui::AdaptiveColor` | `AdaptiveColor::adaptive(light, dark)` | Holds two `Color` values. |
| `ResolvedTheme` | `ftui::ResolvedTheme` | `theme.resolve(is_dark)` | Holds resolved `Color` values per slot. |
| `Theme::detect_dark_mode` | `ftui::Theme` | `Theme::detect_dark_mode() -> bool` | Queries terminal background via OSC 11. |

The Theme is built in `theme.rs`, resolved in `app.rs` during `InstrumentApp::new()`. The `ResolvedTheme` is consumed to produce `InstrumentStyles`, which lives on `InstrumentApp`. The `Theme` itself can be discarded after resolution (or kept if we want to re-resolve on terminal background change detection).

### 5. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| `Theme::detect_dark_mode()` returns wrong value on some terminals | Provide a `--dark`/`--light` CLI flag override. Fall back to dark. |
| Bayesian diff re-enablement causes rendering glitches | Test on 4+ terminals. Keep the disable-diff codepath behind a config flag `tui.bayesian_diff = false`. |
| Large mechanical replacement introduces typos | Run `cargo check` after each file. The compiler catches all `STYLES` references that were missed. |
| `PackedRgba::TRANSPARENT` bg causes issues on some terminals | The current code already uses TRANSPARENT. No regression expected. |

### 6. What Gets Deleted

After this phase:
- `theme.rs`: `CLR_DEFAULT`, `CLR_DIM`, `CLR_AMBER`, `CLR_RED`, `CLR_CYAN`, `CLR_GREEN`, `CLR_SELECTED_BG`, `CLR_BG` (8 constants)
- `theme.rs`: `Styles` struct and `static STYLES: LazyLock<Styles>` (the entire old style system)
- All `use crate::theme::*` imports that were pulling in the old constants (replaced with `use crate::theme::InstrumentStyles` or accessed through `self.styles`)

### 7. Checkpoint Criteria

- [ ] `cargo check` passes with zero warnings related to theme
- [ ] `cargo run --bin werk` renders correctly on a dark terminal (iTerm2/WezTerm/Ghostty)
- [ ] `cargo run --bin werk` renders correctly on a light terminal (Terminal.app with light profile, or any terminal with a white background)
- [ ] Amber urgency colors are visibly different between dark and light backgrounds
- [ ] All 6 original colors (default, dim, amber, red, cyan, green) have visually equivalent rendering on dark terminals (no regression)
- [ ] The `clear_area_styled` defense produces no white-flash artifacts
- [ ] If Bayesian diff is re-enabled: 30 seconds of navigation, descend/ascend, mode switching, and terminal resizing produce no rendering corruption
- [ ] `grep -r "CLR_" werk-tui/src/` returns zero results (all old constants removed)
- [ ] `grep -r "STYLES\." werk-tui/src/` returns zero results referencing the old static

---

## Phase 2: Spatial Skeleton (#163)

**Deadline:** 2026-06
**Essence:** Replace the hand-rolled Flex layout in `update.rs view()` and `deck.rs render_deck()` with PaneLayout for the three-zone spatial model. Add responsive breakpoints. Add focus graph foundations.

### 1. Theory of Closure

Done means:
- The TUI renders in three horizontal panes: desire anchor (top), field (middle), reality anchor (bottom).
- Pane boundaries are conceptually drag-resizable (the PaneLayout API is wired, even if mouse drag isn't handled until Phase 4).
- PaneLayout's snap points enforce min 60% for the field, min 1 line for desire/reality.
- Terminal resize triggers responsive breakpoint recalculation.
- Three breakpoint regimes (compact <80, standard 80-120, expansive >120) drive which elements are visible.
- The FocusGraph is initialized with desire/route/console/accumulated/reality nodes but is not yet used for navigation (Phase 4 wires it to keybindings).
- The gutter (2-char left column) appears on standard+ breakpoints.
- The trace column (right-aligned) degrades at narrow widths.

Files created:
- `werk-tui/src/layout.rs` — LayoutState struct, breakpoint detection, PaneLayout construction
- `werk-tui/src/focus.rs` — FocusGraph construction and node registration (skeleton)

Files modified:
- `werk-tui/src/app.rs` — add `layout: LayoutState`, `focus: FocusState` fields
- `werk-tui/src/update.rs` — replace `view()` top-level layout with PaneLayout split
- `werk-tui/src/deck.rs` — `render_deck()` receives pane Rects instead of computing its own
- `werk-tui/src/render.rs` — `content_area()` replaced by layout system
- `werk-tui/src/survey.rs` — receives pane Rects
- `werk-tui/src/msg.rs` — add `Msg::Resize(u16, u16)` variant

Files deleted:
- None in this phase (old layout code in deck.rs is superseded but stays until Phase 5 cleanup).

### 2. Dependencies

Requires Phase 1 (theme must be resolved for breakpoint-aware color decisions).

Can overlap with: Phase 1's tail end. Once `InstrumentStyles` is on `InstrumentApp`, layout work can begin.

### 3. Detailed Implementation Steps

**Step 1: Create `layout.rs`**

```rust
//! Spatial layout for the Operative Instrument.

use ftui::layout::Rect;
use ftui::layout::pane::{PaneLayout, PaneTree, PaneLeaf};
use ftui::layout::responsive::{Responsive, Breakpoint};

/// Terminal size regime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeRegime {
    Compact,   // < 80 cols or < 24 rows
    Standard,  // 80-120 cols, 24-40 rows
    Expansive, // > 120 cols or > 40 rows
}

impl SizeRegime {
    pub fn detect(width: u16, height: u16) -> Self {
        if width < 80 || height < 24 {
            Self::Compact
        } else if width > 120 || height > 40 {
            Self::Expansive
        } else {
            Self::Standard
        }
    }

    pub fn show_gutter(&self) -> bool {
        !matches!(self, Self::Compact)
    }

    pub fn show_ages(&self) -> bool {
        matches!(self, Self::Expansive)
    }

    pub fn show_sparklines(&self) -> bool {
        matches!(self, Self::Expansive)
    }
}

/// Computed pane rects for the three-zone spatial model.
pub struct PaneRects {
    pub status_top: Rect,    // 1 line: parent breadcrumb
    pub desire: Rect,        // FitContent: desire anchor
    pub field: Rect,          // Fill: the operating surface
    pub reality: Rect,        // FitContent: reality anchor
    pub status_bottom: Rect, // 1 line: session info
    pub hints: Rect,          // 1 line: key hints (hidden in compact)
}

/// Layout state tracked across frames.
pub struct LayoutState {
    pub regime: SizeRegime,
    /// Desire pane height override (from user drag or persistence). None = auto.
    pub desire_height: Option<u16>,
    /// Reality pane height override. None = auto.
    pub reality_height: Option<u16>,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            regime: SizeRegime::Standard,
            desire_height: None,
            reality_height: None,
        }
    }
}

impl LayoutState {
    /// Recompute regime from terminal dimensions.
    pub fn update_regime(&mut self, width: u16, height: u16) {
        self.regime = SizeRegime::detect(width, height);
    }

    /// Split the full terminal area into pane rects.
    pub fn split(&self, area: Rect, desire_lines: u16, reality_lines: u16) -> PaneRects {
        use ftui::layout::{Flex, Constraint};

        let show_hints = area.height >= 6 && !matches!(self.regime, SizeRegime::Compact);

        // Desire height: user override, or content-fit with min 1, max 3
        let desire_h = self.desire_height
            .unwrap_or_else(|| desire_lines.clamp(1, 3));
        let reality_h = self.reality_height
            .unwrap_or_else(|| reality_lines.clamp(1, 3));

        let mut constraints = vec![
            Constraint::Fixed(1),           // status top
            Constraint::Fixed(desire_h),    // desire anchor
            Constraint::Fill,               // field (gets remaining)
            Constraint::Fixed(reality_h),   // reality anchor
            Constraint::Fixed(1),           // status bottom
        ];
        if show_hints {
            constraints.push(Constraint::Fixed(1)); // hints
        }

        let layout = Flex::vertical().constraints(constraints);
        let rects = layout.split(area);

        PaneRects {
            status_top: rects[0],
            desire: rects[1],
            field: rects[2],
            reality: rects[3],
            status_bottom: rects[4],
            hints: if show_hints { rects[5] } else { Rect::default() },
        }
    }
}
```

**Step 2: Create `focus.rs` (skeleton)**

```rust
//! Focus graph for spatial navigation.
//!
//! Phase 2: structure only. Phase 4 wires this to keybindings.

use ftui::widgets::focus::{FocusGraph, FocusId, FocusNode, NavDirection, FocusGroup};

/// Named focus regions.
pub struct FocusIds {
    pub desire: FocusId,
    pub route: FocusId,
    pub console: FocusId,
    pub held: FocusId,
    pub input_point: FocusId,
    pub accumulated: FocusId,
    pub reality: FocusId,
}

pub struct FocusState {
    pub graph: FocusGraph,
    pub ids: FocusIds,
}

impl FocusState {
    pub fn new() -> Self {
        let mut graph = FocusGraph::new();

        let desire = graph.register(FocusNode::new("desire"));
        let route = graph.register(FocusNode::new("route"));
        let console = graph.register(FocusNode::new("console"));
        let held = graph.register(FocusNode::new("held"));
        let input_point = graph.register(FocusNode::new("input_point"));
        let accumulated = graph.register(FocusNode::new("accumulated"));
        let reality = graph.register(FocusNode::new("reality"));

        // Vertical connections (the spatial law axis)
        graph.connect(desire, route, NavDirection::Down);
        graph.connect(route, console, NavDirection::Down);
        graph.connect(console, held, NavDirection::Down);
        graph.connect(held, input_point, NavDirection::Down);
        graph.connect(input_point, accumulated, NavDirection::Down);
        graph.connect(accumulated, reality, NavDirection::Down);

        // Reverse connections
        graph.connect(reality, accumulated, NavDirection::Up);
        graph.connect(accumulated, input_point, NavDirection::Up);
        graph.connect(input_point, held, NavDirection::Up);
        graph.connect(held, console, NavDirection::Up);
        graph.connect(console, route, NavDirection::Up);
        graph.connect(route, desire, NavDirection::Up);

        Self {
            graph,
            ids: FocusIds {
                desire, route, console, held, input_point, accumulated, reality,
            },
        }
    }
}
```

**Step 3: Wire LayoutState into InstrumentApp**

Add to `app.rs`:
```rust
pub layout: crate::layout::LayoutState,
pub focus_state: crate::focus::FocusState,
```

Initialize in `new()`:
```rust
layout: crate::layout::LayoutState::default(),
focus_state: crate::focus::FocusState::new(),
```

**Step 4: Rewrite `view()` top-level layout**

In `update.rs`, the current `view()` method (line 65-154) builds a 3-part vertical Flex (content + lever + hints). Replace with:

```rust
fn view(&self, frame: &mut Frame<'_>) {
    frame.set_cursor_visible(false);
    frame.set_cursor(None);

    let area = Rect::new(0, 0, frame.width(), frame.height());
    crate::helpers::clear_area_styled(frame, area, self.styles.dim.fg.unwrap());

    // Compute desire/reality line counts for pane sizing
    let desire_lines = self.desire_line_count(area.width);
    let reality_lines = self.reality_line_count(area.width);

    let panes = self.layout.split(area, desire_lines, reality_lines);

    // Status bar (top)
    self.render_status_top(&panes.status_top, frame);

    // Desire anchor
    self.render_desire_anchor(&panes.desire, frame);

    // The field
    let in_survey = self.view_orientation == ViewOrientation::Survey;
    if in_survey {
        self.render_survey(&panes.field, frame);
    } else {
        self.render_deck(&panes.field, frame);
    }

    // Reality anchor
    self.render_reality_anchor(&panes.reality, frame);

    // Status bar (bottom)
    self.render_status_bottom(&panes.status_bottom, frame);

    // Overlays (modals render on top)
    self.render_overlays(&panes.field, frame);

    // Hints
    if panes.hints.height > 0 {
        self.render_current_hints(&panes.hints, frame);
    }
}
```

**Step 5: Extract desire/reality anchors from deck.rs**

Currently, desire and reality are rendered inline within `render_deck()`. Extract them into dedicated methods on InstrumentApp:

- `render_desire_anchor(&self, area: &Rect, frame: &mut Frame)` — renders the parent's `desired` text with deadline and age. Uses `Paragraph` widget.
- `render_reality_anchor(&self, area: &Rect, frame: &mut Frame)` — renders the parent's `actual` text with last-update age.
- `desire_line_count(&self, width: u16) -> u16` — computes how many lines the desire text needs at the given width.
- `reality_line_count(&self, width: u16) -> u16` — same for reality.

These methods go in a new section of `render.rs` or in `deck.rs` (builder's choice at implementation time — the important thing is they exist as standalone methods that the new `view()` calls).

**Step 6: Add responsive breakpoint to deck column layout**

Modify `ColumnLayout::compute()` to accept `SizeRegime`:
- In `Compact`: `age_width = 0`, gutter hidden, left column abbreviated.
- In `Standard`: current behavior.
- In `Expansive`: wider left column, sparkline space on right.

**Step 7: Update lib.rs to track regime on resize**

In the `Msg::Resize` handler (to be added), call `self.layout.update_regime(width, height)`.

Add to `msg.rs`:
```rust
Msg::Resize(u16, u16),
```

Add to the `Event` -> `Msg` conversion:
```rust
Event::Resize(w, h) => Msg::Resize(w, h),
```

### 4. ftui API Specifics

| Type | Import | Usage |
|------|--------|-------|
| `PaneLayout` | `ftui::layout::pane::PaneLayout` | Phase 2 prepares the data model but uses `Flex` splits initially. PaneLayout with drag-resize is wired in Phase 3 when modals provide the interaction layer. |
| `Responsive` | `ftui::layout::responsive::Responsive` | `SizeRegime` is the hand-rolled equivalent for Phase 2. Full `Responsive` widget wrapping comes in Phase 5 polish. |
| `Breakpoint` | `ftui::layout::responsive::Breakpoint` | Defined but not yet used as widget-level breakpoints. Phase 2 uses the simpler `SizeRegime` enum. |
| `FocusGraph` | `ftui::widgets::focus::FocusGraph` | Constructed in `FocusState::new()`. Not yet driving navigation. |
| `FocusId` | `ftui::widgets::focus::FocusId` | Opaque handle returned by `graph.register()`. Stored in `FocusIds`. |
| `FocusNode` | `ftui::widgets::focus::FocusNode` | `FocusNode::new("label")` — a named node in the graph. |
| `NavDirection` | `ftui::widgets::focus::NavDirection` | `Up`, `Down`, `Left`, `Right` — used in `graph.connect()`. |

**Note on PaneLayout:** The vision describes PaneLayout with magnetic fields and snap points. Phase 2 does NOT use PaneLayout for rendering — it uses the simpler `Flex::vertical()` split because PaneLayout requires mouse interaction to be meaningful, and that comes in Phase 3/4. The `LayoutState` struct is designed to accept PaneLayout later (the `desire_height`/`reality_height` overrides are the same state that PaneLayout's drag callbacks would set).

### 5. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Desire/reality extraction from deck.rs breaks the rendering pipeline | Keep the old `render_deck()` code intact. Add a feature flag `NEW_LAYOUT` that switches between old and new view() paths. |
| FocusGraph API doesn't match expected signatures | The focus module is a skeleton in Phase 2. If the API differs, adjust the skeleton — it's not wired to anything yet. |
| Terminal resize events flood the layout system | `SizeRegime` is an enum comparison, not a full relayout. Only recompute pane splits when regime changes. |
| Desire/reality text wrapping at narrow widths is wrong | `Paragraph` widget handles wrapping. Test with multi-line desire text at 60, 80, 120, and 200 column widths. |

### 6. What Gets Deleted

After this phase:
- `render.rs`: `content_area()` method (replaced by layout system's centering)
- `update.rs`: The old `view()` method's inline Flex layout (lines 68-84 of current update.rs)
- `deck.rs`: The desire/reality rendering code that was extracted (exact lines depend on current deck.rs structure)

The old code stays as dead code until checkpoint verification, then is removed.

### 7. Checkpoint Criteria

- [ ] The TUI shows desire text at top, field in middle, reality at bottom — matching the one spatial law
- [ ] Resizing the terminal below 80 columns triggers compact regime (route compresses, gutter hides)
- [ ] Resizing above 120 columns triggers expansive regime (more breathing room, side panel possible)
- [ ] Desire/reality anchors size to their content (short text = 1 line, long text = up to 3 lines)
- [ ] All existing navigation (j/k, l/h, Tab, etc.) still works — no behavioral regression
- [ ] FocusGraph compiles and initializes without panic (even though not driving navigation)
- [ ] `cargo test` passes (no regressions in any test)

---

## Phase 3: Interaction Model (#164)

**Deadline:** 2026-06
**Essence:** Replace InputMode-based mode switching with focus-trapped Modals. Add CommandPalette. Add undo/redo. Add Toast notifications.

### 1. Theory of Closure

Done means:
- Adding, editing, confirming, annotating, pathway palettes, and search all render as Modal overlays with backdrop dimming and focus trapping.
- The InputMode enum still exists (for the `update()` dispatch) but each mode's visual rendering is a Modal widget, not a full-screen takeover.
- CommandPalette (`Ctrl+K` or `:`) provides fuzzy search across commands and tensions.
- `Ctrl+Z` undoes the last gesture. `Ctrl+Shift+Z` redoes.
- Toast notifications appear for gesture feedback (replacing `TransientMessage`).
- The field is always visible behind modals (spatial context preserved).

Files created:
- `werk-tui/src/modal.rs` — Modal construction helpers for each interaction type
- `werk-tui/src/palette.rs` — CommandPalette setup with BayesianScorer
- `werk-tui/src/toast.rs` — Toast and NotificationQueue wiring
- `werk-tui/src/undo.rs` — UndoHistory integration with gesture model

Files modified:
- `werk-tui/src/app.rs` — add `undo: UndoHistory`, `toasts: NotificationQueue`, `palette: Option<CommandPaletteState>`
- `werk-tui/src/update.rs` — undo/redo handling in update_normal; modal dismiss/confirm routing
- `werk-tui/src/render.rs` — modal rendering replaces current overlay rendering (lines 117-134)
- `werk-tui/src/msg.rs` — add `Msg::Undo`, `Msg::Redo`, `Msg::PaletteOpen`, `Msg::PaletteSelect`, `Msg::ToastDismiss`, `Msg::ToastAction`
- `werk-tui/src/state.rs` — `TransientMessage` marked deprecated (replaced by Toast)

Files deleted:
- None yet (old overlay rendering stays until Modal rendering is verified).

### 2. Dependencies

Requires Phase 2 (layout system must provide the field Rect for modal positioning).

Can overlap with: Phase 2's tail end. Modal rendering needs pane Rects but not the FocusGraph.

### 3. Detailed Implementation Steps

**Step 1: Create `modal.rs` — Modal constructors**

```rust
//! Modal overlays for the Operative Instrument.

use ftui::widgets::modal::{Modal, ModalConfig, ModalState, dialog};
use ftui::widgets::input::TextInput;
use ftui::widgets::textarea::TextArea;
use ftui::layout::Rect;
use ftui::Frame;

/// Render an add-child modal.
pub fn render_add_modal(
    text_input: &TextInput,
    step_label: &str,  // "desire", "reality", "horizon"
    field_rect: &Rect,
    frame: &mut Frame<'_>,
    styles: &crate::theme::InstrumentStyles,
) {
    let config = ModalConfig::default()
        .with_backdrop_dimming(0.5)
        .with_animation(true);

    let content_height = 5; // label + input + padding
    let modal_rect = center_modal(*field_rect, field_rect.width.min(80), content_height);

    let modal = Modal::new(config);
    // Render modal frame, then render TextInput inside it
    modal.render(modal_rect, frame);
    // ... TextInput rendering inside modal bounds
}

/// Render a confirm modal (resolve/release).
pub fn render_confirm_modal(
    question: &str,
    field_rect: &Rect,
    frame: &mut Frame<'_>,
    styles: &crate::theme::InstrumentStyles,
) {
    let modal = dialog::confirm(question);
    let modal_rect = center_modal(*field_rect, 60, 5);
    modal.render(modal_rect, frame);
}

/// Center a modal rectangle within a parent area.
fn center_modal(parent: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(parent.width.saturating_sub(4));
    let h = height.min(parent.height.saturating_sub(2));
    let x = parent.x + (parent.width.saturating_sub(w)) / 2;
    let y = parent.y + (parent.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}
```

**Step 2: Create `palette.rs` — CommandPalette**

```rust
//! Command palette — unified command/search/navigation surface.

use ftui::widgets::command_palette::{CommandPalette, ActionItem, BayesianScorer, PaletteAction};

pub struct PaletteState {
    pub palette: CommandPalette,
    pub scorer: BayesianScorer,
    pub actions: Vec<ActionItem>,
}

impl PaletteState {
    pub fn new() -> Self {
        let mut scorer = BayesianScorer::new();

        let actions = vec![
            ActionItem::new("resolve", "Resolve the focused tension")
                .with_shortcut("r"),
            ActionItem::new("release", "Release the focused tension")
                .with_shortcut("x"),
            ActionItem::new("add", "Add a child tension")
                .with_shortcut("a"),
            ActionItem::new("edit_desire", "Edit desire")
                .with_shortcut("e → d"),
            ActionItem::new("edit_reality", "Edit reality")
                .with_shortcut("e → r"),
            ActionItem::new("note", "Add a note")
                .with_shortcut("n"),
            ActionItem::new("move", "Move / reparent")
                .with_shortcut("m"),
            ActionItem::new("undo", "Undo last gesture")
                .with_shortcut("Ctrl+Z"),
            // Navigation
            ActionItem::new("ascend", "Go to parent")
                .with_shortcut("h"),
            ActionItem::new("survey", "Switch to survey view")
                .with_shortcut("Tab"),
        ];

        let palette = CommandPalette::new(actions.clone())
            .with_scorer(scorer.clone());

        Self { palette, scorer, actions }
    }
}
```

**Step 3: Create `undo.rs` — Gesture undo integration**

```rust
//! Undo/redo for operative gestures.

use ftui::runtime::undo::{UndoHistory, UndoTransaction};

pub struct GestureUndo {
    pub history: UndoHistory,
}

impl GestureUndo {
    pub fn new() -> Self {
        Self {
            history: UndoHistory::new(),
        }
    }

    /// Begin a transaction before a gesture executes.
    pub fn begin(&mut self, label: &str) -> UndoTransaction {
        self.history.begin(label)
    }

    /// Commit the transaction after gesture succeeds.
    pub fn commit(&mut self, tx: UndoTransaction) {
        self.history.commit(tx);
    }

    /// Undo the last gesture. Returns the label if successful.
    pub fn undo(&mut self) -> Option<String> {
        self.history.undo()
    }

    /// Redo the last undone gesture. Returns the label if successful.
    pub fn redo(&mut self) -> Option<String> {
        self.history.redo()
    }
}
```

**Integration with Engine:** The undo system must capture the database state before a gesture. Two strategies:

1. **Snapshot-based:** Before each gesture, save `engine.store().snapshot()` (if fsqlite supports it). Undo restores the snapshot. Simple but potentially expensive.
2. **Inverse-mutation-based:** Record the inverse of each mutation. Undo applies the inverses in reverse order. Lighter but more complex.

**Recommended approach for Phase 3:** Start with strategy 1 (snapshot). The `InstrumentApp` already reloads from the store after every gesture (`load_siblings()`). A snapshot is just the state of `siblings`, `parent_tension`, and associated cached fields. Store these in the undo history. On undo, restore the snapshot and the corresponding database state (via `Engine::undo_gesture(gesture_id)`). The `Engine` already has gesture IDs on mutations — undo means deleting the mutations with that gesture_id and reversing their effects.

**Investigation needed:** Does `Engine` currently support `undo_gesture()`? If not, this needs to be added to `sd-core`. The TUI side records the gesture_id, and the undo handler calls the engine to reverse it. This is a cross-crate change.

**Step 4: Create `toast.rs` — Notification wiring**

```rust
//! Toast notifications for gesture feedback.

use ftui::widgets::{Toast, NotificationQueue};
use ftui::widgets::toast::{ToastPosition, ToastIcon, ToastAction, ToastConfig};

pub struct ToastState {
    pub queue: NotificationQueue,
}

impl ToastState {
    pub fn new() -> Self {
        Self {
            queue: NotificationQueue::new()
                .with_max_visible(3)
                .with_position(ToastPosition::BottomRight),
        }
    }

    pub fn push_success(&mut self, message: &str) {
        self.queue.push(
            Toast::new(message)
                .with_icon(ToastIcon::Success)
                .with_auto_dismiss(std::time::Duration::from_secs(3))
        );
    }

    pub fn push_undo(&mut self, message: &str) {
        self.queue.push(
            Toast::new(message)
                .with_icon(ToastIcon::Info)
                .with_action(ToastAction::new("Redo", "redo"))
                .with_auto_dismiss(std::time::Duration::from_secs(5))
        );
    }

    pub fn push_warning(&mut self, message: &str) {
        self.queue.push(
            Toast::new(message)
                .with_icon(ToastIcon::Warning)
                .with_auto_dismiss(std::time::Duration::from_secs(5))
        );
    }
}
```

**Step 5: Wire into InstrumentApp**

Add fields:
```rust
pub gesture_undo: crate::undo::GestureUndo,
pub toasts: crate::toast::ToastState,
pub command_palette: Option<crate::palette::PaletteState>,
```

**Step 6: Wire undo into gesture handlers**

Every gesture in `update.rs` that mutates state (resolve, release, add, edit, reorder, note) gets wrapped:

```rust
// Before the gesture
let tx = self.gesture_undo.begin("resolve #42");
let old_state = self.capture_state_snapshot();

// Execute the gesture
self.engine.resolve_tension(&id, self.session_id.as_deref());
self.load_siblings();

// After success
self.gesture_undo.commit_with_snapshot(tx, old_state);
self.toasts.push_success(&format!("Resolved #{}", short_code));
```

**Step 7: Add Ctrl+Z / Ctrl+Shift+Z handling in msg.rs**

In the `Event -> Msg` conversion, the `RawEvent` path already catches Ctrl combos. Add specific matches:

```rust
if key.ctrl() && key.code == KeyCode::Char('z') {
    if key.shift() {
        return Msg::Redo;
    }
    return Msg::Undo;
}
```

In `update_normal`:
```rust
Msg::Undo => {
    if let Some(label) = self.gesture_undo.undo() {
        // Restore state from snapshot
        self.restore_state_snapshot();
        self.toasts.push_undo(&format!("Undone: {}", label));
    }
    Cmd::none()
}
```

**Step 8: Replace overlay rendering with Modal rendering**

In `update.rs view()`, replace the current overlay block (lines 117-134):

```rust
// Current: match on InputMode, render raw overlay
// New: match on InputMode, render Modal overlay
match &self.input_mode {
    InputMode::Adding(step) => {
        crate::modal::render_add_modal(
            &self.text_input, step.label(), &panes.field, frame, &self.styles
        );
    }
    InputMode::Confirming(kind) => {
        crate::modal::render_confirm_modal(
            &kind.question(), &panes.field, frame, &self.styles
        );
    }
    // ... etc
    _ => {}
}

// Toast rendering (always, after everything else)
self.toasts.queue.render(area, frame);
```

### 4. ftui API Specifics

| Type | Import | Constructor | State Location |
|------|--------|-------------|----------------|
| `Modal` | `ftui::widgets::modal::Modal` | `Modal::new(config)` | Stateless — created per frame |
| `ModalConfig` | `ftui::widgets::modal::ModalConfig` | `.with_backdrop_dimming(f32).with_animation(bool)` | Config is constant per modal type |
| `dialog::confirm` | `ftui::widgets::modal::dialog` | `dialog::confirm("question")` → `Modal` | Stateless |
| `CommandPalette` | `ftui::widgets::command_palette::CommandPalette` | `CommandPalette::new(actions).with_scorer(scorer)` | `PaletteState` on InstrumentApp |
| `ActionItem` | `ftui::widgets::command_palette::ActionItem` | `ActionItem::new(id, label).with_shortcut(s)` | Vec inside PaletteState |
| `BayesianScorer` | `ftui::widgets::command_palette::BayesianScorer` | `BayesianScorer::new()` | Mutable, learns from usage |
| `Toast` | `ftui::widgets::Toast` | `Toast::new(msg).with_icon(...).with_auto_dismiss(...)` | Pushed into NotificationQueue |
| `NotificationQueue` | `ftui::widgets::NotificationQueue` | `NotificationQueue::new().with_max_visible(3)` | `ToastState` on InstrumentApp |
| `ToastPosition` | `ftui::widgets::toast::ToastPosition` | `ToastPosition::BottomRight` | Config on NotificationQueue |
| `ToastAction` | `ftui::widgets::toast::ToastAction` | `ToastAction::new("label", "action_id")` | Per-toast |
| `UndoHistory` | `ftui::runtime::undo::UndoHistory` | `UndoHistory::new()` | `GestureUndo` on InstrumentApp |
| `UndoTransaction` | `ftui::runtime::undo::UndoTransaction` | `history.begin("label")` | Temporary, lives during gesture |

### 5. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Modal focus trapping conflicts with TextInput's event handling | Test each modal type individually. The Modal should handle focus trapping; the TextInput inside receives events through the modal's focus scope. |
| Undo at the database level is complex (Engine may not support it) | Phase 3's undo can start as TUI-state-only undo (restore the `siblings` vec and related cache) without database reversal. Full database undo (deleting mutations) is a Phase 5 polish item. |
| BayesianScorer state persistence across sessions | Defer persistence to Phase 5. In Phase 3, the scorer resets each session. Still useful — it learns within a session. |
| CommandPalette rendering conflicts with survey view | The palette is a Modal — it renders on top of whatever view is active. Test in both deck and survey orientations. |
| Toast auto-dismiss requires a tick timer | Add a `Subscription::interval(Duration::from_millis(100))` that checks toast expiry. This is a lightweight tick — not the heavy-duty subscription avoided in the current code. Only active when toasts are visible. |

### 6. What Gets Deleted

After this phase:
- `state.rs`: `TransientMessage` struct and its `new()`, `is_expired()` methods (replaced by Toast)
- `app.rs`: `transient: Option<TransientMessage>` field
- `render.rs`: `render_add_prompt()`, `render_confirm()`, `render_edit_prompt()`, `render_note_prompt()`, `render_pathway()` methods (replaced by modal.rs)
- `update.rs`: The transient message clearing at top of `update()` (lines 20-24)

### 7. Checkpoint Criteria

- [ ] Adding a child shows a centered Modal with dimmed backdrop; the field is visible behind it
- [ ] Editing desire shows a Modal with TextInput pre-filled; Esc cancels without mutation
- [ ] Confirming resolve shows a dialog with Confirm/Cancel; `y` confirms, `n` cancels
- [ ] `Ctrl+Z` after resolving a tension restores the previous state (at minimum, the TUI state — database reversal may be Phase 5)
- [ ] `Ctrl+K` opens the command palette; typing "res" shows "resolve" as top result
- [ ] Selecting a command from the palette executes it (triggers the same path as the direct keybinding)
- [ ] After a gesture, a Toast appears in bottom-right for 3 seconds then fades
- [ ] After undo, a Toast with `[Redo]` action appears; pressing the action triggers redo
- [ ] Multiple rapid gestures stack toasts (max 3 visible)
- [ ] All existing keybindings still work (Modal is additive, not replacing)

---

## Phase 4: Widget Migration (#165)

**Deadline:** 2026-07
**Essence:** Replace hand-rolled list/tree/cursor rendering with ftui widgets. Wire the FocusGraph to navigation. Add VirtualizedList for survey.

### 1. Theory of Closure

Done means:
- Route, held, and accumulated zones in the deck use `ftui::widgets::List` with `ListItem`, selection highlight, and hit testing.
- Survey view uses `ftui::widgets::Tree` with temporal bands as expandable nodes.
- FocusGraph drives j/k navigation (replacing `deck_pitch_up/down` index arithmetic).
- VirtualizedList handles large tension counts in survey (100+ items scroll smoothly).
- The DeckCursor struct is replaced by FocusGraph position queries.
- Reactive state (`Observable<Tension>`, `Computed<Frontier>`) replaces manual reload-after-mutation.

Files created:
- `werk-tui/src/deck_zones.rs` — List widget construction for each frontier zone
- `werk-tui/src/survey_tree.rs` — Tree widget construction for temporal bands

Files modified:
- `werk-tui/src/deck.rs` — zone rendering replaced by List widget calls; old rendering deleted
- `werk-tui/src/survey.rs` — flat rendering replaced by Tree + VirtualizedList
- `werk-tui/src/update.rs` — navigation dispatch through FocusGraph instead of cursor arithmetic
- `werk-tui/src/focus.rs` — expanded from skeleton to full navigation graph with dynamic nodes
- `werk-tui/src/app.rs` — add reactive bindings, remove `deck_cursor`, `survey_cursor`
- `werk-tui/src/msg.rs` — navigation messages route through focus system

Files deleted:
- `werk-tui/src/deck.rs`: `DeckCursor`, `deck_pitch_up()`, `deck_pitch_down()`, all `render_zone_*()` methods
- `werk-tui/src/survey.rs`: manual rendering loop (replaced by Tree widget)

### 2. Dependencies

Requires Phase 3 (modals must exist so that modal focus trapping interacts correctly with the navigation focus graph).

Can overlap with: Phase 3's later stages. List widget migration is independent of modal work.

### 3. Detailed Implementation Steps

**Step 1: Create `deck_zones.rs` — List widgets for frontier zones**

Each frontier zone (route, overdue, next, held, accumulated) becomes a `List` widget:

```rust
//! Frontier zone rendering using ftui List widget.

use ftui::widgets::{List, ListItem};
use ftui::text::{Line, Span};
use ftui::layout::Rect;
use ftui::Frame;

use crate::app::InstrumentApp;
use crate::state::FieldEntry;
use crate::deck::Frontier;

impl InstrumentApp {
    /// Build a List widget for the route zone.
    pub fn build_route_list(&self, frontier: &Frontier) -> List {
        let items: Vec<ListItem> = frontier.route.iter()
            .map(|&idx| {
                let entry = &self.siblings[idx];
                self.entry_to_list_item(entry, false)
            })
            .collect();

        List::new(items)
            .highlight_style(self.styles.selected)
    }

    /// Build a List widget for the overdue zone.
    pub fn build_overdue_list(&self, frontier: &Frontier) -> List {
        let items: Vec<ListItem> = frontier.overdue.iter()
            .map(|&idx| {
                let entry = &self.siblings[idx];
                self.entry_to_list_item(entry, true) // overdue styling
            })
            .collect();

        List::new(items)
            .highlight_style(self.styles.selected)
    }

    /// Convert a FieldEntry to a ListItem with proper column layout.
    fn entry_to_list_item(&self, entry: &FieldEntry, overdue: bool) -> ListItem {
        let mut spans = Vec::new();

        // Left column: deadline
        if let Some(ref label) = entry.horizon_label {
            spans.push(Span::styled(
                format!("{:<6}", label),
                if overdue { self.styles.amber } else { self.styles.dim },
            ));
        } else {
            spans.push(Span::styled("      ", self.styles.dim));
        }

        // Gutter
        spans.push(Span::raw("  "));

        // Main column: desired text
        let text_style = if overdue { self.styles.amber } else { self.styles.text };
        spans.push(Span::styled(&entry.desired, text_style));

        // Right column: ID
        if let Some(sc) = entry.short_code {
            spans.push(Span::styled(
                format!("  #{}", sc),
                self.styles.dim,
            ));
        }

        ListItem::new(Line::from_spans(spans))
    }
}
```

**Step 2: Wire List widgets into deck rendering**

Replace the zone-by-zone rendering loops in `deck.rs` `render_deck()` with:

```rust
// Route zone
if !frontier.route.is_empty() {
    let route_list = self.build_route_list(&frontier);
    // Use Flex to allocate the route zone within the field
    route_list.render(route_rect, frame);
}

// Overdue zone
if !frontier.overdue.is_empty() {
    let overdue_list = self.build_overdue_list(&frontier);
    overdue_list.render(overdue_rect, frame);
}

// ... similar for held, accumulated
```

**Step 3: Create `survey_tree.rs` — Tree widget for temporal bands**

```rust
//! Survey view using ftui Tree widget.

use ftui::widgets::Tree;
use ftui::widgets::tree::TreeNode;
use ftui::widgets::tree::TreeGuides;

use crate::survey::{SurveyItem, TimeBand};
use crate::app::InstrumentApp;

impl InstrumentApp {
    /// Build a Tree widget from survey items grouped by time band.
    pub fn build_survey_tree(&self) -> Tree {
        let mut band_nodes: Vec<TreeNode> = Vec::new();

        // Group items by band
        let bands = [
            TimeBand::Overdue,
            TimeBand::ThisWeek,
            TimeBand::ThisMonth,
            TimeBand::Later,
            TimeBand::NoDeadline,
        ];

        for band in &bands {
            let items: Vec<&SurveyItem> = self.survey_items.iter()
                .filter(|item| item.band == *band)
                .collect();

            if items.is_empty() {
                continue;
            }

            let label = format!("{} ({})", band.label(), items.len());
            let children: Vec<TreeNode> = items.iter()
                .map(|item| {
                    TreeNode::new(&item.display_line)
                })
                .collect();

            let node = TreeNode::new(&label)
                .with_expanded(matches!(band, TimeBand::Overdue | TimeBand::ThisWeek));

            for child in children {
                // node = node.child(child);
                // Exact API depends on TreeNode's builder pattern
            }

            band_nodes.push(node);
        }

        Tree::new(band_nodes)
            .guides(TreeGuides::Rounded)
    }
}
```

**Step 4: Wire FocusGraph to navigation**

Replace `deck_pitch_up()` and `deck_pitch_down()` in `app.rs`:

```rust
pub fn navigate_up(&mut self) {
    self.focus_state.graph.move_focus(NavDirection::Up);
    self.sync_cursor_from_focus();
}

pub fn navigate_down(&mut self) {
    self.focus_state.graph.move_focus(NavDirection::Down);
    self.sync_cursor_from_focus();
}

/// After focus moves, synchronize the old cursor state for backward compatibility.
fn sync_cursor_from_focus(&mut self) {
    // Map FocusId back to cursor position
    // This is the bridge: focus graph drives navigation,
    // but old rendering still reads deck_cursor for highlight position.
    // Once all rendering uses List widget selection, this method is deleted.
}
```

**Step 5: Add dynamic focus nodes**

The static FocusGraph from Phase 2 has 7 zone nodes. Phase 4 expands this: each route item, each held item, each accumulated item gets its own FocusNode, registered dynamically when `load_siblings()` runs.

```rust
impl FocusState {
    /// Rebuild the focus graph for the current frontier.
    pub fn rebuild_for_frontier(&mut self, frontier: &Frontier, siblings: &[FieldEntry]) {
        self.graph.clear();

        // Re-register zone roots
        // Then register individual items within each zone
        for (i, &idx) in frontier.route.iter().enumerate() {
            let node_id = self.graph.register(
                FocusNode::new(&format!("route_{}", i))
            );
            // Connect vertically within the zone
            if i > 0 {
                self.graph.connect(prev_id, node_id, NavDirection::Down);
                self.graph.connect(node_id, prev_id, NavDirection::Up);
            }
        }
        // Similar for overdue, held, accumulated
    }
}
```

**Step 6: Add reactive state**

The ftui reactive system uses `Rc<RefCell<>>` internally — `Observable` is `Clone` (shared state), `Computed` captures clones of its sources (not references). No lifetime or ownership issues with InstrumentApp.

```rust
use ftui::runtime::reactive::{Observable, Computed, BatchScope};

// In InstrumentApp:
pub parent_tension_obs: Observable<Option<Tension>>,
pub siblings_obs: Observable<Vec<FieldEntry>>,
pub frontier: Computed<Frontier>,
```

Construction in `InstrumentApp::new()`:
```rust
let parent_tension_obs = Observable::new(None);
let siblings_obs = Observable::new(Vec::new());

// Computed captures clones of the observables — no borrowing issues
let frontier = Computed::from2(
    &parent_tension_obs,
    &siblings_obs,
    |parent, siblings| Frontier::classify(parent.as_ref(), siblings),
);
```

After any gesture, update the observables instead of calling `load_siblings()` directly:
```rust
// Old pattern:
self.load_siblings();

// New pattern (BatchScope groups updates into one notification):
{
    let _batch = BatchScope::new();
    self.parent_tension_obs.set(new_parent);
    self.siblings_obs.set(new_siblings);
}
// frontier.get() now automatically returns the recomputed frontier
```

The `Computed` automatically marks itself dirty when either observable changes. `frontier.get()` recomputes lazily on next access. This eliminates the `cached_frontier` field and all manual cache invalidation.

**Step 7: VirtualizedList for survey**

```rust
use ftui::widgets::VirtualizedList;
use ftui::widgets::virtualized_list::ItemHeight;

impl InstrumentApp {
    pub fn build_survey_virtualized(&self) -> VirtualizedList {
        VirtualizedList::new(self.survey_items.len())
            .with_item_height(ItemHeight::Fixed(1)) // most items are 1 line
            .with_overscan(5) // render 5 extra items above/below viewport
    }
}
```

For the survey view with 500+ items, VirtualizedList renders only the visible ~40 rows plus overscan buffer. The `ItemHeight::Fixed(1)` works for the common case; items with signals (badges, sparklines) would need `ItemHeight::Variable` — defer that to Phase 5 polish.

### 4. ftui API Specifics

| Type | Import | Constructor | State Location |
|------|--------|-------------|----------------|
| `List` | `ftui::widgets::List` | `List::new(items).highlight_style(style)` | Stateless — built per frame |
| `ListItem` | `ftui::widgets::ListItem` | `ListItem::new(Line)` | Per item |
| `Tree` | `ftui::widgets::Tree` | `Tree::new(nodes).guides(TreeGuides::Rounded)` | Tree expansion state needs tracking |
| `TreeNode` | `ftui::widgets::tree::TreeNode` | `TreeNode::new(label).with_expanded(bool).child(node)` | Built per frame from data |
| `TreeGuides` | `ftui::widgets::tree::TreeGuides` | `TreeGuides::Rounded` | Config constant |
| `VirtualizedList` | `ftui::widgets::VirtualizedList` | `VirtualizedList::new(count).with_item_height(h)` | Scroll offset needs tracking |
| `ItemHeight` | `ftui::widgets::virtualized_list::ItemHeight` | `ItemHeight::Fixed(1)` or `ItemHeight::Variable` | Config |
| `Observable` | `ftui::runtime::reactive::Observable` | `Observable::new(initial_value)` | On InstrumentApp |
| `Computed` | `ftui::runtime::reactive::Computed` | `Computed::new(closure)` (?) | On InstrumentApp |
| `BatchScope` | `ftui::runtime::reactive::BatchScope` | `BatchScope::new()` wraps multiple updates | Temporary, during gesture execution |

### 5. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| List widget's selection model doesn't match DeckCursor's zone-aware navigation | Keep DeckCursor alive during transition. List widget provides the visual rendering; DeckCursor provides the cursor math. Replace DeckCursor only after List proves it can handle the full navigation model. |
| Tree widget's expansion state is lost on re-render | Track `TreeState` on InstrumentApp. The tree expansion (which bands are open) persists across frames. |
| FocusGraph rebuild on every `load_siblings()` is expensive | Profile it. The graph has at most ~100 nodes (realistic tension count). If slow, cache and diff. |
| VirtualizedList with variable-height items is complex | Start with `Fixed(1)`. Only switch to `Variable` if items actually need multiple lines (Phase 5 polish). |
| Reactive `Computed` ownership | RESOLVED: `Computed` uses `Rc` clones internally, fully compatible with struct ownership. No manual recompute needed. |

### 6. What Gets Deleted

After this phase:
- `deck.rs`: `DeckCursor` struct, `DeckCursor::default()`, all `deck_pitch_*` methods
- `deck.rs`: All `render_*_zone()` methods (route, overdue, held, accumulated rendering loops)
- `deck.rs`: `CursorTarget` enum (replaced by FocusGraph queries)
- `survey.rs`: `render_survey()` manual rendering loop
- `app.rs`: `deck_cursor` field, `survey_cursor` field
- `app.rs`: `cached_frontier` field (replaced by reactive Computed or explicit recompute)
- `app.rs`: `last_render_lines` field (List widget handles its own sizing)

Estimated deletion: ~800 lines from deck.rs, ~200 lines from survey.rs, ~50 lines from app.rs.

### 7. Checkpoint Criteria

- [ ] Route items render as a selectable List widget with highlight on the focused item
- [ ] j/k navigation moves focus through the List correctly (same behavior as before, different implementation)
- [ ] Overdue items appear with amber styling in their own List
- [ ] Held items appear with indent in their own List
- [ ] Survey view shows a collapsible Tree with time bands as top-level nodes
- [ ] Expanding a time band in the survey shows the items within it
- [ ] `[` and `]` collapse/expand tree nodes in the survey
- [ ] With 200+ survey items, scrolling is smooth (VirtualizedList handles it)
- [ ] FocusGraph drives navigation: pressing `j` calls `graph.move_focus(Down)` and the UI updates
- [ ] Tab between deck and survey carries selection (the focused tension's ID crosses the transition)
- [ ] `cargo test` passes — no regressions

---

## Phase 5: Polish and Infrastructure (#166)

**Deadline:** 2026-07
**Essence:** Session persistence, dev tools, BayesianScorer persistence, degradation cascade, cleanup of dead code, ProgramSimulator tests.

### 1. Theory of Closure

Done means:
- Workspace snapshots save/restore pane proportions, zoom level, focused tension, and survey expansion state across sessions.
- Inspector overlay (`Ctrl+Shift+I`) shows widget tree and focus graph for debugging.
- DebugOverlay shows constraint visualization (colored borders for Flex regions).
- ProgramSimulator tests verify core flows deterministically.
- BayesianScorer persists to `.werk/palette_scores.json` (or similar).
- Degradation cascade is configured: sparklines drop first, badges simplify, animations disable.
- All dead code from the migration is removed.
- Native TTY backend: ftui-tty is not published; crossterm stays. (Q3 resolved — closed.)

Files created:
- `werk-tui/src/persistence.rs` — WorkspaceSnapshot save/load
- `werk-tui/src/dev.rs` — Inspector/Debug overlay wiring
- `werk-tui/tests/tui_flows.rs` — ProgramSimulator tests

Files modified:
- `werk-tui/src/app.rs` — workspace restore on startup, save on quit
- `werk-tui/src/palette.rs` — scorer persistence
- `werk-tui/src/msg.rs` — add `Msg::InspectorToggle`

Files deleted:
- All remaining dead code identified below.

### 2. Dependencies

Requires Phases 1-4 to be complete. This is the final polish pass.

Can overlap with: Nothing. This is the integration and cleanup phase.

### 3. Detailed Implementation Steps

**Step 1: Workspace persistence**

```rust
//! Session persistence — save/restore workspace state.

use ftui::layout::workspace::{WorkspaceSnapshot, WorkspaceMetadata};
use ftui::runtime::state_persistence::StatePersistenceConfig;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct InstrumentWorkspace {
    pub desire_height: Option<u16>,
    pub reality_height: Option<u16>,
    pub focused_tension_id: Option<String>,
    pub parent_id: Option<String>,
    pub view_orientation: String, // "stream" or "survey"
    pub survey_expanded_bands: Vec<String>,
    pub zoom_level: String,
}

impl InstrumentWorkspace {
    pub fn capture(app: &crate::app::InstrumentApp) -> Self {
        Self {
            desire_height: app.layout.desire_height,
            reality_height: app.layout.reality_height,
            focused_tension_id: app.focused_tension_id(),
            parent_id: app.parent_id.clone(),
            view_orientation: match app.view_orientation {
                crate::state::ViewOrientation::Stream => "stream",
                crate::state::ViewOrientation::Survey => "survey",
            }.to_string(),
            survey_expanded_bands: Vec::new(), // from Tree widget state
            zoom_level: format!("{:?}", app.deck_zoom),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = workspace_path()?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }

    pub fn load() -> Option<Self> {
        let path = workspace_path().ok()?;
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }
}

fn workspace_path() -> Result<std::path::PathBuf, String> {
    let workspace = werk_shared::Workspace::discover().map_err(|e| e.to_string())?;
    Ok(workspace.root().join(".werk").join("tui_workspace.json"))
}
```

On startup in `InstrumentApp::new()`:
```rust
if let Some(ws) = crate::persistence::InstrumentWorkspace::load() {
    app.parent_id = ws.parent_id;
    app.layout.desire_height = ws.desire_height;
    app.layout.reality_height = ws.reality_height;
    // Navigate to the saved tension
    app.load_siblings();
}
```

On quit (in the `Msg::Quit` handler):
```rust
Msg::Quit => {
    crate::persistence::InstrumentWorkspace::capture(self).save().ok();
    Cmd::quit()
}
```

**Step 2: Inspector overlay**

```rust
//! Dev tools — Inspector and DebugOverlay.

use ftui::widgets::inspector::{InspectorOverlay, InspectorState};
use ftui::widgets::debug_overlay::{DebugOverlay, DebugOverlayState};

pub struct DevTools {
    pub inspector: Option<InspectorState>,
    pub debug_overlay: Option<DebugOverlayState>,
}

impl DevTools {
    pub fn new() -> Self {
        Self {
            inspector: None,
            debug_overlay: None,
        }
    }

    pub fn toggle_inspector(&mut self) {
        self.inspector = if self.inspector.is_some() {
            None
        } else {
            Some(InspectorState::new())
        };
    }
}
```

Wire to `Ctrl+Shift+I` in msg.rs. Render after all other content in view().

**Step 3: ProgramSimulator tests**

```rust
//! Deterministic TUI flow tests.

use ftui::runtime::simulator::ProgramSimulator;
use ftui::{Event, KeyCode, Key};

#[test]
fn test_resolve_flow() {
    // Create a test store with known tensions
    let store = create_test_store(5); // 5 children, 2 overdue
    let app = InstrumentApp::new(store, load_entries(&store));
    let mut sim = ProgramSimulator::new(app);

    // Verify initial state
    sim.assert_view(|view| {
        // Console crown should show "2 overdue"
        assert!(view.contains("2 overdue"));
    });

    // Navigate down 3 times to reach first overdue
    sim.inject(Event::Key(Key::new(KeyCode::Char('j'))));
    sim.inject(Event::Key(Key::new(KeyCode::Char('j'))));
    sim.inject(Event::Key(Key::new(KeyCode::Char('j'))));

    // Resolve
    sim.inject(Event::Key(Key::new(KeyCode::Char('r'))));
    // Confirm
    sim.inject(Event::Key(Key::new(KeyCode::Char('y'))));

    // Verify result
    sim.assert_view(|view| {
        assert!(view.contains("1 overdue"));
        // Toast should show "Resolved"
        assert!(view.contains("Resolved"));
    });

    // Undo
    sim.inject(Event::Key(Key::new(KeyCode::Char('z')).with_ctrl()));

    sim.assert_view(|view| {
        assert!(view.contains("2 overdue"));
        assert!(view.contains("Undone"));
    });
}
```

**Step 4: BayesianScorer persistence**

Save scorer state to `.werk/palette_scores.json` on quit. Load on startup. The scorer's internal state (posterior weights per action) should be serializable.

```rust
impl PaletteState {
    pub fn save_scorer(&self) -> Result<(), String> {
        let data = self.scorer.serialize();
        let path = palette_scores_path()?;
        std::fs::write(&path, data).map_err(|e| e.to_string())
    }

    pub fn load_scorer(&mut self) -> Result<(), String> {
        let path = palette_scores_path()?;
        if path.exists() {
            let data = std::fs::read(&path).map_err(|e| e.to_string())?;
            self.scorer = BayesianScorer::deserialize(&data)?;
        }
        Ok(())
    }
}
```

**Step 5: Degradation cascade configuration**

Mark widgets as essential vs. decorative:

```rust
// In deck_zones.rs, when building sparklines:
sparkline.set_essential(false); // drops first under pressure

// In toast.rs:
toast.set_essential(false); // animations disable under pressure

// In List items with badges:
badge.set_essential(true); // text content is structural
```

The degradation cascade (Full -> LimitedEffects -> EssentialOnly) is configured in the `RuntimeDiffConfig`. Phase 1 may have re-enabled Bayesian diff; Phase 5 configures the degradation thresholds.

**Step 6: Dead code cleanup**

Sweep all files for:
- Functions no longer called (the old rendering methods superseded by widgets)
- Types no longer used (DeckCursor, CursorTarget if fully replaced)
- Imports no longer needed
- Comments referencing the old rendering approach

Run `cargo clippy -- -W dead-code` and fix all warnings.

**Step 7: Input macro recording**

```rust
use ftui::runtime::input_macro::{EventRecorder, MacroPlayer};

// Add to InstrumentApp:
pub macro_recorder: Option<EventRecorder>,
```

Recording starts with a debug keybinding. The recorded macro is saved to `.werk/macros/`. Replay is via ProgramSimulator in tests.

### 4. ftui API Specifics

| Type | Import | Usage |
|------|--------|-------|
| `WorkspaceSnapshot` | `ftui::layout::workspace::WorkspaceSnapshot` | Captures layout state. May be used directly or replicated by `InstrumentWorkspace`. |
| `StatePersistenceConfig` | `ftui::runtime::state_persistence::StatePersistenceConfig` | Framework-level persistence. If this handles save/restore automatically, it replaces custom `InstrumentWorkspace`. Investigation needed: does this save arbitrary app state or just widget state? |
| `InspectorOverlay` | `ftui::widgets::inspector::InspectorOverlay` | `InspectorOverlay::new(&inspector_state)` — renders widget tree as overlay |
| `InspectorState` | `ftui::widgets::inspector::InspectorState` | Mutable state tracking which widget is inspected |
| `DebugOverlay` | `ftui::widgets::debug_overlay::DebugOverlay` | Constraint visualization |
| `ProgramSimulator` | `ftui::runtime::simulator::ProgramSimulator` | `ProgramSimulator::new(model)` — runs model deterministically |
| `EventRecorder` | `ftui::runtime::input_macro::EventRecorder` | Records events for replay |
| `MacroPlayer` | `ftui::runtime::input_macro::MacroPlayer` | Replays recorded events |
| `AsciicastRecorder` | `ftui::runtime::asciicast::AsciicastRecorder` | Records terminal output as asciicast |

### 5. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Workspace JSON schema changes between versions | Include a `version: 1` field. On load, check version; if mismatched, discard and use defaults. |
| ProgramSimulator doesn't support the full ftui widget set | Start with simple flow tests (navigate, gesture, verify text). Complex widget state assertions can come later. |
| (ftui-tty question resolved — crossterm stays, no risk here) | N/A |
| Dead code cleanup removes something still needed | Run the full test suite after each deletion batch. Use `git diff` to review every removal. |

### 6. What Gets Deleted

After this phase — the final cleanup:
- `deck.rs`: All remaining hand-rolled rendering code that was superseded by List/Tree widgets in Phase 4. The file should shrink from ~2,066 lines to ~400 lines (Frontier computation + DeckConfig + AccumulatedItem + ZoomLevel).
- `survey.rs`: All remaining hand-rolled rendering code. The file should shrink from ~1,155 lines to ~200 lines (SurveyItem, TimeBand, FieldVitals, data loading).
- `render.rs`: `render_help()` (replaced by HelpSystem), `render_search()` (replaced by CommandPalette), `render_input_hints()` (replaced by HelpSystem hints).
- `state.rs`: `TransientMessage` (if not already deleted in Phase 3).
- `helpers.rs`: Possibly the entire file (17 lines), if `clear_area_styled` is no longer needed because the theme/diff system handles cell defaults properly.
- `session_log.rs`: Evaluate whether the custom telemetry ring buffer is still needed given ProgramSimulator and AsciicastRecorder. If yes, keep. If redundant, remove.

### 7. Checkpoint Criteria

- [ ] Close the TUI, reopen it — cursor is on the same tension, pane proportions are preserved
- [ ] `Ctrl+Shift+I` shows the Inspector overlay with the widget tree
- [ ] `cargo test` in `werk-tui/tests/` runs ProgramSimulator tests that verify resolve, undo, and navigation flows
- [ ] Command palette remembers frequently-used commands across sessions (scorer persists)
- [ ] On a terminal with <80 columns, the TUI degrades gracefully (compact regime)
- [ ] `cargo clippy -- -W dead-code` produces zero warnings in `werk-tui`
- [ ] The TUI codebase is measurably smaller: target ~5,500 lines down from ~7,884 (30% reduction, with more capability)

---

## Appendix A: Module Map After Migration

```
werk-tui/src/
  lib.rs          ~135 lines  (program entry, panic handler)
  app.rs          ~900 lines  (InstrumentApp — reduced from ~1,100)
  update.rs       ~1,200 lines (Model impl — reduced from ~1,550)
  theme.rs        ~120 lines  (AdaptiveColor theme — rewritten from 72)
  layout.rs       ~150 lines  (NEW: PaneLayout, breakpoints)
  focus.rs        ~200 lines  (NEW: FocusGraph, navigation)
  modal.rs        ~200 lines  (NEW: Modal constructors)
  palette.rs      ~100 lines  (NEW: CommandPalette)
  toast.rs        ~80 lines   (NEW: Toast/NotificationQueue)
  undo.rs         ~100 lines  (NEW: Gesture undo)
  persistence.rs  ~80 lines   (NEW: Workspace save/restore)
  dev.rs          ~60 lines   (NEW: Inspector/Debug)
  deck.rs         ~400 lines  (down from 2,066 — frontier computation only)
  deck_zones.rs   ~300 lines  (NEW: List widgets for frontier zones)
  survey.rs       ~200 lines  (down from 1,155 — data loading only)
  survey_tree.rs  ~150 lines  (NEW: Tree widget for survey)
  render.rs       ~200 lines  (down from 548 — shared rendering utilities)
  state.rs        ~140 lines  (mostly unchanged)
  msg.rs          ~120 lines  (expanded with new message variants)
  glyphs.rs       ~130 lines  (unchanged)
  horizon.rs      ~120 lines  (unchanged)
  search.rs       ~100 lines  (reduced — CommandPalette handles search UI)
  session_log.rs  ~155 lines  (unchanged or removed)
  helpers.rs      ~0 lines    (removed or merged into render.rs)
  ---
  Total: ~4,900 lines (down from ~7,884)
```

## Appendix B: Phase Dependency Graph

```
#162 Rendering Foundation (2026-05)
  │
  ▼
#163 Spatial Skeleton (2026-06) ←─── can overlap with #162 tail
  │
  ▼
#164 Interaction Model (2026-06) ←── can overlap with #163 tail
  │
  ▼
#165 Widget Migration (2026-07) ←── can overlap with #164 tail
  │
  ▼
#166 Polish & Infrastructure (2026-07)
```

Phases 2-4 can overlap by ~1 week each. Phase 5 cannot overlap — it requires all prior work to be stable.

## Appendix C: Open Questions — Resolved

All five questions investigated 2026-03-31. Findings:

### Q1: Does `Engine` support `undo_gesture(gesture_id)`?

**No.** Engine has gesture lifecycle (`begin_gesture`, `end_gesture`, `active_gesture`) but no reversal. The mutation model stores `old_value` on every mutation, so reversal is theoretically possible — apply inverse mutations in reverse order.

**Decision:** Phase 3 undo starts as TUI-state-only (snapshot `siblings` vec and related cache before gesture, restore on Ctrl+Z). Database-level `undo_gesture()` is a separate sd-core feature — create a new tension for it, don't block #164.

### Q2+Q5: Does `Computed` work with Rust's ownership model?

**Yes, fully.** The reactive system uses `Rc<RefCell<>>` internally. `Observable` is `Clone` (shared state via `Rc`). `Computed::from_observable(&source, |v| ...)` captures a *clone* of the Observable, not a reference. No lifetime issues.

Confirmed constructor signatures:
- `Computed::from_observable(&Observable<S>, impl Fn(&S) -> T + 'static)` — single dependency
- `Computed::from2(&Observable<S1>, &Observable<S2>, impl Fn(&S1, &S2) -> T + 'static)` — two deps
- `Computed::from3(...)` — three deps
- Dirty tracking is automatic via subscriptions. `get()` recomputes only when dirty.

`InstrumentApp` can own both `Observable<Option<Tension>>` and `Computed<Frontier>` as plain fields. The `Computed` holds its own subscriptions in `_subscriptions: Vec<Subscription>`.

**Decision:** Phase 4 uses real reactive state — `Observable` + `Computed` directly. Not manual recompute. Update Phase 4 implementation steps accordingly.

### Q3: Is `ftui-tty` stable on macOS?

**Not available.** ftui-tty is referenced in ftui-runtime's Cargo.toml behind a `native-backend` feature flag, but the crate is not published to crates.io. The ecosystem uses crossterm via ftui-core.

**Decision:** Closed. Keep crossterm. Remove the ftui-tty evaluation step from Phase 5.

### Q4: Will PaneLayout work for the three-zone spatial model?

**Not the right tool.** PaneLayout is fundamentally ratio-based (`PaneSplitRatio`). There is no absolute-cell-height mode. Default overhead is 2 cells (1 margin + 1 padding) per pane — a 3-line desire anchor would show only 1 line of content. PaneLayout's value is in resizable multi-pane layouts with many equally-sized panes.

The desire/field/reality model needs content-adaptive sizing: `Flex::vertical()` with `Constraint::FitContent` for anchors and `Constraint::Fill` for the field. This gives zero overhead and sizes to content. The `LayoutState` in Phase 2 already uses this approach correctly.

**Decision:** Phase 2 uses `Flex` (not PaneLayout) for the three-zone split. PaneLayout may be useful later for orient zoom's side panel (splitting the field 70/30), but not for the primary spatial skeleton. Remove PaneLayout references from the three-zone design; update tui-reimagined.md to note this refinement.
