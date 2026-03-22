# The Build Plan

## Plan Version: V1 — Architecture from First Principles

### Changes from V0: Everything. Greenfield design informed by v0.1, v0.2, and the discovery phase.

---

## 1. Problem and Synthesis

### Problem Statement

Structural tension — the gap between a desired state and current reality — is the generative force behind all creation (Fritz). Every person, project, and cooperation has structural tensions whether or not they are made explicit. When they are explicit, held honestly, and confronted regularly, they tend toward resolution. When they are implicit, ignored, or dishonestly described, they produce oscillation, stagnation, or dissolution.

No computational system exists that models structural dynamics faithfully. Existing tools either reduce tensions to tasks (project managers), reduce them to metrics (OKR tools), or gamify them (habit trackers). None hold the actual force — the gap itself — as the primitive.

### Existing Solutions Map

| Solution | What it gets right | Where it falls short | Idea extracted |
|----------|-------------------|---------------------|----------------|
| OKR tools (Quantive, Viva Goals) | Operationalization pressure — forces measurable proxies | Reduces tension to metrics; loses the felt quality of the gap | Convergence can be computed when conditions are specific |
| Fritz's teaching (courses, books) | The theory is correct and complete | No computational implementation; remains pedagogical | The grammar IS the theory, implemented faithfully |
| Journaling/reflection tools | Honest confrontation with reality | No structure; no dynamics; no computation | Narrative logging attached to structure |
| Agent harnesses (Claude Code, Hermes) | Context shapes agent behavior through hooks/injection | No structural model; agents work in a void of intention | Agent sessions depart from and return to structural state |
| Alchemical/hermetic systems | The practice transforms the practitioner; the laboratory is both workspace and mirror | Not computational; requires initiation | The operative instrument as consecrated workspace |
| Anima (prediction-observation-surprise) | Self-modeling that converges on truth through surprise | Applied to identity, not to structural dynamics generally | The grammar's own dynamics can be observed and predicted |

### Unique Value Proposition

A faithful computational implementation of Fritz's structural dynamics, designed as a library that any instrument can consume. Plus: the first operative instrument built on that library, for serious practitioners who want an honest mirror and a living workspace.

---

## 2. Binding Constraints

1. **Fritz-pure grammar**: The core library models exactly what Fritz described. No additions, no interpretations, no heft, no tellability, no custom metrics. Instruments extend; the grammar computes. Binary test: can every concept in the grammar be traced to a specific Fritz text?

2. **Two-table schema**: The data model is `tensions` and `mutations`. Everything else is computed. Binary test: if you delete all code except the schema and the data, can you reconstruct all dynamics by re-running the grammar?

3. **Hooks/events as extension boundary**: The grammar emits events (tension_created, reality_updated, conflict_detected, etc.). Instruments subscribe. The grammar never calls instrument code. Binary test: can the grammar crate compile with zero instrument dependencies?

4. **Greenfield**: No code from v0.1 or v0.2 is assumed preserved. Design decisions are made fresh. Prior implementations are reference material, not foundations.

5. **frankensqlite**: Concurrent-write-safe storage from the start. No single-writer assumptions.

6. **Encryption at rest**: Serious users accumulate sensitive data. `age` encryption or equivalent. Binary test: can the database file be read without the key?

7. **No gamification**: No streaks, no achievements, no points, no levels. The gap between desired and actual IS the signal. Binary test: does any feature reward the user for using the tool rather than for closing real gaps?

---

## 3. Architecture Decision Records

### ADR-1: The Grammar Models Structural Dynamics with Structural Dynamics

**Decision**: The data model is a tension (desired, actual) and a mutation log. Everything else — lifecycle, conflict, pulse, neglect — is computed by the grammar from these two primitives. The grammar's own structure is itself an expression of structural dynamics.

**Alternatives considered**:
- Rich schema with lifecycle fields, conflict flags, rank, notes, status enums (v0.2 approach)
- Document-oriented model where each tension is a rich JSON blob
- Graph database with typed edges

**Rationale**: Fritz's insight is that structural tension is the primitive. The data model should embody this. A tension IS a desired state and a current reality. The gap IS the force. Everything else is dynamics computed from the history of how that gap has been confronted. Storing computed state as fields conflates the map with the territory.

