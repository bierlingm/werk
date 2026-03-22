# Horizon: Temporal Aim as Foundational Grammar Primitive

## 1. What This Is and Why It Matters

Horizon is a new field on `Tension` in sd-core. It represents when a tension is temporally aimed at — "this tension operates at this time scale and is aimed at this point in time." The precision of the horizon is itself meaningful: "2026" and "2026-05-15" are structurally different statements about temporal commitment. The grammar preserves this distinction.

**Horizon is not an add-on to the existing dynamics. It is the missing foundation that makes them genuinely computable.**

The dynamics module as it stands — lifecycle phases, conflict detection, oscillation, resolution, neglect, assimilation depth, compensating strategies, structural tendency — operates on a fundamental fiction: that "recency" means the same thing for every tension. Every threshold struct uses `recency_seconds` as an absolute value. The defaults are arbitrary: 1 week for conflict, 30 days for oscillation, 2 weeks for assimilation, 1 week for neglect. These numbers don't come from Fritz. They come from having no better basis.

This is the problem: **without a temporal frame of reference per tension, the dynamics engine cannot distinguish between a tension that's healthily quiet and one that's dangerously stagnant.** A tension aimed at "this year" with no mutations in two weeks is fine. The same silence on a tension aimed at "next Tuesday" is a crisis. The current engine treats them identically.

Fritz's structural dynamics are fundamentally about forces — tensions that seek resolution along the path of least resistance. But forces operate in time. A structural tension with a wide temporal window exerts gentle, sustained pressure. The same tension with a narrow window exerts intense, acute pressure. The dynamics engine needs to know the difference to compute anything meaningful. Without horizon, it's computing dynamics in a vacuum — structurally correct but temporally blind.

This is why horizon belongs in the grammar, not the instrument. It changes what the dynamics engine can compute, not just how results are displayed.

---

## 2. The Type

```rust
/// A temporal horizon with variable precision.
///
/// The precision itself is structurally meaningful — it represents
/// how tightly the practitioner has committed temporally.
///
/// Each variant defines a *range*, not a point. Year(2026) means
/// "sometime in 2026" — the full year is the window.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Horizon {
    /// A full year. Range: Jan 1 – Dec 31.
    Year(i32),
    /// A specific month. Range: 1st – last day of month.
    Month(i32, u32),
    /// A specific day. Range: 00:00 – 23:59:59.
    Day(chrono::NaiveDate),
    /// A specific instant.
    DateTime(DateTime<Utc>),
}
```

### Storage

Single `TEXT` column in the `tensions` table, nullable. Serialized as ISO-8601 partial dates:

| Variant | Stored value | Example |
|---|---|---|
| `Year(2026)` | `"2026"` | Aimed at sometime in 2026 |
| `Month(2026, 5)` | `"2026-05"` | Aimed at May 2026 |
| `Day(2026-05-15)` | `"2026-05-15"` | Aimed at May 15, 2026 |
| `DateTime(...)` | `"2026-05-15T14:00:00Z"` | Aimed at that instant |
| `None` | `NULL` | No temporal aim |

Parsing is unambiguous: count the components. The format is self-describing.

### Computable Properties

Each `Horizon` variant can produce:

- **`range_start() -> DateTime<Utc>`**: The beginning of the horizon window.
- **`range_end() -> DateTime<Utc>`**: The end of the horizon window.
- **`width() -> chrono::Duration`**: The temporal slack — `range_end - range_start`.
- **`contains(now: DateTime<Utc>) -> bool`**: Whether `now` falls within the window.
- **`is_past(now: DateTime<Utc>) -> bool`**: Whether the entire window has elapsed.

| Variant | range_start | range_end | width |
|---|---|---|---|
| `Year(2026)` | 2026-01-01 00:00 UTC | 2026-12-31 23:59:59 UTC | ~365 days |
| `Month(2026, 5)` | 2026-05-01 00:00 UTC | 2026-05-31 23:59:59 UTC | ~31 days |
| `Day(2026-05-15)` | 2026-05-15 00:00 UTC | 2026-05-15 23:59:59 UTC | ~1 day |
| `DateTime(t)` | `t` | `t` | 0 |

### Ordering

`Horizon` implements `Ord`. Ordering is by `range_start`, with ties broken by precision (narrower first — a `Day` within a `Month` sorts before the `Month` itself). This gives siblings a natural temporal sequence.

### What Horizon Is Not

- **Not a deadline.** "Aimed at May 2026" is not "must complete by May 2026." Deadlines are external constraints; horizons are structural aims. An instrument could layer deadline semantics on top via engagement rules, but the grammar doesn't.
- **Not a dependency.** "A before B" is expressed through hierarchy (parent/child telescoping), not through temporal ordering. Two siblings both aimed at May can resolve in any order.
- **Not a schedule.** The grammar doesn't say "work on this Monday." Scheduling is instrument territory.
- **Not a duration.** "This is a 3-month effort" is not stored directly. Duration/scale is *derivable*: `horizon.range_end() - created_at`. Storing a duration separately would create ambiguity about the anchor point.

### Why Not Quarter, Week, Season?

The grammar knows years, months, days, and instants. These are culturally neutral ISO-8601 building blocks. Quarters, weeks, seasons, sprints, lunar phases — these are instrument/calendar concepts. An instrument translates "Q2 2026" to `Month(2026, 6)` or a pair of months. The grammar stays universal.

---

## 3. How Horizon Transforms the Dynamics

