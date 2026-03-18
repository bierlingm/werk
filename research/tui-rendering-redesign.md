# TUI Rendering Redesign

## Premise

The Operative Instrument supports continuous directed action within structural tension. Every visual element should serve that purpose: orienting the practitioner in their structural landscape, communicating temporal pressure, and making the next action obvious. Analytical information (phase, tendency, magnitude) belongs in CLI output (`werk context`), not in the action-oriented TUI.

The rendering layer moves from manual `Span`/`Line`/`Paragraph` construction to ftui's proper widget primitives: `Panel` for cards, `Columns` for side-by-side layout.

## The Temporal Indicator

Replaces the backward-looking activity trail with a forward-looking temporal position indicator. Six dots.

### With Horizon

Six dots representing the temporal window from **last reality update** to **horizon end**. Two markers:

- `◦` — where "now" falls in the window (open, moving)
- `●` — where the horizon end falls (solid, the fixed target)

```
◌◌◦◌●◌  — now is early, horizon marker ahead (breathing room)
◌◌◌◦●◌  — approaching the horizon (pressure building)
◌◌◌◌◦◌  — past the horizon marker (overdue)
```

Color shift: early positions in cyan, later positions in amber, past-horizon in red.

### Without Horizon

Staleness indicator — time since last reality update:

```
◌◌◌◌◌◎  — checked this week (bright, present)
◌◌◌◎◌◌  — 3 weeks ago (dimming)
◎◌◌◌◌◌  — 6+ weeks (faded, stale)
```

## The Minimal Tension Line

```
  ◇ desire text truncated if necessary            Mar ◌◌◦◌●◌
```

Components:
- **Glyph** — color encodes tendency (cyan=advancing, white=stagnant, amber=oscillating)
- **Desire text** — truncated when resting, fully word-wrapped when focused
- **Horizon label** — compact, right-aligned. Adapts: `2026`, `Mar`, `Mar 26`, `Mar 20`
- **Temporal indicator** — six dots

When focused (cursor on it), desire word-wraps to show full text.

## The Descended View

When you descend (`l`) into a tension, you're inside its structural tension chart. A vertical trunk line connects positioned children from reality to desire, with glyphs as nodes directly on the trunk:

```
  desire text                                Mar ◌◌◦◌●◌ · 3w ago
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  │
  ◇ final step, closest to desire                 Mar ◌◌◦◌●◌
  │
  ◆ middle step                                        ◌◌◌◌●◌
  │
  ◇ first step from reality                            ◌◌◌◌◌◎
  · · · · · · · · · · · · · · · ·
    ◇ unpositioned tension                         Jun ◌●◌◌◌◌

  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  reality text                                        yesterday

  1  ⚠ neglected 3 weeks — update reality
  2  ⚠ horizon past 5 days — extend or resolve
```

### Trunk and Glyphs

The `│` between children is the structural path — the bridge from reality to desire. Each positioned child's glyph sits directly on the trunk as a node. No branch, no separator between glyph and text — just a space. The trunk runs from the heavy rule (desire) to the light rule (reality).

Unpositioned children sit off the trunk, indented, without connection. They're not yet part of the chosen path.

The trunk only appears in the descended view. At root level or in sibling lists, there's no overarching directionality (no desire to connect to).

### Descended Header

