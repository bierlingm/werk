# TUI Rebuild — Shaping Doc

**Scope:** Stream view only. Survey view (#18), threshold mechanics (#19), logbook, and ground mode are out of scope.
**Source:** designs/werk-conceptual-foundation.md, tensions #15, #18, #19, shaping sessions 2026-03-23/24.
**Status:** Shape selected (D), breadboarded, sliced. Big picture at `designs/tui-big-picture.md`.

---

## Vocabulary Decisions (this shaping)

| Foundation term | Instrument term | Rationale |
|----------------|----------------|-----------|
| Operating envelope (as full view) | **Deck** | Flight deck / ship's deck — the full working surface. "On deck" = action-relevant. "Clear the deck" = epoch compression. |
| Operating envelope (as action zone) | **Console** | Cockpit console — the action zone at center where signals converge and you operate from. The deck contains the console. |
| Remaining theory / sequence | **Route** | The planned sequence of steps ahead in the order of operations. "Route" implies a chosen path with directionality — the theory of closure IS a route. Flight/navigation resonance. |
| Ghost geometry (one tension) | **Log** | One tension's epoch sequence. Linear (may fork at splits). "Show me #15's log." |
| Ghost geometry (composite) | **Logbook** | The composite lattice of all logs. A DAG linked by provenance, temporal correlation, and semantic content. Captain's logbook — the whole bound volume. Queryable as structure. |
| Epoch boundary on desire change | **Clean close** | Desire change closes epoch, reality carried forward marked stale, new epoch opens neutral. |

### Spatial hierarchy

```
Deck (the full view)
├── Parent breadcrumb (screen boundary signal — ← hint)
├── Desire (top anchor — always visible, deadline on left)
├── Route (remaining theory — positioned steps in order of operations)
├── Console (frontier action zone — center, cursor home)
├── Reality (bottom anchor — always visible)
└── Log indicator (screen boundary signal — ↓ hint)
         ↓ Log (this tension's epochs, navigated into)
```

### Column layout

Three columns with fixed gutters between them:

```
LEFT COL  |gutter|  MAIN COL                              |gutter|  RIGHT COL
(deadline)         (text — desire and children aligned)              (ID, →N, trace/age)
```

Left column: content width accommodating longest deadline in view (6 chars for "Mar 30"). Gutter provides spacing — no trailing space needed.
Main column: flexible, shares alignment between desire and all children.
Right column: right-aligned facts — tension ID (`#19`), children indicator (`→4`), trace age (`2d`).

Left column may also house ordinal position in the order of operations (e.g., `3  Mar 24`). Could be zoom-dependent: show ordinals in orient/focus, hide in normal for cleanliness.

### Glyph decisions

| Status | Glyph | Rationale |
|--------|-------|-----------|
| Resolved | ✓ | Accomplished, closed |
| Released | ~ | Tension dissolved, let go |
| Note | ※ | Japanese reference mark — distinctive, performative, the "notation" sense |
| Active (positioned) | ▸ | Forward-pointing, committed |
| Active (held) | · | Present but uncommitted |
| Has children | →N | Right-side indicator with count (e.g., →3). Absent when no children. |

---

## Resolved Design Questions

### Q1: Layout model → Hybrid with zoom-differentiated rendering

Normal zoom is the hybrid model: fixed anchors (desire/reality always visible), route visible as step list above console, console compact with compressed indicators. The deck-as-stage epiphany lives as focus-zoom behavior — focus renders one element's detail in the console area.

### Q3: Zoom levels → Focal length principle

**Zoom adjusts the weight ratio between center and edges.** It is focal length, not content switching. You're always looking at the same structure; zoom changes what's sharp and what's peripheral.

| Zoom | Center | Edges | Metaphor |
|------|--------|-------|----------|
| **Orient** | Compresses — console/route lose detail | Gain weight — parent, siblings, grandchild signals become visible | Stepping back from painting to see it in the room |
| **Normal** | Balanced — route as list, console compact | Balanced — desire/reality as anchors, boundary signals subtle | Standing at viewing distance |
| **Focus** | Expands — one element fills console with full detail | Compress/blur — everything else becomes indicators at edges | Leaning in to examine brushwork |

Overflow: extra space → center gets more room (the sharp part breathes).
Underflow: too little space → edges compress first (peripheral blurs more aggressively).

### Q5: Left/right spine → Column-based with contextual content

Left column = intent (deadlines). Right column = trace (IDs, children indicator, ages, facts). Content is contextual to zone:
- Desire: deadline left, age right
- Route items: deadline left, ID + children indicator + age right
- Console items: deadline left (for overdue/next), ID + trace right
- Reality: age right

### Q6: Above desire → Parent breadcrumb

Above desire is a screen boundary signal: the parent tension as a breadcrumb line with `←` hint indicating h/← takes you there. At root level (no parent), empty.

### Q7: Zoom overflow/underflow → Follows focal length principle

Extra space → center breathes (compressed elements partially uncompress). Too little space → edges compress first per constraint hierarchy.

### Q8: Peek/preview mechanism

The →N children indicator creates an expectation: "how do I glance at children without fully descending?" Focus-zoom (Enter) shows full detail including children. But is there a lighter-weight preview? Options carried forward as shape alternatives:
- **Inline expansion** (old gaze — card opens in place in the list)
- **Side panel** (slides in from right — children preview without leaving context)
- **Focus is sufficient** (Enter to see detail, Shift+Enter to return — no separate peek)

**Status:** Shape decision.

### Q9: Reality verbosity and short descriptors

Reality statements can grow large (10+ lines). This crowds the bottom anchor. Options:
- **Summary + expandable**: first line as summary, zoom-focus to see full text
- **Max lines with truncation**: show N lines, dim `...` indicator if more
- **Practice-level signal**: the instrument surfaces verbosity as a signal ("your reality is 12 lines — consider compressing")
- **Short descriptor + elaboration pattern**: both desire and reality could have a one-line summary plus expandable body. Desire may not need this (aims are naturally concise). Reality often does.

**Status:** Open. Affects layout, zoom, and the meaning of "glance down to ground yourself."

### Q10: Age/trace placement — inline vs. right column

The "12h ago" age annotation could go:
- **Right column** (current) — predictable position, costs horizontal space even when text is short
- **Inline after text** — `desire text · 1d ago` — recovers space when text doesn't fill the line, but position varies

Previous TUI used inline with ` · ` separator. The right column approach is more structured but wastes space. Could be configurable (`deck.age_position`).

**Status:** Open. Design experiment needed.

### Q11: Information distribution across deck space

The deck has available space beyond the 4 zones (desire, route, console, reality) — the bottom bar, the edges, the blank areas. How to use this space to distribute information about the current tension:
- Short code (#N) in bottom bar
- Help hint in bottom bar
- Log indicator could merge into bottom bar
- The lever concept (old TUI) is scrapped for deck mode
- Other tension metadata (status, created date, parent path) could surface in peripheral positions

**Status:** Open. The bottom bar design is immediate; the broader information distribution question is ongoing.

### Q12: Log indicator and bottom bar

The log indicator (`↓ N prior events`) and help hint (`? help`) should share the bottom line rather than stacking. The short code (#N) should also be there. The old lever is replaced by a deck-native bottom bar.

**Status:** Implemented. Bottom bar shows #N left, ↓ N prior events center, ? help right.

### Q13: Rules edge-to-edge vs content-width

The heavy rule (━━━) under desire and light rule (──) above the bottom bar currently render within the content area margins. Suggestion: rules should extend from terminal edge to terminal edge while text respects margins. This creates a stronger visual frame.

**Status:** Open. Try during V2+ refinement.

### Q14: Top/bottom breathing room

The deck currently adds 1 line of top padding when terminal height > 30. Suggestion: remove top/bottom padding entirely — let content pin to the very first and last available rows. The margins add breathing room left/right; top/bottom should be used fully.

**Status:** Open. Try during V2+ refinement.

### Q15: Age inline consistency with children

With desire/reality using inline age (`text · Nd ago`), children in the route/console will use right-column age (`#ID →N Nd`). This inconsistency may feel wrong. Options:
- Revert desire/reality to right-column age for consistency
- Make children also use inline age (but they have more right-column data: ID, children indicator)
- Accept the difference: anchors (desire/reality) are text-primary, children are data-primary

**Status:** Tentatively accepted as inline. Revisit after V2 when children are visible alongside desire/reality.

### Q16: Held expansion when space allows

At normal zoom, held items are compressed to `· N held`. But when the middle zone has ample free space (e.g., tension with only held children and no route/overdue), this wastes screen real estate. The intelligent compression principle (R9.2) says extra space → compressed elements uncompress.

Options:
- **Space-aware heuristic**: if free lines in middle > held count, show individual held items
- **Defer to V6 compression engine**: proper constraint hierarchy decides
- **Orient zoom expands held**: held items shown individually only at orient zoom

**Status:** Implemented with gradual compression — shows as many as fit, summary for the rest.

### Q17: Default cursor position

Cursor auto-rests on the next committed step. Alternative: rest on the input point (inviting creation). Both valid. Could depend on state:
- If next step exists → cursor on next (action vector)
- If no next step → cursor on input point (invite creation)

**Status:** Current behavior (next step) accepted. Revisit if it feels wrong in practice.

### Q18: Space key = inline edit/action interface

Pressing Space on the cursor position should open the edit/action interface inline, expanding the element to show available gestures and editable fields. This is the bridge between NORMAL and INPUT/FOCUSED states. Space becomes the universal "act on this" key.

**Status:** Open. Implement in V4 (gestures + INPUT state) or V7 (focus zoom).

### Q19: Middle zone content placement

Currently all children render top-down from the top of the middle zone. This means the console content (overdue, next, held, accumulated) is pressed up against the route, with empty space between accumulated and reality. Alternative placements:
- Console content at vertical center of middle zone (current mockup design)
- Console content pinned just above reality (close to the "ground")
- Route pinned below desire, console pinned above reality, gap in between

**Status:** Open. The right answer depends on how it feels with real data. Experiment in V6 (compression engine).

### Q20: Wider terminal usage

MAX_CONTENT_WIDTH is 104 chars. Wider terminals waste space on both sides. Options:
- Increase MAX_CONTENT_WIDTH
- Remove the cap entirely (fill terminal)
- Scale up: wider text budget for main column, more breathing room
- Use extra width for dual-column layout at very wide sizes (>200 chars)

**Status:** Open. Current cap prevents readability issues at extreme widths. Revisit when the deck has more content.

### Q21: Bottom edge gap

Possible WezTerm padding below the last rendered line. Not a werk issue — terminals may add padding. Verify by checking if our code renders to the last available row.

### Q22: Held indentation

The old TUI indented held (unpositioned) steps to the right, visually distinguishing them from positioned steps. Consider adding 2-3 char extra indent for held items.

**Status:** Implemented. 2-char extra indent (HELD_INDENT constant) applied to held items and their summary.

### Q23: Separator position — ordered vs unordered

The console boundary separator separates ordered items (route + overdue + next) from unordered items (held + input + accumulated). Next step is part of the action sequence, not a separate console element.

**Status:** Implemented. Separator now falls below next step.

### Q24: Accumulated items gravity toward reality

Resolved/released/notes are facts — they belong closer to reality than to NOW. Layout: route+next (top, ordered theory) → held+input (middle, frontier) → [breathing room / stats surface] → accumulated (bottom, facts settling toward reality).

**Status:** Implemented. Accumulated renders bottom-up from middle_end (just above reality). Breathing space between input point and accumulated.

### Q25: Desire right-column treatment (revisit inline age)

Try: desire uses the right column like its children — ID, children count/arrow, age. E.g.: `Jun    desire text...    2 →5  1d`

**Status:** Implemented. Desire line shows ID + →N + age in the right column, matching children layout.

### Q26: Notes in the deck

Notes are mutations (field="note"), not tensions. They don't appear as children and aren't classified by the frontier. For notes to show in accumulated, need to load mutations and filter. Deferred to V8 (peek + signals). Note gesture works but notes don't display in deck yet.

### Q27: "Path" as canonical term for accumulated facts

The accumulated zone (resolved, released, notes) could be called "path" — the trail of facts left by action. Route (ahead) vs Path (behind). Mirrors trajectory (desire) vs trace (reality).

**Status:** Candidate naming.

### Q28: Unified summary line above NOW

When route and held are both compressed, their summaries could merge into one line: `▸ 3 more route · 2 held` — a single line capturing everything between desire and the input point. Reduces chrome, maintains information.

**Status:** Implemented. When both route and held are fully compressed, renders `▸ N route · N held` as one line.

### Q29: Compression priority order

When space is tight, compression should proceed in this order (first to compress → last):
1. **Accumulated** compresses first (facts, lowest action-relevance)
2. **Held** compresses second (available but uncommitted)
3. **Route** compresses last (the ordered plan, highest information value)

Currently the priority is route > held > accumulated (route compresses last). But the user observed that once accumulated is fully compressed to a summary line, that line should NOT be pushed off-screen — instead held should start compressing, then route. The summary lines for each category should always remain visible if the category has items.

**Status:** Implemented. Two-pass algorithm: first reserves 1 summary line per non-empty category, then distributes remaining space by priority (route > held > accumulated). No summary gets pushed off-screen.

### Q30: Resolved steps shown in-place on the route ("trajectory view")

The old field view shows resolved/released steps in-place alongside active ones. This provides different information from the deck's separation into accumulated: you see the full route including what's been accomplished, giving a trajectory/progress sense. Could be a useful alternate viewing mode — toggle between "frontier" (deck default: resolved moves to accumulated) and "trajectory" (resolved stays in position). Possibly a Shift+T toggle or a `deck.resolved` config setting.

**Status:** Implemented as Shift+T toggle. Key combo is placeholder — the function is useful, binding TBD.

### Q31: Meta data visibility across zoom levels

What traces/signals/facts should be visible at each zoom level?
- **Normal zoom** (current): ID, →, age in right column. Could this be too much? The right column currently shows on every child line.
- **Orient zoom** (V9): parent context, siblings, grandchild counts. This is where structural signals (→N counts, ordinals, deviance annotations) arguably belong — orient is "stepping back to see it in the room."
- **Focus zoom** (V7): full detail of one element. All facts shown.

Option A: Normal shows only text + deadline (minimal), orient adds right-column traces.
Option B: Normal shows right column as now, orient adds *more* (ordinals, grandchild counts, deviance).
Option C: Configurable per `deck.chrome` setting.

The question is whether the right column is noise at normal zoom or essential wayfinding. Age and → help decide whether to descend; ID is for reference. But they consume horizontal space on every line.

**Status:** Open. Evaluate after orient zoom (V9) is implemented.

### Q32: Gestures on resolved/released tensions

Creating a child under a resolved tension succeeds but leaves the parent resolved — a structural inconsistency (the closure fraction ignores it, the parent claims "done" while having active sub-structure). Options:
- **Auto-reopen:** Creating a child under a resolved tension auto-reopens it (simple, may be surprising)
- **Pathway palette:** "This tension is resolved. Reopen it first?" with options [reopen + create] [cancel]
- **Block:** Prevent child creation on resolved/released tensions entirely (too restrictive — you might want to decompose a resolved insight)

The right answer is probably the pathway palette (V4/V8 scope) — it's a structural signal that deserves a conscious decision, not an automatic one.

**Status:** Open. Address in V4 (gestures) or V8 (signals).

### Q33: Desire line right-column treatment

The desire is an anchor (bold, wide-wrapping), not a list item. Putting ID + age on the first line creates visual awkwardness — it looks crammed compared to children's naturally-spaced right columns. Options:
- **Drop from desire:** ID lives only on the bottom bar or breadcrumb. Desire stays clean bold text.
- **Below the rule:** A subtle `02 · 6d` annotation line between desire rule and route.
- **Left column companion:** `Jun  02  6d` next to the deadline.

Current state: ID + age on first line (Q25). May revert to clean desire with ID elsewhere.

**Status:** Open. Experiment.

### Q34: Edit overlay vs deck-native INPUT state

The current edit mode (e, !, ?) opens the old field-view overlay — full-screen text input with Tab cycling between desire/reality/horizon. This works functionally but feels disconnected from the deck. The breadboard specifies P3 (INPUT state) as: dimmed deck background (U37), input prompt/label (U36), text field with cursor (U35). This is a visual overhaul of the edit experience, not a logic change. The old overlay can serve until this is built.

**Status:** Open. Visual overhaul deferred — old overlay functional.

### Q35: Horizon edit silent failure

When editing horizon via Tab-cycling and the input doesn't parse, the edit is silently dropped. Now shows a transient error message. But the deeper issue: should horizon be part of the Tab cycle at all in deck mode, or should it be a separate gesture?

**Status:** Partial fix (error feedback added). Interaction design open.

### Q36: Descent into resolved/released tensions

l/→ descends into resolved and released tensions, which lets you explore their sub-structure. This is good behavior — a resolved tension may have interesting internal structure worth reviewing. No change needed.

**Status:** Confirmed as desired behavior.

### Q37: Accumulated ordering — most recent at top

Accumulated items (resolved/released) now render with most recent nearest the breathing space (top of accumulated zone) and oldest nearest reality. This matches the expectation that recent activity is more salient.

**Status:** Implemented.

---

## Requirements (R) — FINAL

| ID | Requirement | Status |
|----|-------------|--------|
| **R0** | **The deck is the primary interaction surface — console at screen center (cursor home), everything radiates outward** | Core goal |
| **R1** | **Stream view is a 4-zone layout: desire, route, console, reality** | Core goal |
| R1.1 | Desired outcome at top edge — always visible, deadline on left, age on right | |
| R1.2 | Route above console — remaining positioned steps, future-facing, shown as list at normal zoom. Selected step wraps to full text, others truncate to one line. | |
| R1.3 | Console at vertical center — the frontier of action as interaction surface. Cursor rests here by default; pitch moves outward (k/↑ toward desire, j/↓ toward reality). | |
| R1.4 | Current reality at bottom edge — always visible, age on right | |
| R1.5 | Log (this tension's prior epochs) accessed by navigating past reality downward — a transition, not a visible zone | |
| R1.6 | Parent breadcrumb above desire as screen boundary signal (`← #N parent text...`). Empty at root level. | |
| R1.7 | Log indicator below reality as screen boundary signal (`↓ N prior epochs`). | |
| **R2** | **Console contains everything action-relevant in the current epoch** | Core goal |
| R2.1 | Overdue steps (positioned steps past deadline) — shown fully, they need action | |
| R2.2 | The next committed step (primary action vector) — shown fully | |
| R2.3 | Held steps — compressed to indicator at normal zoom (`· 3 held`), expandable | |
| R2.4 | Input point — space for creating new elements, expandable inline configuration | |
| R2.5 | Resolved/released steps and notes since last epoch — compressed to indicator at normal zoom (`✓ 2 resolved · ~ 1 released · ※ 1 note`), interleaved chronologically when expanded, each type visually distinct | |
| R2.6 | Console extent is dynamic — expands and contracts with decision load; visual size is itself a signal | |
| R2.7 | Fresh epoch starts neutral (no signals); signals accumulate with activity — "no signals = fresh" as learnable heuristic | |
| R2.8 | Empty console (no children, fresh epoch) is a meaningful state — shows affordances: create child, make note, update desire, update reality | |
| **R3** | **Console aggregates signals from direct children (locality: one level)** | Must-have |
| R3.1 | Child lines get ONE annotation, only when deviant (desire changed, reality stale, etc.). No annotation = nothing noteworthy. Signal by exception. | |
| R3.2 | Child internal activity (sub-step resolution counts, freshness) shows as compressed structural summary on the child's line | |
| R3.3 | Grandchild+ mutations do NOT surface — they belong to the child's own stream view | |
| R3.4 | The compressed indicators (R2.3, R2.5) ARE the epoch summary. No separate epoch-level aggregation signal needed. | |
| **R4** | **Navigation uses pitch and roll; zoom controls density** | Must-have |
| R4.1 | Pitch (j/↓, k/↑) — continuous movement through order of operations. Cursor starts at console; k/↑ moves through route toward desire; j/↓ moves toward reality and (past it) log. | |
| R4.2 | Roll (l/→) — descend into selected step (opens its stream view as new context) | |
| R4.3 | Roll (h/←) — ascend to parent tension | |
| R4.4 | Zoom: Enter = focus (close shot), Shift+Enter = orient (long shot). See zoom specification below. | |
| R4.5 | Sibling navigation (direct, without round-tripping through parent) is a candidate gesture for #16. Must not conflict with `[`/`]` frame controls in survey view. | |
| **R5** | **Plan-facing and trace-facing information are distinguishable and consistently positioned** | Must-have |
| R5.1 | Left column = intent (deadlines, temporal commitments). Right column = trace (IDs, children indicator →N, resolution dates, drift facts, age). | |
| R5.2 | Route items have trace data: at minimum creation/update age on right. | |
| R5.3 | Vertical position reflects order of operations, NOT calendar time. | |
| **R6** | **Signal by exception — silence is the default** | Must-have |
| R6.1 | Screen boundary signals: parent breadcrumb above (with ← hint), log indicator below (with ↓ hint). Left/right edges are ambient (structural depth via indentation, children via →N indicator), not text content. | |
| R6.2 | Stable state is visually quiet; deviations pop. | |
| **R7** | **Epoch mechanics govern console contents** | Must-have |
| R7.1 | Desire change closes current epoch (clean close) — accumulated facts compress into log | |
| R7.2 | Reality update closes current epoch — same compression | |
| R7.3 | Resolution of the tension closes its epoch (terminal) | |
| R7.4 | Release of the tension closes its epoch (terminal) | |
| R7.5 | Reopened tension creates a new epoch (epochs are immutable once closed) | |
| R7.6 | New epoch inherits reality as-is; if opened by desire change, reality is marked stale ("written about a different desire") | |
| R7.7 | If no activity occurred, epoch boundary is lightweight (nothing to compress) | |
| **R8** | **The TUI has distinct experiential states** | Must-have |
| R8.1 | NORMAL — navigating. Pitch/roll/zoom available. The deck at rest. | |
| R8.2 | INPUT — creating or editing text (desire, reality, note). Text input active, other content dims but stays visible as context. | |
| R8.3 | FOCUSED — zoomed into one element. Its detail fills the console. All mutations relevant to that element available (resolve, release, edit, note, add child). Zoom out or navigate within. | |
| R8.4 | PATHWAY — a decision fork. 3-5 options inline. Select, confirm, or dismiss. | |
| R8.5 | Full state × gesture transition table deferred to #16. | |
| **R9** | **Layout responds intelligently to available space** | Must-have |
| R9.1 | Primary targets: full screen (120×40+), half screen (80×40), sidecar (40-60×40+) | |
| R9.2 | Intelligent compression governed by constraint hierarchy: edges compress first (peripheral blurs), center persists (focal point stays sharp). | |
| R9.3 | Design for comfort (full/half screen) first; graceful degradation for narrow. | |
| R9.4 | Step text truncation behavior (selected wraps, others one-line) may be configurable. | |

### Zoom specification (R4.4)

**Principle:** Zoom is focal length. Center sharpens or blurs relative to edges. Spatial order always preserved.

**ORIENT (long shot) — Shift+Enter from normal:**

Edges gain weight. Parent expands from breadcrumb to full desire/reality. Siblings become visible. Console items show grandchild counts. Route compresses to summary. Center stays compact.

```
  ← #2 werk is a mature tool for practicing structural
       dynamics, used daily by at least 10 practitioners  Jun
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  siblings: #23 skills ✓ · #46 yank                    [dim]
  ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─

  Mar      TUI rebuilt around operating envelope               4d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
           4 remaining · next Mar 24
  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄
  Mar 18   ▸ finalize spec              OVERDUE    #18 →2  3d
  Mar 22   ▸ write frontier comp.      ← cursor   #17      5d
           · 3 held · ✓ 2 resolved · ~ 1 released · ※ 1 note
  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄

           TUI exists with field chart layout, gaze
           cards, temporal indicators                          3d
  ──────────────────────────────────────────────────────────────
           werk is FrankenTUI-first, field chart works,        5d
           no conceptual architecture implemented yet
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**NORMAL (medium shot) — default:**

Balanced. Route as step list, console compact, desire/reality as anchors.

```
  ← #3 werk is a FrankenTUI-first application...         [dim]

  Mar      TUI rebuilt around operating envelope               4d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Mar 30   design threshold mechanics                  #22      3d
  Mar 28   implement survey view                       #21      5d
  Mar 26   build temporal spine                        #20      5d
  Mar 24   implement data model                        #19 →4   2d

  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄
  Mar 18   ▸ finalize spec                    OVERDUE  #18 →2
  Mar 22   ▸ write frontier computation       ← here  #17
           · 3 held
           + ___
           ✓ 2 resolved · ~ 1 released · ※ 1 note
  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄

           TUI exists with field chart layout, gaze
           cards, temporal indicators                          3d
  ──────────────────────────────────────────────────────────────
  ↓ 2 prior epochs                                       [dim]
```

**FOCUS (close shot) — Enter on a selected element:**

Center expands. Everything above the focused element compresses to top indicator. Everything below compresses to bottom indicator. Spatial order preserved. Children shown individually.

Example: focusing on #19 from the route:

```
  ← #3 werk is a FrankenTUI-first application...         [dim]

  Mar      TUI rebuilt around operating envelope               4d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ▲ 3 more in route
  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄

  Mar 24   implement data model                ← focus #19 →4

           implement the persistence layer for deck
           state, frontier computation, and epoch              4d
           boundaries
           ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
           ▸ write migration scripts                   #24 →0
           ▸ integrate with render loop                #25 →0
           · benchmark query performance               #26
           ✓ design schema                             #27
           ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
           schema designed, migration written, not
           yet integrated with the TUI render loop             2d

  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄
  ▼ 1 overdue · ▸ next · 3 held · ✓ 2 resolved · ~ 1 · ※ 1

           TUI exists with field chart layout, gaze
           cards, temporal indicators                          3d
  ──────────────────────────────────────────────────────────────
```

---

## Architectural Insights (captured for future shaping)

### The accreted structure as compiled model

Captured in full in `designs/werk-conceptual-foundation.md` Part II, "The Logbook as Compiled Model."

Key points: the logbook is compacted meaning, not dead history. Live structure + logbook = complete project model. Logbook should be queryable as structure: semantic search, temporal bucketing, cross-tension correlation, decision chain reconstruction. Reference: cass (coding agent session search) architecture — multi-backend search (BM25 + semantic + hybrid), temporal filtering, context windows, relationship discovery, aggregation, field selection.

### Log / Logbook distinction

- **Log** — one tension's epoch sequence. Linear, may fork at splits. The unit of history.
- **Logbook** — the composite lattice of all logs. A DAG linked by provenance (splits/merges), temporal correlation, and semantic content. The queryable whole. Captain's logbook — the bound volume of the voyage.

---

## Out of Scope

- Survey view (#18) — separate shaping
- Threshold mechanics (#19) — separate shaping
- Logbook/log design — future work (see Architectural Insights; foundation doc updated)
- Ground mode — too vague yet
- Session lifecycle (takeoff/landing) — depends on threshold mechanics
- Yaw toggle (depends on survey view existing)
- Root-level view — acknowledged as different, design deferred

---

## Shapes

The R set is specific enough that the shape space is narrow. The layout (4-zone), navigation (pitch/roll/zoom), and epoch mechanics are constrained. The shape alternatives cluster around four axes:

1. **Visual density / chrome** — how much line-drawing, whitespace, separation
2. **Compression hierarchy** — what compresses first when space is tight
3. **Peek mechanism** — how children preview works (Q8)
4. **Signal rendering** — how deviations surface visually

### A: Quiet Instrument

Maximally sparse. The instrument recedes; the content speaks.

| Part | Mechanism |
|------|-----------|
| **A1** | **Separators are whitespace.** Blank lines between zones. No line-drawing characters except the desire rule (━━━) and reality rule (──). Console boundaries are blank lines, not dotted lines. |
| **A2** | **Monochrome palette.** Two weights: normal text and dim text. Dim for: parent breadcrumb, log indicator, ages, IDs, glyphs. Normal for: desire, step text, reality, cursor. No color. |
| **A3** | **Route compresses first.** When space is tight: route becomes count (`▲ 4 remaining · next Mar 24`), console stays expanded. Rationale: the route is future/theory; the console is action/now. |
| **A4** | **Focus-only peek.** No inline expansion. Enter to focus, Shift+Enter to return. The →N indicator tells you children exist; focus shows them. Simplest interaction model. |
| **A5** | **Signals as text annotations.** Deviations show as dim text after the step: `OVERDUE`, `desire changed 2d`, `stale 5d`. No color, no special glyphs beyond the base set (▸ · ✓ ~ ※). |
| **A6** | **Console indicators always show category words.** `✓ 2 resolved · ~ 1 released · ※ 1 note` — never abbreviated to just glyphs+counts. |

**Normal zoom in Shape A:**
```
  ← #3 werk is a FrankenTUI-first application...

  Mar      TUI rebuilt around operating envelope               4d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Mar 30   design threshold mechanics                  #22      3d
  Mar 28   implement survey view                       #21      5d
  Mar 26   build temporal spine                        #20      5d
  Mar 24   implement data model                        #19 →4   2d

  Mar 18   ▸ finalize spec                    OVERDUE  #18 →2
  Mar 22   ▸ write frontier computation       ← here   #17
           · 3 held
           + ___
           ✓ 2 resolved · ~ 1 released · ※ 1 note

  TUI exists with field chart layout, gaze
  cards, temporal indicators                                   3d
  ──────────────────────────────────────────────────────────────
  ↓ 2 prior epochs
```

**Feel:** A blank page with ink on it. Calm. The content *is* the instrument. Nothing decorates or frames beyond the minimum. The user who knows the structure sees everything; the user who doesn't sees clean text.

---

### B: Structured Cockpit

Line-drawing defines zones. One accent color for the action zone. Denser information.

| Part | Mechanism |
|------|-----------|
| **B1** | **Line-drawing separators.** ━━━ for desire, ┄┄┄ for console boundaries, ── for reality. Trunk line (│) connects route steps. Zone boundaries are visible structure. |
| **B2** | **Monochrome + one accent.** Base palette is dim/normal like Shape A. One accent color (cyan or amber) used ONLY for: cursor line, OVERDUE text, the console boundary lines. Everything else is neutral. |
| **B3** | **Symmetric compression.** Route and console compress equally — route loses items top-down, console compresses indicators. Neither zone is privileged over the other. |
| **B4** | **Inline expansion peek.** Pressing a peek key (e.g., Space) on a step expands a compact card inline — shows desire snippet, children list, reality snippet. Closes on next navigation. Like the old gaze but lighter — no panel chrome, just indented content appearing below the step. |
| **B5** | **Signals as gutter marks + text.** Left gutter (1 char) shows a signal dot: amber for overdue, dim dot for stale. Text annotations supplement: `OVERDUE`, `desire changed`. The gutter provides scannable vertical signal before you read the text. |
| **B6** | **Ordinals visible.** Left column shows order position: `4  Mar 18` for the 4th step in the order of operations. Makes the sequence explicit. |

**Normal zoom in Shape B:**
```
  ← #3 werk is a FrankenTUI-first application...

  Mar      TUI rebuilt around operating envelope               4d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  │ 8  Mar 30   design threshold mechanics             #22      3d
  │ 7  Mar 28   implement survey view                  #21      5d
  │ 6  Mar 26   build temporal spine                   #20      5d
  │ 5  Mar 24   implement data model                   #19 →4   2d

  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄
● 4  Mar 18   ▸ finalize spec                 OVERDUE  #18 →2
  3  Mar 22   ▸ write frontier computation    ← here   #17
              · 3 held
              + ___
              ✓ 2 resolved · ~ 1 released · ※ 1 note
  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄

           TUI exists with field chart layout, gaze
           cards, temporal indicators                          3d
  ──────────────────────────────────────────────────────────────
  ↓ 2 prior epochs
```

**Feel:** A well-organized instrument panel. The line-drawing gives structural clarity. The trunk line (│) makes the route feel like a connected sequence. The gutter dot (●) draws the eye to the overdue step before you read the text. Ordinals make position explicit. More "designed" than Shape A.

---

### C: Adaptive Hybrid

Starts quiet, gains structure under pressure. The instrument responds to its own content.

| Part | Mechanism |
|------|-----------|
| **C1** | **Separators appear contextually.** Console boundaries (┄┄┄) appear when the console has 2+ zones of content. When the console is simple (just next step + input), no boundary lines. The instrument adds chrome only when disambiguation is needed. |
| **C2** | **Monochrome + one accent**, same as B2. Accent for cursor, OVERDUE, console boundaries. |
| **C3** | **Route compresses first** (like A3), but with a nuance: the first and last route items persist longest (first = most imminent after the console, last = most distant — the bookends give trajectory shape even when the middle compresses). |
| **C4** | **Hybrid peek.** Focus (Enter) for full detail. BUT: if you pause on a step with children (dwell for ~500ms or press Space), a lightweight inline expansion shows — just the children list, no desire/reality. Quick glance without full context switch. |
| **C5** | **Signals as text only** (like A5), but with intensity: signals that have been true for a long time get stronger visual weight (e.g., `OVERDUE` in normal weight for 1 day, bold/bright for 7 days). Time amplifies signal. |
| **C6** | **Ordinals hidden at normal zoom, visible at orient.** Clean at working distance, explicit when you step back for overview. |

**Normal zoom in Shape C (light console — 1 overdue, 1 next, no held, no resolved):**
```
  ← #3 werk is a FrankenTUI-first application...

  Mar      TUI rebuilt around operating envelope               4d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Mar 30   design threshold mechanics                  #22      3d
  Mar 28   implement survey view                       #21      5d
  Mar 26   build temporal spine                        #20      5d
  Mar 24   implement data model                        #19 →4   2d

  Mar 18   ▸ finalize spec                    OVERDUE  #18 →2
  Mar 20   ▸ write frontier computation       ← here   #17
           + ___

           TUI exists with field chart layout, gaze
           cards, temporal indicators                          3d
  ──────────────────────────────────────────────────────────────
  ↓ 2 prior epochs
```

**Normal zoom in Shape C (heavy console — overdue, next, held, resolved, notes):**
```
  ← #3 werk is a FrankenTUI-first application...

  Mar      TUI rebuilt around operating envelope               4d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Mar 30   design threshold mechanics                  #22      3d
  Mar 28   implement survey view                       #21      5d
  Mar 26   build temporal spine                        #20      5d
  Mar 24   implement data model                        #19 →4   2d

  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄
  Mar 18   ▸ finalize spec                    OVERDUE  #18 →2
  Mar 22   ▸ write frontier computation       ← here   #17
           · 3 held
           + ___
           ✓ 2 resolved · ~ 1 released · ※ 1 note
  ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄ ┄

           TUI exists with field chart layout, gaze
           cards, temporal indicators                          3d
  ──────────────────────────────────────────────────────────────
  ↓ 2 prior epochs
```

**Feel:** The instrument adapts to its content. Light state → clean. Heavy state → structured. Chrome appears when needed and recedes when not. The instrument breathes with the structure.

---

## Fit Check

| Req | Requirement | Status | A | B | C |
|-----|-------------|--------|---|---|---|
| R0 | Deck is primary interaction surface, console center | Core goal | ✅ | ✅ | ✅ |
| R1 | 4-zone layout: desire, route, console, reality | Core goal | ✅ | ✅ | ✅ |
| R2 | Console contains epoch action-relevant items | Core goal | ✅ | ✅ | ✅ |
| R3 | Console aggregates child signals (locality) | Must-have | ✅ | ✅ | ✅ |
| R4 | Pitch/roll/zoom navigation | Must-have | ✅ | ✅ | ✅ |
| R5 | Left=intent, right=trace, consistently positioned | Must-have | ✅ | ✅ | ✅ |
| R6 | Signal by exception — silence default | Must-have | ✅ | ✅ | ✅ |
| R7 | Epoch mechanics govern console | Must-have | ✅ | ✅ | ✅ |
| R8 | Distinct experiential states | Must-have | ✅ | ✅ | ✅ |
| R9 | Intelligent compression for available space | Must-have | ✅ | ✅ | ✅ |

**Notes:** All shapes pass all requirements — the R set constrains the layout/behavior, shapes vary on treatment. The differentiators are:

| Axis | A: Quiet | B: Structured | C: Adaptive |
|------|----------|---------------|-------------|
| Chrome | Whitespace only | Line-drawing always | Contextual — appears under load |
| Color | None (dim/normal) | One accent (cursor, overdue, boundaries) | One accent (cursor, overdue, boundaries) |
| Compression | Route first | Symmetric | Route first, bookends persist |
| Peek | Focus-only | Inline expansion | Dwell/Space for children, Enter for full |
| Ordinals | Hidden | Always visible | Orient-only |
| Signal rendering | Text annotations | Gutter marks + text | Text with time-amplified intensity |

---

## Selected Shape: D (C + configurability)

**Shape D = C1 + C2 + C3 + C4 + C5 + C6 + D7 (configuration layer)**

Shape C as the default experience, with visual treatment options configurable via ground mode, CLI, or MCP.

| Part | Mechanism | Flag |
|------|-----------|:----:|
| **C1** | **Separators appear contextually.** Console boundaries (┄┄┄) appear when console has 2+ zones of content. Chrome only when disambiguation is needed. | |
| **C2** | **Monochrome + one accent.** Accent (cyan default) for cursor, OVERDUE, console boundaries when visible. Everything else dim/normal. | |
| **C3** | **Route compresses first, bookends persist.** Route → count when space is tight, but first and last items persist longest for trajectory shape. | |
| **C4** | **Hybrid peek.** Space on a step with children → lightweight inline children list. Enter → full focus. Shift+Enter → orient. | |
| **C5** | **Time-amplified signals.** Text annotations whose visual weight intensifies with duration. `OVERDUE` normal for 1 day, bold/bright for 7+. | |
| **C6** | **Ordinals at orient, hidden at normal.** Clean working state, explicit overview. | |
| **D7** | **Configuration layer.** See table below. Configurable via `werk config`, MCP tool, or ground mode settings surface. | |

### D7: Configurable options

| Setting | Default (Shape C) | Alternatives | Config key |
|---------|-------------------|--------------|------------|
| Chrome level | `adaptive` — contextual separators | `quiet` (whitespace only, Shape A), `structured` (always visible, Shape B) | `deck.chrome` |
| Color mode | `accent` — mono + one accent color | `mono` (no color, Shape A), `accent` (default) | `deck.color` |
| Accent color | `cyan` | `amber`, `green`, or any ANSI color | `deck.accent` |
| Ordinals | `orient` — visible only at orient zoom | `always` (Shape B), `never` (Shape A) | `deck.ordinals` |
| Peek style | `hybrid` — Space for children, Enter for full | `focus-only` (Shape A), `inline` (Shape B — full card) | `deck.peek` |
| Signal style | `amplified` — intensity grows with time | `flat` (constant weight, Shape A/B) | `deck.signals` |
| Compression | `route-first` — route compresses before console | `symmetric` (Shape B) | `deck.compression` |
| Step text wrap | `selected` — selected step wraps, others truncate | `always` (all wrap), `never` (all truncate) | `deck.wrap` |
| Trunk line | `off` — no trunk connecting route steps | `on` (Shape B — │ connects route) | `deck.trunk` |
| Gutter signals | `off` — no left gutter marks | `on` (Shape B — ● for overdue, · for stale) | `deck.gutter` |

**Sacred (NOT configurable):**
- 4-zone layout (desire, route, console, reality)
- Column structure (left=intent, right=trace)
- Zoom levels and focal length principle
- Pitch/roll navigation semantics
- Epoch mechanics
- Signal by exception principle
- Locality (one-level signal propagation)

The configuration surface is the visual *treatment* layer. The structural *architecture* is invariant.