This section is the heart of the spec. Each subsection examines a dynamics function that currently exists in `sd-core/src/dynamics.rs`, explains what it does now, why that's insufficient, and how horizon makes it genuinely meaningful.

### 3.1 The Recency Problem

Every dynamics function in the current codebase follows the same pattern:

```rust
let cutoff = now - chrono::Duration::seconds(thresholds.recency_seconds);
let relevant_mutations = mutations.iter()
    .filter(|m| m.timestamp() >= cutoff)
    .collect();
```

This is the temporal foundation of the entire dynamics engine. "Recent" means "within `recency_seconds` of now." The threshold defaults are:

| Dynamic | `recency_seconds` default | Meaning |
|---|---|---|
| Conflict | 604,800 (1 week) | Mutations older than 1 week don't count as "active" |
| Oscillation | 2,592,000 (30 days) | Look back 30 days for oscillation patterns |
| Resolution | 604,800 (1 week) | Look back 1 week for resolution progress |
| Lifecycle | 604,800 (1 week) | Mutations older than 1 week don't indicate assimilation |
| Neglect | 604,800 (1 week) | No mutation in 1 week = stagnant |
| Assimilation depth | 1,209,600 (2 weeks) | Analyze mutation frequency over 2 weeks |
| Compensating strategy | 2,592,000 (30 days) | Look back 30 days for compensating patterns |

These numbers are defensible as global defaults, but they're structurally meaningless. A practitioner working on a 3-year vision and a practitioner working on a task due tomorrow are measured by the same clock. The instrument can set different thresholds, but it sets them globally for all tensions — it can't say "this tension operates on a different time scale than that one."

With horizon, the dynamics engine gains a per-tension reference frame. The recency window becomes relative:

```rust
fn effective_recency(thresholds: &SomeThresholds, horizon: Option<&Horizon>, now: DateTime<Utc>) -> i64 {
    match horizon {
        Some(h) => {
            let total_window = (h.range_end() - now).num_seconds().max(1);
            let horizon_width = h.width().num_seconds().max(1);
            // Scale recency to a fraction of the horizon width
            // e.g., "recent" for a year-horizon = ~1 month; for a day-horizon = ~2 hours
            (horizon_width as f64 * 0.1) as i64
        }
        None => thresholds.recency_seconds, // fallback to absolute
    }
}
```

The `0.1` fraction above is illustrative. The precise scaling is a design decision — but the point is that "recent" is now proportional to the tension's temporal scale. This single change ripples through every dynamics function without changing any of their signatures. The threshold structs remain. The instrument still sets them. Horizon modulates them per-tension.

### 3.2 Urgency: A New Core Dynamic

Urgency is a pure computation that requires no threshold. It's the ratio of elapsed time to total time window:

```
time_elapsed = now - created_at
total_window = horizon.range_end() - created_at
urgency = time_elapsed / total_window
```

- `urgency = 0.0` → just created, full window ahead
- `urgency = 0.5` → halfway through the time window
- `urgency = 1.0` → at the horizon's end
- `urgency > 1.0` → past the horizon

Urgency is computable only when a horizon is present. `None` horizon → no urgency value (not zero — genuinely absent). This is semantically important: a tension without a horizon is not "not urgent," it is "outside the urgency frame entirely."

**Why urgency is a core dynamic, not an instrument concern:**

Urgency interacts structurally with every other dynamic:

- **Urgency + stagnation** = structural emergency. The gap is large, time is running out, and nothing is moving. This combination is the dynamics engine's strongest signal that something is wrong.
- **Urgency + resolution** = healthy completion. The gap is closing and there's still time. Fritz's completion phase becomes much more precisely classifiable.
- **Urgency + oscillation** = destructive oscillation. Oscillating with time to spare is different from oscillating as the horizon closes.
- **Urgency + germination** = delayed start. A tension in germination (no confrontation) with rising urgency tells a different story than one in germination with a distant horizon.

Without urgency, the dynamics engine can say "this tension is stagnant." With urgency, it can say "this tension is stagnant and has consumed 80% of its time window" — a qualitatively different and far more actionable signal.

### 3.3 Structural Tension — Magnitude Gains Temporal Dimension

`compute_structural_tension()` currently returns a `StructuralTension` with a `magnitude` based on the string distance between desired and actual. This is a spatial measurement — how big is the gap?

With horizon, structural tension gains a temporal dimension. The *force* of a structural tension isn't just the size of the gap — it's the size of the gap relative to the time remaining to close it. A large gap with a year to close it exerts gentle force. The same gap with a week exerts enormous force.

```rust
pub struct StructuralTension {
    pub magnitude: f64,
    pub has_gap: bool,
    /// Temporal pressure: magnitude scaled by urgency.
    /// Only present when the tension has a horizon.
    pub pressure: Option<f64>,
}
```

`pressure = magnitude * urgency` (or a more nuanced function). This is Fritz's "path of least resistance" made computable: when pressure is high, the structural force toward resolution is intense. The system predicts that high-pressure tensions will either resolve, get released, or have their horizons pushed — because the structure can't sustain the force.

### 3.4 Conflict Detection — From Activity Asymmetry to Temporal Competition

`detect_structural_conflict()` currently detects conflict by looking for asymmetric activity among siblings: one sibling has many recent mutations, another has few. This is a reasonable heuristic, but it misses a crucial Fritz concept: **structural conflict arises when two tensions compete for the same structural resources.**

Horizon makes an entirely new class of conflict detectable: **temporal crowding.** When multiple siblings are aimed at the same narrow horizon window, they compete for the practitioner's time and attention — even before any activity pattern reveals the conflict.

