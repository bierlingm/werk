# Ground Mode Redesign

**Opened:** 2026-03-30

**Status:** Design proposal. No code changes until approved.
**Depends on:** Observational analysis stance (resolved), Standard of Measurement (sacred core #10), Operative identity (Part II)

---

## What This Document Is

A comprehensive design for ground mode — the debrief and study surface. The current implementation (357 lines, four flat sections) is provisional. This document proposes what ground mode should actually be, grounded in the resolved stance (#65), the standard of measurement principle, and the data that already exists in sd-core.

---

## Why Ground Mode Exists

The conceptual foundation: "When you're not flying, you're on the ground. Ground mode is the debrief and study surface."

The flight metaphor carries real meaning. The descended view and survey view are flight instruments — you're in the structure, acting, navigating. Ground mode is what happens between flights: you examine the aircraft, study the weather, review the flight log, prepare for the next sortie. You are not acting on tensions. You are studying the field that contains them.

Ground mode serves the practitioner who sits down and asks: **"What is the state of my practice?"** Not "what should I do next" (that's the frontier) and not "what's urgent across the field" (that's the survey). Ground mode asks: where has my energy gone, what has changed structurally, what temporal situation am I in, and what does the record show about how I've been engaging?

---

## Design Principles

### 1. Study surface, not action surface

Ground mode does not invite gestures. It is read-only. You are studying, not operating. The information hierarchy serves reflection, not decision-making. There are no "you should..." prompts.

### 2. Standard of measurement governs all layers

Ground mode presents three layers, each with increasing distance from user-supplied standards:

| Layer | Basis | Treatment |
|-------|-------|-----------|
| **Factual** | Directly derived from user actions and user-set primitives | Presented as fact |
| **Metric** | Computed from the factual record using fixed, transparent methods | Presented as measurement |
| **Analytical** | Classification and projection using instrument-originated thresholds | Presented as analysis, explicitly framed |

Each layer is visually and structurally distinct. The practitioner always knows which layer they're reading from.

### 3. Structure over flatness

The current implementation lists every tension in a flat trajectory table. Ground mode should respect the structure the user built. Root tensions are coherence generators. Attention distributes across a tree, not a list. The field has shape — ground mode should show that shape.

### 4. Recent over distant

What happened in the last session, the last week, the last epoch matters more than cumulative statistics. Ground mode foregrounds the recent and makes the distant available on request (via `--days` or future LogBase queries).

### 5. Silence on the practice layer

Ground mode does not tell the practitioner what their patterns mean. It shows the patterns. Interpretation is theirs. This is the standard of measurement principle applied to the study surface: the instrument shows what it can derive from what the user did and declared. It does not say "you are avoiding #12" or "this looks like compensating." It says "#12 has 0 mutations in 14 days while its siblings have 23."

---

## The Five Sections

Ground mode output is organized into five sections, each answering a distinct question. Sections appear in order. Empty sections are omitted (signal by exception — if there's nothing to say, say nothing).

### Section 1: Field Vitals

**Question:** What is the basic shape of the field right now?

**Content:**

```
Field (last 7 days)
  62 active tensions  54 resolved  7 released
  19 deadlined  0 overdue  23 positioned  39 held
  Activity: 47 mutations across 14 tensions (3.2/day avg)
```

This is the current stats section, tightened. One line for structure counts, one for temporal counts, one for activity summary. The activity line is new: total mutations, how many tensions were touched (breadth of engagement), and average daily rate. These are facts.

**JSON:**

```json
{
  "section": "vitals",
  "active": 62, "resolved": 54, "released": 7,
  "deadlined": 19, "overdue": 0, "positioned": 23, "held": 39,
  "mutations": 47, "tensions_touched": 14, "avg_per_day": 3.2,
  "period_days": 7
}
```

### Section 2: Temporal Situation

**Question:** What deadlines are approaching, crowding, or violated? What's the temporal weather?

**Content:**

```
Temporal situation
  Approaching (next 14 days)
    #18  survey view [2026-04-10]         urgency 68%
    #16  state machine spec [2026-04-10]  urgency 68%
  Critical path
    #13 → #18  survey view crowds conceptual foundation (slack 21d)
    #2  → #13  conceptual foundation crowds root (slack 62d)
  Sequencing pressure
    #10  CLI [2026-05] — ordered after #3 [2026-05-30] but due earlier
  Containment violations
    #82  GUI [2026-08] exceeds parent #2 [2026-06] by 61d
    #36  business [2026-09] exceeds parent #2 [2026-06] by 92d
```

This section assembles temporal facts already computed by `temporal.rs` — urgency, critical path, sequencing pressure, containment violations — into a field-wide picture. None of this is new computation. It's the temporal signals that `show` surfaces per-tension, but aggregated across the field.

**Sub-sections appear only when non-empty.** A field with no approaching deadlines, no critical paths, and no violations shows nothing here. That silence is itself a signal: the temporal situation is clean.

**"Approaching" threshold:** Tensions with urgency > 50% or deadline within 14 days (whichever catches more). Sorted by urgency descending. Shows at most 10 — if there are more, a count line: `  ... and 3 more`.

**Critical path:** The recursive critical path from `detect_critical_path_recursive`, shown as chains. This is the most structurally valuable temporal signal and currently lives only in per-tension `show` output. Ground mode is where the field-wide critical path belongs.

**Data sources:** `compute_urgency()`, `detect_critical_path_recursive()`, `detect_sequencing_pressure()`, `detect_containment_violations()`, all in `temporal.rs`. No new computation needed.

**JSON:**

```json
{
  "section": "temporal",
  "approaching": [{"short_code": 18, "desired": "...", "deadline": "2026-04-10", "urgency": 0.68}],
  "critical_path": [{"from": 13, "to": 18, "slack_seconds": 1814400}],
  "sequencing_pressure": [...],
  "containment_violations": [...]
}
```

### Section 3: Attention Distribution

**Question:** Where has energy gone? Where hasn't it?

This is the section that replaces the flat trajectory list. It answers the practitioner's real question: am I engaging with my structure honestly, or am I pouring energy into some areas and ignoring others?

**Content:**

```
Attention (last 7 days)
  Root tensions
    #2  werk is a mature tool...            31 mutations across 9 descendants
      #13  conceptual foundation             12 mutations (4 tensions)
      #3   FrankenTUI                         8 mutations (3 tensions)
      #10  CLI                                6 mutations (2 tensions)
      #4   multi-participant                  3 mutations (2 tensions)
      #36  business model                     0 mutations ← 7 children, none touched
      #82  GUI                                0 mutations
      #51  documentation                      0 mutations
```

**What this shows:** Mutation counts aggregated up the tree to root tensions and their immediate children. This respects the structure the user built — root tensions are coherence generators, and their children are the primary areas of directed energy. The practitioner sees at a glance: "I've been working on the conceptual foundation and TUI. Business model and documentation have had no attention."

**The `← 7 children, none touched` annotation** appears only when a tension has children but zero mutations across all of them. This is a factual observation, not a judgment. The practitioner decides whether that's appropriate (the business model may be deliberately deferred) or a pattern worth examining.

**Computation:** For each root tension, sum mutations across `get_descendant_ids()` within the time window. Group by immediate children of the root. Sort children by mutation count descending. This is a tree aggregation over the existing mutation data — no new infrastructure needed.

**Not shown here:** Individual leaf tensions. The attention section operates at the structural level (roots and their major branches), not at the leaf level. If the practitioner wants leaf-level detail, that's what `show` and `insights` are for.

**JSON:**
```json
{
  "section": "attention",
  "roots": [
    {
      "short_code": 2,
      "desired": "...",
      "total_mutations": 31,
      "descendants_touched": 9,
      "branches": [
        {"short_code": 13, "desired": "...", "mutations": 12, "tensions_touched": 4},
        {"short_code": 36, "desired": "...", "mutations": 0, "tensions_touched": 0, "untouched_children": 7}
      ]
    }
  ]
}
```

### Section 4: Structural Changes

**Question:** What has actually changed in the structure recently?

The current "Recent gestures" section shows the last 15 mutations — a flat list of field/timestamp pairs. This is too granular to be useful for study and too narrow to show structural change.

**Content:**
```
Structural changes (last 7 days)
  Epochs
    #13  conceptual foundation    epoch 3 (reality shift, 30 min ago)
    #4   multi-participant        epoch 1 (reality shift, 41 min ago)
    #65  observational stance     epoch 1 (reality shift, resolved)

  Resolutions
    #65  observational analysis stance decided
    #30  epoch creation trigger path established
    #122 repository clean and organized

  New tensions
    #128 deep addressability + gesture vocabulary + session semantics
    #129 signal state persistence
    #130 horizon drift surfacing
    #131 TUI inline signals
    #132 next release shipping

  Reality shifts (most recent first)
    #13  "Standard of measurement principle (#10) and operative-not-managerial..."
    #4   "Authority, provenance, and multi-player identity surfaced..."
    #46  [updated]
```

**What this shows:** Structural events — the things that change the shape of the field — organized by type. Epochs (phase transitions), resolutions (gap closures), new tensions (new structural commitments), and reality shifts (ground truth updates). These are the narrative beats of the practice.

**Why this order:** Epochs first because they are phase transitions — the most structurally significant events. Resolutions second because they close gaps. New tensions third because they open new gaps. Reality shifts last because they are the most frequent and lowest-signal structural change.

**Reality shift display:** Shows the first ~60 characters of the new reality text for the most recent shifts (max 5). Older or less significant ones show `[updated]`. The practitioner can run `show` for full text. The point is to remind them what happened, not to display the full content.

**Computation:** Query mutations within window. Filter by field: `status` changes to resolved/released = resolutions. `created` field = new tensions. `actual` field = reality shifts. Epochs from `get_epochs()` filtered by timestamp. All existing queries.

**JSON:**
```json
{
  "section": "changes",
  "epochs": [{"short_code": 13, "desired": "...", "epoch_number": 3, "trigger": "reality", "age": "30 min ago"}],
  "resolutions": [{"short_code": 65, "desired": "..."}],
  "new_tensions": [{"short_code": 128, "desired": "..."}],
  "reality_shifts": [{"short_code": 13, "new_value_preview": "Standard of measurement..."}]
}
```

### Section 5: Analytical Layer

**Question:** What do the engagement patterns suggest when the instrument applies its own analytical framework?

This is where trajectory classification and projection live — explicitly framed as practice-layer analysis, separated from the factual sections above.

**Content:**
```
Analysis (practice layer — instrument-originated thresholds, not user-supplied standards)
  Trajectory distribution
    Resolving: 0    Drifting: 23    Stalling: 43    Oscillating: 0

  Urgency collisions (next 30 days)
    #18 + #16  [2026-04-10]  combined urgency 1.36

  Engagement patterns
    Field frequency: 6.7 mutations/day (trending up +0.3)
    Most engaged: #13 conceptual foundation (2.1/day)
    Least engaged (with deadlines): #37 public presence (0.0/day, due 2026-07)

  Horizon drift
    #82  GUI — repeated postponement (3 shifts, net +61 days)
```

**Framing is critical.** The header explicitly says "practice layer — instrument-originated thresholds, not user-supplied standards." This is not decoration. It is the standard of measurement principle applied: the practitioner must know they are now reading the instrument's analytical framework, not derived facts.

**Trajectory distribution** replaces the per-tension trajectory list. The current ground mode lists every active tension with its trajectory — 66 lines of mostly-identical output. The distribution (how many are resolving vs stalling vs drifting vs oscillating) is the useful signal. Individual trajectories are available via `werk trajectory`.

**Urgency collisions** surface from `project_field()` — moments when multiple high-urgency tensions overlap. This is the temporal crowding signal that currently lives only in the trajectory command. It belongs here because it's a field-level temporal pattern the practitioner should see during debrief.

**Engagement patterns** show field-wide engagement metrics: overall frequency, trend direction, most/least engaged tensions. "Least engaged with deadlines" is the actionable variant — a tension with no deadline and no engagement is just held; a tension with a deadline and no engagement is a pattern worth noticing.

**Horizon drift** surfaces `detect_horizon_drift()` results that are currently computed but not displayed anywhere in the CLI. Repeated postponement is a factual pattern (the horizon moved N times in one direction). It appears here, not in the factual sections, because the "repeated" threshold is instrument-originated.

**JSON:**
```json
{
  "section": "analysis",
  "trajectory_distribution": {"resolving": 0, "drifting": 23, "stalling": 43, "oscillating": 0},
  "urgency_collisions": [...],
  "engagement": {
    "field_frequency": 6.7,
    "field_trend": 0.3,
    "most_engaged": {"short_code": 13, "frequency": 2.1},
    "least_engaged_with_deadline": {"short_code": 37, "frequency": 0.0, "deadline": "2026-07"}
  },
  "horizon_drift": [{"short_code": 82, "drift_type": "RepeatedPostponement", "changes": 3, "net_shift_days": 61}]
}
```

---

## What Is NOT in Ground Mode

### Per-tension trajectory listings
The current flat list of every tension with its trajectory classification is noise. Trajectory distribution (Section 5) and per-tension queries (`werk trajectory`, `werk show`) serve this better.

### Recommendations or interpretive labels
Ground mode does not say "you're compensating," "this is neglected," "you should focus here." It shows patterns. The practitioner reads them.

### Session history
Sessions are currently thin (start/end timestamps, gesture list). When session semantics deepen (per the addressing/sessions/gestures exploration), ground mode will gain a session review section. Not yet — the data isn't rich enough to justify the section.

### LogBase queries
LogBase (#89) will eventually make ground mode a portal into the full structural history. Until LogBase exists, ground mode works with the current mutation and epoch data.

### Coaching prompts or debrief questions
The conceptual foundation mentions a "landing threshold" with an optional debrief invitation. That belongs to the TUI session lifecycle, not to ground mode. Ground mode is a CLI command, not an interactive debrief.

---

## Parameters

```
werk ground [--days N] [--json] [--section SECTION]
```

- `--days N` — Time window for attention, changes, and engagement analysis. Default: 7. The temporal situation section is not windowed (it shows the current state, not history).
- `--json` — Structured output. Each section is a JSON object in a top-level array.
- `--section SECTION` — Show only one section: `vitals`, `temporal`, `attention`, `changes`, `analysis`. For composability — agents can request just the piece they need.

---

## Information Hierarchy

Reading top to bottom, the practitioner encounters:

1. **The field's shape** (vitals) — how big is this, how active is it
2. **The temporal weather** (temporal situation) — what's approaching, what's crowding, what's violated
3. **Where energy went** (attention) — structural distribution of engagement
4. **What changed** (structural changes) — the narrative beats since last time
5. **The analytical reading** (analysis) — what the patterns look like through the instrument's lens

This follows the principle of decreasing certainty: vitals are pure counts, temporal signals are computed from user-set primitives, attention is aggregated from the factual record, structural changes are filtered from mutations, and analysis applies instrument-originated thresholds. The practitioner starts on solid ground and moves toward softer ground, always knowing which layer they're on.

---

## Relationship to Other Commands

| Command | Purpose | Overlap with ground |
|---------|---------|-------------------|
| `survey` | What's pressing across the field right now (action-facing) | Temporal situation overlaps with survey's overdue/due-soon; ground shows the analytical picture, survey shows the action surface |
| `trajectory` | Per-tension and field-wide trajectory analysis | Analysis section shows the distribution; trajectory shows per-tension detail |
| `insights` | Behavioral facts — attention, postponement, activity patterns | Attention section subsumes and improves insights' attention distribution; insights' day-of-week pattern is unique to insights |
| `health` | Structural statistics and integrity alerts | Vitals section overlaps with health's counts; health includes data integrity checks that ground doesn't |
| `diff` | What changed recently (mutation-level) | Structural changes section is a curated, categorized version of diff's raw output |
| `show` | Full detail on a single tension | Ground is field-wide; show is per-tension |

**Insights may become redundant** once ground mode is rebuilt. The attention distribution in ground is strictly better than insights' version (it respects tree structure). The day-of-week activity pattern from insights could either move into ground's analysis section or remain as a niche detail in insights. This is a separate decision.

---

## Implementation Notes

### Data flow

All five sections operate on the same data load: `list_tensions()` + `all_mutations()` (or per-tension mutations for tree aggregation). The temporal section additionally calls the temporal signal functions. The analysis section calls the projection engine. No new database queries or tables are needed.

### Performance

The current implementation iterates all tensions and their mutations once. The redesign adds tree aggregation (attention section) and temporal signal computation (temporal section). For a 127-tension workspace with ~700 mutations, this is well under 100ms. For larger workspaces, the `--section` flag allows selective computation.

### Text width

The text output is designed for 80-column terminals. Tension descriptions are truncated. Short codes are left-aligned in fixed-width columns. The layout follows the existing CLI output conventions (from `designs/werk-conceptual-foundation.md` and the CLI output design principles in CLAUDE.md).

### Backward compatibility

The `--json` output structure changes completely. The current GroundJson struct is replaced. This is a breaking change for JSON consumers. The text output changes completely but has no formal backward-compatibility guarantee.

---

## Open Questions

1. **Should ground mode show a "since last session" variant?** If the last session's end time is known, `werk ground --since-session` could show everything since the practitioner last closed the instrument. This requires session timestamps to be reliable, which they currently are.

2. **Should the analysis section be opt-in?** The `--section analysis` flag already allows selective access. But should the analysis section appear by default in the text output, or only when explicitly requested? The framing ("practice layer") makes it safe to show by default. But some practitioners may prefer facts-only ground mode. A `--no-analysis` flag is simple enough.

3. **What happens when the workspace is very large?** The attention section's tree aggregation scales with the number of root tensions and their immediate children. For a workspace with 5 root tensions and 50 branches, this is 55 rows at most. For a workspace with 20 root tensions, it's potentially unwieldy. A depth limit (show roots + one level of children) keeps it bounded.

4. **Should ground mode timestamp itself?** The analysis is a point-in-time reading. Printing `Ground analysis as of 2026-03-30 14:23 UTC` at the top makes this explicit and supports future comparisons. Slight chrome cost; honest.
