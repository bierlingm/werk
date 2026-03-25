# TUI Console V2 — Design Document

**Emerged:** 2026-03-25 through sustained exploration (two independent idea-generation passes, critical evaluation, synthesis)
**Status:** Active design. Phase 1 (surgical fixes) approved for implementation. Phase 2+ pending.
**Scope:** Console zone inside the deck view — the operating envelope's action surface.
**Anchors:** `designs/werk-conceptual-foundation.md`, `designs/tui-shaping.md`, `designs/tui-breadboard.md`, `designs/tui-big-picture.md`, `designs/tui-console-v2-proposal.md`

---

## What This Document Is

This is the design document for the console zone redesign. It synthesizes two independent analyses (a surgical improvement pass and an architectural proposal pass), identifies where they agree, where they disagree, and what neither addressed. It specifies the phased implementation path and records all unresolved design questions.

The console is the conceptual heart of the instrument. Per the foundation:
- R0: "The deck is the primary interaction surface — console at screen center"
- R1.3: "Console at vertical center — the frontier of action as interaction surface"
- R2: "Console contains everything action-relevant in the current epoch"
- R2.6: "Console extent is dynamic — visual size is itself a signal"
- R2.8: "Empty console is a meaningful state — shows affordances"

---

## Current State

The console zone sits between route (above) and accumulated (below) in the deck's middle zone. It currently has:

1. An enriched separator/header: `┄┄ 5/11 · epoch 3d · next Mar 30 ┄┄` (all dim)
2. Overdue items (amber, `·` glyph, same as route items)
3. Next committed step (`·` glyph, visually identical to route items)
4. Held items (indented 2 chars, `·` glyph, dim when compressed)
5. Input line: `▸ ___` (resting) / `▸ a add · n note · ! desire · ? reality` (selected)
6. Accumulated items (bottom-up gravity toward reality)

### What's Good

- Frontier classification is real and working.
- Compression with gradual expansion is real (two-pass algorithm).
- Epoch mechanics work (boundary filtering, fresh epochs).
- Focus zoom (Enter) and peek (Space) are functional.
- Column layout is clean and consistent.

### What's Wrong

1. **The console is not a component.** It's assembled inline in the middle render pass, mixed with route and accumulated logic. Lines 749–1063 of `deck.rs` are a single incremental top-down/bottom-up pass with no separation of planning from rendering.
2. **The header is monochrome prose.** Epoch age, overdue count, closure fraction all render identically in `STYLES.dim`. No per-cell color, no priority ordering, no scanability.
3. **The next step is invisible.** Uses `·` glyph (same as every other item). The primary action vector has no visual distinction. The shaping doc specifies `▸` for active positioned items — this wasn't implemented.
4. **Overdue items don't escalate.** N7/C5 (time-amplified signals) are specified in the breadboard but unimplemented. All overdue items look the same regardless of duration.
5. **Empty console is barren.** Just `▸ ___`. No message, no invitation, no affordance. Violates R2.8.
6. **No internal whitespace.** Items stack pixel-tight with no micro-gaps between conceptual groups.
7. **No visual weight hierarchy.** Everything in the console has the same "loudness." Next step, overdue, held, input — all equivalent weight.
8. **Console doesn't claim vertical center.** Renders wherever route items end, contradicting R0/R1.3.

---

## Design Principles (from synthesis)

### Converged agreements (both analyses)

1. The next step needs visual distinction — it's the steering wheel.
2. Overdue needs stronger, escalating treatment — time-amplified signals.
3. Empty console needs purposeful design — silence as meaning, not absence.
4. The header needs per-cell styling — readouts should scan, not parse.
5. The console needs vertical presence — it should claim space, not take leftovers.
6. Gesture hints should be contextual — relevant controls at the point of action.
7. The action center must survive compression — helm and input never disappear.
8. Accumulated facts stay near reality — gravity toward the ground.

### Key disagreement: chassis borders

**Surgical analysis** rejected boxing: "breaks the vertical flow from desire through action to reality."
**Architectural proposal** made boxing the core thesis: "visual ownership, center gravity."

**Resolution:** Use horizontal rules (Rule widget) as top and bottom boundaries — the crown IS the top border, the footer IS the bottom border. No vertical borders. This gives visual ownership (the proposal's goal) without boxing (the surgical analysis's concern). Items inside use the full column width. The chassis is made of rules, not of boxes.

