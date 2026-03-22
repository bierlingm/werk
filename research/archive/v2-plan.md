# sd + ride — Build Plan

## Plan Version: V2 — Self-Contained Design

---

## 1. What This Is

Two things, built together:

**sd-core**: A Rust library that computes structural dynamics. Structural tension — the gap between a desired state and current reality — is the generative force behind creation (Robert Fritz). sd-core makes this force computational: it stores tensions and their mutation histories, computes dynamics (lifecycle, conflict, movement, neglect), and emits events when things change. It is a faithful implementation of Fritz's structural dynamics and nothing else. No opinions about rendering, interaction, or engagement. A grammar that any instrument can consume.

**ride**: The first instrument built on sd-core. An operative instrument — a persistent software system that holds the structural forces behind a practitioner's work, organizes engagement with those forces, renders them visibly, models the practitioner through encounter with reality, and launches agent activity within the structural field. ride is maximally opinionated, built for serious practitioners, and makes no attempt to accommodate casual use.

sd-core is the grammar. ride is a horse. Harnesses (Claude Code, etc.) direct the horse. The rider is the human.

---

## 2. Architecture

### The Grammar (sd-core)

A Rust library crate. Zero dependencies on any instrument.

**Data model** — two tables, nothing else:

```sql
CREATE TABLE tensions (
    id          TEXT PRIMARY KEY,   -- ULID
    desired     TEXT NOT NULL,      -- what the practitioner wants to be true
    actual      TEXT NOT NULL,      -- what is currently true
    parent_id   TEXT,               -- parent tension (nullable for roots)
    created_at  TEXT NOT NULL,      -- ISO-8601 timestamp
    status      TEXT NOT NULL       -- 'active', 'resolved', 'released'
);

CREATE TABLE mutations (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    tension_id  TEXT NOT NULL,
    timestamp   TEXT NOT NULL,      -- ISO-8601
    field       TEXT NOT NULL,      -- which field changed (or 'created', 'resolved', 'released')
    old_value   TEXT,
    new_value   TEXT
);
```

Everything else is computed. Lifecycle phase, structural conflict, movement, stagnation, neglect — all derived from these two tables by the dynamics engine. No flags, no cached state, no instrument concerns.

**Dynamics** — Fritz's structural dynamics, computed:

- **Lifecycle**: germination (tension declared, no confrontation yet) → assimilation (active confrontation, reality moving) → completion (reality converging on desired). Computed from mutation frequency and trajectory.
- **Structural conflict**: sibling tensions whose resolution paths compete. Computed from sibling activity patterns — one moving while another stagnates indicates conflict.
- **Movement / stagnation**: whether reality is being confronted. Computed from mutation recency.
- **Neglect**: parent/child asymmetry — active children with stagnant parent, or active parent with stagnant children.
- **Path of least resistance**: the structural tendency toward resolution when both desired and actual are clearly held.

All dynamics thresholds (what counts as "recent," what frequency indicates "active") are parameters passed by the calling instrument, not hardcoded in the grammar.

**Events** — typed, emitted for every state change and dynamic transition:

```
TensionCreated { id, desired, actual, parent_id }
RealityConfronted { id, old_actual, new_actual }
DesireRevised { id, old_desired, new_desired }
TensionResolved { id, reason }
TensionReleased { id, reason }
ConflictDetected { tension_a, tension_b }
ConflictResolved { tension_a, tension_b }
LifecycleTransition { id, from_phase, to_phase }
NeglectDetected { parent_id, neglected_children }
StructureChanged { parent_id, child_id, change_type }
```

Instruments subscribe. The grammar never calls instrument code.

**Storage** — frankensqlite for concurrent-write-safe SQLite with page-level MVCC. Multiple instruments and agents can read/write simultaneously.

**Store location** — `.sd/` directory:
- Walk up from CWD looking for `.sd/` (project-scoped)
- Fall back to `~/.sd/` (global)
- `sd::Store::init(path)` creates `.sd/sd.db` at a specific location
- `sd::Store::open()` finds the nearest `.sd/` or uses global

### The Instrument (ride)

A Rust binary crate. Depends on sd-core.

