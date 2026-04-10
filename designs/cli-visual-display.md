# CLI Visual Display: The Instrument Surface

The CLI is the primary reading surface for practitioners who live in the terminal. When you run `werk tree` or `werk show`, you are looking *through* the display at the structural dynamics of your tension field. The display is not chrome around data — it IS the instrument. Its visual language must be as precise as musical notation: every glyph, color, and spatial relationship carries meaning.

This document establishes the complete visual language for werk's CLI output, drawing on learnings from GitButler's CLI (the current best-in-class terminal tool) while going far beyond it — because werk's domain (structural dynamics across time) is richer than version control.

## Design Principles

### 1. Desired above actual — visually

The sacred core says identity is the desired state. In every display context, the desire is the first and most prominent text. Reality, metadata, signals — all secondary. When your eye hits a tension, it reads the aspiration first.

### 2. Signal by exception — visually

A healthy tension should be quiet. No color, no glyphs, no decoration. Signals appear only when something demands attention. The absence of visual noise IS the signal of health. A field where everything is colored is a field where nothing stands out.

### 3. Structure determines behavior — visually

The tree rendering IS the architecture of space. Depth, branching, containment — these aren't metadata labels, they're the spatial relationships themselves, drawn with line characters that map to the conceptual foundation's spatial model.

### 4. Information hierarchy through visual weight

Not everything deserves the same visual intensity. The hierarchy:

1. **Identity** (desire text) — default weight, full brightness
2. **Signals** (exceptions) — colored, drawing the eye
3. **Temporal facts** (deadline, urgency) — dimmed unless exceptional
4. **Chrome** (connectors, labels, separators) — lightest weight, never competing

### 5. Breathing room is structural

Empty lines, zone boundaries, and whitespace are not waste. They create visual "paragraphs" that let the eye rest and group related information. A wall of text is harder to read than text with air around it.

## Color Palette

Seven semantic roles. No decorative color. Every color means something.

| Role | Color | ANSI | When |
|------|-------|------|------|
| **Identity** | default (white/fg) | — | desire text, tension IDs |
| **Chrome** | dim gray | `\x1b[2m` | connectors, labels, metadata, brackets |
| **Signal: danger** | red | `\x1b[31m` | overdue, critical path, containment violation |
| **Signal: warning** | yellow | `\x1b[33m` | approaching deadline, sequencing pressure, drift |
| **Signal: structure** | cyan | `\x1b[36m` | spine, hub, reach — structural observations |
| **Resolved** | green | `\x1b[32m` | resolved status, closure progress, healthy momentum |
| **Testimony** | magenta | `\x1b[35m` | notes, user-authored content in activity log |

**Rules:**
- A tension with no signals gets zero color. Just default text and dim chrome.
- Color is earned — it means "look here."
- Bold is reserved for IDs (`#42`) and danger signals. Bold is emphasis, not decoration.
- `NO_COLOR` environment variable disables all ANSI. Glyphs alone must be sufficient.
- Every command gets color, not just `tree`. The palette is universal.

## Glyph Registry

One canonical location for every glyph. No glyph is defined in command files — they all come from the registry.

### Status glyphs
| Glyph | Unicode | Meaning |
|-------|---------|---------|
| `▸` | U+25B8 | positioned (followed by position number) |
| `✓` | U+2713 | resolved |
| `~` | U+007E | released |
| `⏸` | U+23F8 | held/unpositioned |

### Signal glyphs
| Glyph | Unicode | Severity | Meaning |
|-------|---------|----------|---------|
| `!` | U+0021 | danger | overdue |
| `‡` | U+2021 | danger | critical path |
| `↥` | U+21A5 | danger | containment violation |
| `⇅` | U+21C5 | warning | sequencing pressure |
| `↝` | U+219D | warning | drift |
| `┃` | U+2503 | structure | spine (longest path) |
| `◉` | U+25C9 | structure | hub (high centrality) |
| `◎` | U+25CE | structure | reach (many descendants) |

