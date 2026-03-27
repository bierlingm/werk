# Command Center, Time Axis, and the 2D Lattice

**Tensions:** #18 (survey view), #89 (logbase), #90 (root-level command center)
**Foundation:** [werk-conceptual-foundation.md](./werk-conceptual-foundation.md)
**Prior art:** [tui-big-picture.md](./tui-big-picture.md) (deck/stream V1-V8)
**Date:** 2026-03-26

---

## The Central Insight

The deck is a 1D cursor in a list. The command center inverts this: **you are the fixed point, the field moves around you.** The deck and the command center aren't different features — they're different zoom levels on the same 2D surface.

The surface has two axes:
- **Vertical: time** — desire/future at top, reality/past at bottom, NOW at center
- **Horizontal: structure** — parent ← you → children, siblings laterally

Everything the instrument shows is a projection of this surface:
- **Stream (deck)** = zoomed tight on one column. Structure fills the screen. Time annotated on the spine.
- **Survey** = zoomed tight on one row. Time fills the screen. Structure annotated on each element.
- **Command center** = zoomed out. Both axes visible. The field around you.

Tab (yaw) doesn't switch modes — it switches which axis dominates the projection. Zoom (Enter/Shift+Enter) controls how much of the lattice is visible. These are independent: you can be stream-oriented at wide zoom (seeing many tensions in parallel columns) or survey-oriented at tight zoom (seeing one time window in deep detail).

---

## Part I: The Command Center Interaction Model

### The Fixed-Point Principle

In the deck, the cursor moves through a static layout. In the command center, you are the fixed point. The field orients around your current focus — a tension at a moment. This is the Google Maps principle: the pin stays centered, the map pans.

But "fixed point" is semantic, not literal. The cursor does move within the visible field. What's fixed is the *framing*: your focused tension is always the widest column, at screen center. Everything else is context radiating outward from that center.

### Navigation on the 2D Surface

The same gestures work everywhere. Their meaning shifts with context:

| Axis | Keys | In Stream (tight) | In Command Center (medium) | In Survey (wide) |
|------|------|-------------------|---------------------------|-------------------|
| Pitch | j/k | Move through steps in order | Pan the time window up/down | Move between time bands |
| Roll | h/l | Ascend/descend structural depth | Shift focus to adjacent tension | Move between structural groups |
| Zoom | Enter/Shift+Enter | Focus/orient within one tension | Zoom into/out of the lattice | Focus/orient within time band |
| Yaw | Tab | Switch to survey orientation | Transpose the grid | Switch to stream orientation |

**Key insight:** Pitch and roll don't change meaning — they always move along time and structure respectively. What changes is the *granularity* at each zoom level. At tight zoom, pitch moves one step at a time. At wide zoom, pitch moves one time band at a time.

### Zoom as Focal Length

The zoom axis has three named stops, but the surface is continuous:

```
Stream ←────────── Command Center ──────────→ Survey
(1 column, full)   (N columns, balanced)      (1 row, full)

  One tension's       Both axes              All tensions
  depth through        visible                in one time
  time                 simultaneously         window
```

| Level | Columns | Time depth | Detail |
|-------|---------|------------|--------|
| Stream | 1 (full width) | Full order of operations | Every step, every annotation |
| Peek | 3 (focused + 2 neighbors) | Current epoch | Focused full, neighbors compressed |
| Command | 5-7 (focused + context) | Current epoch + edges | Focused summary, neighbors as indicators |
| Survey | All visible | One time band | Each tension as a cell with status glyph |

Shift+Enter widens (stream → peek → command → survey). Enter narrows (survey → command → peek → stream). The current V9 "orient" zoom slot maps to peek or command — it's the first widening past the deck.

### What You See at Each Level

**Stream (current deck):** Exactly what V1-V8 implement. One tension, full depth.

**Peek (first widening):**

```
       sibling ←   │    FOCUSED      │   → sibling
       ┊            │                  │           ┊
       compressed   │   route:         │   compressed
       summary      │     step A       │   summary
                    │     step B       │
       ┊            │   ═══ NOW ═══   │           ┊
                    │   accumulated:   │
       compressed   │     ✓ step X    │   compressed
       summary      │     ✧ note     │   summary
       ┊            │                  │           ┊
```
Focused column gets ~60% width. Neighbors get ~20% each. Neighbors show: desire (truncated), closure ratio, next step, deadline, recent glyph.