**Instrument-specific schema** — ride adds its own tables to the same `sd.db`:

```sql
CREATE TABLE ride_meta (
    tension_id  TEXT PRIMARY KEY,
    rank        INTEGER,
    notes       TEXT,
    tags        TEXT,              -- comma-separated
    time_scope  TEXT,              -- ISO-8601 duration or date
    FOREIGN KEY (tension_id) REFERENCES tensions(id)
);

CREATE TABLE ride_narrative (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    tension_id  TEXT,              -- nullable (can be general)
    timestamp   TEXT NOT NULL,
    content     TEXT NOT NULL,
    FOREIGN KEY (tension_id) REFERENCES tensions(id)
);

CREATE TABLE ride_engagement (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_type   TEXT NOT NULL,     -- constraint type identifier
    config      TEXT NOT NULL,     -- JSON rule configuration
    active      INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE ride_oracle (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp   TEXT NOT NULL,
    prediction  TEXT NOT NULL,
    outcome     TEXT,              -- 'confirmed', 'violated', 'unobservable'
    tension_id  TEXT,
    FOREIGN KEY (tension_id) REFERENCES tensions(id)
);
```

Grammar tables are sacred. ride's tables are ride's business. Other instruments would create their own prefixed tables.

**Capabilities** — what ride does that sd-core does not:

1. **CLI and TUI** — interactive access to the structure, built on ftui
2. **Practice ceremony** — structured confrontation with reality, one tension at a time
3. **Engagement rules** — configurable constraints (force desired revision after N updates, maximum children per tension, action-required gates, temporal rhythms)
4. **Narrative logging** — free-form notes attached to the structure or specific tensions
5. **Ambient presence** — compact pane and status bar modes for persistent structural awareness
6. **"You are here"** — tracks and displays the practitioner's current location in the structure
7. **Visual dynamics** — staleness degradation (fading relative to time scope), activity indicators, conflict visualization
8. **Oracle** — prediction-observation-surprise loop, shadow tension detection, behavioral audit
9. **Agent interface** — MCP server (fastmcp-rust) exposing structural state to AI agents
10. **Encryption** — database encryption at rest via age

---

## 3. Binding Constraints

1. **Fritz-pure grammar**: sd-core models exactly what Fritz described. Every concept traces to a Fritz text. No instrument concerns leak into the grammar.

2. **Two-table schema**: tensions and mutations. All dynamics are computed. If you delete all code and keep only the data, you can reconstruct all dynamics by re-running the grammar.

3. **Events as extension boundary**: The grammar emits; instruments subscribe. The grammar never calls instrument code. The grammar compiles with zero instrument dependencies.

4. **frankensqlite**: Concurrent-write-safe from day one. Multiple instruments and agents can operate on the same `.sd/sd.db`.

5. **Encryption at rest**: A serious practitioner's structural tensions are sensitive. The database is unreadable without the key.

6. **No gamification**: No streaks, achievements, points, or rewards for tool usage. The gap between desired and actual is the only signal.

7. **Instruments are maximally opinionated**: ride makes its own decisions about engagement, rendering, and interaction without attempting to serve all users. Other instruments can exist and make entirely different choices.

---

## 4. Architecture Decisions

### ADR-1: Structural Dynamics Modeled with Structural Dynamics

The data model is a tension (desired, actual) and a mutation history. Everything else — lifecycle, conflict, movement, neglect — is computed from these two primitives. The grammar's own structure embodies its subject matter.

This means: no lifecycle field, no conflict flag, no rank, no notes. Instruments that need additional per-tension data maintain their own tables. The grammar stores forces and their history. It computes dynamics. That's all.

### ADR-2: Grammar Is a Library; Instruments Are Binaries

sd-core is a Rust library crate. ride is a binary crate in the same workspace. Other instruments are additional binary crates. The grammar has no IO dependencies beyond frankensqlite. Instruments can be built by anyone who depends on the crate.

### ADR-3: Events as Public API

The grammar's public interface is: query functions (read structural state) and events (react to changes). Events are comprehensive enough that instruments don't need to poll. Events are deterministic: same data = same events regardless of which instruments are listening.