```
┄─ 5/11 ─ epoch 3d ─ next Mar 30 ─ ⚠ 1 overdue ────────────────────┄
Mar 24  · threshold detection implemented                OVERDUE  46
Mar 30  ▸ refine console shell and helm                          15 →3
          · orient zoom layout study                             18
          · threshold signal polish                              19
        ▸ [a add] [n note] [! desire] [? reality]
┄─ ✓ 2 resolved · ~ 1 released ─ last act 17m ──────────────────────┄
```

### The rendering model problem

The current incremental pass (`my += 1` top-down) cannot support a bounded component that needs to know its total height before drawing. The architectural proposal's `ConsolePlan` approach (compute layout, then render) is necessary for Phase 2, but Phase 1 can ship surgical improvements within the existing pass.

---

## Phase 1: Surgical Fixes

Implementable now, within the existing rendering architecture. No structural refactors required.

### S1. Next step glyph: `▸` instead of `·`

**Change:** One character in `render_deck()` (line ~927 of deck.rs).

```rust
// Before:
self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{00B7}", is_selected, false, 0);
// After:
self.render_child_line(frame, area.x, my, w, &cols, entry, "\u{25B8}", is_selected, false, 0);
```

**Rationale:** The shaping doc glyph table says `▸` = "Forward-pointing, committed" for active positioned items. The next step is the primary action vector. One character, zero risk.

**Confidence:** 97%

### S2. Time-amplified overdue intensity

**Change:** In `render_child_line`, scale overdue styling based on `temporal_urgency`.

```rust
let base_style = if is_selected {
    STYLES.selected
} else if is_overdue {
    if entry.temporal_urgency > 2.0 {
        Style::new().fg(PackedRgba::rgb(230, 190, 60)).bold()
    } else if entry.temporal_urgency > 1.3 {
        Style::new().fg(CLR_AMBER).bold()
    } else {
        STYLES.amber // just crossed deadline
    }
} else if is_done {
    STYLES.dim
} else {
    STYLES.text
};
```

Three tiers: just overdue (amber), moderately overdue (bold amber), severely overdue (bright bold amber).

**Rationale:** Implements N7/C5 from the breadboard. Uses existing `temporal_urgency` field. Signal by exception — days matter.

**Confidence:** 90%

### S3. Color-coded header readout

**Change:** Replace the monochrome header construction (lines 937–992) with per-span colored rendering.

Build readout as `Vec<(String, Style)>` pairs:
- Closure fraction: `STYLES.text` (brighter than dim — key info)
- Epoch age: `STYLES.green` if fresh (< 1h), `STYLES.dim` if normal, `STYLES.amber` if stale (> 7d)
- Next deadline: `STYLES.dim`
- Overdue count: `STYLES.amber`

Render by painting a `Rule` widget first (for the `┄` line), then overlaying colored spans at the center.

**Rationale:** The header is the console's most information-dense line. Color-coding makes it scannable by type rather than requiring parsing as prose. Uses existing palette with semantic meaning.

**Confidence:** 85%

### S4. Empty console state

**Change:** When `frontier.route.is_empty() && frontier.overdue.is_empty() && frontier.next.is_none() && frontier.held.is_empty()`, render a purposeful empty state before the input line.

```
        no steps yet

        ▸ a add first step · n note · ! desire · ? reality
```

When held items exist but no committed next step:

```
        no committed next step

          · survey view designed and implemented               #18
          · threshold mechanics implemented                    #19
          · pathway palettes in TUI                            #58

        ▸ a add · n note · ! desire · ? reality
```

**Rationale:** R2.8 requires meaningful empty state. The held-only case matters for `#15` right now. "No committed next step" is honest structural language.

**Confidence:** 87%

### S5. Breathing line above console header

**Change:** Insert one blank line between the last ordered item (route/overdue/next) and the console header separator. Gated on `middle_lines > 10` to avoid stealing space on short terminals.

```rust
// After next step rendering, before console header:
if my < top_limit && middle_zone.height > 10 {
    my += 1; // breathing line above console
}
```

**Rationale:** The console header is the entrance to the operating envelope. A micro-gap creates a visual threshold. Stripe-level whitespace management.

**Confidence:** 82%

### S6. Context-sensitive hints in status bar

**Change:** In `render_deck_bar`, show gesture hints in the left section based on the current cursor target.

```rust
let hints = match target {
    CursorTarget::Route(_) | CursorTarget::Next(_) => "Enter focus · l descend · r resolve",
    CursorTarget::Overdue(_) => "r resolve · ~ release · l descend",
    CursorTarget::HeldItem(_) => "r resolve · ~ release",
    CursorTarget::InputPoint => "a add · n note · ! desire · ? reality",
    CursorTarget::AccumulatedItem(_) => "l descend · Enter focus",
    _ => "",
};
```

