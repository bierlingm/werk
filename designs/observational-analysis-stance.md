# Observational Analysis: Where the Line Sits

**Status:** Decided. Implemented 2026-03-30.

**Emerged:** 2026-03-25 from reviewing the projection engine (`sd-core/src/projection.rs`) against the conceptual foundation's fact/signal/dynamic hierarchy.

**Decision:** Standard of Measurement (Sacred Core #10) draws the line between metric and classification, not between classification and projection. The instrument's standard surfaces expose engagement metrics (mutation frequency, gap trend, gap samples, intervals) — facts anchored to what the user did. Trajectory classification (the four-category enum) and projections both use instrument-originated thresholds and belong to the practice layer — available through analytical commands (`trajectory`, `ground`) but not asserted as instrument output on standard surfaces (`context`, `show`, hooks). See "Resolution" section at end.

---

## The Problem

The conceptual foundation defines three levels of instrument output:

| Level | Definition | Example | Who produces it |
|---|---|---|---|
| **Fact** | Directly observable from data without inference | "12 mutations in 30 days" | The instrument |
| **Signal** | A fact recognized as potentially action-relevant, curated for salience | "No mutations in 14 days on a tension with a deadline in 7 days" | The instrument |
| **Dynamic** | An interpretive framework from structural dynamics practice | "This tension is oscillating — the underlying structure doesn't favor resolution" | The practitioner or AI |

The foundation is explicit: dynamics are "not computed by the instrument — requires life context the instrument doesn't capture."

The projection engine (`sd-core/src/projection.rs`) currently computes trajectory classifications (`Resolving`, `Stalling`, `Drifting`, `Oscillating`), projected gap magnitudes, estimated time-to-resolution, and will-resolve predictions. These span levels 2 through 4 on a five-level gradient:

1. **Raw fact**: "12 mutations in 30 days, gap unchanged"
2. **Computed metric**: "frequency: 0.4/day, gap_trend: 0.0"
3. **Pattern classification**: "Drifting"
4. **Projection**: "at this rate, resolution in 47 days"
5. **Recommendation**: "you should recompose"

The foundation clearly permits 1 and 2, clearly prohibits 5, and is silent on 3 and 4. This document resolves that silence.

---

## What Exists in Code

### Mutation Pattern Extraction (level 2 — computed metrics)

`extract_mutation_pattern()` computes from the mutation history within a sliding window:

- `mutation_count` — count of mutations in window
- `frequency_per_day` — mutations per day
- `frequency_trend` — first-half vs second-half mutation rate ratio
- `gap_trend` — linear regression slope over gap samples
- `gap_samples` — recent gap magnitudes at each "actual" mutation
- `mean_interval_seconds` — average time between mutations

These are all computed metrics derivable directly from the factual record. They do not interpret. They summarize.

**Note on gap_magnitude:** The current implementation (`temporal.rs:33`) is binary — string equality between desired and actual yields 0.0 or 1.0. This means gap_samples is a sequence of 0s and 1s, and gap_trend is the slope across a binary series. The metric infrastructure is richer than the data it currently operates on. If gap_magnitude becomes continuous (e.g. through semantic similarity or child-resolution rate), the metrics gain real analytical power. Until then, they are pattern indicators over a discrete signal.

### Trajectory Classification (level 3 — pattern classification)

`classify_trajectory()` maps mutation patterns to four categories:

- **Oscillating**: 3+ gap samples with alternating diffs (advance-then-regress)
- **Stalling**: frequency below neglect threshold or sharply declining
- **Drifting**: engaged (frequency > 0) but gap not closing (gap_trend >= -0.001)
- **Resolving**: engaged and gap closing (the default)

These are named patterns. The naming carries interpretive weight — "Drifting" tells you what to think about "engaged but gap flat" in a way the raw numbers do not.

### Projection (level 4 — extrapolation)

`project_gap_at()`, `project_frequency_at()`, `estimate_time_to_resolution()`, and `TensionProjection` linearly extrapolate current trends forward to 1-week, 1-month, and 3-month horizons. They predict:

- Future gap magnitude
- Future engagement frequency
- Whether the tension will resolve before its deadline
- Seconds until resolution

These are forecasts. They assert what the instrument thinks will happen if patterns continue. This is a theory of meaning — exactly what the foundation says the instrument should not have.

### Risk Flags (level 2.5 — threshold signals)

- `neglect_risk`: boolean, true when frequency < threshold
- `oscillation_risk`: boolean, true when gap variance > threshold and engagement present

These sit between metric and classification. They are threshold crossings on computed metrics — closer to signal ("this metric crossed a configured boundary") than to interpretation ("this tension is being neglected").

---

## The Design Question

Where does the instrument's responsibility end and the practitioner's (or agent's) begin?

Three possible stances:

### Stance A: Metrics Only

The instrument computes and exposes the `MutationPattern` fields (frequency, trend, gap samples, intervals) and the risk-flag threshold crossings. It does not name patterns or project forward. Trajectory classification and projection are practice-layer concerns — computed by the practitioner, a coaching AI, or an external system like Mist.

**Consequence:** The MCP surface, hook payloads, and survey JSON expose raw metrics. Every consumer does its own classification. Most honest. Most composable. Most work for every consumer.

### Stance B: Classification Without Projection

The instrument computes metrics AND classifies trajectory (the four-category enum). It does not project forward. The enum is treated as a **signal** — a pattern recognition applied to the factual record, analogous to how the instrument already recognizes "overdue" (a threshold crossing on urgency) or "containment violation" (a structural relationship between child and parent deadlines).

Projection (time-to-resolution, will-resolve, projected gap) is quarantined to ground mode or excluded entirely, available only as practice-layer computation.

**Consequence:** The trajectory enum appears in MCP, hooks, and survey JSON. Projections do not. The instrument says "here is the pattern" but not "here is where the pattern leads." The practitioner or agent decides what the pattern means for action.

### Stance C: Full Observational Layer with Explicit Confidence Marking

The instrument computes metrics, classifies trajectory, AND projects forward — but all projections are explicitly marked as low-confidence observational analysis, presented only in ground mode or behind an explicit opt-in flag, and never surfaced as signals in the main instrument or as default fields in MCP/hook/survey output.

**Consequence:** Ground mode becomes the space where the instrument offers its full analytical capability, clearly labeled. The main instrument and its external surfaces remain fact-and-signal only. Consumers who want projections request them explicitly.

---

## Recommendation

**Stance B**, with one refinement.

The trajectory enum is defensible as a signal if we are precise about what it is: a pattern classification applied to the mutation record, not a judgment about the tension's health or future. The vocabulary matters. The four current names are acceptable:

- **Resolving** — engaged, gap closing. Factual pattern.
- **Stalling** — low engagement or declining. Factual pattern.
- **Drifting** — engaged, gap not closing. Factual pattern (though the name carries more interpretive weight than the others).
- **Oscillating** — alternating advance and regress. Factual pattern, and the one with the deepest structural dynamics significance.

The refinement: consider whether the vocabulary should be explicitly neutral rather than interpretively loaded. "Gap-closing + engaged", "low-engagement", "engaged + gap-flat", "alternating" are the same classifications without the narrative freight. The current names are better for human readability. The neutral forms are better for preserving the practitioner's interpretive authority. This is a vocabulary decision, not an architectural one.

The projection layer (`project_gap_at`, `project_frequency_at`, `estimate_time_to_resolution`, `TensionProjection`) should be either:
- Moved to ground mode as explicit observational analysis, clearly labeled as extrapolation
- Or removed from the instrument entirely and left to external consumers who want to build their own models from the metrics the instrument provides

The risk flags (`neglect_risk`, `oscillation_risk`) are acceptable as signals — they are threshold crossings on computed metrics, no different in kind from "overdue" as a threshold crossing on urgency.

---

## What This Means for External Surfaces

If Stance B is adopted:

| Surface | Exposes | Does not expose |
|---|---|---|
| Main TUI | Trajectory enum as contextual signal | Projections, time-to-resolution |
| Survey JSON | Trajectory enum, risk flags, raw metrics | Projections |
| Hook payloads | Trajectory enum, risk flags | Projections |
| MCP tools | Trajectory enum, risk flags, raw metrics | Projections (unless explicitly requested via a ground-mode tool) |
| Ground mode | Everything — metrics, classification, projections, full analytical layer | Nothing excluded |

This preserves "honest facts and signals only" for the main instrument while giving ground mode the analytical depth it needs as a study surface. External consumers (Mist, agents, coaching AIs) get the classification as a starting signal and the raw metrics to build their own models.

---

## Sub-Questions (resolved)

1. **Should trajectory classification be opt-in or default in external surfaces?** Resolved: neither. Trajectory classification does not appear on standard surfaces at all. Standard surfaces carry engagement metrics. Classification is available through analytical surfaces (`trajectory` command/tool, `ground` mode).

2. **Does gap_magnitude need to become continuous for trajectory to be meaningful?** Still open as a separate design question, but no longer blocking. The engagement metrics are exposed regardless — gap_samples shows the binary sequence, consumers can interpret it. A continuous gap_magnitude would enrich the metrics.

3. **How does this interact with the Mist bridge?** Resolved cleanly. Hook payloads carry engagement metrics (frequency, trend, gap data). Mist receives the raw material and applies its own analytical framework. This is the correct architecture — Mist's Prism verification layer should operate on its own models, not trust the instrument's interpretive readings.

---

## Resolution

**Decided 2026-03-30.** The Standard of Measurement (Sacred Core #10) provided the deciding criterion. The key passage:

> "the instrument will not label a pattern as 'compensating' or 'oscillating' or 'neglected' because these are interpretive acts that require standards the user has not given the instrument"

The line falls between Level 2 (computed metrics) and Level 3 (pattern classification), not between Level 3 and Level 4 as the original Stance B recommendation proposed. This is a refinement of Stance B, not a rejection — the architectural shape is the same (standard surfaces vs analytical surfaces), but the cut is one layer deeper.

**Three layers, two surfaces:**

| Layer | Examples | Surface |
|---|---|---|
| **Metric** (Level 1-2) | frequency_per_day, gap_trend, gap_samples, mean_interval_seconds | Standard: `context`, `show`, hooks |
| **Classification** (Level 3) | Resolving, Stalling, Drifting, Oscillating; neglect_risk, oscillation_risk | Analytical: `trajectory`, `ground` |
| **Projection** (Level 4) | projected_gap, time_to_resolution, will_resolve | Analytical: `trajectory`, `ground` |

**What changed in code:**
- `context` (CLI and MCP): replaced `projection` object with `engagement` object containing raw `MutationPattern` metrics
- `trajectory` tool (MCP): reframed as practice-layer analysis with explicit description
- `ground` command: added trajectory section as part of the analytical bundle
- `show` and hooks: unchanged (already carried no classification/projection)

**The vocabulary question** (interpretive names vs neutral descriptors) is now moot for standard surfaces — they carry numbers, not names. The trajectory enum retains its current names (Resolving, Stalling, Drifting, Oscillating) on analytical surfaces where the practice-layer framing is explicit.