Desire fully word-wrapped, left-aligned. Right side shows:
- Horizon label (compact)
- Temporal indicator (six dots — the parent tension's own action window)
- Temporal distance since desire was last articulated (dim, e.g. `· 3w ago`)

Format: `Mar ◌◌◦◌●◌ · 3w ago`

Separator: heavy rule (━) — firm, anchored. The vision is your fixed point.

### Descended Footer

Reality fully word-wrapped, left-aligned. Right side shows:
- Temporal distance since reality was last checked (dim, e.g. `yesterday`)

Separator: light rule (┄) — fluid, shifting. Reality changes as you act.

### Descended States

1. **Default** — the structural tension chart as shown above
2. **Child expanded** (Space on a child) — gaze opens inline: that child's children + reality. A peek without descending.
3. **Editing** (e) — inline editing in a Panel card

## Alerts

Alerts are **stateless computations**, not mutation events. They're derived from the current state of the tension (mutation history + current time). When action resolves the condition, the alert disappears — it was never a stored thing, just a lens on state.

### What Appears as Alerts

Only actionable signals:
- `neglected 3 weeks` — no reality check in 3 weeks
- `oscillating` — desire/reality swinging back and forth
- `conflict: competing tensions` — structural conflict detected
- `horizon past (5d)` — temporal window has elapsed
- `multiple root tensions` — no senior organizing principle (root level only)

### Where Alerts Appear

Below reality in the descended view. Below the structural ground. When no alerts exist, nothing appears there.

Each alert shows its recommended action with a number key for direct action:

```
  1  ⚠ neglected 3 weeks — update reality
  2  ⚠ horizon past 5 days — extend or resolve
```

Pressing `1` acts on alert 1 (e.g., opens reality for editing). Pressing `2` opens options for alert 2. Numbers don't conflict with child positioning since children use the trunk line instead of ordinals.

### Root Level Alert

When multiple root tensions exist, a permanent alert fires:

```
  1  ⚠ multiple root tensions — no senior organizing principle
     → create a parent for all / move inside another / acknowledge
```

The alert reappears whenever root count changes. The tool observes the structural problem and prompts resolution without enforcing it.

## Creating Tensions

`a` opens a new empty card inline **at the cursor position**. The currently selected tension and everything below it shifts down (toward reality). The new tension takes the cursor's spot in the sequence.

The card has the same structure as editing: desire → reality → horizon via Tab. First field focused for immediate typing.

## Editing

`e` opens a Panel card with the desire text becoming an editable field. Tab cycles: desire → reality → horizon. Enter confirms. Escape cancels.

The `Input` widget from ftui handles cursor movement and text editing with standard macOS shortcuts (CMD+DEL, OPT+DEL, CMD+Arrow, OPT+Arrow).

## Interaction Model

| Key | Action |
|-----|--------|
| j/k | Move cursor (closes any gaze) |
| Space | Toggle gaze on focused tension (children + reality peek) |
| l | Descend into tension (closes gaze) |
| h | Ascend to parent (closes gaze) |
| e | Edit focused tension (opens Panel card in edit mode) |
| a | Add new tension at cursor position |
| Shift+K | Move tension toward desire (up) |
| Shift+J | Move tension toward reality (down) |
| r | Resolve focused tension |
| x | Release focused tension |
| f | Cycle filter |
| i | Focus alerts (in descended view) |
| 1-9 | Act on numbered alert |
| ? | Help (with glyph/color legend) |

### Navigation: Strict Containment (Model A)

One level visible at a time. Gaze is a peek. To work with children, descend. This aligns with Fritz's principle of holding one structural tension at a time.

## Glyph and Color System

### Glyph Shape = Status

Active tensions show phase progression:
- ◇ — Germination
- ◆ — Assimilation
- ◈ — Completion
- ◉ — Momentum

Non-active:
- ✦ — Resolved
- · — Released

### Glyph Color = Tendency

- **Cyan** — advancing
- **White/default** — stagnant
- **Amber** — oscillating

### Temporal Indicator in Help Legend

```
  ◌◌◦◌●◌  ◦ = now  ● = horizon end
  ◌◌◌◌◌◎  staleness (no horizon)
```

## Separators as Meaning

- **━** (heavy) below desire — firm, anchored
- **┄** (light) above reality — fluid, shifting
- **· · ·** between positioned/unpositioned — boundary of deliberate choice

## ftui Widgets

| Widget | Used For |
|--------|----------|
| `Panel` | Expanded card, editing surface |
| `Columns` | Side-by-side layout where needed |
| `Paragraph` | Text content |
| `Input` | Inline editing |
| `StatusLine` | Footer bar |

## Implementation Architecture

### From Monolithic Lines to Rect-Slicing

Replace the single `Vec<Line>` / `Paragraph` approach with per-element `Rect` assignment. Non-card elements render as `Paragraph` into sub-rects. Card elements render as `Panel`.

### Implementation Order

1. ~~Temporal indicator~~ (done)
2. ~~Tension line with horizon label + temporal indicator~~ (done)
3. ~~Glyph color by tendency~~ (done)
4. Descended view with trunk line and temporal annotations
5. Alerts below reality in descended view
6. Rect-slicing render_field
7. Panel-based gaze/expanded card
8. Inline editing with Input widget
9. Root-level multi-tension alert
10. Help screen legend