**Rationale:** Progressive disclosure. The user doesn't need to memorize all gestures — relevant ones appear contextually.

**Confidence:** 80%

### S7. OVERDUE tag for overdue items

**Change:** Render an explicit `OVERDUE` text annotation before the right column on overdue items.

```
Mar 24  · threshold detection implemented        OVERDUE  46    5d
```

**Rationale:** The breadboard spec (U10) says `overdue step line (deadline, ▸, text, OVERDUE, ID, →N)`. This is accessible (works without color) and explicit. Costs ~9 characters of text budget.

**Confidence:** 72%

---

## Phase 2: Console as Component

Requires architectural changes to the rendering model. Should be implemented after Phase 1 feels right in the terminal.

### Architecture: ConsolePlan

Extract the console from the monolithic middle render pass into an explicit compute-then-render model:

```rust
pub struct ConsolePlan {
    pub rect: Rect,
    pub crown: CrownPlan,
    pub warning_rows: Vec<WarningRow>,
    pub helm: HelmPlan,
    pub command_well: CommandWellPlan,
    pub held_tray: TrayPlan,
    pub footer: FooterPlan,
}

pub struct ConsoleState {
    pub local: ConsoleLocalState,
    pub band: ConsoleBand,          // persisted for hysteresis
    pub previous_band: ConsoleBand, // for hysteresis comparison
}

pub struct ConsoleMetrics {
    pub inner_width: u16,
    pub crown_height: u16,
    pub footer_height: u16,
    pub minimum_center_height: u16,
}
```

### Console Anatomy

1. **Crown:** A Rule-based top boundary with telemetry chips. Priority-ordered: overdue count > next deadline > closure > epoch age > held count > last act. Chips drop off right-to-left under width pressure.

2. **Warning Lane:** Overdue items, amber-styled, capped at 2 visible rows. Overflow collapses into a crown chip (`+N overdue`).

3. **Helm Row:** The next committed step as the visual center. If no next step exists, a structural prompt: `no committed next step` (with held items below) or `nothing action-relevant` (empty console).

4. **Command Well:** Two rows when space allows: prompt/typing row + contextual command row. Collapses to one row under compression. Commands adapt to cursor target.

5. **Held Tray:** Held items below the helm, HELD_INDENT preserved. Show up to 2 individually, collapse rest to chip.

6. **Footer:** A Rule-based bottom boundary with trace summary. Contains accumulated counts, prior event count, last act age. Replaces current bottom-gravity accumulated rendering.

### Render Split

Refactor `render_deck()` so the middle zone becomes three separate passes:

```rust
let console_plan = compute_console_plan(&frontier, &self.console_state, ...);
let route_rect = Rect { ... };
let console_rect = console_plan.rect;

self.render_route(frame, route_rect, &frontier, &cols, ...);
self.render_console(frame, &console_plan, &cols, ...);
// accumulated items now inside console footer — no separate pass
```

### Dynamic Extent

Console height computed from load score, with banded thresholds and hysteresis:

```rust
enum ConsoleBand { Idle, Light, Loaded, Pressure }

fn compute_load(frontier: &Frontier, has_dock: bool) -> ConsoleBand {
    let score = frontier.overdue.len().min(3) * 3
        + if frontier.next.is_some() { 3 } else { 0 }
        + frontier.held.len().min(3)
        + frontier.accumulated.len().min(2)
        + if has_dock { 2 } else { 0 };
    match score {
        0..=1 => ConsoleBand::Idle,
        2..=4 => ConsoleBand::Light,
        5..=8 => ConsoleBand::Loaded,
        _ => ConsoleBand::Pressure,
    }
}

fn target_height(band: ConsoleBand) -> u16 {
    match band {
        ConsoleBand::Idle => 5,
        ConsoleBand::Light => 7,
        ConsoleBand::Loaded => 9,
        ConsoleBand::Pressure => 11,
    }
}
```

Hysteresis: persist `band` on `ConsoleState`. Don't shrink until load drops by one full band.

### Sticky Action Center

Under compression, never compress away:
- Helm target row (1 line)
- One command row (1 line)
- One footer telemetry row (1 line)

Everything else compresses around them. Compression priority:
1. Individual accumulated items → footer chips
2. Individual held items → tray summary chip
3. Route items → crown chip
4. Warning lane → crown chip
5. Helm/command/footer: preserved

### Chip-First Compression

