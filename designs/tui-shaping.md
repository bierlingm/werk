# TUI Rebuild — Shaping Doc

**Scope:** Stream view only. Survey view (#18), threshold mechanics (#19), ledger, and ground mode are out of scope.
**Source:** designs/werk-conceptual-foundation.md, tensions #15, #18, #19, shaping sessions 2026-03-23/24.
**Status:** Shaping — R finalized, ready for shapes

---

## Vocabulary Decisions (this shaping)

| Foundation term | Instrument term | Rationale |
|----------------|----------------|-----------|
| Operating envelope (as full view) | **Deck** | Flight deck / ship's deck — the full working surface. "On deck" = action-relevant. "Clear the deck" = epoch compression. |
| Operating envelope (as action zone) | **Console** | Cockpit console — the action zone at center where signals converge and you operate from. The deck contains the console. |
| Remaining theory / plan | **Map** | Cartographic — the planned route ahead, above the console. |
| Ghost geometry (one tension) | **Log** | One tension's epoch sequence. Linear (may fork at splits). "Show me #15's log." |
| Ghost geometry (composite) | **Ledger** | The full lattice of all logs, linked by provenance, temporal correlation, and semantic content. Queryable as structure. "Search the ledger for X." |
| Epoch boundary on desire change | **Clean close** | Desire change closes epoch, reality carried forward marked stale, new epoch opens neutral. |

### Spatial hierarchy

```
Deck (the full view)
├── Parent breadcrumb (screen boundary signal — ← hint)
├── Desire (top anchor — always visible, deadline on left)
├── Map (remaining theory — positioned steps, future-facing)
├── Console (frontier action zone — center, cursor home)
├── Reality (bottom anchor — always visible)
└── Ledger indicator (screen boundary signal — ↓ hint)
         ↓ Log (this tension's epochs, navigated into)
```

### Column layout

Three columns, fixed alignment regardless of content width:

```
LEFT COL       MAIN COL                                      RIGHT COL
(deadline)     (text — desire and children share alignment)   (ID, children→N, trace/age)
```

Left column: fixed width accommodating longest deadline in view (e.g., 7 chars for "Mar 30 ").
Main column: flexible, shares alignment between desire and all children.
Right column: right-aligned facts — tension ID (`#19`), children indicator (`→3`), trace age (`2d`).

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

Normal zoom is the hybrid model: fixed anchors (desire/reality always visible), map visible as step list above console, console compact with compressed indicators. The deck-as-stage epiphany lives as focus-zoom behavior — focus renders one element's detail in the console area.

### Q3: Zoom levels → Focal length principle

**Zoom adjusts the weight ratio between center and edges.** It is focal length, not content switching. You're always looking at the same structure; zoom changes what's sharp and what's peripheral.

| Zoom | Center | Edges | Metaphor |
|------|--------|-------|----------|
| **Orient** | Compresses — console/map lose detail | Gain weight — parent, siblings, grandchild signals become visible | Stepping back from painting to see it in the room |
| **Normal** | Balanced — map as list, console compact | Balanced — desire/reality as anchors, boundary signals subtle | Standing at viewing distance |
| **Focus** | Expands — one element fills console with full detail | Compress/blur — everything else becomes indicators at edges | Leaning in to examine brushwork |

Overflow: extra space → center gets more room (the sharp part breathes).
Underflow: too little space → edges compress first (peripheral blurs more aggressively).

### Q5: Left/right spine → Column-based with contextual content

Left column = intent (deadlines). Right column = trace (IDs, children indicator, ages, facts). Content is contextual to zone:
- Desire: deadline left, age right
- Map items: deadline left, ID + children indicator + age right
- Console items: deadline left (for overdue/next), ID + trace right
- Reality: age right

### Q6: Above desire → Parent breadcrumb

Above desire is a screen boundary signal: the parent tension as a breadcrumb line with `←` hint indicating h/← takes you there. At root level (no parent), empty.

### Q7: Zoom overflow/underflow → Follows focal length principle

Extra space → center breathes (compressed elements partially uncompress). Too little space → edges compress first per constraint hierarchy.

---

## Requirements (R) — FINAL

| ID | Requirement | Status |
|----|-------------|--------|
| **R0** | **The deck is the primary interaction surface — console at screen center (cursor home), everything radiates outward** | Core goal |
| **R1** | **Stream view is a 4-zone layout: desire, map, console, reality** | Core goal |
| R1.1 | Desired outcome at top edge — always visible, deadline on left, age on right | |
| R1.2 | Map above console — remaining positioned steps, future-facing, shown as list at normal zoom. Selected step wraps to full text, others truncate to one line. | |
| R1.3 | Console at vertical center — the frontier of action as interaction surface. Cursor rests here by default; pitch moves outward (k/↑ toward desire, j/↓ toward reality). | |
| R1.4 | Current reality at bottom edge — always visible, age on right | |
| R1.5 | Log (this tension's prior epochs) accessed by navigating past reality downward — a transition, not a visible zone | |
| R1.6 | Parent breadcrumb above desire as screen boundary signal (`← #N parent text...`). Empty at root level. | |
| R1.7 | Ledger access indicator below reality as screen boundary signal (`↓ N prior epochs`). | |
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
| R4.1 | Pitch (j/↓, k/↑) — continuous movement through order of operations. Cursor starts at console; k/↑ moves through map toward desire; j/↓ moves toward reality and (past it) log. | |
| R4.2 | Roll (l/→) — descend into selected step (opens its stream view as new context) | |
| R4.3 | Roll (h/←) — ascend to parent tension | |
| R4.4 | Zoom: Enter = focus (close shot), Shift+Enter = orient (long shot). See zoom specification below. | |
| R4.5 | Sibling navigation (direct, without round-tripping through parent) is a candidate gesture for #16. Must not conflict with `[`/`]` frame controls in survey view. | |
| **R5** | **Plan-facing and trace-facing information are distinguishable and consistently positioned** | Must-have |
| R5.1 | Left column = intent (deadlines, temporal commitments). Right column = trace (IDs, children indicator →N, resolution dates, drift facts, age). | |
| R5.2 | Map items have trace data: at minimum creation/update age on right. | |
| R5.3 | Vertical position reflects order of operations, NOT calendar time. | |
| **R6** | **Signal by exception — silence is the default** | Must-have |
| R6.1 | Screen boundary signals: parent breadcrumb above (with ← hint), log/ledger indicator below (with ↓ hint). Left/right edges are ambient (structural depth via indentation, children via →N indicator), not text content. | |
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

Edges gain weight. Parent expands from breadcrumb to full desire/reality. Siblings become visible. Console items show grandchild counts. Map compresses to summary. Center stays compact.

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

Balanced. Map as step list, console compact, desire/reality as anchors.

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

Example: focusing on #19 from the map:

```
  ← #3 werk is a FrankenTUI-first application...         [dim]

  Mar      TUI rebuilt around operating envelope               4d
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ▲ 3 more in map
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

Captured in full in `designs/werk-conceptual-foundation.md` Part II, "The Ledger as Compiled Model."

Key points: the ledger is compacted meaning, not dead history. Live structure + ledger = complete project model. Ledger should be queryable as structure: semantic search, temporal bucketing, cross-tension correlation, decision chain reconstruction. Reference: cass (coding agent session search) architecture — multi-backend search (BM25 + semantic + hybrid), temporal filtering, context windows, relationship discovery, aggregation, field selection.

### Log / Ledger distinction

- **Log** — one tension's epoch sequence. Linear, may fork at splits. The unit of history.
- **Ledger** — the composite lattice of all logs. A DAG linked by provenance (splits/merges), temporal correlation, and semantic content. The queryable whole.

---

## Out of Scope

- Survey view (#18) — separate shaping
- Threshold mechanics (#19) — separate shaping
- Ledger/log design — future work (see Architectural Insights; foundation doc updated)
- Ground mode — too vague yet
- Session lifecycle (takeoff/landing) — depends on threshold mechanics
- Yaw toggle (depends on survey view existing)
- Root-level view — acknowledged as different, design deferred

---

## Shapes

(R finalized — exploring shapes next)
