# Design J: Field Resonance & Interference Detection

**Status:** Proposed
**Priority:** Next sd-core dynamic (14th)
**Depends on:** sd-core mutation log, DynamicsEngine, existing 13 dynamics

## Thesis

Every dynamic in werk is computed per-tension. But Robert Fritz's structural dynamics is a *field* theory — tensions don't exist in isolation. They exert force on each other. When you advance one tension, others move too — sometimes forward (constructive resonance), sometimes backward (destructive interference). werk doesn't see this yet.

Field resonance detection cross-correlates mutation patterns across tensions to discover which ones are structurally coupled, even when they share no parent-child relationship. It turns werk from "N independent tension meters" into a field dynamics instrument that reveals hidden structure in the practitioner's creative practice.

## Why This Is the One Right Now

### It multiplies the value of everything already built

The 13 existing dynamics become more valuable when you can see how they interact across tensions. Oscillation in tension A is interesting. Oscillation in A that correlates with stagnation in B is actionable — it reveals a compensating structure. Every existing dynamic gains a relational dimension.

### It makes Lever dramatically smarter

Lever currently scores tensions independently. A tension with high constructive resonance is higher leverage than its individual score suggests — advancing it pulls others forward. A tension with high destructive interference against urgent items is a hidden blocker even if its own dynamics look healthy. Resonance feeds directly into lever scoring as a cascade multiplier.

### It turns werk from N independent meters into a field instrument

The 13 dynamics are particle physics — each tension analyzed in isolation. Resonance is field theory — the same tensions analyzed as a system. This is the difference between "tension A is oscillating" (interesting) and "tension A oscillates because it structurally competes with tension B for your creative energy" (actionable). The architecture already supports this: immutable mutation log provides the data, DynamicsEngine provides the computation host, EventBus provides the notification channel, forest topology provides relational context. The only missing piece is the cross-correlation computation itself.

### It discovers structure the user didn't declare

Parent-child relationships are explicit. The user chose them. Resonance relationships are emergent — they arise from how the user actually behaves, not how they think they behave. This is the Fritz insight: structural tendencies operate below conscious awareness. Surfacing them is the entire point.

### It's pure computation on existing data

No new user input. No LLM. No external dependencies. Just cross-correlation of mutation timestamps and dynamics trajectories that are already stored. The mutation log has everything needed.

### It compounds with time-travel (Design I)

When field replay lands, resonance patterns across time become visible — you can see *when* two tensions became coupled, and *when* the coupling broke. This is the temporal structure of creative practice made legible.

## The Model

### Resonance Definition

Two tensions A and B are in **resonance** when mutations to A reliably predict mutations to B within a coupling window.

Formally: given all mutations to A, compute the probability that B receives a mutation within τ hours (default: 48h). If this probability significantly exceeds the base rate of B mutations, A→B has positive coupling. Compute both directions — resonance can be asymmetric.

### Three Coupling Types

**Constructive resonance (co-advancing):**
Both tensions advance together. Advancing reality on A predicts advancing reality on B. Working on one genuinely helps the other.

Signal: A.actual mutation → B.actual mutation within τ, and both mutations move gap downward (reality approaches desire).

**Destructive interference (competing):**
Advancing one stalls or reverses the other. A.actual mutation forward → B stagnates, B.actual regresses, or B oscillation increases.

Signal: A.actual mutation → B gap increases or B movement shifts to Stagnant/Oscillating within τ.

**Harmonic resonance (phase-locked):**
Both tensions oscillate in sync — same reversal cadence, same stall periods. Neither is causing the other; they share a common structural root.

Signal: Oscillation events (reality-reversal mutations) in A and B cluster within τ of each other, above chance.

### Resonance Score

For each directed pair (A→B):

```
coupling_strength = P(B mutated within τ | A mutated) / P(B mutated within τ)
```

Where:
- Numerator: empirical conditional probability from mutation co-occurrence
- Denominator: base rate of B mutations in any random τ-window
- coupling_strength > 1.0 = positive coupling
- coupling_strength < 1.0 = negative coupling (A activity suppresses B)
- coupling_strength ≈ 1.0 = no coupling

Classify:
- coupling_strength > 2.0 AND gap-direction concordant → **Constructive**
- coupling_strength > 2.0 AND gap-direction discordant → **Destructive**
- oscillation correlation > 0.5 → **Harmonic**
- coupling_strength < 0.5 → **Competitive** (A suppresses B)

Minimum data threshold: at least 5 A-mutations and 5 B-mutations to compute. Below that, resonance is `None`.

### Resonance Group

Tensions with mutual constructive resonance form a **resonance group** — a creative front. The group moves together. Identifying groups reveals the actual structure of the user's work, which may differ from the declared parent-child hierarchy.