### ADR-4: Pure Fritz in Grammar

Every concept in sd-core traces to Fritz's published work: structural tension, structural conflict, the creative cycle (germination/assimilation/completion), resolution, release, path of least resistance. The grammar's documentation cites Fritz chapter and verse. Anything Fritz didn't describe belongs in an instrument, not the grammar.

### ADR-5: Shared .sd/ With Instrument-Prefixed Extensions

All instruments share one `.sd/sd.db` per scope. Grammar tables are shared ground. Each instrument creates its own prefixed tables (`ride_*`, `future_*`). This enables: multiple instruments operating on the same structural state, agents interacting through the grammar without needing any specific instrument, and clean separation of concerns.

### ADR-6: Dynamics Thresholds as Parameters

The grammar computes dynamics but does not hardcode what "recent" or "active" or "stagnant" means. These thresholds are parameters passed by the calling instrument. Different instruments (or the same instrument in different contexts) can use different thresholds. The grammar is the physics engine; instruments set the constants.

---

## 5. Workspace Structure

```
sd/                                 # Workspace root (git repo)
  Cargo.toml                        # Workspace manifest

  sd-core/                          # The Grammar
    Cargo.toml
    src/
      lib.rs                        # Public API surface
      tension.rs                    # Tension type, creation, modification
      mutation.rs                   # Mutation log — append, query, history
      dynamics.rs                   # Fritz computations
        lifecycle.rs                #   germination / assimilation / completion
        conflict.rs                 #   structural conflict detection
        movement.rs                 #   movement / stagnation / neglect
      events.rs                     # Typed event system
      store.rs                      # frankensqlite — schema, CRUD, .sd/ discovery
      tree.rs                       # Tree construction, traversal, query

  ride/                             # The First Instrument
    Cargo.toml
    src/
      main.rs                       # Entry: CLI (clap), dispatch
      schema.rs                     # ride_* tables — meta, narrative, engagement, oracle
      engagement.rs                 # Rule engine — configurable structural constraints
      narrative.rs                  # Free-form logging attached to structure
      encryption.rs                 # age encryption for sd.db
      agent.rs                      # MCP server via fastmcp-rust
      oracle/
        mod.rs                      # Oracle orchestration
        predict.rs                  # Prediction generation from structural state
        observe.rs                  # Observation collection from session evidence
        surprise.rs                 # Surprise detection — prediction vs. observation
        shadow.rs                   # Shadow tension detection
      tui/
        mod.rs                      # TUI core — app state, mode routing, Elm architecture
        tree.rs                     # Tree view — living structure with computed dynamics
        detail.rs                   # Tension detail — desired, actual, history, children
        practice.rs                 # Practice ceremony — confrontation interaction
        pulse.rs                    # Pulse dashboard — structural vital signs
        spatial.rs                  # Spatial/graph view of the structure
        ambient.rs                  # Compact pane mode, status bar mode
        presence.rs                 # "You are here" — location and attention tracking
        editing.rs                  # Inline creation and editing
        palette.rs                  # Command palette — fuzzy search and navigation
```

---

## 6. Stop-Ship Criteria

All must pass before v1.0.

- [ ] sd-core compiles and passes all tests with zero instrument dependencies
- [ ] Every sd-core concept cites a Fritz source in doc comments
- [ ] Two-table schema (tensions + mutations) is the grammar's only stored state
- [ ] All dynamics (lifecycle, conflict, movement, neglect) are computed, not stored
- [ ] Dynamics thresholds are parameters, not hardcodes
- [ ] Events fire for every state change; an instrument can reconstruct state from events
- [ ] frankensqlite concurrent writes: two processes writing simultaneously without corruption
- [ ] `.sd/` discovery works: walk up from CWD, fall back to `~/.sd/`
- [ ] ride's tables are prefixed and do not touch grammar tables
- [ ] Encryption at rest: sd.db unreadable without key
- [ ] Practice ceremony forces honest confrontation with the gap between desired and actual
- [ ] Engagement rules are configurable and enforced (at least: forced desired revision, child limits, action gates)
- [ ] Agent sessions receive full structural context via MCP (tension, ancestors, siblings, children, dynamics)
- [ ] "You are here" shows the practitioner's current structural location
- [ ] Ambient mode renders in a compact tmux-compatible pane
- [ ] Oracle runs prediction-observation-surprise across sessions
- [ ] No gamification anywhere
- [ ] A serious practitioner uses this daily for a month and finds it indispensable