**Command (full lattice):**

```
  #3 TUI     │ #13 Found  │ #10 CLI    │ #36 Biz    │ #82 GUI
  [4/8]⏱May  │ [4/11]⏱Jun │ [6/9]     │ [0/7]     │ [1/4]
              │            │            │            │
  survey v.   │ state m.   │ staging    │ public...  │ parity
  threshold   │ gesture g. │ ergonomic  │ open fmt   │ design
  pathway p.  │ epoch cr.  │ batch pos  │ revenue    │
══════════════╪════════════╪════════════╪════════════╪═══════ NOW
  ✓ consol.   │ ✓ test     │ ✓ 6 done  │            │ ✓ tauri
  ✓ 2 more    │            │            │            │
```
All siblings visible. Each gets proportional width (focused column wider). Each shows: desire snippet, closure ratio, deadline, next steps, recent activity. Shared NOW line.

**Survey (time-first):**

```
  ── overdue ──────────────────────────────────────────
  (none)

  ── this week ────────────────────────────────────────
  #18 survey view designed...          ← #15 TUI
  #19 threshold mechanics...           ← #15 TUI
  #30 epoch creation trigger...        ← #13 found

  ── this month ───────────────────────────────────────
  #45 TUI yank                         ← #3 TUI
  #48 staging mechanism                ← #10 CLI

  ── held across field ────────────────────────────────
  #58 pathway palettes                 ← #15 TUI
  #65 observational analysis           ← #13 found
```
Time bands as primary organization. Each step shows its structural annotation (parent tension). Cursor moves between steps within a band, between bands with j/k at wider pitch.

### Root Level vs. Descended

At root level, the command center shows root tensions as columns. There's typically one root (#2 in the current tree), so the "columns" are its direct children — the major workstreams.

At a descended level, the command center shows the current tension's children as columns.

The root level is special because it's where the logbase lives. Panning down past the current epoch at root level enters the logbase — the accumulated history across the entire field, not just one tension.

### Transitions Between Zoom Levels

The transition must carry context. The current cursor position maps to the new view:
- **Stream → Peek:** The tension you're in becomes the focused column. Its siblings appear alongside.
- **Peek → Command:** More siblings become visible. Focused column narrows. Neighbors narrow further.
- **Command → Survey:** The grid transposes. Your current time position becomes the selected band. Your current tension becomes the structural annotation.
- **Survey → Command → Stream:** Reverse. The selected element's tension becomes the focused column, then the full deck.

Tab (yaw) is a shortcut: stream ↔ survey, carrying the cursor across.

---

## Part II: The Time Axis

### What Flows Along Time

The vertical axis carries events (gestures) anchored to moments. Each event belongs to a tension and has structural relationships:

- **Mutations:** created, resolved, released, desire updated, reality updated, repositioned, reparented
- **Notes:** observations, reflections, insights
- **Epoch boundaries:** phase transitions where the delta changes
- **Sessions:** periods of engagement (opening → closing)

### Rendering Time: The Stratigraphic Principle

Time is not uniform. Dense periods (bursts of activity) need more space. Sparse periods (long gaps) need less. The rendering compresses and expands dynamically:

```
  3 weeks ago ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄  (compressed gap)

  2 days ago   #90 created
               #89 created
               #18 note: survey view...
  yesterday    #90 reality updated
               #89 desire refined
  6 hours ago  #18 note: root-level...
               #90 desired refined
               #89 reality updated

  ─────────── NOW ───────────

  approaching  #18 survey view           ⏱ (no deadline)
               #19 threshold mechanics   ⏱ (no deadline)
  May-30       #3 FrankenTUI-first       ⏱ May-30
  June         #2 werk is mature         ⏱ Jun
```

Above NOW: future steps ordered by deadline/urgency (closest first).
Below NOW: past events ordered by recency (most recent first).
The NOW line is the frontier — the same frontier from the deck, extended across the field.

### Lanes, Streams, and Labels

**In stream orientation** (columns), time flows within each tension's column. Events appear at their temporal position in the column they belong to. Cross-column simultaneity is visible by horizontal alignment.