### Tree glyphs
| Glyph | Unicode | Meaning |
|-------|---------|---------|
| `├─` | U+251C U+2500 | child connector (non-last) |
| `└─` | U+2514 U+2500 | child connector (last) |
| `│` | U+2502 | vertical continuation |
| `╭─` | U+256D U+2500 | zone opener (root tension boundary) |
| `╰─` | U+2570 U+2500 | zone closer (root tension boundary) |
| `┊` | U+250A | zone continuation (inside a root tension's subtree) |
| `…` | U+2026 | truncation |
| `·` | U+00B7 | separator between inline metadata items |

### Closure glyphs
| Pattern | Meaning |
|---------|---------|
| `[3/7]` | 3 resolved of 7 active+resolved children |
| `✓5` | 5 resolved children |
| `~3` | 3 released children |

## Tree Rendering

The tree is the flagship display. It is the first thing a practitioner sees.

### Current state (v1.5)

```
├── #2 ▸1 [2026-06] werk is a mature tool for practicing structural dynamics — individually a…  [14/23] (5 released)  ┃◎
│   ├── #154 ▸7 [2026-07] EXCEEDS_PARENT Generic user-configurable typed edges — practitioners define their own rela…  ‡
│   ├── #4 ▸1 a clear structural model exists for how multiple participants hold and evolve shared tension fields  [0/2]
```

**What's wrong:**
- Everything on one line — position, deadline, desire, closure, signals all compete for attention
- No visual breathing room between root-level subtrees
- Signals crammed at line end, often past terminal width
- Deadline in brackets looks like closure brackets — visual collision
- `EXCEEDS_PARENT` as text label wastes horizontal space
- No color on most commands

### Vision (v2.0)

```
╭─ #2 ▸1  werk is a mature tool for practicing structural dynamics
│  2026-06 · 82d · 16%                                [14/23] ✓5 ~5  ┃◎
│
│  ├─ #154 ▸7  Generic user-configurable typed edges
│  │  ‡ critical  ↥ exceeds parent by 31d                     2026-07
│  │
│  ├─ #4 ▸1  a clear structural model for cooperative dynamics
│  │  ├─ #5  Foundations course completed                   2026-05-01
│  │  └─ #34  a public web surface exists                  [0/0] ~1
│  │
│  ├─ #90  root level is a command center                  [1/6] ~3  ◉◎
│  │  ├─ #175  TUI logbase view                              [0/1]
│  │  │  └─ #179  logbase in-view filtering
│  │  ├─ #203  Vitals zone
│  │  ├─ #204  Attention zone
│  │  ├─ #205  Recent zone
│  │  └─ #206  zone navigation
│  │
│  ├─ #208  werk CLI absorbs GitButler UX patterns           [0/7]  ◉◎
│  │  ├─ #209  --show-after on mutations
│  │  ├─ #210  session focus
│  │  ...
│  │
│  └─ #207  activity-derived signals
│
╰─ 35 active · 108 resolved · 56 released
```

### What changed:

**Zone boundaries.** Root-level tensions get `╭─`/`╰─` openers and closers. This creates a visual container — your eye knows where one root's subtree starts and ends. An empty line after the root's metadata separates identity from children.

**Two-line root tensions.** Line 1: ID + position + desire. Line 2: temporal facts + closure + signals. The desire is never truncated to make room for metadata. Metadata goes below.

**Desire-first.** The desire text immediately follows the ID. No `[2026-06]` in the middle of the line competing for first-read attention.

**Dimmed metadata.** The second line (deadline, days remaining, urgency percentage, closure counts) renders in dim gray. It's readable but doesn't shout.

**Signal lines.** When a tension has danger or warning signals, they get their own line below the tension, indented. Red for danger, yellow for warning. This means signals are never lost off the right edge of the terminal.

**Breathing room.** An empty line between depth-1 subtrees inside a root zone. The tree is still compact at depth 2+, but the major branches have air between them.

**Consistent tree characters.** `├─` and `└─` with single-dash (not double `──`). The vertical continuation `│` is dimmed. Inside a root zone, the vertical line could optionally use `┊` (dotted) at depth 0 to distinguish "zone continuation" from "tree edge."

**Footer summary.** The field total at the bottom, dimmed, with a `·` separator between categories.

### Depth-sensitive rendering

Not every depth level gets the same treatment:

- **Depth 0 (roots):** Zone boundaries, two-line layout, breathing room
- **Depth 1 (major branches):** Single-line, empty line between siblings with children
- **Depth 2+ (leaves):** Dense single-line, no extra whitespace

This mirrors how practitioners actually read: the top level is strategic (needs breathing room), the bottom is tactical (density is fine).

### Compact mode (`--compact` or terminal < 80 cols)

Falls back to single-line rendering without zone boundaries. Useful for narrow terminals and scripting. The current v1.5 rendering is essentially this mode.

## Show Rendering

`werk show` is the detail view. It's already well-structured. Improvements:

### Section headers get color

```
Tension #2                              ← bold
  Desired:  werk is a mature tool...    ← default weight
  Reality:  v1.5+dev: Source ahead...   ← dimmed slightly

  Status:   Active          Created: 2 weeks ago
  Deadline: 2026-06 (Jun 2026, 82 days remaining)
  Position: 1 (positioned)    Last act: 3 days ago (actual)

Signals:                                ← bold section header
  ‡ CRITICAL   #154 matches...          ← red
  ↥ VIOLATION  #154 deadline exceeds... ← red
  ┃ SPINE      on longest structural... ← cyan
  ◎ REACH      131 transitive...        ← cyan

Frontier:                               ← bold section header
  Next:     #4 a clear structural...
  Held:     6 unpositioned
  Recent:   3 resolved since last epoch ← green

Children:                               ← bold section header
  #4 a clear structural model...
  #154 Generic user-configurable...     ← red if signals
  #36 werk has a sustainable...

Activity:                               ← bold section header
    3 days ago  reality updated
    3 days ago  note: Field audit...    ← magenta for notes
    1 week ago  reality updated
```

### Color for status fields

- Overdue deadline: red
- Approaching deadline (< 14d): yellow
- Resolved status: green
- Urgency > 80%: yellow; > 100%: red
- Activity entries with notes: magenta

### Mutation echo

After any mutation, `show` output for the affected tension prints automatically. The practitioner never needs a second command to see what changed. This is the `--show-after` behavior from GitButler, but as the default.

## List Rendering

`werk list` is the query surface. It needs the most work.

### Column layout with alignment

```
#2   ▸1  werk is a mature tool for...          2026-06   16%  ┃◎
#4   ▸1  a clear structural model for...                      
#154 ▸7  Generic user-configurable typed...    2026-07   22%  ‡↥
#90      root level is a command center        2026-05   45%  ◉◎
#36      a sustainable business model                         ◉◎
```

- ID column: right-aligned, fixed width (3-4 chars)
- Position column: fixed width (3 chars), dim if absent
- Desire: left-aligned, truncated to available width
- Deadline: right-aligned, dimmed
- Urgency: right-aligned, colored by severity
- Signals: fixed right-edge column

### Signal color in list

- `!` and `‡` and `↥`: red
- `⇅` and `↝`: yellow  
- `┃` and `◉` and `◎`: cyan

### Band separators (for `list --signals`)

When listing tensions with signals, group by severity band with a dim separator:

```
─── danger ───
#154 ▸7  Generic user-configurable...    2026-07   22%  ‡↥
#5       Foundations course...            2026-05-01 OVERDUE  !
─── warning ───
#42      coherence offering...                        ⇅
─── structure ───
#2   ▸1  werk is a mature tool...        2026-06   16%  ┃◎
#90      root level is a command...      2026-05   45%  ◉◎
```

### Footer with actionable hints

```
12 of 35 active tensions · 2 overdue · 5 signals
Hint: werk list --overdue · werk list --signals · werk show <id>
```

The hint line is dimmed and contextual:
- If there are overdue items, suggest `--overdue`
- If viewing all, don't suggest `--all`
- If results are filtered, hint how to broaden

Hints disappear when piped (non-TTY) or with `--no-hint`.

## Log Rendering

`werk log` shows the temporal substrate — epochs, mutations, provenance.

### Epoch boundaries as zone openers

```
╭─ Epoch 8 · #2 · 3 days ago ─────────────────────────
│  reality updated  "v1.5+dev: Source ahead of..."
│  note: "Field audit 2026-04-06: Systematic..."
│  reality updated
╰──────────────────────────────────────────────────────

╭─ Epoch 7 · #2 · 1 week ago ─────────────────────────
│  reality updated  "v1.5+dev: Three interface..."
│  note: "Session 2026-04-02b: #178 resolved..."
│  deadline set to 2026-06
╰──────────────────────────────────────────────────────
```

Each epoch is a visual zone. The opener has the epoch number, tension ID, and relative time. Mutations are indented inside.

## Stats Rendering

`werk stats` shows field aggregates. Bar charts should use the block elements:

```
Status          Count    ████████████████████████████████
  Active           35    ████████░░░░░░░░░░░░░░░░░░░░░░░░  18%
  Resolved        108    ████████████████████████░░░░░░░░  54%
  Released         56    ████████████████████████████████  28%
```

Green for resolved, default for active, dim for released.

## Shared Infrastructure

### The `cli_display` module

All visual rendering logic lives in one shared module in `werk-shared`:

```
werk-shared/src/cli_display.rs
  pub mod glyphs     — the glyph registry (one source of truth)
  pub mod palette    — the 7-role color palette
  pub mod tree       — tree line rendering
  pub mod show       — show section rendering
  pub mod list       — list column rendering
  pub mod format     — shared formatters (truncation, timestamps, mutation summaries)
```

Commands import rendering functions instead of reimplementing display logic.

### Color helpers

```rust
pub struct Palette {
    pub enabled: bool,  // false when NO_COLOR set or non-TTY
}

impl Palette {
    pub fn chrome(&self, s: &str) -> String      // dim
    pub fn danger(&self, s: &str) -> String       // red
    pub fn warning(&self, s: &str) -> String      // yellow
    pub fn structure(&self, s: &str) -> String    // cyan
    pub fn resolved(&self, s: &str) -> String     // green
    pub fn testimony(&self, s: &str) -> String    // magenta
    pub fn bold(&self, s: &str) -> String         // bold (IDs, section headers)
    pub fn identity(&self, s: &str) -> String     // bold (tension IDs specifically)
}
```

When `enabled` is false, every method returns the input unchanged. No conditional logic scattered through commands.

### Glyph constants

```rust
pub mod glyphs {
    // Status
    pub const POSITIONED: &str = "\u{25b8}";   // ▸
    pub const RESOLVED: &str = "\u{2713}";      // ✓
    pub const RELEASED: &str = "~";
    pub const HELD: &str = "\u{23f8}";          // ⏸

    // Signals
    pub const OVERDUE: &str = "!";
    pub const CRITICAL: &str = "\u{2021}";      // ‡
    pub const VIOLATION: &str = "\u{21a5}";     // ↥
    pub const PRESSURE: &str = "\u{21c5}";      // ⇅
    pub const DRIFT: &str = "\u{219d}";         // ↝
    pub const SPINE: &str = "\u{2503}";         // ┃
    pub const HUB: &str = "\u{25c9}";           // ◉
    pub const REACH: &str = "\u{25ce}";         // ◎

    // Tree
    pub const BRANCH: &str = "\u{251c}\u{2500}";       // ├─
    pub const LAST_BRANCH: &str = "\u{2514}\u{2500}";  // └─
    pub const VERTICAL: &str = "\u{2502}";              // │
    pub const ZONE_OPEN: &str = "\u{256d}\u{2500}";    // ╭─
    pub const ZONE_CLOSE: &str = "\u{2570}\u{2500}";   // ╰─
    pub const ZONE_CONT: &str = "\u{250a}";            // ┊
    pub const TRUNCATION: &str = "\u{2026}";            // …
    pub const SEPARATOR: &str = "\u{00b7}";             // ·
}
```

## Phased Implementation

### Phase 1: Foundation (the shared module)

Create `werk-shared/src/cli_display.rs` with the glyph registry, palette, and color helpers. Migrate tree.rs to use them. No visual changes yet — just centralization.

### Phase 2: Universal color

Extend color from tree.rs to show, list, stats, log, survey. Every command that outputs human text goes through the palette. `NO_COLOR` and non-TTY detection work everywhere.

### Phase 3: Tree redesign

Two-line root tensions. Zone boundaries. Breathing room. Depth-sensitive rendering. Signal lines. This is the flagship change.

### Phase 4: Show and list polish

Section headers, mutation echo, signal colors, column alignment, band separators, footer hints.

### Phase 5: Log and stats

Epoch zones, bar chart rendering, temporal color gradient.

## Relationship to TUI

The TUI already has `AdaptiveColor` (from the reimagination), `ftui` widgets, and its own rendering pipeline. The CLI visual display is a separate surface — it outputs to stdout, not a terminal buffer. But the color *semantics* should match: danger is red in both, structure is cyan in both, resolved is green in both. A practitioner who sees red in the CLI should have the same visceral response as seeing red in the TUI.

The glyph registry is shared between CLI and TUI via `werk-shared`.

## Relationship to MCP / JSON

JSON output (`--json`) is completely unaffected. No ANSI codes in JSON. The `Palette` is bypassed entirely when `output.is_structured()` is true.

The `--show-after` and mutation echo behaviors apply to both human and JSON modes (in JSON, the response wraps `{"result": ..., "show": ...}`).