Current code (`dynamics.rs:628-700`):

```rust
// Count recent mutations for each tension
for mutation in mutations {
    if mutation.timestamp() >= cutoff {
        if let Some(count) = activity.get_mut(mutation.tension_id()) {
            *count += 1;
        }
    }
}
// Check if ratio exceeds threshold
```

This can only detect conflict *after* it manifests as behavioral asymmetry. With horizon, the engine can detect it *structurally* — before the practitioner has even started:

- **5 siblings all aimed at `Month(2026, 3)`**: Temporal crowding. The practitioner cannot give all of these adequate attention within one month.
- **1 sibling aimed at `Day(2026-03-15)`, another at `Year(2026)`**: No temporal competition. Different scales, different forces.
- **2 siblings aimed at `Month(2026, 3)`, one very active, one stagnant**: Both the old signal (activity asymmetry) and the new signal (temporal competition) fire together — the conflict is structural *and* behavioral.

The `ConflictThresholds` struct doesn't need to change. But the `detect_structural_conflict` function gains access to horizon data on each sibling, and its detection logic becomes richer.

### 3.5 Oscillation — Temporal Oscillation as a Distinct Pattern

`detect_oscillation()` currently looks for direction changes in the `actual` field mutation history — advances followed by regressions. This detects content oscillation: the practitioner's reality keeps swinging back and forth.

Horizon enables detection of a second, independent oscillation pattern: **temporal oscillation.** This is detectable purely from `field == "horizon"` mutations:

- Horizon set to `Month(2026, 3)`
- Horizon pushed to `Month(2026, 5)`
- Horizon pulled back to `Month(2026, 4)`
- Horizon pushed to `Month(2026, 6)`

The horizon itself is oscillating. The practitioner's temporal commitment is unstable. This is a Fritz oscillation pattern operating at the meta-level — the structure of the tension's *time frame* is oscillating, independent of whether its content is.

This is not a refinement of the existing oscillation detection. It's a new detectable signal that doesn't exist without horizon.

### 3.6 Resolution — Velocity Becomes Meaningful

`detect_resolution()` computes `velocity` — the rate of progress toward desired. Currently, velocity is measured in "units of gap closure per mutation" (via `compute_resolution_direction`). This has no physical meaning. Is a velocity of 0.3 good? Bad? The engine has no way to know.

With horizon, velocity becomes interpretable against a temporal frame:

```
required_velocity = remaining_gap / time_remaining
```

The engine can now compute whether the current velocity is *sufficient* to reach the desired state within the horizon. This turns resolution detection from "is this tension making progress?" into "is this tension making progress *fast enough*?"

This is Fritz's completion phase made precise. A tension is genuinely entering completion when its velocity meets or exceeds the required velocity — convergence is on track relative to the temporal aim. Without horizon, "completion" is just "the gap is small," which says nothing about whether it will actually close in time.

### 3.7 Creative Cycle Phase — Horizon as Phase Discriminator

`classify_creative_cycle_phase()` is the weakest function in the current dynamics module. It classifies phases based on:

- **Germination**: No recent mutations (default/fallback)
- **Assimilation**: Mutation count exceeds threshold
- **Completion**: Convergence ratio below threshold
- **Momentum**: Recent resolution in the network

These classifications are unreliable because they have no temporal reference frame. The function can't distinguish between:

- A tension created yesterday with no mutations (healthy germination) and one created six months ago with no mutations (abandoned, or genuinely slow germination for a multi-year horizon)
- A tension with 3 mutations this week (assimilation for a year-long project? completion sprint for a daily task?)
- A tension at 80% convergence (imminent completion? or permanently parked near-done for months?)

Horizon resolves all three ambiguities:

**Germination**: A tension is in germination when it's new relative to its horizon. "New" means: `(now - created_at) / (horizon.range_end() - created_at)` is small. A tension created a month ago but aimed at 2028 is still in early germination. A tension created a month ago and aimed at this month is not in germination at all — it's in crisis or completion.

**Assimilation**: The "invisible progress" phase. With horizon, the engine can distinguish genuine assimilation from stagnation by asking: is the silence proportional to the horizon? A year-long tension with no mutations for two weeks might be in genuine assimilation (learning, incubation). A week-long tension with no mutations for two days is stagnant.

**Completion**: Convergence relative to remaining time. A tension at 80% convergence with 80% of its time window remaining is healthy. The same convergence with 5% of time remaining is cutting it close. The same convergence with urgency > 1.0 (past horizon) tells a completely different story.

**Momentum**: Currently detected by "new tensions created shortly after resolution." With horizon, this can be refined: momentum is when completed tensions' energy flows into new tensions *with similar or progressive horizons* — not just any temporally adjacent creation.

### 3.8 Neglect — Horizon-Relative Silence

`detect_neglect()` looks for activity asymmetry between parent and children: an active parent with stagnant children, or stagnant parent with active children. "Active" means "has recent mutations" where "recent" is, again, `recency_seconds`.

With horizon, neglect detection becomes structurally precise:

- **Parent aimed at `Year(2026)` with child aimed at `Month(2026, 3)`**: If the child has no mutations and March is approaching, the child is neglected *relative to its own temporal pressure*. The parent might be fine — its horizon is far. The neglect is in the asymmetry between the child's urgency and the attention it receives.

- **Parent aimed at `Month(2026, 6)` with children aimed at various months**: Children whose horizons are approaching faster than the parent's should be getting proportionally more attention. If attention is uniform or inverted, that's neglect.