---

## 7. Beads

### Phase A: The Grammar (sd-core)

Everything depends on this. Ship it first, test it thoroughly, trust it completely.

#### A1: Tension Primitive
- **Produces**: `sd::Tension` type — id (ULID), desired, actual, parent_id, created_at, status
- **Blocked by**: —
- **Blocks**: A2, A3, A4, A5, A6, A8
- **Acceptance**: Type defined. Validates on construction (desired and actual non-empty). Serializable. Status enum: Active, Resolved, Released.
- **Effort**: S

#### A2: Mutation Log
- **Produces**: Append-only mutation recording and retrieval
- **Blocked by**: A1
- **Blocks**: A4, A5, A6
- **Acceptance**: Every change to a tension produces an immutable mutation record. Full history retrievable per-tension and globally. Fields tracked: desired, actual, status, parent_id, plus synthetic events (created, resolved, released).
- **Effort**: S

#### A3: frankensqlite Store
- **Produces**: Storage layer — schema creation, CRUD, `.sd/` directory discovery and initialization
- **Blocked by**: A1, A2
- **Blocks**: A4, A5, A6, A8, all ride work
- **Acceptance**: Two-table schema created on init. Tension CRUD works. Mutation log appends on every change. `.sd/` discovery walks CWD upward, falls back to `~/.sd/`. Two concurrent connections write without corruption.
- **Effort**: M

#### A4: Lifecycle Computation
- **Produces**: `sd::dynamics::lifecycle()` — computes Fritz's creative cycle phase
- **Blocked by**: A2, A3
- **Blocks**: A7
- **Acceptance**: Given a tension and its mutations, returns Germination (no confrontation history), Assimilation (active confrontation — mutation frequency above threshold), or Completion (reality converging on desired — trajectory analysis). Thresholds passed as parameters. All logic sourced to Fritz.
- **Effort**: M

#### A5: Structural Conflict Detection
- **Produces**: `sd::dynamics::conflict()` — detects Fritz's structural conflict between siblings
- **Blocked by**: A2, A3
- **Blocks**: A7
- **Acceptance**: Given siblings with mutation histories, detects when one tension's activity pattern suggests its resolution impedes another's. Asymmetric activity among siblings where structural relationship creates competition. Thresholds parameterized.
- **Effort**: M

#### A6: Movement, Stagnation, Neglect
- **Produces**: `sd::dynamics::movement()`, `sd::dynamics::neglect()`
- **Blocked by**: A2, A3
- **Blocks**: A7
- **Acceptance**: Movement = reality confronted within threshold period, trajectory shows change. Stagnation = active tension with no recent confrontation. Neglect = parent/child activity asymmetry. All thresholds parameterized.
- **Effort**: M

#### A7: Event System
- **Produces**: Typed events emitted on state changes and dynamic transitions
- **Blocked by**: A4, A5, A6
- **Blocks**: B1
- **Acceptance**: All event types defined (TensionCreated, RealityConfronted, DesireRevised, TensionResolved, TensionReleased, ConflictDetected, ConflictResolved, LifecycleTransition, NeglectDetected, StructureChanged). Instruments subscribe via callback. Grammar is deterministic.
- **Effort**: M

#### A8: Tree Construction and Query
- **Produces**: `sd::Tree` — recursive tree from flat table, traversal, ancestor/descendant/sibling queries
- **Blocked by**: A1, A3
- **Blocks**: B1
- **Acceptance**: Builds full tree from tensions table. Subtree extraction. Ancestor chain (root-first). Sibling groups. Efficient for trees up to ~1000 tensions.
- **Effort**: S