**Consequences**:
- Lifecycle, conflict, and pulse are functions, not fields. They must be efficient to compute.
- The mutation log is the single source of truth for all dynamics. It must be append-only, immutable, and complete.
- Instruments that need additional per-tension metadata (rank, notes, tags, time-scoping) store it in their own tables, not in the grammar's schema.
- Migration from v0.2 is a data extraction, not a schema migration.

### ADR-2: Grammar Is a Library Crate; Instruments Are Binaries

**Decision**: `sd-core` (or whatever name) is a Rust library crate that exposes the data model, grammar computations, and an event system. Instruments are separate binary crates that depend on the library.

**Alternatives considered**:
- Monolithic binary with module boundaries (v0.2 approach)
- API server with instrument clients
- WASM-based plugin architecture

**Rationale**: Multiple instruments are possible and desirable. The grammar must be separable. A library crate is the simplest, most Rust-idiomatic way to achieve this. An API server adds latency and complexity that a library avoids. WASM is premature.

**Consequences**:
- The grammar crate has zero TUI, CLI, or IO dependencies beyond frankensqlite.
- Instruments can be built by anyone who depends on the crate.
- The first instrument (your operative instrument) is a binary crate in the same workspace.
- Testing the grammar is independent of testing any instrument.

### ADR-3: Events as Extension Boundary

**Decision**: The grammar emits typed events for every state change and computed transition. Instruments subscribe to events and react. The grammar never calls into instrument code.

**Alternatives considered**:
- Callback registration (grammar calls instrument functions)
- Polling (instruments query grammar state periodically)
- Shared mutable state

**Rationale**: Events preserve the grammar's purity. The grammar doesn't know what instruments exist or what they do with state changes. This is the same pattern as Claude Code's hooks: the system emits; consumers react. It also enables concurrent instruments — multiple consumers can subscribe to the same grammar's events.

**Consequences**:
- The event type system must be comprehensive enough that instruments don't need to poll.
- Events are the grammar's public API alongside query functions.
- The grammar must be deterministic: same data + same computations = same events, regardless of which instruments are listening.

### ADR-4: Pure Fritz, No Extensions in Grammar

**Decision**: Every concept in the grammar traces to Fritz's published work. The grammar implements structural tension, structural conflict, the creative cycle (germination/assimilation/completion), resolution, release, and the path of least resistance. Nothing else.

**Alternatives considered**:
- Including pulse/heft/tellability in the grammar
- Including time-scoping and attention budgets
- Including agent-related concepts

**Rationale**: The grammar is a faithful computational model of Fritz's structural dynamics. Its value comes from correctness, not from features. Instruments add everything else. This keeps the grammar small, stable, and trustworthy. It also means improvements to the grammar are improvements to Fritz's theory made computational — a scholarly contribution, not just a product feature.

**Consequences**:
- The grammar must be expressible in Fritz's vocabulary. If a concept can't be sourced to Fritz, it doesn't belong.
- Instruments carry the weight of innovation. Heft, tellability, cosmic calendars, anima, formations, agent integration — all instrument concerns.
- The grammar's API documentation should cite Fritz chapter and verse.
- The grammar may need to be revisited as Fritz's work is studied more deeply. The event system allows this without breaking instruments.

### ADR-5: Greenfield Design, frankensqlite Storage

**Decision**: Clean-slate implementation using frankensqlite for concurrent-write-safe SQLite with page-level MVCC.

**Alternatives considered**:
- Continuing with rusqlite (v0.2 approach)
- PostgreSQL / other server database
- File-based storage (JSON, YAML)
- Custom append-only log

**Rationale**: frankensqlite gives concurrent writes (multiple agents, multiple instruments), clean-room implementation, and SQLite's embeddability. No server process needed. The grammar remains a library, not a service.

**Consequences**:
- Dependency on Emanuel's frankensqlite crate. Monitor maturity.
- Concurrent multi-instrument access is architecturally supported from day one.
- Encryption at rest can be layered on the SQLite file (age, SQLCipher, or frankensqlite-native if supported).
- Migration tooling needed for anyone coming from v0.2.

### ADR-6: The Instrument Is Maximally Opinionated