This is more nuanced than the current binary "active vs. stagnant" comparison. Horizon-relative neglect accounts for the fact that different tensions *should* receive different amounts of attention at different times.

### 3.9 Assimilation Depth — Frequency Relative to Scale

`measure_assimilation_depth()` classifies assimilation as Shallow (high mutation frequency, same outcomes), Deep (decreasing frequency, maintained outcomes), or None. The `high_frequency_threshold` is absolute: 5 mutations per 2-week window.

With horizon, "high frequency" becomes relative. 5 mutations per 2 weeks is frantic for a year-long tension and sluggish for a day-long one. The engine should scale:

```
relative_frequency = mutation_count / (horizon.width().num_weeks() as f64)
```

A tension with 5 mutations across a year-wide horizon has `relative_frequency ≈ 0.1` (shallow is unlikely). The same 5 mutations across a week-wide horizon has `relative_frequency ≈ 5.0` (could be shallow assimilation — lots of thrashing).

### 3.10 Compensating Strategies — Temporal Persistence

`detect_compensating_strategy()` checks for oscillation persisting without structural change for `persistence_threshold_seconds` (default: 2 weeks). With horizon, persistence becomes relative: oscillating for 2 weeks within a year-long horizon might not yet be a compensating strategy — it could be a normal working-through. Oscillating for 2 weeks within a month-long horizon is structurally significant.

### 3.11 Structural Tendency — Horizon as Predictive Input

`predict_structural_tendency()` predicts whether a tension will advance, oscillate, or stagnate. Currently, its only inputs are: does a gap exist? Is there conflict?

Horizon adds temporal prediction: a tension with no conflict, a clear gap, and a distant horizon tends toward slow advance. The same tension with an imminent horizon tends toward rapid advance or release — the structural forces intensify. A tension with conflict and an imminent horizon tends toward either forced resolution (breaking through the oscillation under time pressure) or release (abandoning the aim). These are Fritz-faithful predictions that the current engine simply cannot make.

### 3.12 Horizon Drift — A New Dynamic

Not an enhancement of an existing dynamic but a wholly new one, enabled only by horizon. Detectable from mutations where `field == "horizon"`:

- **Postponement**: Horizon moved later. Single occurrences are normal. Repeated postponement (3+ times, always later) is a structural signal — the desired state may not be genuinely held, or there's hidden conflict.
- **Tightening**: Horizon moved earlier or to higher precision (Year → Month → Day). Commitment is increasing. Structural tension intensifies.
- **Loosening**: Horizon moved later or to lower precision (Day → Month → Year). Commitment is decreasing, or deliberate scope expansion.
- **Oscillation**: Horizon moves back and forth — pulled closer then pushed further. Temporal oscillation, a meta-level Fritz pattern.

**Horizon drift is a computable dynamic with no equivalent in the current system.** It requires no new data structures — just pattern detection on the existing mutation log — but it's only possible when horizon exists as a tracked, mutable field.

### 3.13 Summary: Before and After

| Dynamic | Without Horizon | With Horizon |
|---|---|---|
| Structural tension | Gap magnitude (spatial only) | Gap magnitude + temporal pressure |
| Conflict | Activity asymmetry among siblings | Activity asymmetry + temporal crowding |
| Oscillation | Content oscillation (actual field) | Content oscillation + temporal oscillation |
| Resolution | Velocity (uninterpretable units) | Velocity vs. required velocity (sufficient?) |
| Lifecycle phase | Absolute mutation count thresholds | Relative mutation density, urgency-aware phase boundaries |
| Neglect | Binary active/stagnant comparison | Urgency-weighted attention asymmetry |
| Assimilation depth | Absolute frequency thresholds | Scale-relative frequency |
| Compensating strategy | Absolute persistence threshold | Horizon-relative persistence |
| Structural tendency | Gap existence + conflict presence | Gap + conflict + urgency + temporal pressure |
| Horizon drift | *Impossible* | Postponement, tightening, loosening, temporal oscillation |
| Urgency | *Impossible* | Pure computation, no threshold needed |

The "Without Horizon" column is the current state of sd-core. The "With Horizon" column is where every dynamic gains a temporal reference frame that makes it structurally meaningful rather than arbitrarily calibrated.

---

## 4. Four Distinct Temporal Concepts

For clarity and to prevent future confusion:

1. **Horizon** (this spec): "This tension is aimed at this time frame." Structural. Stored in the grammar. Enables dynamics computation. Variable precision carries meaning.

2. **Logical dependency**: "This can't resolve until that resolves." Expressed through hierarchy (parent/child telescoping). Already in the grammar. Horizon does not replace this.

3. **Pragmatic sequence**: "In practice, these were worked on in this order." Observable from mutation timestamps. Already captured in the mutation log. No field needed — it's emergent from history.

4. **Deadline / external constraint**: "Must complete by X due to external forces." An instrument concern, expressible as an engagement rule or a narrative annotation. Not in the grammar — deadlines are circumstantial, not structural.

---

## 5. Schema Change

### SQL

```sql
CREATE TABLE tensions (
    id          TEXT PRIMARY KEY,
    desired     TEXT NOT NULL,
    actual      TEXT NOT NULL,
    parent_id   TEXT,
    created_at  TEXT NOT NULL,
    status      TEXT NOT NULL,
    horizon     TEXT              -- ISO-8601 partial date, nullable
);
```

### Migration

Existing databases: `ALTER TABLE tensions ADD COLUMN horizon TEXT;`. All existing tensions get `NULL` (no horizon), which is semantically correct — they were created without temporal aim. All dynamics functions fall back to absolute thresholds when horizon is `None`, preserving existing behavior exactly.

