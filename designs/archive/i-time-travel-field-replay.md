# Design I: Time-Travel Field Replay

**Status:** Proposed
**Priority:** Next major feature
**Depends on:** sd-core mutation log, DynamicsEngine

## Thesis

The single most accretive feature werk can add is temporal field replay — letting the user scrub backward through time and see the entire tension field reconstructed at any historical moment, with all 13 dynamics recomputed for that point.

Every other feature has diminishing returns. This one compounds in value every day the tool is used.

## Why This Is the One

### Compounds forever
A year of mutation data becomes a structural autobiography. The longer you use werk, the richer the replay. No other feature has this property.

### Theoretically core
Fritz's entire framework is about recognizing your own structural tendencies — oscillation patterns, compensating strategies, neglect cycles. You can't see those from a snapshot. You need the film, not the photograph. Time-travel makes the theory's deepest insight accessible: *understanding how you move through structural tension over time is what enables you to change*.

### Technically elegant
No new dependencies, no LLM integration, no external services. Pure computation over data already stored. The mutation log is append-only by design — replaying it to any timestamp is a natural operation. The dynamics engine already computes all 13 dynamics for any tension state. The TUI already renders dashboards, trees, and detail views. This feature combines existing infrastructure in a novel way.

### Nobody else has this
Task managers show what's on your plate. Journals show what you wrote. This shows *how you move through creative tension over time* — when you oscillate, when you break through, what conditions precede resolution vs. stagnation. It's a new category of self-knowledge tool.

## Feature Specification

### Core: Point-in-Time Reconstruction

The foundation is `engine.reconstruct_at(timestamp)` in sd-core:

1. Query all tensions that existed at timestamp T (created_at <= T, not yet deleted)
2. Replay mutations up to T to reconstruct each tension's state at that moment
3. Recompute dynamics for the reconstructed field
4. Return a `FieldSnapshot` — the full tension set + computed dynamics as they would have been at time T

This is the hard part. Everything else is rendering a state the TUI can already render.

### View: Replay Mode

Activated via `Cmd+R` or command palette action "Time Travel".

**Layout:**
- Top: Time scrubber bar spanning terminal width
  - Draggable cursor (left/right arrow keys, or `h`/`l` for coarse jumps)
  - Mutation density sparkline underneath (busy periods glow brighter)
  - Current replay timestamp displayed prominently
- Center: Dashboard or tree view rendered from the reconstructed `FieldSnapshot`
  - Visually distinguished from live view (dimmed border, "REPLAY" badge, muted palette)
  - All read-only — no mutations allowed in replay mode
- Bottom: Hint bar with replay-specific keybindings

**Scrubber granularity:**
- Arrow keys: jump to next/previous mutation event
- `H`/`L` (shift): jump by day
- `[`/`]`: jump by week
- Number keys 1-9: jump to decile of total history (1 = 10% through, 9 = 90%)
- `g`/`G`: jump to first/last event

**Scrubber range:** From earliest mutation in the database to now.

### Overlay: Ghost Diff Mode

Toggle with `Cmd+G` while in replay mode.

Renders the historical state alongside (or overlaid on) the current state:
- Tensions that exist now but didn't then: shown in green (new)
- Tensions that existed then but not now: shown in red (gone)
- Tensions present in both: show delta in dynamics (e.g., gap was 0.8, now 0.3)
- Summary line: "Since [replay date]: +N created, -N resolved, -N released, N still active"

This is the "then vs. now" view that makes structural drift visible at a glance.

### Analysis: Pattern Highlights

Computed across the full mutation history, surfaced as an overlay (`Cmd+P` in replay mode):

- **Oscillation clusters:** Time periods where multiple tensions oscillated simultaneously
- **Resolution waves:** Periods of high resolution velocity (breakthroughs)
- **Neglect seasons:** Extended periods where active tensions received no mutations
- **Horizon drift patterns:** Systematic postponement tendencies (e.g., "you push horizons back 70% of the time")
- **Structural tendency arcs:** How the dominant structural tendency (advancing/oscillating/stagnant) shifts over months

Each highlight is a clickable region on the scrubber — selecting it jumps to that time period.

## Implementation Plan

### Phase 1: Core Reconstruction (sd-core)

Add to `engine.rs`:

```
pub fn reconstruct_at(&self, timestamp: DateTime<Utc>) -> Result<FieldSnapshot>
```

- New type `FieldSnapshot`: Vec of (Tension, ComputedDynamics) pairs + metadata (timestamp, tension count, aggregate stats)
- Implementation: query mutations where `timestamp <= T`, group by tension_id, replay in order to build tension state
- Must handle: tensions created after T (exclude), tensions deleted before T (exclude), tensions whose status changed after T (rewind status)
- Test: reconstruct at T, then at T+1 mutation, verify exactly one field changes

### Phase 2: Replay View (werk-tui)

- New `View::Replay` variant with `ReplayState` (current timestamp, scrubber position, cached snapshot)
- Render reuses existing `dashboard::view()` / `tree::view()` but reads from snapshot instead of live engine
- Scrubber widget: horizontal bar with cursor, density sparkline, timestamp label
- Keybinding integration: replay-specific bindings active only in View::Replay
- `Esc` exits replay mode and returns to live dashboard

### Phase 3: Ghost Diff

- New overlay: `overlays/ghost.rs`
- Compute diff between `FieldSnapshot` at replay time and current live state
- Render as color-coded annotations on the dashboard view
- Summary statistics line

### Phase 4: Pattern Highlights

- New analysis pass in sd-core: `engine.compute_historical_patterns() -> HistoricalPatterns`
- Scans full mutation log for temporal clusters of oscillation, resolution, neglect
- Returns annotated time ranges with pattern labels
- Rendered as colored regions on the scrubber bar
- Overlay with pattern descriptions and navigation

## Open Questions

- **Performance:** For large mutation histories (thousands of events), should reconstruction be incremental (cache snapshots at intervals and replay from nearest cache point)?
- **Scrubber UX:** Is keyboard-only sufficient, or should the scrubber support mouse click-to-seek?
- **Scope:** Should replay show the tree view as well, or only dashboard? (Recommendation: both, toggled with Tab as in live mode.)
- **Persistence:** Should favorite time points be bookmarkable? ("Show me what my field looked like the week I finished the book.")

## Success Criteria

The feature is successful when a user who has been using werk for 3+ months can:
1. Scrub to any historical moment and see their field as it was
2. See at a glance how their field has changed since then (ghost diff)
3. Identify their own structural tendencies from the temporal patterns
4. Have the "aha" moment Fritz describes — recognizing the structure that drives their behavior