**Decision**: The first instrument (your operative instrument) makes its own decisions about engagement, rendering, self-modeling, and agent integration without attempting to be general-purpose or configurable for all possible users.

**Alternatives considered**:
- Building a configurable platform that accommodates many styles
- Building a minimal reference instrument
- Building the instrument as a framework others customize

**Rationale**: Other instruments can exist. This one is for serious, high-ambition practitioners who want an honest mirror. It doesn't soften vocabulary, doesn't simplify for onboarding, doesn't accommodate casual use. Maximum opinion, zero apology. The grammar is the general-purpose layer; the instrument is the specific expression.

**Consequences**:
- UX decisions serve the target practitioner, not the broadest audience.
- Features like the self-model (oracle/anima), agent origination, ambient presence, and configurable engagement rules are instrument decisions, not grammar decisions.
- Other instruments built on the same grammar can make entirely different choices.
- The instrument's name, vocabulary, and metaphor system are its own.

---

## 4. Stability Rings

### Ring 0: The Grammar (Sacred Core)

The computational model of Fritz's structural dynamics.

**Contains**:
- Data model: Tension (id, desired, actual, parent_id, created_at, status) + Mutation log (tension_id, timestamp, field, old_value, new_value)
- Structural dynamics computation: lifecycle phase, structural conflict detection, movement/stagnation, neglect, path of least resistance analysis
- Event emission: typed events for all state changes and computed transitions
- Query interface: tree construction, history retrieval, dynamics computation
- Store abstraction over frankensqlite

**Does not contain**: Anything an instrument adds. No rank, no notes, no tags, no time-scoping, no heft, no rendering, no CLI, no TUI, no agent concepts, no self-model.

**Change cost**: Extremely high. Changes here ripple to all instruments.

### Ring 1: The Instrument Core

The first operative instrument's foundational capabilities, built on Ring 0.

**Contains**:
- Instrument-specific schema extensions (rank, notes, tags, time-scoping, encryption, narrative log)
- Engagement configuration (temporal rhythms, conditional rules, integrity constraints, personal evolutionary strategy)
- CLI command structure and REPL
- Agent interface (MCP via fastmcp-rust)
- Import/export (structure serialization, migration from v0.2)

### Ring 2: The Palace

Rendering and presence.

**Contains**:
- TUI (ftui-based, Elm/Bubbletea architecture)
- Practice ceremony (the confrontation interaction)
- Ambient modes (compact pane, status bar, inline)
- Tree view, detail view, pulse dashboard, spatial view
- Visual dynamics (degradation/staleness rendering, activity indicators, conflict visualization)
- "You are here" indicator and attention mapping

### Ring 3: The Oracle

Self-modeling and intelligence.