### Rust Struct

```rust
pub struct Tension {
    pub id: String,
    pub desired: String,
    pub actual: String,
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub status: TensionStatus,
    pub horizon: Option<Horizon>,
}
```

---

## 6. Changes to sd-core Modules

### 6.1 New module: horizon.rs

A new module for the `Horizon` type and its computations. Self-contained, no dependencies on other sd-core modules.

**Contains:**
- `Horizon` enum (Year, Month, Day, DateTime)
- `impl Horizon`: `range_start()`, `range_end()`, `width()`, `contains()`, `is_past()`
- `impl Ord for Horizon`
- `Horizon::parse(s: &str) -> Result<Horizon, HorizonParseError>` — parse ISO-8601 partial date
- `Horizon::to_string(&self) -> String` — serialize to ISO-8601 partial date
- `Horizon::urgency(created_at: DateTime<Utc>, now: DateTime<Utc>) -> f64` — compute urgency
- `Horizon::staleness(last_mutation: DateTime<Utc>, now: DateTime<Utc>) -> f64` — compute staleness ratio
- `HorizonParseError` error type
- Full test suite: construction, parsing roundtrip, range computation for all variants, edge cases (leap years, month boundaries), ordering, urgency computation, staleness computation

### 6.2 tension.rs

**New field**: `pub horizon: Option<Horizon>`.

**Constructor changes:**
- `Tension::new(desired, actual)` — unchanged, horizon defaults to `None`.
- `Tension::new_with_parent(desired, actual, parent_id)` — unchanged, horizon defaults to `None`.
- New: `Tension::new_full(desired, actual, parent_id, horizon)` — constructor accepting all optional fields.

**New method**: `Tension::update_horizon(&mut self, new_horizon: Option<Horizon>) -> Result<Option<Horizon>, SdError>` — follows the pattern of `update_desired`/`update_actual`. Rejects updates on non-Active tensions.

**Validation**: A horizon in the past at creation time is allowed. The grammar doesn't judge; it computes.

**Test additions**: Construction with horizon, update_horizon on active/resolved/released, serialization roundtrip with horizon, horizon field in JSON output.

### 6.3 mutation.rs

**New recognized field**: `"horizon"` joins `"desired"`, `"actual"`, `"parent_id"`, `"status"`, `"created"`.

**`apply_mutation`**: Add `"horizon"` arm — parse new_value as `Horizon` via `Horizon::parse()`, or empty string for None.

**`ReconstructedTension`**: Add `pub horizon: Option<Horizon>`. Update `to_tension()`.

**Creation mutation format**: Extend to include horizon when present:
- Without: `"desired='...';actual='...'"`
- With: `"desired='...';actual='...';horizon='2026-05'"`

Note: the current creation format uses single-quote-delimited fields without escaping. This works but is fragile. This spec accepts the existing format for continuity and notes that a future migration to JSON-in-new_value would be cleaner.

**Test additions**: Replay with horizon field, replay creation with horizon, horizon mutations in sequence.

### 6.4 store.rs

**Schema**: Add `horizon TEXT` column to `CREATE TABLE tensions`.

**New constructor variant**: `create_tension_full(desired, actual, parent_id, horizon)` — or extend `create_tension_with_parent` signature.

**`persist_tension`**: Add horizon to INSERT. Serialize via `Horizon::to_string()` or NULL.

**All query methods** (`get_tension`, `list_tensions`, `get_children`, `get_roots`, `parse_tension_rows`): Add `horizon` to SELECT. Parse column 6 as `Option<Horizon>`.

**New method**: `update_horizon(id, new_horizon: Option<Horizon>)` — follows pattern of `update_desired`/`update_actual`: get tension, validate Active, persist change, record mutation, emit event.

**Tension construction in queries**: All places that construct `Tension { id, desired, actual, parent_id, created_at, status }` gain `horizon` field.

### 6.5 events.rs

**`TensionCreated` event**: Add `horizon: Option<String>` field.

**New event variant**:
```rust
HorizonChanged {
    tension_id: String,
    old_horizon: Option<String>,
    new_horizon: Option<String>,
    timestamp: DateTime<Utc>,
}
```

**`EventBuilder`**: Add `horizon_changed(...)`, update `tension_created(...)`.

### 6.6 dynamics.rs — Reworked

This is the most substantive change. The dynamics functions themselves don't change signature dramatically, but their internal computation becomes horizon-aware.

**New types**:

```rust
/// Urgency — the temporal pressure on a tension.
/// Only computable when horizon is present.
pub struct Urgency {
    pub tension_id: String,
    pub value: f64,           // 0.0 to unbounded (>1.0 = past horizon)
    pub time_remaining: i64,  // seconds until horizon.range_end()
    pub total_window: i64,    // seconds from created_at to horizon.range_end()
}

/// Horizon drift — pattern of horizon changes over time.
pub struct HorizonDrift {
    pub tension_id: String,
    pub drift_type: HorizonDriftType,
    pub change_count: usize,
    pub net_shift_seconds: i64,  // positive = postponed, negative = tightened
}

pub enum HorizonDriftType {
    Stable,          // No horizon changes
    Tightening,      // Net shift earlier / higher precision
    Postponement,    // Net shift later (single)
    RepeatedPostponement, // 3+ shifts later
    Oscillating,     // Back and forth
}
```

**`StructuralTension`**: Add `pub pressure: Option<f64>` — magnitude scaled by urgency.