#### A9: Grammar Test Suite
- **Produces**: Comprehensive tests for all dynamics
- **Blocked by**: A4, A5, A6, A7, A8
- **Blocks**: — (but stop-ship criterion)
- **Acceptance**: Every dynamics function tested. Test names reference Fritz concepts. Edge cases: empty trees, single tensions, deep hierarchies, simultaneous mutations, all lifecycle transitions, conflict detection and resolution, neglect in both directions.
- **Effort**: M

---

### Phase B: Instrument Foundation (ride core)

The binary exists. CLI works. Extensions don't touch grammar tables. Agents can talk to the structure.

#### B1: CLI Skeleton
- **Produces**: ride binary with clap CLI, sd-core dependency, basic commands
- **Blocked by**: A7, A8
- **Blocks**: B2, B3, B4, B5, B6, C1
- **Acceptance**: `ride init` creates `.sd/`. `ride add` creates tension (prompts for desired, actual). `ride show <id>` displays tension with computed dynamics. `ride tree` renders text tree. `ride reality <id> "new actual"` updates reality. `ride resolve <id>`, `ride release <id>` close tensions.
- **Effort**: M

#### B2: Instrument Schema
- **Produces**: ride_meta, ride_narrative, ride_engagement, ride_oracle tables
- **Blocked by**: B1
- **Blocks**: B3, B4, C3, C5
- **Acceptance**: Tables created on first ride invocation. Foreign keys to grammar's tensions table. Grammar tables untouched. Instrument data joins with grammar data at query time.
- **Effort**: S

#### B3: Engagement Rules Engine
- **Produces**: Configurable structural constraints enforced during interaction
- **Blocked by**: B2
- **Blocks**: C3
- **Acceptance**: Rules stored in ride_engagement. At minimum: "revise desired after N actual updates" (forces re-examination of the desired state), "maximum M children per tension" (prevents diffusion), "action required before restructure" (must update an actual before you can change tree structure). Rules checked and enforced at appropriate interaction points. Rules are TOML-configurable.
- **Effort**: M

#### B4: Encryption
- **Produces**: Transparent database encryption using age
- **Blocked by**: B1
- **Blocks**: — (but stop-ship criterion)
- **Acceptance**: `ride init --encrypt` creates encrypted sd.db. Key management via passphrase or keyfile. Encryption/decryption transparent to grammar operations (sd-core reads decrypted connection). `ride lock` / `ride unlock` for session-based access.
- **Effort**: M

#### B5: MCP Agent Interface
- **Produces**: MCP server via fastmcp-rust exposing structural state
- **Blocked by**: B1
- **Blocks**: D3
- **Acceptance**: MCP tools: query_tree, query_tension, query_dynamics (lifecycle/conflict/movement for a tension), update_reality, create_tension, resolve_tension. Agent receives typed responses. Server runs as a background process or on-demand.
- **Effort**: L

#### B6: Narrative Logging
- **Produces**: Free-form notes attached to the structure
- **Blocked by**: B2
- **Blocks**: C3
- **Acceptance**: `ride note <id> "observation about this tension"` appends to ride_narrative. `ride note "general observation"` appends without tension attachment. Notes visible in detail view. Notes searchable.
- **Effort**: S

---

### Phase C: The Palace (ride TUI and presence)

The instrument becomes habitable. Practice ceremony makes it an operative instrument. Ambient modes make it persistent.

#### C1: TUI Core
- **Produces**: ftui-based TUI — app state, mode routing, responsive breakpoints, Elm architecture
- **Blocked by**: B1
- **Blocks**: C2, C3, C4, C5, C6, C7, C8, C9
- **Acceptance**: `ride tui` launches fullscreen. Modes: Tree, Detail, Practice, Pulse, Spatial. Mode switching via keybindings. Responsive breakpoints (Xs/Sm/Md/Lg) with degradation. Status bar with context-sensitive keybinding hints.
- **Effort**: M

#### C2: Tree View
- **Produces**: Interactive tree with computed dynamics rendered visually
- **Blocked by**: C1
- **Blocks**: C4
- **Acceptance**: Tree renders all tensions with: lifecycle indicators, conflict markers, movement/stagnation signals, staleness degradation (visual fading proportional to time since last confrontation relative to time scope). Navigation: j/k, expand/collapse, enter for detail. Inline creation: a (child), A (sibling), o (root).
- **Effort**: L