**Contains**:
- Anima-equivalent: prediction-observation-surprise loop applied to the structure
- Shadow tension detection (proposing tensions the practitioner hasn't articulated)
- Behavioral audit (checking whether the structure matches actual behavior)
- Pattern recognition (named structural positions, recurring dynamics)
- Meta-synthesis (periodic structural review across the whole tree)

### Ring 4: Extensions

Optional, composable capabilities.

**Contains**:
- Interpretive lenses (Doolittle ternary, custom dimensions)
- Cosmic calendar / custom time structures
- Template automation from resolved tensions
- Named positions / pattern library
- Kelly criterion commitment sizing
- Cooperation primitives (shared tensions, structural contracts)
- Additional agent integrations

---

## 5. Workspace Structure

```
workspace/
  sd-core/                    # Ring 0: The Grammar
    src/
      lib.rs                  # Public API
      tension.rs              # Tension primitive (desired, actual, parent, status)
      mutation.rs             # Mutation log (append-only event history)
      dynamics.rs             # Fritz computations (lifecycle, conflict, movement, neglect)
      events.rs               # Typed event system
      store.rs                # frankensqlite abstraction
      tree.rs                 # Tree construction and traversal
    Cargo.toml

  instrument/                 # Rings 1-4: The Operative Instrument
    src/
      main.rs                 # Entry point, CLI (clap)
      schema.rs               # Instrument-specific schema extensions
      engagement.rs           # Engagement configuration and rules engine
      agent.rs                # MCP server interface (fastmcp-rust)
      import.rs               # v0.2 migration, structure import/export
      oracle/
        mod.rs                # Self-model orchestration
        anima.rs              # Prediction-observation-surprise loop
        shadow.rs             # Shadow tension detection
        audit.rs              # Behavioral audit
      tui/
        mod.rs                # TUI core (ScApp, modes, routing)
        tree.rs               # Tree view
        detail.rs             # Detail view
        practice.rs           # Practice ceremony
        pulse.rs              # Pulse dashboard
        spatial.rs            # Spatial/sigil view
        ambient.rs            # Compact pane, status bar modes
        presence.rs           # "You are here" indicator
      extensions/
        mod.rs                # Extension registry
        lenses.rs             # Interpretive lenses
        calendar.rs           # Cosmic calendar / custom time
        templates.rs          # Resolved tension templates
        patterns.rs           # Named structural positions
    Cargo.toml

  Cargo.toml                  # Workspace root
```

---

## 6. Stop-Ship Criteria

All must pass before v1.0 ships.

- [ ] The grammar crate compiles and passes all tests with zero instrument dependencies
- [ ] Every grammar concept traces to a specific Fritz text (documented in code comments)
- [ ] The two-table schema (tensions + mutations) is the only stored state in the grammar
- [ ] All dynamics (lifecycle, conflict, movement, neglect) are computed, not stored
- [ ] Events fire for every state change; an instrument can reconstruct full state from events alone
- [ ] frankensqlite concurrent writes work: two instrument instances can write simultaneously without corruption
- [ ] Encryption at rest: the database file is unreadable without the key
- [ ] Practice ceremony exists and forces honest confrontation (desired displayed, actual edited, gap held)
- [ ] Agent sessions can be launched from a tension with full structural context injected
- [ ] "You are here" indicator shows the practitioner's current location in the structure
- [ ] Ambient mode renders structural state in a compact tmux-compatible pane
- [ ] The self-model (oracle) runs the prediction-observation-surprise loop across sessions
- [ ] No gamification anywhere. No streaks, badges, points, or rewards for tool usage.
- [ ] A serious practitioner can use this daily for a month and find it indispensable

---

## 7. Bead Graph

### Phase A: The Grammar (Ring 0)

The foundation. Everything depends on this.

#### A1: Data Model — Tension Primitive
- **Produces**: Rust types for Tension and TensionNode
- **Blocked by**: Nothing
- **Blocks**: Everything else
- **Acceptance**: Tension struct with (id, desired, actual, parent_id, created_at, status). Serializable. Roundtrips through frankensqlite.
- **Effort**: S

#### A2: Data Model — Mutation Log
- **Produces**: Append-only mutation recording and retrieval
- **Blocked by**: A1
- **Blocks**: A4, A5, A6
- **Acceptance**: Every field change on a tension produces a mutation record. Mutations are immutable once written. Full history retrievable per tension and globally.
- **Effort**: S

#### A3: Store — frankensqlite Integration
- **Produces**: Storage layer with concurrent write support
- **Blocked by**: A1, A2
- **Blocks**: A4, A5, A6, all instrument work
- **Acceptance**: Two-table schema created. CRUD operations work. Two concurrent connections can write without corruption.
- **Effort**: M

#### A4: Dynamics — Lifecycle Computation
- **Produces**: Functions that compute Fritz's creative cycle phases from mutation history
- **Blocked by**: A2, A3
- **Blocks**: A7
- **Acceptance**: Given a tension and its mutation history, correctly computes germination (no confrontation), assimilation (active confrontation), completion (convergence). All logic sourced to Fritz.
- **Effort**: M

#### A5: Dynamics — Structural Conflict Detection
- **Produces**: Functions that detect structural conflict between sibling tensions
- **Blocked by**: A2, A3
- **Blocks**: A7
- **Acceptance**: Given a set of siblings with mutation histories, detects when one's resolution path impedes another's. Fritz's structural conflict, not just activity asymmetry.
- **Effort**: M

#### A6: Dynamics — Movement, Stagnation, Neglect
- **Produces**: Functions that compute whether a tension is moving, stuck, or neglected
- **Blocked by**: A2, A3
- **Blocks**: A7
- **Acceptance**: Movement = reality confronted recently, trajectory toward desired. Stagnation = no confrontation despite active status. Neglect = parent/child asymmetry. Thresholds configurable (instrument provides them).
- **Effort**: M

#### A7: Event System
- **Produces**: Typed events emitted for all state changes and dynamic transitions
- **Blocked by**: A4, A5, A6
- **Blocks**: B1
- **Acceptance**: Events for: tension_created, tension_updated, reality_confronted, desire_revised, conflict_detected, conflict_resolved, lifecycle_transition, tension_resolved, tension_released, neglect_detected. Instruments can subscribe. Grammar is deterministic.
- **Effort**: M

#### A8: Tree Construction and Query
- **Produces**: Functions to build tension trees, traverse, query ancestors/descendants/siblings
- **Blocked by**: A1, A3
- **Blocks**: B1
- **Acceptance**: Build full tree from flat table. Efficient subtree extraction. Ancestor chain. Sibling groups. All used by dynamics computations.
- **Effort**: S

#### A9: Grammar Test Suite
- **Produces**: Comprehensive tests for all dynamics, sourced to Fritz
- **Blocked by**: A4, A5, A6, A7, A8
- **Blocks**: Nothing (but required for stop-ship)
- **Acceptance**: Every dynamics function has tests. Test names reference Fritz concepts. Edge cases covered: empty trees, single tensions, deep hierarchies, concurrent mutations.
- **Effort**: M

### Phase B: Instrument Foundation (Ring 1)

The operative instrument's core, built on the grammar.

#### B1: CLI Skeleton and Workspace Setup
- **Produces**: Binary crate with clap CLI, workspace Cargo.toml, sd-core dependency
- **Blocked by**: A7, A8
- **Blocks**: B2, B3, B4, B5
- **Acceptance**: `werk init`, `werk add`, `werk show`, `werk tree` work at minimum. Binary compiles. Grammar crate is a dependency.
- **Effort**: S

#### B2: Instrument Schema Extensions
- **Produces**: Instrument-specific tables (rank, notes, tags, time-scoping, narrative log)
- **Blocked by**: B1
- **Blocks**: C1, C3
- **Acceptance**: Extensions stored in separate tables, not in grammar tables. Grammar schema untouched. Instrument joins its data with grammar data at query time.
- **Effort**: M

#### B3: Engagement Configuration
- **Produces**: Rule engine for personal evolutionary strategy (configurable constraints)
- **Blocked by**: B1
- **Blocks**: C3
- **Acceptance**: Rules like "force desired revision after N actual updates", "maximum M children per tension", "require action before restructuring" are configurable and enforced. Rules stored as instrument config, not grammar data.
- **Effort**: M

#### B4: Encryption at Rest
- **Produces**: Database encryption using age or equivalent
- **Blocked by**: B1
- **Blocks**: Nothing (but required for stop-ship)
- **Acceptance**: Database file unreadable without key. Key management is simple (passphrase or keyfile). Transparent to grammar operations.
- **Effort**: M

#### B5: Agent Interface (MCP)
- **Produces**: MCP server via fastmcp-rust exposing structural state to AI agents
- **Blocked by**: B1
- **Blocks**: D3
- **Acceptance**: Agent can query tension tree, read dynamics (lifecycle, conflict, movement), update reality, receive structural context. Replaces v0.2's `--robot` flag with proper MCP protocol.
- **Effort**: L

#### B6: v0.2 Migration
- **Produces**: Import tool that reads v0.2 SQLite and produces grammar-compatible data
- **Blocked by**: B1, B2
- **Blocks**: Nothing
- **Acceptance**: All tensions, mutation history, and structure preserved. Instrument-specific data (rank, notes) migrated to instrument tables.
- **Effort**: M

### Phase C: The Palace (Ring 2)

Rendering and presence.

#### C1: TUI Core — Mode Routing and Layout
- **Produces**: ftui-based TUI with Elm architecture, mode system, responsive breakpoints
- **Blocked by**: B2
- **Blocks**: C2, C3, C4, C5, C6, C7
- **Acceptance**: TUI launches. Modes route correctly. Responsive breakpoints work. Status bar shows keybindings.
- **Effort**: M

#### C2: Tree View — The Living Structure
- **Produces**: Interactive tree rendering with computed dynamics (lifecycle, conflict, movement) visualized
- **Blocked by**: C1
- **Blocks**: C4
- **Acceptance**: Tree renders with visual dynamics: lifecycle indicators, conflict markers, movement/stagnation signals, staleness degradation (fading relative to time-scoped horizon). Navigation with j/k/expand/collapse.
- **Effort**: L

#### C3: Practice Ceremony
- **Produces**: The confrontation interaction — one tension at a time, desired displayed, actual edited, gap held
- **Blocked by**: C1, B2, B3
- **Blocks**: Nothing
- **Acceptance**: Practice walks tensions. For each: desired state shown prominently, current actual shown, practitioner edits actual honestly. Engagement rules enforced (e.g., filtered practice — stuck only, subtree only). No skip-without-consequence. Narrative logging available during practice.
- **Effort**: L

#### C4: Pulse Dashboard
- **Produces**: Structural vital signs — moving, stuck, conflicting, neglected, lifecycle distribution
- **Blocked by**: C2
- **Blocks**: Nothing
- **Acceptance**: All dynamics computed by grammar displayed in dashboard layout. No stored flags — all computed live.
- **Effort**: M

#### C5: "You Are Here" Indicator
- **Produces**: Persistent display of the practitioner's current location and attention center of gravity
- **Blocked by**: C1
- **Blocks**: Nothing
- **Acceptance**: The instrument tracks which tension the practitioner is "in" (working within). Activity history shows attention distribution across time scales. "You live in the week, you work on X" is derivable.
- **Effort**: M

#### C6: Ambient Modes
- **Produces**: Compact pane mode (20-30 cols), status bar mode (single line), for tmux/persistent display
- **Blocked by**: C1
- **Blocks**: Nothing
- **Acceptance**: Compact mode shows tree with dynamics indicators in narrow pane. Status bar shows structural health summary in one line. Both update live.
- **Effort**: M

#### C7: Spatial View
- **Produces**: Graph/spatial rendering of the tension structure
- **Blocked by**: C1
- **Blocks**: Nothing
- **Acceptance**: Tensions rendered as nodes with relationship edges. Layout conveys structural properties (depth, weight, activity). Navigation with directional keys.
- **Effort**: M

### Phase D: The Oracle (Ring 3)

Self-modeling and intelligence.

#### D1: Prediction-Observation-Surprise Loop
- **Produces**: Per-session predictions about structural movement, checked against actual changes
- **Blocked by**: A7 (needs grammar events)
- **Blocks**: D2, D3
- **Acceptance**: Before session: generate predictions about which tensions will move/stall/conflict. After session: compare predictions to actual events. Surprises logged. Model updates.
- **Effort**: L

#### D2: Shadow Tension Detection
- **Produces**: System proposes tensions the practitioner hasn't articulated, based on behavioral patterns
- **Blocked by**: D1
- **Blocks**: Nothing
- **Acceptance**: After sufficient history, the oracle suggests: "You keep updating actuals in area X but have no tension for it." Practitioner confirms, rejects, or refines.
- **Effort**: L

#### D3: Structural Context for Agents
- **Produces**: Rich context injection for agent sessions departing from tensions
- **Blocked by**: B5, D1
- **Blocks**: Nothing
- **Acceptance**: Agent receives: the tension it's working within, ancestors (full path to root), siblings (structural context), children (sub-structure), dynamics (lifecycle, conflicts, movement), oracle predictions. Agent's work feeds back as reality updates and new sub-tensions.
- **Effort**: M

### Phase E: Extensions (Ring 4)

Optional capabilities, built last.

#### E1: Cosmic Calendar / Custom Time Structures
- **Blocked by**: B3
- **Effort**: M

#### E2: Named Structural Positions / Pattern Library
- **Blocked by**: A9
- **Effort**: M

#### E3: Template Automation from Resolved Tensions
- **Blocked by**: A7
- **Effort**: S

#### E4: Kelly Criterion Commitment Sizing
- **Blocked by**: B3
- **Effort**: M

#### E5: Cooperation Primitives (Shared Tensions, Structural Contracts)
- **Blocked by**: B5
- **Effort**: L

---

### Critical Path

```
A1 → A2 → A3 → A4/A5/A6 (parallel) → A7 → B1 → C1 → C2 → C3 (practice ceremony)
                                              → B5 → D1 → D3
```

The critical path runs through the grammar, into the instrument CLI, into the TUI, and terminates at practice ceremony (the thing that makes it an operative instrument) and agent context injection (the thing that makes it an agent originator).

### Foundational Beads (most downstream dependents)
- **A1** (Tension primitive) — everything depends on this
- **A3** (frankensqlite store) — all persistence depends on this
- **A7** (Event system) — all instrument reactivity depends on this
- **C1** (TUI core) — all rendering depends on this

### Quick Wins
- **A1 + A2** can be built in a day — they're type definitions and append logic
- **A8** (tree construction) is mostly v0.2's `build_tree` cleaned up
- **B6** (v0.2 migration) is a standalone tool with no downstream dependents

---

## 8. Implementation Phases

### Phase 1: The Grammar
- Beads: A1 through A9
- Goal: `sd-core` crate compiles, passes all tests, computes Fritz's structural dynamics from two tables. No instrument code exists yet. The grammar stands alone.

### Phase 2: The Foundation
- Beads: B1 through B6
- Goal: The instrument binary exists. CLI works. Schema extensions separate from grammar. Engagement rules configurable. MCP server running. Encryption working. v0.2 data importable.

### Phase 3: The Palace
- Beads: C1 through C7
- Goal: The TUI is alive. Practice ceremony forces honest confrontation. The tree shows computed dynamics with visual degradation. Ambient modes work in tmux. "You are here" shows location. The instrument is daily-driveable.

### Phase 4: The Oracle
- Beads: D1 through D3
- Goal: The instrument learns. Predictions generated and checked. Shadow tensions proposed. Agents receive rich structural context. The feedback loop is complete.

### Phase 5: Extensions
- Beads: E1 through E5
- Goal: The instrument grows. Cosmic calendar, patterns, templates, commitment sizing, cooperation. These ship when they're ready, not as a batch.

---

## 9. Open Questions

1. **Naming**: The grammar, the instrument, the CLI command. `sd-core`? `werk`? `werk.horse`? Horse naming research in progress.

2. **Fritz fidelity**: How deep do we go? Do we implement "path of least resistance" analysis (the grammar computes which resolution path has least structural resistance)? Or stop at lifecycle + conflict + movement?

3. **Practice ceremony interaction design**: The v0.2 linear walk was too rigid. What's the right interaction? Filtered practice (stuck only, subtree only)? Conditional practice (only tensions meeting engagement rules)? Free-form confrontation (practitioner chooses)?

4. **Oracle architecture**: Compiled into the instrument binary, or separate process (like anima's hooks)? Separate process is cleaner architecturally but adds operational complexity.

5. **Cooperation model**: How do shared tensions work technically? Shared frankensqlite file? Federation? Event sync? This is Phase E but architectural assumptions propagate.

6. **Time-scoping specifics**: How does a tension's time horizon interact with staleness degradation? Is the horizon stored (instrument schema) or computed (from desired state text)?

7. **The "in/on/off" grammar**: Is this a first-class mode in the instrument, or an emergent property of how the practitioner uses it? If first-class, what changes between modes?

8. **Narrative logging**: Beats-equivalent attached to structure. Is this part of the mutation log (a mutation type) or a separate instrument table? The grammar says mutations are field changes; narrative isn't a field change. Instrument table seems right.

---

## References

### Fritz Sources (Grammar Grounding)
- *Creating* (1991) — structural tension primitive, creative cycle, resolution
- *The Path of Least Resistance* (1989) — structural conflict, oscillation, path analysis
- *Corporate Tides* (1996) — organizational structural dynamics
- *Elements* (2007) — distilled principles
- *Identity* (2016) — structural dynamics applied to self
- *Your Life as Art* (2003) — structural tension as creative practice

### Research Artifacts
- `research/synthesis-map.md` — 65+ tools/systems, cross-cutting patterns
- `research/v1_category_and_seeds.md` — Operative Instrument definition, seed enrichment
- `research/v1_persona_reports.md` — 10 user persona reports
- `research/v1_collection_of_thoughts.md` — Discovery phase musings
- `research/anima_plan.md` — Prediction-observation-surprise self-modeling design