**New functions**:
- `compute_urgency(tension: &Tension, now: DateTime<Utc>) -> Option<Urgency>` — pure computation, no thresholds.
- `detect_horizon_drift(tension_id: &str, mutations: &[Mutation]) -> HorizonDrift` — pattern detection on horizon mutations.
- `compute_temporal_pressure(tension: &Tension, now: DateTime<Utc>) -> Option<f64>` — magnitude * urgency.

**Modified functions** — internal computation changes, not signature changes:

- `detect_structural_conflict`: When siblings have horizons, also check for temporal crowding (overlapping narrow horizons). This is an additional detection path alongside the existing activity-asymmetry path.
- `detect_oscillation`: When horizon mutations exist, also check for temporal oscillation pattern. Returns oscillation with a note about which dimension (content vs. temporal).
- `detect_resolution`: Compute `required_velocity = remaining_gap / time_remaining` when horizon is present. Resolution trend gains "sufficient" / "insufficient" interpretation.
- `classify_creative_cycle_phase`: Phase boundaries become horizon-relative. Germination is early-in-window. Assimilation is mid-window with activity. Completion is late-in-window with convergence.
- `detect_neglect`: Weight attention asymmetry by relative urgency of parent vs. children.
- `measure_assimilation_depth`: Scale frequency thresholds by horizon width.
- `detect_compensating_strategy`: Scale persistence threshold by horizon width.
- `predict_structural_tendency`: Factor urgency into prediction — high urgency biases toward advancing or release.

**Threshold behavior**: All threshold structs remain unchanged. When horizon is `None`, behavior is identical to today. When horizon is present, the effective recency window is scaled to the horizon's width internally. No instrument changes needed.

### 6.7 tree.rs

**New query methods**:

```rust
/// Children of a node, sorted by horizon (earliest range_start first).
/// Tensions without horizons sort last.
pub fn children_by_horizon(&self, parent_id: &str) -> Vec<&Node>

/// All active tensions whose horizon is past.
pub fn tensions_past_horizon(&self, now: DateTime<Utc>) -> Vec<&Node>

/// All active tensions whose horizon ends within the given duration.
pub fn tensions_approaching_horizon(&self, now: DateTime<Utc>, within: chrono::Duration) -> Vec<&Node>
```

### 6.8 engine.rs

**`DynamicsEngine::compute_and_emit_for_tension`**: Add urgency computation and horizon drift detection to the computation cycle. Emit new event types for urgency thresholds crossed (instrument-defined) and horizon drift detected.

**`DynamicsEngine` creation methods**: Add variants that accept `Option<Horizon>` for tension creation.

### 6.9 lib.rs

Re-export `Horizon`, `HorizonParseError`, `Urgency`, `HorizonDrift`, `HorizonDriftType`, and the new functions.

---

## 7. Changes to werk-cli

### 7.1 `Commands::Add` — `--horizon` flag

```rust
Add {
    desired: Option<String>,
    actual: Option<String>,
    #[arg(short, long)]
    parent: Option<String>,
    /// Temporal horizon (e.g., "2026", "2026-05", "2026-05-15").
    #[arg(long)]
    horizon: Option<String>,
}
```

The CLI parses the string into a `Horizon` variant based on format. Invalid formats produce a user-facing error with examples.

### 7.2 New command: `werk horizon`

```rust
/// Set or clear the temporal horizon of a tension.
Horizon {
    /// Tension ID or prefix.
    id: String,
    /// New horizon value (e.g., "2026-05", or "none" to clear).
    value: Option<String>,
}
```

- `werk horizon <id> 2026-05` — set horizon
- `werk horizon <id> none` — clear horizon
- `werk horizon <id>` — display current horizon with computed urgency

### 7.3 `Commands::Show` — Horizon display

```
Desired:  Write the novel
Actual:   Have a rough outline for chapters 1-5
Horizon:  2026-05 (May 2026, 84 days remaining)
Status:   Active
Created:  2026-01-15
```

With `--verbose`:
```
Urgency:     0.42 (42% of time window elapsed)
Pressure:    0.29 (gap magnitude * urgency)
Staleness:   0.08 (relative to horizon)
Horizon drift: stable (no changes)
```

### 7.4 `Commands::Tree` — Horizon-sorted siblings

Sort siblings by horizon (earliest first), no-horizon tensions last:

```
Write the novel                          [2026]
  ├─ Outline all chapters                [2026-03]
  ├─ Draft chapters 1-5                  [2026-05]
  ├─ Draft chapters 6-10                 [2026-08]
  ├─ Revision pass                       [2026-11]
  └─ Find beta readers                   [—]
```

### 7.5 `Commands::Context` — Enriched agent context

```json
{
  "tension": {
    "id": "...",
    "desired": "...",
    "actual": "...",
    "horizon": "2026-05",
    "horizon_range": {
      "start": "2026-05-01T00:00:00Z",
      "end": "2026-05-31T23:59:59Z"
    },
    "urgency": 0.42,
    "pressure": 0.29,
    "staleness_ratio": 0.08
  },
  "siblings": [
    { "id": "...", "desired": "...", "horizon": "2026-03", "urgency": 0.78 },
    { "id": "...", "desired": "...", "horizon": "2026-08", "urgency": 0.15 }
  ]
}
```

---

## 8. What Horizon Does Not Change