#### C3: Practice Ceremony
- **Produces**: Structured confrontation — one tension at a time
- **Blocked by**: C1, B2, B3, B6
- **Blocks**: —
- **Acceptance**: `ride practice` or `p` from TUI. For each tension: desired state displayed prominently, current actual displayed, practitioner edits actual honestly. Supports filtered practice: `--stuck` (stagnant tensions only), `--subtree <id>` (branch only), `--conflicted` (tensions in structural conflict). Engagement rules enforced (e.g., after N actualities, prompt: "Is this still what you want?" forcing desired revision). Narrative logging available mid-practice. Summary on completion: what moved, what was skipped, what was revised.
- **Effort**: L

#### C4: Pulse Dashboard
- **Produces**: Structural vital signs — all dynamics computed live
- **Blocked by**: C2
- **Blocks**: —
- **Acceptance**: Shows: moving tensions, stagnant tensions, structural conflicts, neglect asymmetries, lifecycle distribution. All computed from grammar, nothing stored. Tension selection jumps to detail.
- **Effort**: M

#### C5: "You Are Here"
- **Produces**: Location tracking and attention visualization
- **Blocked by**: C1, B2
- **Blocks**: —
- **Acceptance**: ride tracks which tension the practitioner is currently "in" (last interacted with, or explicitly set). Activity history shows attention distribution: "your gravity is here" across time scales. Rendered as a persistent indicator in tree view and ambient modes.
- **Effort**: M

#### C6: Ambient Compact Pane
- **Produces**: 20-30 column rendering for tmux split or persistent terminal tab
- **Blocked by**: C1
- **Blocks**: —
- **Acceptance**: Tree with dynamics indicators (lifecycle badge, staleness, conflict) in narrow format. Updates live. Usable as a tmux pane alongside editor.
- **Effort**: M

#### C7: Ambient Status Bar
- **Produces**: Single-line structural summary for tmux status bar or prompt integration
- **Blocked by**: C1
- **Blocks**: —
- **Acceptance**: Format: "[location] N moving · N stuck · N conflicted" or similar. Callable as `ride status --oneline`. Integrates with tmux status-right.
- **Effort**: S

#### C8: Detail View
- **Produces**: Full tension portrait — desired, actual, mutation history, children, siblings, dynamics
- **Blocked by**: C1
- **Blocks**: —
- **Acceptance**: Shows: desired state, current actual, full mutation history (reality journal), children tree, siblings, computed dynamics (lifecycle, conflicts, movement), narrative log. Inline editing of actual and desired. Navigation to parent, children, siblings.
- **Effort**: M

#### C9: Spatial View
- **Produces**: Graph rendering of the tension structure
- **Blocked by**: C1
- **Blocks**: —
- **Acceptance**: Tensions as nodes, parent-child as edges. Layout conveys structural properties. Activity indicators on nodes. Navigation with directional keys. Selection jumps to detail.
- **Effort**: M

#### C10: Command Palette
- **Produces**: Fuzzy search and navigation across all tensions
- **Blocked by**: C1
- **Blocks**: —
- **Acceptance**: `/` or `:` opens palette. Fuzzy match on desired/actual text. Results show dynamics indicators. Selection navigates to tension. Parent-search mode for reparenting.
- **Effort**: M

---

### Phase D: The Oracle (ride intelligence)

The instrument learns. The feedback loop closes.

#### D1: Prediction Engine
- **Produces**: Per-session predictions about structural movement
- **Blocked by**: A7, B2
- **Blocks**: D2, D4
- **Acceptance**: Before session: analyze mutation history and dynamics, generate predictions about which tensions will move/stall/conflict. Store predictions in ride_oracle. After session: compare predictions to actual events. Classify outcomes: confirmed, violated, unobservable. Calibration tracking over time.
- **Effort**: L

#### D2: Shadow Tension Detection
- **Produces**: System proposes tensions the practitioner hasn't articulated
- **Blocked by**: D1
- **Blocks**: —
- **Acceptance**: After sufficient history (configurable threshold), analyze patterns of narrative logs and reality updates that cluster around unnamed themes. Propose: "You keep confronting reality in area X but have no tension for it." Practitioner confirms, rejects, or refines.
- **Effort**: L

