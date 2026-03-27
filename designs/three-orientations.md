# Three Orientations and the NOW Zone

**Date:** 2026-03-27
**Tensions:** #18 (survey view), #90 (command center)
**Foundation:** [werk-conceptual-foundation.md](./werk-conceptual-foundation.md)
**Prior:** [command-center-and-time-axis.md](./command-center-and-time-axis.md)

---

## The Three Times

The foundation document describes pitch (time axis) and yaw (structure ↔ time orientation) but treats time as singular. In practice, three distinct temporal concepts organize action:

**Chronos** — calendar/clock time. When things are due. External, shared, measured in dates. Deadlines, horizons, "April 10." The survey view organizes by chronos: bands of temporal proximity to the calendar present.

**Taxis** — sequence/order. What comes before what. Internal, structural, measured in position. The order of operations in a theory of closure. The stream/deck view organizes by taxis: steps arranged in the user's committed sequence, independent of when they're due.

**Kairos** — the right moment. What's ripe for action NOW. Neither calendar position nor sequence position, but a synthesis: what is ready, unblocked, pressing, and actionable at this moment. The operating envelope in the deck computes this for one tension — kairos at local scope.

These three aren't alternatives. They're independent axes that can agree or conflict:
- A step can be first in sequence (taxis) but not due for months (chronos) and not ripe yet (kairos — its predecessor isn't done).
- A step can be overdue (chronos), unpositioned (no taxis), but screaming for attention (kairos).
- A step can be next in sequence (taxis), due tomorrow (chronos), AND the most urgent thing in the field (kairos). All three align. This is the clear-path case.

Conflict between these is structurally meaningful:
- **Taxis vs chronos** = sequencing pressure (foundation: "order says wait, deadline says now")
- **Chronos vs kairos** = temporal pressure without readiness (deadline approaching but blocked)
- **Kairos vs taxis** = opportunity out of order (something lower in sequence is ripe before something higher)

---

## Three Views as Projections

Each view foregrounds one temporal axis and shows the others as annotations:

| View | Primary axis | j/k navigates | Shows secondarily | The question it answers |
|------|-------------|---------------|-------------------|----------------------|
| **Stream** | Taxis (sequence) | Steps in order of operations | Chronos on left spine, kairos via envelope | "What's my plan for closing this gap?" |
| **Survey** | Chronos (calendar) | Tensions by deadline proximity | Structure via tree lines, taxis implied by position | "What's due when across the whole field?" |
| **Frontier** | Kairos (ripeness) | Action-ready items by urgency | Structure + chronos as context | "What should I do RIGHT NOW?" |

The frontier view doesn't exist yet. The survey was the second orientation. The frontier would be the third.

---

## The Frontier View: Kairos as Primary Axis

### What it shows

The frontier view shows every tension's **operating envelope** — the zone of actionable items — collapsed into a single cross-field surface. It answers: "across everything I'm working on, what is most ripe for action?"

Where the survey groups by WHEN things are due, the frontier groups by WHY they're ripe:

**Ripe because overdue** — deadline passed, step unresolved. Urgency is chronos-driven.

**Ripe because next** — this is the frontier step in its tension's sequence. The predecessor is done. The path is clear. Urgency is taxis-driven.

**Ripe because blocking** — this step is on the critical path of a parent tension. Until it moves, nothing above it can advance. Urgency is structural.

**Ripe because converging** — multiple tensions' next steps point to the same action or the same resource. Doing this one thing advances several concerns. Urgency is leverage.

**Ripe because neglected** — this tension hasn't been touched in N sessions while others have. Not urgent by any clock, but the pattern of attention has starved it. Urgency is balance.

### How it organizes

The frontier view doesn't use time bands. It uses **action-readiness tiers**:

```
── act now ────────────────────────────────────────
  ▸ survey view designed...       #18 [next] ← #15
  ▸ first public post...          #41 [next, overdue] ← #37

── unblock ────────────────────────────────────────
  → CLI is ergonomic...           #52 [critical path] ← #10

── attend ─────────────────────────────────────────
  → Waterlight collaboration...   #62 [neglected 3w] ← #36
  → Fritz's tool researched...    #79 [neglected 4w] ← #78

── prepare ────────────────────────────────────────
  · epoch creation trigger...     #30 [held, approaching] ← #13
  · staging mechanism...          #48 [held, approaching] ← #10
```

The tiers aren't hardcoded — they emerge from the computation:
- **Act now**: frontier step + (overdue OR deadline imminent OR clear path)
- **Unblock**: on critical path, predecessor not yet resolved
- **Attend**: neglected (no gesture in >2× median inter-session gap)
- **Prepare**: held steps with approaching deadlines (could be positioned)

### How it differs from the survey

The survey says: "here's everything organized by WHEN it's due."
The frontier says: "here's everything organized by WHY you should touch it."

The survey is neutral — it shows temporal facts. The frontier is opinionated — it computes action-relevance from multiple signals. The survey is the map. The frontier is the navigator's recommendation.

### Relationship to the deck's envelope

The deck's operating envelope computes kairos for ONE tension (the next step, overdue steps, held steps). The frontier view does the same computation across ALL tensions and merges the results. It's the field-wide envelope.

This means the frontier view's data source is: for each active tension, run the envelope computation, then rank and merge the results by action-readiness.

---

## The NOW Zone in the Survey

The survey currently runs from top (future/later) to bottom (past/overdue) with no center. But NOW is the most important place — it's where chronos meets the present moment.

### Where NOW falls

NOW is between "imminent" (approaching from above) and "overdue" (receding below). It's the frontier of the chronos axis, analogous to the frontier of the taxis axis in the deck.

```
── later ──────────────────────────────────────────
  ...

── approaching ────────────────────────────────────
  ...

── imminent ───────────────────────────────────────
  ...

════════════════════════ NOW ══════════════════════

── overdue ────────────────────────────────────────
  ...
```

### What NOW affords

The NOW zone in the survey could show aggregate information — not individual tensions, but field-wide facts:

- **Field vitals**: 52 active · 3 overdue · 7 imminent · 12 held unframed
- **Session summary**: last session 6h ago · 4 gestures · 2 tensions touched
- **Held pressure**: 12 held steps across 8 tensions (uncommitted potential)
- **Reality freshness**: 3 tensions with stale reality (>1 week since last update)

This is the "command center" data from tension #90, surfaced at the temporal center of the survey rather than as a separate view. The NOW zone IS the command center, embedded in the time axis.

### Cursor behavior at NOW

When the cursor reaches the NOW line:
- j/k crosses it (moves from imminent to overdue or vice versa)
- Enter on the NOW zone could toggle an expanded vitals display
- The NOW zone expands/contracts based on how much field-level information is action-relevant

---

## How the Three Views Coexist

### Navigation

| Key | Action |
|-----|--------|
| Tab | Cycle: stream → survey → frontier → stream |
| Shift+Tab | Return to previous view (same as current) |

Or: Tab toggles between stream and the last-used field-wide view (survey or frontier). A separate key (maybe `[`/`]` or a mode key) switches between survey and frontier. This avoids a three-way Tab cycle which could feel disorienting.

**Proposed**: Tab = stream ↔ survey (current). A new key switches the survey between chronos mode and kairos mode. The survey view becomes a container for two temporal orientations:

| Key | From stream | From survey |
|-----|------------|-------------|
| Tab | → survey (chronos) | → stream (pivot) |
| Shift+Tab | → survey (resume) | → stream (return) |
| `f` in survey | switch to frontier (kairos) mode | — |
| `s` in frontier | switch to survey (chronos) mode | — |

### The Command Center

The command center (#90) was designed as the zoomed-out view where both axes are visible. With three temporal concepts, the command center becomes:

- **Structure axis** (horizontal): tensions as columns
- **Temporal axis** (vertical): whichever temporal mode is active (chronos/taxis/kairos)
- **NOW** at center: the present moment as crossing point

The three views are zoom levels on this surface:
- Stream = zoomed tight on one column (one tension, full taxis depth)
- Survey = zoomed tight on one row (all tensions, chronos bands)
- Frontier = zoomed tight on one row (all tensions, kairos tiers)
- Command center = zoomed out (multiple columns × temporal depth)

---

## Risk: Bleeding Things Together

The user correctly identified the risk of bleeding. Three views + command center = four modes, potentially confusing. The defense:

1. **Tab is always stream ↔ field-wide.** Just two modes the user switches between. Which field-wide view (survey/frontier) is a sub-choice within the field-wide mode.

2. **The views share the same data and cursor.** Switching between them pivots on the selected tension. You're always looking at the same structure from different angles.

3. **Each view answers a distinct question.** If the user knows which question they're asking, they know which view to use:
   - "What's my plan?" → stream
   - "What's due when?" → survey
   - "What should I do now?" → frontier

4. **Start with two, add the third when it earns its place.** The frontier view is the most computationally complex (it requires envelope computation across all tensions). Build it after the survey is solid. The survey's NOW zone can carry some of the frontier's purpose in the interim.

---

## Open Questions

1. **Should the frontier view be a separate mode or a sort option within the survey?** Pressing a key in the survey could re-sort all items by action-readiness instead of deadline. Same data, different ordering. This is simpler than a fully separate view.

2. **How does the frontier compute "ripe"?** The envelope computation for one tension is well-defined (frontier.rs). Extending it across all tensions requires: (a) running per-tension envelope, (b) ranking results, (c) deduplication (a tension appears in multiple parents' envelopes). The ranking function is the design decision.

3. **Does kairos need its own time-like axis, or is it a computed property that can be shown as an annotation on the other two views?** If kairos is just "urgency shading" (the horizon-chart idea from the viz research), it might not need its own view — it could be a layer toggled on any view.

4. **The NOW zone: is it a fixed element in the survey or does it only appear when there's actionable information?** Signal by exception says: no NOW zone when everything is calm. NOW appears (expands) when there's overdue items, stale realities, or held pressure.