## Data Structures

```rust
/// Coupling between two tensions
pub struct Coupling {
    pub source_id: String,
    pub target_id: String,
    pub kind: CouplingKind,
    pub strength: f64,         // ratio (>1.0 = coupled, <1.0 = suppressed)
    pub confidence: f64,       // 0.0-1.0, based on sample size
    pub sample_count: usize,   // number of source mutations analyzed
    pub window_hours: f64,     // τ used for this computation
}

pub enum CouplingKind {
    Constructive,  // co-advancing
    Destructive,   // competing
    Harmonic,      // phase-locked oscillation
    Competitive,   // one suppresses the other
}

/// Full resonance analysis for one tension
pub struct ResonanceResult {
    pub tension_id: String,
    pub couplings: Vec<Coupling>,       // all detected couplings
    pub resonance_group: Vec<String>,   // IDs of mutual constructive resonance partners
    pub net_field_effect: f64,          // aggregate: positive = field-aligned, negative = field-disruptive
}

/// Field-level resonance map
pub struct ResonanceField {
    pub groups: Vec<Vec<String>>,       // resonance groups (clusters)
    pub strongest_constructive: Option<(String, String, f64)>,  // most coupled pair
    pub strongest_destructive: Option<(String, String, f64)>,   // most competing pair
    pub isolated: Vec<String>,          // tensions with no significant coupling
}
```

## Algorithm

### Step 1: Build Mutation Timeline

For each active tension, extract the ordered list of mutations with timestamps and gap-direction (did this mutation move reality closer to desire, further from it, or sideways?).

Gap direction for a mutation:
- Compute magnitude before and after (using the existing hybrid Levenshtein+Jaccard metric on desired vs. actual)
- If magnitude decreased: **advancing** (+1)
- If magnitude increased: **retreating** (-1)
- If magnitude unchanged (note, horizon change, etc.): **neutral** (0)

### Step 2: Compute Pairwise Co-occurrence

For each ordered pair (A, B) where both have ≥ 5 mutations:

1. For each mutation mₐ in A, check if B has any mutation within τ hours after mₐ
2. Count hits / total A-mutations = conditional probability P(B|A)
3. Compute base rate: total B-mutations × τ / total observation window = P(B)
4. coupling_strength = P(B|A) / P(B)

### Step 3: Classify Direction

For each pair with coupling_strength > 2.0:

1. For each co-occurring pair (mₐ, m_b), check gap directions
2. If both advancing or both retreating in >60% of co-occurrences → **Constructive**
3. If directions are discordant (A advances, B retreats) in >60% → **Destructive**
4. Otherwise → **Harmonic** (coupled but direction-ambiguous)

For pairs with coupling_strength < 0.5:
- **Competitive** (A's activity suppresses B's)

### Step 4: Detect Resonance Groups

Build undirected graph where edge exists iff mutual constructive resonance (A→B constructive AND B→A constructive). Find connected components. Each component is a resonance group.

### Step 5: Compute Net Field Effect

For each tension, sum the coupling strengths to all other tensions:
- Constructive couplings contribute positively
- Destructive/Competitive couplings contribute negatively
- Net positive = field-aligned (working on this tension helps the field)
- Net negative = field-disruptive (working on this tension hurts other work)

## Integration Points

### sd-core: 14th Dynamic

```rust
// In dynamics.rs or new resonance.rs
impl DynamicsEngine {
    pub fn compute_resonance(&self, tension_id: &str, now: DateTime<Utc>) -> Option<ResonanceResult>
    pub fn compute_resonance_field(&self, now: DateTime<Utc>) -> ResonanceField
}
```

Added to `ComputedDynamics`:
```rust
pub resonance: Option<ResonanceResult>,
```

Computation is expensive relative to other dynamics — O(n²) in tension count. Cache at field level on the same 5-minute cycle as `FieldProjection`. Per-tension results are slices of the cached field computation.

### Lever Enhancement

Add `resonance_multiplier` to the 11-factor lever scoring:

```
resonance_multiplier = 1.0 + (0.3 × constructive_coupling_count / total_active_tensions)
                     - (0.2 × destructive_coupling_count / total_active_tensions)
```

A tension with 3 constructive couplings out of 10 active tensions gets a 1.09× multiplier. A tension with 2 destructive couplings gets a 0.96× multiplier. The lever becomes field-aware.

### TUI: Detail View — Resonance Section

Between Dynamics and Forecast in the Detail view:

```
  Resonance
  ◈ Moves with    Ship feature (0.82), Fix auth (0.71)
  ◇ Competes with Plan Q2 roadmap (0.64)
  Field effect    +1.4 (field-aligned)
```