When content compresses, collapse into telemetry chips before falling back to generic summary lines. A chip is a compact inline label:

Visual format: `keyword value` rendered as a styled span (no brackets in production — brackets are documentation notation only). Chips use `STYLES.dim` by default, `STYLES.amber` for warnings, `STYLES.green` for positive signals.

---

## Phase 3: Interaction Refinements

Requires state model changes. Should follow Phase 2.

### Split Peek from Focus

Current: both Enter and Space use `ZoomLevel::Focus` machinery.
Target: Enter = full focus zoom (takes over middle zone), Space = dock peek (local to console).

```rust
enum ConsoleLocalState {
    Rest,
    DockPeek { sibling_index: usize },
    HelmInput,
}

// ZoomLevel unchanged — Focus and Orient remain deck-level states
// ConsoleLocalState is console-level — coexists with Normal zoom
```

### Hierarchy Dock

Space on a child opens a compact dock inside the console showing up to 3 children and (if room) a one-line reality stub. Falls back to `→N children` chip under height pressure.

### Contextual Command Chips

Command well content adapts to cursor target:

```
On next step:    Enter focus · Space peek · e edit · r resolve
On held item:    Enter focus · Space peek · e edit · m position
On input point:  a add · n note · ! desire · ? reality
On accumulated:  l descend · Enter focus
```

---

## Unresolved Design Questions

### Q-C1: Cursor navigation within the chassis

When the console becomes a bounded component, which elements are selectable?

| Element | Selectable? | Rationale |
|---------|------------|-----------|
| Crown | No | Telemetry readout, not actionable |
| Warning lane items | Yes | Overdue items need gesture access |
| Helm row | Yes | The primary action target |
| Command well | No | Commands are activated by key, not by cursor |
| Held tray items | Yes | Need gesture access (resolve, position) |
| Footer | No | Trace summary, not actionable |

**Status:** Proposed. Pending implementation experience.

### Q-C2: Focus zoom vs. chassis

When Enter activates focus zoom on an item inside the console, what happens to the chassis?

Options:
- **A.** Focus dissolves the chassis — detail takes over the middle zone (current behavior, extended)
- **B.** Focus expands within the chassis — chassis grows to accommodate
- **C.** Focus replaces the console body — crown and footer persist, body becomes focus detail

**Recommendation:** Option A. Focus zoom is a deck-level state that replaces the normal middle zone entirely. The chassis is a normal-zoom feature. This is consistent with the focal-length principle: focus sharpens the center, blurs everything else.

**Status:** Proposed. Needs validation.

### Q-C3: Compression degradation path

At what height thresholds does the console degrade?

| Available height | Console rendering |
|-----------------|-------------------|
| ≥ 11 | Full: crown + warning + helm + command well (2 rows) + held tray + footer |
| 8–10 | Compact: crown + helm + command well (1 row) + held chip + footer |
| 5–7 | Minimal: crown-as-header + helm + command (1 row) |
| 3–4 | Emergency: helm + command (1 row) |
| < 3 | Inline: just the input line (current behavior) |

**Status:** Proposed. Thresholds need tuning with real data.

### Q-C4: Chip visual format

How do chips render in the terminal?

Options:
- **A.** Plain styled text: `a add` in dim, `⚠ 2 overdue` in amber
- **B.** Bracketed: `[a add]` `[⚠ 2 overdue]`
- **C.** Badge widget from ftui (`badge.rs` exists in ftui-widgets)

**Recommendation:** Option A for telemetry chips (crown/footer), Option B or C for command chips (command well). Telemetry should be quiet; commands should look interactive.

**Status:** Open. Needs visual experimentation.

### Q-C5: Route and console boundary

When route items exist, should there always be a breathing line between the last route item and the console crown? Or should the crown rule directly follow the last route item?

**Recommendation:** Breathing line when height > 10, direct contact when tight. This is Phase 1 fix S5.

**Status:** Implemented in Phase 1.

### Q-C6: Trajectory mode interaction

When Shift+T enables trajectory mode (resolved steps stay on route), the console's accumulated zone shrinks. Does the console chassis shrink correspondingly?

**Recommendation:** Yes. Dynamic extent recalculates when trajectory mode toggles. The console reflects the frontier, which changes in trajectory mode.

**Status:** Deferred to Phase 2 (dynamic extent implementation).

### Q-C7: Vertical borders

Should the console have vertical borders (│) on left and right?

**Decision:** No. Vertical borders cost 2 columns of text width and create a visual discontinuity with route items (which are borderless). The chassis is defined by horizontal rules only — crown rule on top, footer rule on bottom. This preserves the deck's vertical flow while giving the console visual ownership.