#### D3: Agent Structural Context
- **Produces**: Rich context injection for agent sessions
- **Blocked by**: B5, D1
- **Blocks**: —
- **Acceptance**: When an agent session departs from a tension, it receives via MCP: the tension (desired, actual), ancestor chain (path to root), siblings (structural context), children (sub-structure), computed dynamics (lifecycle, conflicts, movement), oracle predictions (what the system expects to happen). Agent work feeds back as reality updates and new sub-tensions through MCP.
- **Effort**: M

#### D4: Behavioral Audit
- **Produces**: Periodic check of structure against actual behavior
- **Blocked by**: D1
- **Blocks**: —
- **Acceptance**: After configurable interval, compare: what the structure says the practitioner cares about vs. where they actually spend attention (mutation frequency, narrative log content, "you are here" history). Surface contradictions: "Your structure says X matters but you haven't confronted it in 30 days while spending all attention on Y."
- **Effort**: L

---

### Phase E: Extensions

Built when ready. Each independent.

#### E1: Cosmic Calendar
- **Produces**: Custom time structures — daily, weekly, monthly, quarterly, seasonal, lunar, arbitrary
- **Blocked by**: B3
- **Effort**: M

#### E2: Named Structural Positions
- **Produces**: Pattern library — recognized structural configurations with names
- **Blocked by**: A9
- **Effort**: M

#### E3: Resolved Tension Templates
- **Produces**: Automation that captures resolved tension patterns for reuse
- **Blocked by**: A7
- **Effort**: S

#### E4: Commitment Sizing
- **Produces**: Kelly criterion-inspired commitment scoping — always set an end point
- **Blocked by**: B3
- **Effort**: M

#### E5: Cooperation Primitives
- **Produces**: Shared tensions, structural contracts between practitioners
- **Blocked by**: B5
- **Effort**: L

---

## 8. Critical Path

```
A1 → A2 → A3 → A4/A5/A6 (parallel) → A7 → B1 → C1 → C2/C3 (parallel)
                                              ↘ B5 → D1 → D3
```

Grammar first. Then instrument CLI. Then TUI core. Then practice ceremony and tree view in parallel (the two things that make it usable daily). Agent interface and oracle in parallel on a second track.

**Foundational beads** (most downstream dependents):
- A1 (tension primitive) — everything depends on this
- A3 (store) — all persistence depends on this
- A7 (events) — all reactivity depends on this
- B1 (CLI skeleton) — all instrument work depends on this
- C1 (TUI core) — all rendering depends on this

**First usable milestone**: After B1, the instrument is usable as a CLI. After C3, it's an operative instrument (practice ceremony exists). After C6/C7, it's ambient. After D1, it learns.

---

## 9. Open Questions

1. **Naming confirmation**: sd-core and ride. Are these final?

2. **Fritz fidelity depth**: Do we implement "path of least resistance" analysis (computing which resolution path has least structural resistance)? Or stop at lifecycle + conflict + movement + neglect?

3. **Practice ceremony interaction**: Filtered modes (--stuck, --subtree, --conflicted) are specified. What about conditional modes from engagement rules? How does "action required before restructure" present in the TUI?

4. **Oracle architecture**: Compiled into the ride binary, or separate process? Separate is cleaner but adds operational complexity.

5. **Encryption implementation**: age on the whole sd.db file (simple, encrypt at rest, decrypt on open) or SQLite-level encryption (more complex, column-level possible)?

6. **The "in/on/off" grammar**: Should ride have explicit modes for "working in the structure" vs. "working on the structure"? If so, what changes between them? Or does this emerge from the distinction between practice ceremony (in) and tree editing (on)?

7. **Narrative logging scope**: ride_narrative stores notes. Should the oracle also read narrative logs as evidence? This would mean narrative is both human expression and oracle input.

8. **Instrument discovery**: Should instruments be able to discover each other's presence in a shared `.sd/`? A simple registry table (`sd_instruments`) listing which instruments have been active?