**In survey orientation** (rows), time bands contain events from all tensions. Structural labels (tension name, parent path) annotate each event.

**In command orientation** (both), each column has its own mini time stream, all sharing the same vertical time scale. Events across columns at the same moment read as a cluster — coordinated push, burst of activity, or epoch.

### Epoch Boundaries

Epoch boundaries are horizontal rules that span relevant columns:

```
  ═══════════════ epoch 3 ═══════════════

    events from epoch 3...

  ═══════════════ epoch 2 ═══════════════

    events from epoch 2...
    (desire was different here)
    (reality was different here)
```

When a tension has a phase transition (desire or reality change), its epoch boundary appears. If multiple tensions transition simultaneously, their boundaries align. The visual effect is a sedimentary cross-section — layers of the project's geological history.

### Dense and Sparse Time

The time axis adapts to density:
- **Dense bursts:** Each event gets its own line. Timestamps show hours/minutes.
- **Normal activity:** Events grouped by day. Day labels on the left margin.
- **Sparse gaps:** Compressed to a single line: "3 weeks ┄ ┄ ┄" with gap duration.
- **Very long gaps:** May contain sub-labels: "3 weeks (7 sessions, 23 gestures)".

This is the Marey principle: a train schedule doesn't allocate equal space to every hour. Stations where trains stop get space. Stretches of empty track compress. The information density determines the spatial allocation.

### The Logbase as Geological Substrate