**Status:** Decided.

---

## Implementation Order

### Phase 1 (now)

1. **S1** — Next step `▸` glyph (1 line change)
2. **S2** — Time-amplified overdue intensity (~15 lines)
3. **S3** — Color-coded header readout (~30 lines)
4. **S4** — Empty/held-only console states (~25 lines)
5. **S5** — Breathing line above header (~3 lines)
6. **S6** — Context-sensitive status bar hints (~20 lines)
7. **S7** — OVERDUE tag on overdue items (~15 lines)

### Phase 2 (after Phase 1 is validated)

1. Extract `ConsolePlan` and `render_console()` without changing visible behavior
2. Replace header + input with crown + helm + footer scaffolding
3. Add dynamic extent and sticky action-center rules
4. Add warning lane, held tray, and trace footer
5. Add command well (2-row / 1-row adaptive)

### Phase 3 (after Phase 2 is validated)

1. Split Space peek from Enter focus in state model
2. Add hierarchy dock
3. Add contextual command chips
4. Add chip-first compression

---

## Mockups

### Current state (for reference)

```
  Jun    werk is a mature tool for practicing structural dynamics   02  1d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  May    · data model extended — gesture grouping                  20 → 2d
  May    · complete state machine specification                    16    3d
  ┄┄┄┄┄┄┄ 6/11 · epoch 3d · next Mar 30 ┄┄┄┄┄┄┄
  Mar 30  · finalize the console redesign                         15 →  2d
            · 2 held
            ▸ ___
            ✓ 2 resolved
  a clear structural model exists — multi-participant, shared · 3d
  ──────────────────────────────────────────────────────────────────────
```

### After Phase 1

```
  Jun    werk is a mature tool for practicing structural dynamics   02  1d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  May    · data model extended — gesture grouping                  20 → 2d
  May    · complete state machine specification                    16    3d

  ┄─ 6/11 ─ epoch 3d ─ next Mar 30 ─────────────────────────────────┄
  Mar 30  ▸ finalize the console redesign                         15 →  2d
            · 2 held
            ▸ a add · n note · ! desire · ? reality
            ✓ 2 resolved
  a clear structural model exists — multi-participant, shared · 3d
  ──────────────────────────────────────────────────────────────────────
  r resolve · l descend · Enter focus                        ? help
```

Key changes visible:
- `▸` on next step (S1)
- Color-coded header: `6/11` brighter, `epoch 3d` dim, no overdue so no amber (S3)
- Breathing line above header (S5)
- Status bar shows context hints for selected item (S6)

### After Phase 1 — with overdue

```
  ┄─ 5/11 ─ epoch 3d ─ next Mar 30 ─ ⚠ 1 overdue ──────────────────┄
  Mar 24  · threshold detection implemented      OVERDUE  46    5d
  Mar 30  ▸ finalize the console redesign                 15 →  2d
            · 2 held
            ▸ a add · n note · ! desire · ? reality
```

Key changes: `OVERDUE` tag (S7), amber `⚠ 1 overdue` in header (S3), overdue intensity based on duration (S2).

### After Phase 1 — held-only state (like #15)

```
  ┄─ 0/3 ─ fresh ──────────────────────────────────────────────────┄

            no committed next step

            · survey view designed and implemented             #18
            · threshold mechanics implemented                  #19
            · pathway palettes in TUI                          #58

            ▸ a add · n note · ! desire · ? reality
```

Key changes: `fresh` in green (S3), "no committed next step" message (S4).

### After Phase 2 (target)

```
  Jun    werk is a mature tool for practicing structural dynamics   02  1d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  May    · data model extended — gesture grouping                  20 → 2d
  May    · complete state machine specification                    16    3d

  ┄─ 5/11 ─ epoch 3d ─ ⚠ 1 overdue ─ next Mar 30 ─ 2 held ─────────┄
  Mar 24  · threshold detection implemented          OVERDUE  46    5d
  Mar 30  ▸ refine console shell and helm                     15 →3 2d
            ▸ [a add] [n note] [! desire] [? reality]
            · orient zoom layout study                        18
            · threshold signal polish                         19
  ┄─ ✓ 2 resolved · ~ 1 released ─ last act 17m ────────────────────┄

  a clear structural model exists — multi-participant, shared · 3d
  ──────────────────────────────────────────────────────────────────────
```

Key changes: crown and footer rules as chassis, helm row prominent, command well row, held tray distinct, trace footer with accumulated.