- **Two-table schema**: Still `tensions` + `mutations`. Horizon is a column on `tensions`, tracked through `mutations`. No new tables.
- **All dynamics computed, nothing stored**: Urgency, staleness ratio, pressure, horizon drift — all computed from `tensions.horizon` + `mutations`. No cached state.
- **Events as extension boundary**: New event types follow the existing pattern. Grammar emits; instruments subscribe.
- **Hierarchy = logical dependency**: Parent/child remains the mechanism for structural dependency. Horizon provides temporal context, not dependency ordering.
- **Backward compatibility**: `horizon = None` produces exactly the current behavior for all dynamics functions. No existing behavior changes. No existing tests break.

---

## 9. Implementation Plan

### Bead H1: Horizon Type
- **Produces**: `sd-core/src/horizon.rs` — the `Horizon` enum with all computations
- **Blocked by**: nothing
- **Contains**:
  - `Horizon` enum: `Year(i32)`, `Month(i32, u32)`, `Day(NaiveDate)`, `DateTime(DateTime<Utc>)`
  - `range_start()`, `range_end()`, `width()`, `contains()`, `is_past()`
  - `Ord` implementation
  - `parse(s: &str) -> Result<Horizon, HorizonParseError>`
  - `Display` implementation (ISO-8601 partial date output)
  - `Serialize` / `Deserialize` (as ISO-8601 partial date string)
  - `urgency(created_at, now) -> f64`
  - `staleness(last_mutation, now) -> f64`
  - `HorizonParseError` type
- **Tests**:
  - Parse all four variants from string
  - Roundtrip: parse → display → parse
  - Range computation for each variant, including edge cases (Feb 28/29, Dec 31)
  - Width computation
  - `contains()` at boundaries
  - `is_past()` at boundaries
  - Ordering: Year < Month < Day < DateTime within same period; cross-period ordering
  - Urgency at 0%, 50%, 100%, >100%
  - Staleness at various ratios
  - Invalid parse inputs produce clean errors
- **Acceptance**: All tests pass. Type is self-contained. No dependencies on other sd-core modules.
- **Effort**: S

### Bead H2: Tension Struct Integration
- **Produces**: `horizon` field on `Tension`, constructors, update method
- **Blocked by**: H1
- **Contains**:
  - Add `pub horizon: Option<Horizon>` to `Tension`
  - `Tension::new_full(desired, actual, parent_id, horizon)`
  - `Tension::update_horizon(new_horizon) -> Result<Option<Horizon>, SdError>`
  - Update existing constructors to default horizon to `None`
- **Tests**:
  - Existing tests pass unchanged (horizon = None)
  - `new_full` with all horizon variants
  - `update_horizon` on Active tension
  - `update_horizon` rejected on Resolved/Released
  - Serialization roundtrip with horizon present and absent
- **Acceptance**: All existing tension tests pass. New tests pass. Backward-compatible.
- **Effort**: S

### Bead H3: Mutation Log Integration
- **Produces**: `"horizon"` as a recognized mutation field, replay support
- **Blocked by**: H1, H2
- **Contains**:
  - `apply_mutation`: new `"horizon"` arm
  - `ReconstructedTension`: add `horizon` field, update `to_tension()`
  - Creation format extended: `";horizon='...'"` appended when present
  - `parse_creation_value` updated to extract horizon if present
- **Tests**:
  - Replay with horizon mutations
  - Replay creation mutation with horizon
  - Replay creation mutation without horizon (backward-compatible)
  - Horizon set → updated → cleared via replay
- **Acceptance**: All existing mutation tests pass. New tests pass.
- **Effort**: S

### Bead H4: Store Integration
- **Produces**: Schema change, CRUD operations for horizon
- **Blocked by**: H2, H3
- **Contains**:
  - Schema: `horizon TEXT` column added to `CREATE TABLE tensions`
  - Migration logic for existing databases
  - `create_tension_full(desired, actual, parent_id, horizon)` method
  - `update_horizon(id, new_horizon)` method (get → validate → persist → mutate → emit)
  - All query methods updated: SELECT adds horizon column, parse_tension_rows reads it
  - `persist_tension` updated for horizon column
- **Tests**:
  - Create tension with horizon, retrieve, verify
  - Create tension without horizon, retrieve, verify None
  - Update horizon, verify mutation recorded
  - List/get_children/get_roots all return horizon correctly
  - Migration: open existing DB without horizon column, verify ALTER succeeds
- **Acceptance**: All existing store tests pass. New tests pass. In-memory and file-based stores both work.
- **Effort**: M

### Bead H5: Event System Integration
- **Produces**: `HorizonChanged` event, updated `TensionCreated`
- **Blocked by**: H4
- **Contains**:
  - `Event::HorizonChanged { tension_id, old_horizon, new_horizon, timestamp }`
  - `Event::TensionCreated` gains `horizon: Option<String>` field
  - `EventBuilder::horizon_changed(...)`, updated `tension_created(...)`
  - Store's `update_horizon` and `create_tension_full` emit appropriate events
- **Tests**:
  - `HorizonChanged` event emitted on update
  - `TensionCreated` event includes horizon when present, None when absent
  - Event serialization roundtrip
  - Subscriber receives horizon events
- **Acceptance**: All existing event tests pass. New events fire correctly.
- **Effort**: S

### Bead H6: Dynamics — Urgency, Pressure, Drift
- **Produces**: New dynamics functions that only exist with horizon
- **Blocked by**: H1, H2
- **Contains**:
  - `Urgency` type, `compute_urgency(tension, now) -> Option<Urgency>`
  - `HorizonDrift` type, `HorizonDriftType` enum
  - `detect_horizon_drift(tension_id, mutations) -> HorizonDrift`
  - `compute_temporal_pressure(tension, now) -> Option<f64>`
  - `StructuralTension` gains `pressure: Option<f64>` field
  - `compute_structural_tension` updated to compute pressure when horizon present