Symbols:
- `◈` constructive (filled diamond — connection)
- `◇` destructive (empty diamond — friction)
- `◎` harmonic (circled dot — synchronized)

Each listed tension is navigable (Enter to jump).

### TUI: Dashboard Column

Optional column in wide terminals (140+ cols):

```
Res
◈2
◇1
 —
◈3
```

Compact: diamond + count of strongest coupling type.

### TUI: Resonance Overlay

New overlay accessible via `Ctrl+F` or command palette "Field Resonance":

```
┌─ Field Resonance ────────────────────────────────┐
│                                                   │
│  Resonance Groups                                 │
│  ┌─ Group 1 ─────────────────────────────────┐   │
│  │ Ship feature ◈──◈ Fix auth ◈──◈ Deploy    │   │
│  └────────────────────────────────────────────┘   │
│  ┌─ Group 2 ─────────────────────────────────┐   │
│  │ Write docs ◈──◈ Update branding            │   │
│  └────────────────────────────────────────────┘   │
│                                                   │
│  Interference                                     │
│  Ship feature ◇──◇ Plan Q2 roadmap (−0.64)       │
│                                                   │
│  Isolated (no significant coupling)               │
│  • Refactor auth middleware                       │
│  • Set up monitoring                              │
│                                                   │
│  Strongest link: Ship feature ↔ Fix auth (3.2×)   │
│  Strongest friction: Ship ↔ Q2 plan (0.3×)        │
│                                                   │
│  Field coherence: 0.72 (3 groups, 1 conflict)     │
│                                                   │
└──────────────────────────────────── Esc to close ─┘
```

### CLI

```bash
werk resonance                    # field resonance summary (human)
werk resonance 01KK               # resonance for one tension
werk resonance --json              # full resonance field as JSON
werk resonance --groups            # show only resonance groups
```

### Agent Context

Include resonance in `werk context` output so agents can reason about field structure:

```json
{
  "resonance": {
    "constructive": [
      { "id": "01KK...", "desired": "Fix auth", "strength": 3.2 }
    ],
    "destructive": [
      { "id": "02BB...", "desired": "Plan Q2", "strength": 0.3 }
    ],
    "group": ["01KK...", "a9c1...", "b2d4..."],
    "field_effect": 1.4
  }
}
```

## Performance

### Complexity

- Pairwise computation: O(n² × m) where n = active tensions, m = average mutations per tension
- For typical use (10-30 active tensions, 20-100 mutations each): < 50ms
- For large fields (100+ tensions): cache aggressively, compute incrementally

### Caching Strategy

- Full `ResonanceField` cached alongside `FieldProjection` on 5-minute cycle
- Invalidated on any mutation (same as projection cache)
- Per-tension `ResonanceResult` sliced from cached field — no recomputation

### Minimum Data Requirements

- Tension must have ≥ 5 mutations to participate in resonance analysis
- Observation window must span ≥ 7 days for reliable base rates
- Below thresholds: return `None` (resonance unknown, not resonance absent)

## Relationship to Other Designs

**Design I (Time Travel):** Resonance computed at historical timestamps reveals when couplings formed and broke. "These two tensions became coupled on March 3rd when you started working on them in the same session." Resonance + time-travel = structural autobiography. Structural rhythm adds a third dimension: not just what coupled when, but *your personal cadence* through those couplings — when you engage, what precedes breakthroughs, how neglect cascades through the field over time.

**Projection Engine (calm-wandering-crab):** Trajectory projection + resonance = coupled trajectories. "If A resolves, B's trajectory improves from Stalling to Resolving." This is the what-if for the whole field, not just parent-child cascades.

**Lever:** Resonance is the missing "cascade" signal. Current lever scoring uses `cascade_potential` based on child count. Resonance replaces this with empirical cascade evidence — not "this tension has children that might benefit" but "this tension's advancement has historically advanced 3 others."

**Insights:** The Insights overlay currently shows per-tension behavioral patterns. Resonance adds field-level patterns: "You have a recurring interference between creative work and planning work — they compete for structural energy."

## Structural Rhythm Detection

Beyond pairwise coupling, the mutation log contains a signal about the practitioner's own creative cadence that no other tool surfaces.

### What Rhythms Reveal

Mutation timestamps aren't random. They cluster around the user's actual work sessions, and those sessions have structure:

- **Breakthrough precursors:** What consistently happens before a tension transitions from Stalling to Resolving? Often it's a desire refinement (re-engaging with what you actually want), not a reality update. If the system can detect this pattern, it can nudge: "Your breakthroughs tend to follow desire updates — consider revisiting what you want here."
- **Oscillation triggers:** What time patterns correlate with oscillation onset? Maybe oscillation spikes on Mondays (re-entry friction) or after 3+ days of neglect (loss of structural engagement). Knowing your trigger pattern lets you intervene before oscillation starts.
- **Resolution velocity by context:** Tensions with horizons resolve 3× faster than those without. Tensions worked in morning sessions advance more than evening sessions. These aren't universal truths — they're *your* structural tendencies, discoverable from *your* data.
- **Neglect cascades:** When you neglect one tension, do others follow? Or do you compensate by over-focusing elsewhere? The mutation log reveals whether you have a "one at a time" or "all or nothing" engagement pattern.

### Data Structures

```rust
/// Personal creative rhythm extracted from mutation history
pub struct StructuralRhythm {
    /// Hours of day with highest mutation density (work sessions)
    pub peak_hours: Vec<(u32, f64)>,          // (hour, relative_density)
    /// Days of week with highest advancement rate
    pub peak_days: Vec<(Weekday, f64)>,       // (day, advancement_ratio)
    /// What mutation type most often precedes a trajectory shift to Resolving
    pub breakthrough_precursors: Vec<(MutationKind, f64)>,  // (kind, frequency)
    /// Average gap between "last mutation" and "oscillation onset"
    pub neglect_to_oscillation_lag: Option<Duration>,
    /// Whether neglect tends to cascade (multiple tensions neglected together)
    pub neglect_cascades: bool,
    /// Horizon effect: ratio of resolution velocity with vs without horizon
    pub horizon_acceleration: f64,            // >1.0 means horizons help
}
```

### Computation

1. **Peak hours/days:** Histogram of all mutation timestamps, normalized by observation window. Filter to advancement-only mutations (gap decreased) for the advancement variant.
2. **Breakthrough precursors:** For each tension that transitioned from Stalling/Oscillating to Resolving, look at the mutation immediately before the transition. Tally mutation kinds (desire update, reality update, note, horizon set, child created). Report the distribution.
3. **Neglect-to-oscillation lag:** For each oscillation onset event, measure the gap since the previous mutation. Report the median.
4. **Neglect cascades:** When a tension enters neglect, check if 2+ other tensions also enter neglect within 48 hours. If this happens >50% of the time, `neglect_cascades = true`.
5. **Horizon acceleration:** Compare mean resolution velocity for tensions that had a horizon at creation vs. those that never had one.

Minimum data: 30+ mutations across 14+ days for meaningful rhythm detection. Below that, return `None`.

### Integration

**DynamicsEngine:**
```rust
pub fn compute_rhythm(&self, now: DateTime<Utc>) -> Option<StructuralRhythm>
```

Cached alongside `ResonanceField` on the 5-minute cycle. Recomputed only when mutation count changes.

**TUI — Insights overlay:**
Add a "Your Rhythm" section:
```
  Your Rhythm (last 30 days)
  Peak hours      09-11, 14-16
  Peak days       Tue, Thu
  Breakthroughs   preceded by desire update (62%), note (23%)
  Neglect lag     3.2 days before oscillation starts
  Horizon effect  2.8× faster resolution with deadlines
```

**CLI:**
```bash
werk rhythm              # human-readable rhythm summary
werk rhythm --json       # structured rhythm data
```

**Agent context:**
Include rhythm in `werk context --all` so agents can reason about the practitioner's patterns:
```json
{
  "rhythm": {
    "peak_hours": [9, 10, 14, 15],
    "breakthrough_precursors": [["desire_update", 0.62], ["note", 0.23]],
    "horizon_acceleration": 2.8
  }
}
```

## Open Questions

- **Asymmetric coupling direction:** Should the TUI show A→B and B→A separately, or collapse to the stronger direction? (Recommendation: show the stronger direction, note if asymmetric with an arrow: `Ship feature →◈ Fix auth` means Ship advancing pulls Fix auth, but not necessarily vice versa.)
- **Coupling window τ:** Default 48 hours. Should this be user-configurable? Should it adapt to the user's mutation cadence? (Recommendation: auto-calibrate τ to 2× the median inter-mutation interval for the field.)
- **Transitive resonance:** If A↔B and B↔C, should A↔C be inferred even without direct evidence? (Recommendation: no. Only report direct empirical coupling. Resonance groups already capture transitivity through connected components.)
- **Negative results:** Should "no coupling detected" be shown, or only significant couplings? (Recommendation: show isolated tensions in the field overlay, but not in per-tension Detail view. Absence of coupling is informative at the field level but noisy at the tension level.)

## Success Criteria

The feature is successful when a practitioner can:
1. See which tensions move together and which compete — without having declared any relationship
2. Use resonance information to choose what to work on next (field-aware lever)
3. Recognize their own structural patterns at the field level ("I always stall on docs when I'm shipping features")
4. Discover that reparenting or restructuring tensions to match resonance groups improves their flow