The logbase (#89) resolves naturally as the time axis extending below the current epoch. What you see when you pan down past the frontier:

1. **Current epoch** — the active zone, the working surface
2. **Prior epochs** — previous desire/reality layers, collapsed by default, expandable
3. **Cross-tension history** — at root level, the combined history of all tensions

Each epoch layer shows:
- The desire at that time (trajectory snapshot)
- The reality at that time (trace snapshot)
- What was accomplished during that epoch
- What changed to trigger the transition

The logbase is not a simple list — it's a **queryable lattice**. The command center provides visual navigation (pan down through layers). The CLI and MCP provide query access (search, filter, correlate). The GUI (future) provides rich visual archaeology.

### Time Windows: Smart Selection

When the survey shows "this week" or "this month," how does it choose the window? Smart defaulting:

1. If the cursor is on a tension with a deadline → use that deadline's window
2. If no deadline → use the previous step's deadline (structural inheritance)
3. If no structural deadline → use the current calendar period
4. Frame widening ([/]) expands the window: day → week → month → quarter → year → all

The time window is relative to the cursor position. Moving the cursor to a different tension may shift the window. This is the "field moves around you" principle applied to time: the temporal frame adapts to your focus.

---

## Part III: Visualization Toolkit

Full research: [visualization-research.md](./visualization-research.md). This section extracts what's directly applicable to the command center and time axis.

### Ten Principles from the Research

The research organized techniques not by domain but by structural principle. The ten:

1. **Flow as Width** (Minard, Sankey) — encode quantity in band thickness. Closure progress as band width: `████░░` vs `██░░░░`.
2. **Slope as Rate** (Marey, burndown) — angle encodes speed. The rate of frontier advance as a sparkline slope.
3. **Strata as Accumulated Time** (geology, horizon charts) — layers for epochs. Older below, newer above. Thickness = duration or density.
4. **Focus + Context** (fisheye, DOI trees) — sharp center, fuzzy periphery. The command center's fisheye columns ARE this principle.
5. **Parallel Tracks** (conductor's score, timing diagrams) — multiple independent sequences aligned on a common axis. The survey view.
6. **Topology as Information** (phase portraits, causal loops) — shape encodes qualitative behavior. A tension's engagement *type* as a micro-glyph.
7. **Contour and Isoline** (topographic, isochrone) — gradient through spacing. Temporal pressure as whitespace between steps.
8. **Compression Through Folding** (horizon charts, small multiples, interlinear glossing) — reduce dimensions by overlaying. Small multiples IS the command center's column layout.
9. **Observer at Center** (military situation map, HUD) — the display organized around your position. Werk's core metaphor.
10. **Glyph Alphabets** (Labanotation, MIL-STD-2525, weather stations) — encode multiple dimensions per symbol. The existing glyph vocabulary, enriched.

### Seven Techniques to Adopt

From the research's top-10 synthesis, these seven map directly to the command center and time axis:

**1. Sparkline micro-histories** — every tension carries a 4-8 character sparkline (`▁▃▅▇`) showing its activity shape. Appears in the right column, costs zero vertical space. The trace shape the foundation calls "diagnostic," made visible.

```
  ▸ survey view designed...        #18  ▂▄▆▇  6h
  ▸ threshold mechanics...         #19  ▁▁▁▁  2d
```

**2. Conductor's score survey** — the survey view as parallel horizontal tracks, each tension a lane, time flowing left to right, NOW as a vertical cursor line. Read vertically = what's happening everywhere right now. Read horizontally = one tension's journey.

```
  Now ──────────────────┐
  #3 TUI   ▸▸▸═══▸▸✓───│───────
  #13 Fnd  ▸▸▸▸▸────────│▸▸▸════
  #10 CLI  ──────▸▸▸────│▸──────
  #36 Biz  ─────────────│▸▸▸▸▸▸▸
```

**3. DOI-driven route compression** — instead of compressing route by position (bookends), compute Degree of Interest: critical path + deadline proximity + mutation recency + cursor distance. Show what matters, not what's sequentially adjacent.

**4. Urgency contour spacing** — whitespace between route steps encodes temporal pressure. Tight spacing = compressed timeline. Wide spacing = breathing room. The density of the display IS the urgency field. No additional characters needed.

**5. Epoch strata in the log** — the logbase renders epochs as geological layers. Band height proportional to activity density. Unconformities (gaps in the record) explicitly marked. The visual pattern is diagnostic: steady productive epochs vs. pivots vs. stalls.

```
  Epoch 4  ████████████████████  "Ship v2.0"    12d, 8 gestures
           ── unconformity: 9d silence ──
  Epoch 3  ████████              "Ship v2.0"    5d, 3 gestures
  Epoch 2  ██████████████████    "Refactor"     10d, 12 gestures
  Epoch 1  ████                  "Prototype"    3d, 6 gestures
```

**6. Overlay-mode information layers** — the military situation map model. Base display + toggleable overlays that add data to the same spatial positions:
- `t` — trace layer: sparkline micro-histories on each step
- `u` — urgency layer: colored urgency gradient
- `d` — dependency layer: inline indicators (`←#31`, `→#35`)
- `e` — epoch layer: epoch boundaries and membership

Same spatial layout, different information depth. The user's mental map of positions is never disrupted.

**7. Phase portrait micro-glyph** — a single character encoding the *qualitative type* of engagement with a tension:
- `→` Executing (reality advancing, desire stable)
- `↑` Re-envisioning (desire shifting, reality paused)
- `↗` Converging (both advancing toward closure)
- `↺` Oscillating (rework loop)
- `◯` Equilibrium (nothing moving)

Computed from recent gesture history. Tells you *what kind of thing is happening*, not just what state it's in.

### Principles That Should Inform Design

From the research's "honorable mentions":

- **The Rest** (music notation): Silence should be notated. A tension where nothing is happening is different from one that doesn't exist. Consider a "quiescent" marker.
- **The Unconformity** (geology): Gaps in the record are information. Long periods of no gestures should be visible as structural facts, not smoothed over.
- **The Light Cone** (physics): From any step, only some futures are reachable given dependencies and deadlines. Steps outside the light cone are structurally inaccessible from here.
- **The Reinforcing Loop** (systems dynamics): When progress on A enables progress on B enables progress on A, that's a leverage point. Detectable from dependency structure.

### Terminal Rendering Palette

The research catalogued what's available in a character grid:
- `▁▂▃▄▅▆▇█` — 8-level sparklines
- `▏▎▍▌▋▊▉█` — 8-level horizontal fill (width encoding)
- `░▒▓█` — 4-level density fill
- ANSI brightness (dim/normal/bold/bright) as urgency channel
- Whitespace as data (line spacing encodes temporal pressure)
- Braille `⠁⠂⠃⠄...⣿` for high-resolution mini-plots (2x4 dot matrix per cell)

---

## Part IV: Rendering in the Terminal

### The Column Layout

The command center rendering is a **small multiples** layout — the same frame (desire/route/NOW/accumulated/reality) repeated for each sibling tension, with the focused tension getting more width.

Terminal implementation:
- Each column has minimum width (enough for desire snippet + closure ratio)
- Focused column gets remaining space after minimums are allocated
- If too many siblings to fit, compress: show focused + N nearest, with "← M more" / "→ M more" indicators
- The shared NOW line crosses all columns at the same vertical position

### Column Content by Zoom Level

| Zone | Stream (full) | Peek (60%) | Command (~20%) | Survey (cell) |
|------|---------------|------------|----------------|---------------|
| Desire | Full text, word-wrapped | Truncated to 2 lines | First ~20 chars | Glyph only |
| Route | All steps with deadlines | Next 2-3 steps | Count + next | Count only |
| NOW | Input point, held summary | Input point | Closure ratio | Status glyph |
| Accumulated | Individual items | Summary line | Count | Count |
| Reality | Full text, word-wrapped | Truncated to 1 line | First ~20 chars | — |

### Structural Connections

In the command center, structural relationships show through:
- **Adjacency:** Siblings are adjacent columns
- **Width:** Focused column is wider (fisheye principle)
- **Depth indicators:** "→N" showing children available for descent
- **Parent frame:** At the top, the shared parent's desire (if descended)
- **Activity correlation:** Simultaneous events across columns align horizontally

Lines/arrows between columns are not needed — adjacency and alignment carry the structural information. This keeps the rendering clean for terminal constraints.

### The Fisheye Principle

The focused column gets disproportionate space. This is the fisheye/hyperbolic lens applied to a discrete grid:

```
  Before focus:    [A] [B] [C] [D] [E]     (equal width)
  After focus on C: [a] [B] [C C C] [D] [e]  (C expanded, edges compressed)
```

In terminal terms: focused column gets 40-60% of width. Immediate neighbors get 15-20%. Distant siblings get minimum width or collapse to glyphs. The cursor moving to a new column causes the widths to rebalance — the field redistributes around the new center.

### Lever Bar Adaptation

The lever bar (bottom status line) adapts to zoom level:

| Zoom | Lever content |
|------|---------------|
| Stream | Current tension's vitals (as today) |
| Peek | Focused tension name + 2 neighbor names |
| Command | Field summary: total/active/overdue + focused tension name |
| Survey | Time window label + count of items in window |

### Color and Glyph Strategy

The command center extends the existing monochrome + accent approach:
- **Focused column:** Full rendering (current deck treatment)
- **Neighbor columns:** Dimmed text (existing dim style)
- **NOW line:** Accent color across all columns
- **Epoch boundaries:** Double-line rule (═) in dim
- **Activity glyphs in compressed columns:** ✓ resolved, ✧ note, ◆ desire change, ◇ reality change, ✦ created

---

## Part V: Implementation Strategy

### What Changes from the Current Architecture

The current TUI is built around a 1D cursor (`deck_cursor: usize`) indexing into a flat list of selectable items. The command center requires:

1. **2D cursor:** (column_index, row_index) — which tension, which position within that tension
2. **Multi-column state:** Loading siblings' children, not just the focused tension's children
3. **Shared time scale:** Computing a unified vertical time axis across all visible columns
4. **Column width computation:** Fisheye allocation based on focus position
5. **Cross-column rendering:** A render function that lays out multiple columns side by side

### What Stays the Same

- The deck (stream view) is unchanged — it's just the command center at tight zoom
- The frontier classification (route/overdue/next/held/accumulated) works per column
- Navigation gestures (j/k/h/l) keep their meaning
- The state machine (NORMAL/INPUT/FOCUSED/PATHWAY) works the same

### Incremental Path

1. **V9 (Orient/Peek):** The current V9 placeholder. Implement as peek — show 2 neighbor columns in compressed form alongside the full deck. This is the first step toward the 2D surface.

2. **V10 (Command):** Expand to show all siblings. Implement fisheye column widths. Add horizontal cursor movement (new key or repurpose roll at command zoom).

3. **V11 (Survey):** Implement time-band rendering. Tab transposes the grid. Smart time window selection.

4. **V12 (Logbase):** Implement downward time navigation past the current epoch. Epoch layer rendering. Query substrate (CLI/MCP first, TUI display later).

### Data Requirements

The command center needs data that the current `load_siblings()` doesn't provide:
- Each sibling's children (for route/accumulated summaries)
- Each sibling's recent mutations (for activity indicators)
- Each sibling's deadline and urgency (for temporal positioning)
- Cross-tension event timeline (for the unified time axis)

This could be:
- A new `load_field_context(parent_id)` that returns siblings + their child summaries
- Or lazy loading: load focused tension fully, load neighbors on demand as columns become visible

### Open Questions

1. **Column scroll vs. column select:** When more siblings than fit, does roll (h/l) scroll the column set or move focus within visible columns? (Proposed: roll moves focus, columns auto-scroll to keep focus centered.)

2. **Peek vs. Orient:** Are these the same thing (V9) or distinct zoom stops? The foundation defines orient as seeing parent context. The command center defines it as seeing sibling context. These may combine: orient shows parent frame + sibling columns.

3. **Yaw at root level:** At root level with a single root tension, Tab (stream ↔ survey) is clear. With multiple roots, what does "survey" mean? All roots in one time band? (Proposed: yes. Survey at root = all root tensions organized by time.)

4. **Cross-workspace survey:** The foundation mentions survey naturally unifying multiple workspaces. This is future work but the data model should not prevent it.

5. **Keyboard mapping:** The command center introduces a second axis of cursor movement. Options: (a) repurpose h/l at command zoom to move between columns, (b) use new keys (H/L or </>) for column navigation, (c) use Tab for column cycling. (Proposed: h/l at peek/command zoom switches to column navigation. l at stream zoom descends. This is a context-dependent overload — roll means "move along structure axis" which is descent at tight zoom and column-shift at wide zoom.)

---

## Part VI: The Logbase

### What It Is

The logbase is the searchable, queryable substrate of all prior desire/reality/epoch layers across the entire field. It's not a log viewer — it's a **structural resource**.

### What It Contains

Every epoch snapshot: desire at that time, reality at that time, children state at that time, what triggered the transition. Every note. Every resolution and release. Every gesture.

### How It's Accessed

**TUI (visual navigation):** Pan below the current epoch in the command center. See prior layers as collapsed rows. Expand to see the delta at that point. Compare current vs. prior desire/reality.

**CLI (query):**
```
werk log 18                    # tension #18's epoch history
werk log --search "survey"     # semantic search across all epochs
werk log --since "last week"   # temporal filter
werk log --compare 18          # show desire/reality evolution
```

**MCP (structured query):**
```json
{"tool": "query_logbase", "params": {"search": "survey view", "tension_id": 18}}
{"tool": "epoch_history", "params": {"tension_id": 18}}
{"tool": "compare_deltas", "params": {"tension_id": 18, "epoch_a": 2, "epoch_b": 5}}
```

### The Ghost Geometry

Prior desire-reality pairs form triangles that expand and contract over time. The desire rises (aim sharpens/expands). The reality rises (ground gained). The gap (the tension) narrows or widens. The sequence of these triangles — the morphology of the delta over time — is the ghost geometry.

In the logbase, this could render as:

```
  epoch 5 (current)    ◆ desire ──────── 60% ────────── ◇ reality
  epoch 4              ◆ desire ────────────── 40% ──── ◇ reality
  epoch 3              ◆ desire ──── 70% ──────────────── ◇ reality
  epoch 2              ◆ desire ─────────────────── 20% ─ ◇ reality
  epoch 1              ◆ desire ─────────────────────── 5% ◇ reality
```

The width of the bar between ◆ and ◇ represents closure progress. The label shows the gap. Reading down shows the trajectory: the tension tightening, loosening, pivoting.

---

## Appendix: Vocabulary Additions

| Term | Meaning |
|------|---------|
| **Lattice** | The 2D surface of structure × time. The conceptual space the instrument projects onto the screen. |
| **Column** | A tension's vertical stripe in the command center — its events through time. |
| **Band** | A horizontal stripe in the survey — a time window across all tensions. |
| **Fisheye** | The principle that the focused element gets disproportionate space, with compression increasing toward the edges. |
| **Ghost geometry** | The shape of desire-reality triangles evolving across epochs. The morphology of directed action over time. |
| **Logbase** | The searchable substrate of all prior epochs. Not a logbook (sequential) but a base (queryable, structural). |