- **Tests**:
  - Urgency at 0%, 25%, 50%, 75%, 100%, 150% (past horizon)
  - Urgency returns None when no horizon
  - Pressure = magnitude * urgency
  - Horizon drift: stable (no changes), single postponement, repeated postponement, tightening, loosening, oscillation
  - Drift detection from actual mutation sequences
- **Acceptance**: New functions are fully tested. Existing `compute_structural_tension` tests pass unchanged (pressure = None when no horizon).
- **Effort**: M

### Bead H7: Dynamics — Horizon-Relative Recency
- **Produces**: Internal horizon-relative threshold scaling across all dynamics functions
- **Blocked by**: H6
- **Contains**:
  - Internal helper: `effective_recency(absolute_recency, horizon, now) -> i64`
  - `detect_structural_conflict`: internal use of effective_recency + temporal crowding detection
  - `detect_oscillation`: internal use of effective_recency + temporal oscillation detection
  - `detect_resolution`: required velocity computation when horizon present
  - `classify_creative_cycle_phase`: horizon-relative phase boundaries
  - `detect_neglect`: urgency-weighted attention comparison
  - `measure_assimilation_depth`: horizon-relative frequency scaling
  - `detect_compensating_strategy`: horizon-relative persistence scaling
  - `predict_structural_tendency`: urgency as predictive input
- **Tests**:
  - Each modified function tested with and without horizon, verifying:
    - Without horizon: identical behavior to current (regression tests)
    - With horizon: behavior changes appropriately
  - Conflict: temporal crowding detected when siblings share narrow horizon
  - Resolution: "sufficient velocity" computed against horizon
  - Lifecycle: phase classification shifts with urgency
  - Neglect: asymmetry weighted by child urgency
- **Acceptance**: All existing dynamics tests pass unchanged. New tests demonstrate horizon-aware behavior. No dynamics function changes signature.
- **Effort**: L

### Bead H8: Tree Integration
- **Produces**: Horizon-aware queries on Forest
- **Blocked by**: H2
- **Contains**:
  - `Forest::children_by_horizon(parent_id) -> Vec<&Node>`
  - `Forest::tensions_past_horizon(now) -> Vec<&Node>`
  - `Forest::tensions_approaching_horizon(now, within) -> Vec<&Node>`
- **Tests**:
  - Children sorted correctly (earliest horizon first, None last)
  - Past-horizon query returns only active tensions with elapsed horizons
  - Approaching-horizon query with various durations
  - Empty results when no horizons set
- **Acceptance**: All existing tree tests pass. New queries work.
- **Effort**: S

### Bead H9: Engine Integration
- **Produces**: Urgency and drift in the dynamics computation cycle
- **Blocked by**: H6, H7
- **Contains**:
  - `DynamicsEngine::compute_and_emit_for_tension` computes urgency and drift
  - `DynamicsEngine::create_tension_full` method
  - `DynamicsEngine::update_horizon` method
  - `PreviousDynamics` gains `had_urgency_above_threshold: bool` (for transition events)
- **Tests**:
  - Engine computes urgency for tensions with horizon
  - Engine skips urgency for tensions without horizon
  - Engine detects horizon drift
  - Full cycle: create tension with horizon → update actual → compute dynamics → verify urgency and pressure in results
- **Acceptance**: All existing engine tests pass. New integration tests demonstrate the full computation cycle with horizon.
- **Effort**: M

### Bead H10: werk-cli Integration
- **Produces**: CLI commands and display for horizon
- **Blocked by**: H4, H5, H8
- **Contains**:
  - `Commands::Add` gains `--horizon` flag
  - New `Commands::Horizon { id, value }` command
  - `Commands::Show` displays horizon, urgency, pressure, drift when verbose
  - `Commands::Tree` sorts siblings by horizon
  - `Commands::Context` includes horizon data in JSON
  - Horizon parsing from CLI string input with validation errors
- **Tests**:
  - `werk add --horizon 2026-05` creates tension with horizon
  - `werk horizon <id> 2026-05` sets horizon, mutation recorded
  - `werk horizon <id> none` clears horizon
  - `werk show` displays horizon line
  - `werk tree` sorts siblings correctly
  - `werk context` JSON includes horizon fields
  - Invalid horizon strings produce clear error messages
- **Acceptance**: All existing CLI tests pass. New commands work. JSON and human output both include horizon data.
- **Effort**: M

### Dependency Graph

```
H1 (Horizon type)
├── H2 (Tension struct)
│   ├── H3 (Mutation log)
│   │   └── H4 (Store)
│   │       └── H5 (Events)
│   ├── H6 (Urgency/Pressure/Drift)
│   │   └── H7 (Horizon-relative recency)
│   │       └── H9 (Engine)
│   └── H8 (Tree)
└── H10 (werk-cli) ← depends on H4, H5, H8
```

### Critical Path

```
H1 → H2 → H3 → H4 → H5 (plumbing complete)
              ↘ H6 → H7 → H9 (dynamics complete)
              ↘ H8 (tree complete)
                         ↘ H10 (CLI complete)
```

H1 through H5 are plumbing — mechanical, well-defined, small. H6 and H7 are where the intellectual work lives. H7 is the largest bead because it touches every dynamics function.

### Parallelism

After H2 completes:
- H3 → H4 → H5 (store path)
- H6 → H7 → H9 (dynamics path)
- H8 (tree path)

These three paths are independent until H10, which depends on all of them.
