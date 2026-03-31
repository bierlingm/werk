# FrankenSuite × werk: Systematic Evaluation

Evaluation of every FrankenSuite tool/library against werk's current architecture and roadmap. Written 2026-03-30.

Reference: https://github.com/Dicklesworthstone#the-frankensuite

## Already In Use

### FrankenSQLite (fsqlite) — persistence layer

werk uses fsqlite v0.1.2 as its sole storage engine. Currently treated as a basic SQLite replacement.

**Untapped potential:**

- **MVCC concurrent writers.** Could remove the advisory file lock and the "never run parallel CLI commands" constraint. Unlocks true multi-process access — TUI open while CLI commands fire, multiple MCP agents writing simultaneously.
- **RaptorQ self-healing.** Could replace the manual backup logic in `Store::init()`. Torn writes become mathematically recoverable instead of operationally dangerous.
- **FTS5 extension.** `fsqlite-ext-fts5` could give built-in full-text search over desired/actual fields without adding frankensearch as a separate dependency. Lighter path to searchable logbase.
- **WASM target.** `fsqlite-wasm` crate exists. If the web frontend ever needs client-side persistence (offline-capable web app), the same DB engine runs in the browser.

**Upgrade path:** Already coupled, so deepening the integration adds capability without adding new dependency risk. Track fsqlite releases for MVCC stability.

#### Deep dive: internals and maximal adoption (2026-03-30)

**Crate architecture.** 28-crate workspace, all v0.1.2: foundation (types, errors), storage (VFS, pager, WAL, MVCC, B-tree), SQL (AST, parser, planner, VDBE, functions), extensions (FTS3, FTS5, JSON, R-tree, Session, ICU, misc), integration (core, public facade, CLI, WASM, observability, e2e, harness, C API). Edition 2024, nightly. **This IS the same project as fsqlite 0.1.2 that werk already uses** — the upgrade path is zero breaking changes.

**What was dismissed as "just upgrade the DB" — and what it actually is:**

**`BEGIN CONCURRENT` removes the single-writer constraint.** Page-level MVCC with Serializable Snapshot Isolation. Multiple transactions write simultaneously as long as they modify different pages. When two writers touch the same page, first to commit wins; second gets `SQLITE_BUSY_SNAPSHOT` and retries. Max 64 concurrent writers. For werk: TUI session open, CLI command fires, MCP agent writes — all simultaneously, no advisory file lock needed. The `Store::init()` lock can be removed.

**RaptorQ WAL repair is transparent.** WAL frames carry RFC 6330 fountain-code repair symbols in an optional `.wal-fec` sidecar file. If torn writes or corruption are detected, the decoder reconstructs from surviving symbols. No API changes — existing code gains resilience automatically. The manual backup logic in `Store::init()` becomes redundant. Data loss transforms from an operational risk into a mathematical near-impossibility.

**The Session extension is the multi-workspace sync mechanism.** Full changeset/patchset API for recording and applying row-level changes across databases. When a mutation happens in workspace A, it's captured as a changeset. That changeset can be serialized, transmitted, and applied to workspace B. This is #96 (global workspace) and #100 (cross-workspace addressing) — not as a custom sync protocol, but as a database-native changeset stream. Changesets carry the exact rows affected, old values, and new values — the same information werk's mutation log captures, but at the storage layer.

**FTS5 as lightweight search without FrankenSearch.** `fsqlite-ext-fts5` provides full-text indexing with BM25 ranking, phrase queries, NEAR operator, column filters, and tokenizer pipeline (unicode61, porter stemming). For the TUI search upgrade, FTS5 might be sufficient — index `desired` and `actual` columns, query with BM25 ranking, no embedding models needed. FrankenSearch then becomes the upgrade path for semantic search and the logbase, while FTS5 handles the structural search tier.

**WASM crate for browser.** In-memory only (no file I/O in browser sandbox), but functional for running queries. The web frontend could load the tension database into memory and query it client-side — offline-capable, no server round-trips. Combined with FrankenMermaid's WASM rendering, the entire instrument could run in a browser tab.

**Maximal adoption means using fsqlite not just as storage but as infrastructure:** concurrent writers for multi-process access, RaptorQ for resilience, FTS5 for search, Session for sync, WASM for browser. The database becomes the platform, not just the persistence layer.

### FrankenTUI (ftui) — terminal interface

werk uses ftui v0.2 with crossterm backend. The TUI hand-rolls significant rendering logic.

**Untapped potential:**

- **Widget library.** ftui ships 106 widgets across 20 crates. werk's TUI custom-builds tree views, tables, temporal indicators, and survey layouts. Some of these likely have ftui equivalents.
- **Layout primitives.** The Bayesian diff renderer was disabled due to sync issues. ftui's built-in layout system may handle this correctly.
- **Theme infrastructure.** #101 (dark/light theme support) requires dual palettes. ftui likely has theme machinery that could replace the hand-built `theme.rs`.

**Near-term tickets this affects:** #127 (rendering corruption), #138 (ID formatting inconsistency), #101 (dark/light backgrounds), #15 (envelope-based TUI rebuild).

#### Deep dive: internals and maximal adoption (2026-03-30)

**Crate architecture.** 22-crate workspace, v0.2.1: core (terminal lifecycle, events, cursor), render (cells, buffers, ANSI, diff engine), style (colors, themes, adaptive dark/light), text (measurement, wrapping, markup), layout (flex + grid constraint solver), widgets (69 modules), runtime (Elm-style loop, Model trait, subscriptions, commands), backend (platform abstraction), tty (native Unix TTY via rustix/nix, replacing Crossterm), a11y, extras, i18n, pty, web (WASM), simd, render-cache, harness, demo-showcase (46 screens). Edition 2024, nightly.

**What was dismissed as "use more widgets" — and what it actually is:**

**The Tree widget is a direct replacement for custom tree rendering.** `TreeNode` with `label`, `icon`, `children`, `lazy_children` (collapsed by default), `expanded` state. Guide types: Unicode (│├──└──), Rounded (│├──╰──), Bold (┃┣━━┗━━), Double (║╠══╚══), ASCII. Undo support via `TreeUndoExt` trait. Mouse interaction registration. This replaces werk's hand-built tree traversal rendering in `render.rs`.

**AdaptiveColor solves #101 completely.** Every color in the theme is an `AdaptiveColor { light: Color, dark: Color }` that resolves based on `is_dark_mode: bool`. The full `Theme` struct has 20+ adaptive color slots: primary, secondary, accent, background, surface, overlay, text (3 levels), semantic (success/warning/error/info), border (normal/focused), selection (bg/fg), scrollbar (track/thumb). werk's hand-built `theme.rs` with manual palette construction is replaced by a single `Theme` that works on both backgrounds.

**Sparkline is the temporal indicator upgrade.** 9-level Unicode blocks (` ▁▂▃▄▅▆▇█`) with optional color gradient and baseline support. werk's 6-dot temporal visualization could become a sparkline of urgency over time — compact, information-dense, and built-in.

**DriftVisualization is a Bayesian confidence display.** Per-domain confidence sparklines with traffic-light signals (Green >80%, Yellow 40-80%, Red <40%), fallback trigger indicators, regime transition banners. This is trajectory visualization — showing the confidence of the instrument's own computations. Combined with FrankenNetworkX's ComplexityWitness, the instrument could display "how confident am I about what I'm telling you" as a visual signal.

**VirtualizedList with Fenwick tree handles 100K+ items.** Binary Indexed Tree for O(log n) prefix sums, supporting both fixed and variable row heights. As tension counts grow, the survey view stays responsive. The Elias-Fano encoding and LOUDS (Level-Order Unary Degree Sequence) succinct tree structures are available for memory-efficient tree representation.

**Command palette for fuzzy search.** A built-in widget for TUI search — fuzzy matching, keyboard-driven selection, configurable scoring. Replaces werk's custom search with a framework-native widget.

**Elm-style Model-Update-View-Cmd runtime.** `trait Model { type Msg; fn update(&mut self, msg) -> Cmd<Msg>; fn view(&self) -> View; fn subscriptions() -> Vec<Subscription> }`. werk's TUI already follows this pattern (app.rs/update.rs/render.rs/msg.rs) but hand-rolled. The framework runtime handles the event loop, frame budgeting, subscriptions (tick timers, file watchers), and command dispatch. Migration means implementing `Model` for the existing app state.

**Bayesian diff rendering — what werk disabled.** The framework's diff engine uses row-major cache-aligned comparison (cells = 16 bytes, blocks = 4 cells = 64 bytes = one cache line), with Bayesian regime detection and conformal prediction for frame timing. The degradation cascade (Full → LimitedEffects → EssentialOnly) adapts rendering fidelity under resource pressure. Widgets implement `is_essential()` to opt into degradation. This is what the TUI needs for smooth rendering without the corruption bugs (#127).

**Native TTY backend replaces Crossterm.** `ftui-tty` uses rustix/nix for direct termios control — no external event loop dependency. Crossterm is a legacy/optional path. werk should migrate to the native backend.

**Focus management graph.** Proper focus handling with a graph of focusable widgets, instead of manual state tracking. Drag-and-drop protocol for tension repositioning. Inspector overlay for debugging widget trees during development. Toast notifications for transient signals. All built-in.

**Maximal adoption means rewriting the TUI on top of ftui's runtime, not just using its widgets.** The Model trait becomes werk's app state. The Tree, Table, Sparkline, and CommandPalette widgets replace custom rendering. AdaptiveColor replaces theme.rs. The Bayesian diff replaces the disabled custom diff. VirtualizedList replaces the flat tension list. The operating envelope becomes a framework-native layout with Flex constraints. Every view (deck, survey, ground) becomes a Screen implementing the framework's Screen trait, like the 46 demo screens.

---

## High-Value Near-Term Additions

### FrankenSearch — hybrid retrieval (BM25 + vector + RRF fusion)

**What it does:** Two-tier search engine. Phase 1: BM25 lexical + fast embedding → RRF fusion (~12ms). Phase 2: quality embedding, re-scoring, optional cross-encoder rerank (~150ms). Graceful degradation — phase 1 results always return even if phase 2 fails. Uses fsqlite for metadata storage.

**Where it fits in werk:**

1. **TUI search upgrade.** Current search is case-insensitive substring matching with simple scoring (prefix > word boundary > substring). Works for 134 tensions. BM25 alone would give proper term relevance — a search for "deadline" ranks tensions about deadlines above those that merely contain the word in passing.

2. **Logbase (#89).** This is the killer use case. "A searchable, queryable substrate of all prior design decisions." The logbase needs to index design decisions, session notes, mutation histories, and make them retrievable by meaning. FrankenSearch's two-tier approach (fast structural match, then semantic refinement) matches werk's "signal by exception" philosophy.

3. **MCP semantic queries.** The `list` tool filters are structural (overdue, stale, positioned). Semantic search via MCP would let agents ask "tensions related to revenue" or "anything about temporal signals" — natural language over the tension corpus.

4. **Documentation search (#51).** Once practitioner guides and integration docs exist, they need to be findable.

**Integration considerations:**

- Same storage engine (fsqlite) — compatible metadata layer.
- Uses `asupersync` instead of Tokio for async. werk's web/MCP layer is Tokio-based. Needs bridging or dedicated thread (same pattern as Store).
- Embedding models add binary size and cold-start time. Use `fast-only` mode for TUI, full two-tier for logbase/MCP.
- Vector index (FSVI) is memory-mapped — fits the local-first, single-machine model.

**Effort:** Medium.

#### Deep dive: internals and maximal adoption (2026-03-30)

**Crate architecture.** 13-crate workspace: core (traits/types), embed (3 embedder tiers), index (FSVI vector storage), lexical (Tantivy BM25), fusion (RRF + two-tier orchestration), rerank (cross-encoder), storage (fsqlite metadata), durability (RaptorQ), fsfs (CLI), tui, ops, optimize_params, and public facade. Edition 2024, nightly. Not on crates.io.

**What was dismissed as "add a search library" — and what it actually is:**

**The hash embedder makes indexing effectively free.** FNV-1a hash → 256d deterministic embeddings in 11 microseconds per document. For 1000 tensions: 11ms total, ~512KB index. You can re-index the entire tension corpus on every mutation — search is always current, no stale index problem. Zero model downloads, negligible binary size, deterministic embeddings (same text → same vector, always). This is the "structural fact" tier of search.

**The two-tier model maps to the output taxonomy.** Phase 1 (BM25 + hash embedding, 10-50ms) produces structural search results — term relevance, exact matches. This is Layer 1/2 output. Phase 2 (quality embedding + RRF reranking, 150ms) produces semantic results — meaning-based retrieval. This is Layer 3 output. The instrument's boundary is preserved: structural search is fast and factual, semantic search is slower and interpretive.

**The phase gate learns when quality adds value.** A Bayesian e-process with anytime-valid stopping decides per-query whether to invoke Phase 2. Over 500 queries, it accumulates evidence about whether semantic refinement changes rankings for your corpus. If your tensions use distinctive vocabulary, it learns to skip Phase 2. If they overlap semantically, it learns to invoke it. The instrument adapts to the practitioner's language.

**Circuit breaker for graceful degradation.** If the quality tier fails, the circuit breaker trips and returns Phase 1 results. No search failure ever — matches signal-by-exception.

**Storage shares fsqlite.** Embedding queue, search history, index metadata — all in the same DB engine werk uses. Could share the database or use a sibling file.

**The logbase substrate.** Index not just tensions but mutations, notes, epochs, session summaries, design documents. `IndexableDocument` carries `id`, `content`, `title`, `metadata_json`. "Show me every epoch that touched this concern" from the conceptual foundation becomes: BM25 for keyword matches across epoch snapshots, semantic for meaning-based retrieval.

**MCP semantic queries.** Agent asks "tensions related to revenue." BM25 finds tensions containing "revenue." Semantic also finds #36 ("sustainable business model"), #69 ("first $1000 earned"), #71 ("something obviously valuable"). The agent gets structurally relevant results without knowing the exact vocabulary.

**FTS5 as lightweight alternative.** FrankenSearch's storage crate includes an `fts5_adapter` that implements the `LexicalSearch` trait against fsqlite's FTS5 extension — no Tantivy needed. For the TUI search upgrade, FTS5-only might suffice. FrankenSearch becomes the upgrade path for semantic search.

### FrankenMermaid — diagram rendering, WASM-ready, deterministic SVG

**What it does:** 25+ diagram types (flowchart, gantt, timeline, mindmap, sequence, sankey, etc.). Renders to SVG, PNG, terminal (braille/block/half-block), Canvas2D, JSON IR. Deterministic output — same input produces byte-identical SVG. WASM package available. Best-effort parsing recovers from malformed syntax.

**Where it fits in werk:**

1. **Web frontend (#34, #82).** The tension tree rendered as an interactive flowchart or mindmap. Deterministic SVG means it's diffable and cacheable. The WASM package could power this directly in the browser.

2. **Desktop app (#82-#85).** "Visual design that expresses structural dynamics." Gantt and timeline diagram types map directly to the calculus of time. Flowchart/mindmap types map to architecture of space.

3. **CLI visualization.** `tree` is currently ASCII. `werk tree --svg` or `werk tree --diagram` for richer output. Pipe to a file or viewer.

4. **TUI minimap.** Terminal rendering mode (braille/block) could render a small structural diagram directly in the terminal — overview of the full tree in the survey view.

5. **Documentation (#51).** Auto-generated architecture diagrams from the live tension tree. Always current.

6. **Public presence (#37, #41).** "First public post about werk" — auto-generated visual diagrams of real tension structures are compelling content. Deterministic rendering means diagrams are reproducible.

7. **Coaching dashboards (#80, #81).** Visual progress reports for clients. Gantt of their tension timelines, mindmap of their structural hierarchy.

**Integration considerations:**

- SVG output for web/CLI is nearly drop-in.
- Terminal rendering for TUI requires mapping the tension tree to Mermaid syntax, then rendering. Straightforward but adds a translation layer.
- WASM package for web frontend is self-contained.
- 10 layout algorithms selected per diagram type — no manual layout tuning needed.

**Effort:** Low-Medium.

#### Deep dive: internals and maximal adoption (2026-03-30)

**Crate architecture.** 8-crate workspace: fm-core (IR + types), fm-parser (chumsky-based), fm-layout (16 algorithms), fm-render-svg, fm-render-term, fm-render-canvas, fm-wasm, fm-cli. Edition 2024, nightly. Not on crates.io.

**What was dismissed as "just a diagram renderer" — and what it actually is:**

**Programmatic IR construction bypasses Mermaid syntax entirely.** `MermaidDiagramIr::empty(DiagramType::Flowchart)` creates a blank diagram. You push `IrNode` and `IrEdge` structs directly. This means werk can build diagrams from `Forest<Node>` without ever generating text — the tension tree IS the diagram source. Each `IrNode` carries a string `id` that maps back to tension IDs. Each node has a `NodeShape` (21 options: Box, Diamond, Circle, Hexagon, Stadium...), custom labels, CSS classes, tooltips. The diamond family from werk's glyph vocabulary could become the node shape vocabulary of the structural diagram.

**16 layout algorithms with automatic selection.** Sugiyama (hierarchical) for the tension tree. Radial for mindmap views. Timeline for temporal visualization. Gantt for deadline-based views. Sankey for flow visualization. The algorithm is selected per diagram type automatically, or you can force one. Layout constraints (`SameRank`, `MinLength`, `Pin`, `OrderInRank`) let you enforce structural rules — e.g., pin root tensions at the top, force siblings to the same rank, set minimum edge lengths proportional to deadline gaps.

**Terminal rendering with sub-cell resolution.** Braille mode: 2x4 pixels per terminal cell (Unicode U+2800-U+28FF). Block mode: 2x2. HalfBlock: 1x2. The Canvas API has `set_pixel`, `draw_line` (Bresenham), `draw_rect`, `fill_circle` (midpoint algorithm), and `render() → String`. A minimap renderer produces density-aware scaled overview with viewport indicator. This is the TUI structural overview — a braille-rendered mindmap of the entire tension tree in a corner of the screen.

**Diagram diffing for structural change visualization.** When the tension tree changes (move, split, resolve), FrankenMermaid can diff the before/after IR and classify changes as added/removed/changed nodes/edges. This is gesture impact rendered visually — "you reparented #42, here's what the structure looks like now vs. before."

**Source span tracking maps layout back to domain objects.** Every SVG element carries a `SourceSpanRecord` linking it to the IR node index and original source position. For werk, this means every rendered node in a diagram is clickable/hoverable and traces back to its tension.

**The four frameworks as diagram types:**
- Architecture of Space → Flowchart (Sugiyama hierarchical) or Mindmap (Radial)
- Grammar of Action → State diagram (lifecycle states)
- Calculus of Time → Gantt (deadlines as bars) or Timeline (temporal events)
- Logic of Framing → The operating envelope as a highlighted subgraph within the full diagram

**Degradation planning** adapts rendering fidelity to resource constraints — tier modes (Compact/Normal/Rich/Auto) match the instrument's responsiveness needs. Full rendering for web/desktop, compact for TUI minimap, auto for MCP/CLI.

**WASM exports** (`render_diagram`, `render_to_canvas`, `get_diagram_metadata`) mean the same rendering engine works in browser, terminal, and desktop — one visual language across all surfaces.

### FrankenNetworkX — graph algorithms in Rust

**What it does:** Full graph algorithm library. Shortest path, centrality (PageRank, betweenness, closeness, eigenvector), clustering, topological sort, DAG operations (longest path, generations, ancestors, descendants, transitive closure), community detection, flow algorithms. Pure Rust, no async complications.

**Where it fits in werk:**

1. **Bottleneck detection.** `betweenness_centrality` on the tension tree reveals structural bottlenecks — which tensions, if resolved, would unblock the most others. This is a signal werk doesn't currently compute. Would surface in `stats --health` and as a structural signal.

2. **Topological waves.** `topological_generations` powers a "waves" view — what can be worked on now vs. what's blocked by unresolved predecessors. This is the operating envelope expressed as graph layers.

3. **Critical path (proper).** Currently recursive in `temporal.rs`. `dag_longest_path` gives this directly and correctly.

4. **Ancestors/descendants.** Already needed for containment violation checking. The library provides these as first-class operations with proper cycle detection.

5. **Transitive closure.** "What depends on what, transitively" — useful for the operating envelope and for understanding blast radius of resolving or restructuring a tension.

6. **Community detection.** As tension counts grow beyond hundreds, automatic clustering could suggest natural groupings — structural self-organization visible to the practitioner.

7. **Multi-participant (#4).** When multiple people share a workspace, graph analysis of who touches which subtrees reveals collaboration patterns and structural ownership.

8. **Impact analysis for gestures.** Before a `move` or `split`, compute what changes structurally. Graph diff before/after.

**Integration considerations:**

- The tension tree is already a `Forest<Node>` in `sd-core/src/tree.rs`. Converting to a FrankenNetworkX directed graph is straightforward — each tension is a node, each parent-child relationship is an edge.
- Pure Rust, no async, no runtime conflicts.
- Algorithms return standard Rust collections — easy to map back to tension IDs.

**Effort:** Low. Almost a no-brainer.

#### Deep dive: internals and maximal adoption (2026-03-30)

**Crate architecture.** FrankenNetworkX is an 11-crate workspace:

- `fnx-runtime` (3,300 lines) — CGSE policy engine, evidence ledger, compatibility modes, value types
- `fnx-classes` (3,300 lines) — `Graph`, `DiGraph`, `MultiGraph`, `MultiDiGraph` with full adjacency, attributes, snapshots
- `fnx-algorithms` (38,000 lines) — 300+ pure functions, all taking `&DiGraph` or `&Graph` and returning result structs
- `fnx-python` — PyO3 bindings (thin wrapper, fully separate)
- Plus `fnx-views`, `fnx-dispatch`, `fnx-convert`, `fnx-generators`, `fnx-readwrite`, `fnx-durability`, `fnx-conformance`

The algorithms crate depends only on `fnx-classes` and `fnx-runtime`. Every algorithm is a pure function — no mutation, no side effects, no async. The `GraphView` trait (11 methods: `nodes_ordered`, `get_node_index`, `get_node_name`, `neighbors_iter`, `in_neighbors_iter`, `neighbor_count`, `has_node`, `has_edge`, `is_directed`, `node_count`, `edge_count`) is the sole abstraction boundary.

**Not on crates.io.** Git dependency or vendor. Requires `edition = "2024"` and nightly Rust.

**What was initially dismissed as ballast — and what it actually is:**

**The Evidence Ledger is gesture memory at the graph level.** Every mutation to the graph (add_node, add_edge, remove) records a `DecisionRecord`:

```
DecisionRecord {
    ts_unix_ms,
    operation,
    mode: Strict | Hardened,
    action: Allow | FullValidate | FailClosed,
    incompatibility_probability: f64,
    rationale: String,
    evidence: Vec<EvidenceTerm { signal, observed_value, log_likelihood_ratio }>,
}
```

Werk already logs field-level mutations ("reality changed from X to Y"). The evidence ledger captures *structural-decision-level* memory: "the relationship between #42 and #10 was created because [these signals] with [this rationale]." This is what the conceptual foundation calls gesture — "a compression of intentionality into enacted form" — applied to the graph topology itself. When a practitioner reparents a tension, the ledger records *why the structure changed*, not just *that it changed*. This feeds the logbase (#89) as structural decision history.

**The ComplexityWitness is the instrument observing itself.** Every algorithm returns:

```
ComplexityWitness {
    algorithm: "brandes_betweenness_centrality",
    complexity_claim: "O(|V| * |E|)",
    nodes_touched: 47,
    edges_scanned: 89,
    queue_peak: 12,
}
```

The witness tells you how hard the instrument worked to understand your structure. A workspace where centrality touches 47 nodes is structurally different from one touching 200. The witness doesn't interpret this (that would cross the instrument boundary), but it surfaces the fact. Ground mode could show: "structural complexity this session: 47 nodes touched, 89 edges scanned. Last session: 32, 54." The practitioner reads what that means. Structure determines behavior — including the computational behavior of the instrument itself.

**AttrMap on edges makes relationships first-class.** Currently the parent-child relationship is werk's only structural relationship. `DiGraph` edges carry `AttrMap` (`BTreeMap<String, CgseValue>` where `CgseValue` is Bool/Int/Float/String). This enables typed edges:

- **containment** — "this child is part of closing this parent" (the current default)
- **enables** — "resolving this makes that possible" (operative, not managerial — the practitioner's own judgment)
- **competes** — "these tensions draw from the same well of energy/attention"
- **informs** — "what I learn from this changes how I approach that"

These aren't dependency links for a project manager. They're the practitioner's articulation of how their structural tensions relate. They stay within the standard of measurement principle — the user supplies them, the instrument computes from them. And once edges have types, the full algorithm surface activates:

- **Max flow** with "enables" edges and attention-capacity tells you maximum throughput of the creative structure — the Napoleonic field survey, formalized
- **Min cut** reveals the minimum set of relationships that, if severed, disconnect aims from actions — load-bearing structural commitments
- **Community detection** (Louvain, label propagation) on a multi-edge graph reveals emergent clusters that don't match the declared hierarchy — the gap between declared and emergent structure IS a structural insight
- **Betweenness centrality** reveals bridge tensions structurally connecting otherwise separate concerns — resolving one doesn't just close a gap; it restructures the field
- **Articulation points** — tensions whose removal fragments the structure — structural vulnerabilities in the operative sense
- **Graph coloring** — minimum independent attention streams needed to work all competing tensions — Napoleonic distribution of forces, computed
- **Topological generations** — simultaneous possibility waves across the entire field, not just "what's next" but "what's concurrently possible"
- **Transitive reduction** — the essential edges, stripping redundancies from the practitioner's articulated theory

**Compatibility modes as practice modes.** `Strict` = the instrument checks every structural gesture carefully (learning, coaching). `Hardened` = the instrument refuses ambiguous structural changes (shared workspaces, high-stakes). The `DecisionAction` enum (Allow/FullValidate/FailClosed) gives the instrument a principled way to push back without crossing the instrument boundary.

**The real architectural question: DiGraph as structural substrate.** Full adoption means DiGraph beneath Forest. The tree remains the primary relationship layer. Typed cross-cutting edges are the secondary layer. All current tree operations continue unchanged. New structural gestures let the practitioner articulate relationships the tree can't express. This would make werk's structural model fundamentally richer — not just "what tensions exist and how they're hierarchically organized" but "how do all tensions in your creative field relate to each other, what are the load-bearing relationships, where are structural vulnerabilities, and what does the topology of your creative process look like."

**Three adoption paths:**

1. **Full integration.** Git dependency on the workspace. Use `DiGraph` as substrate beneath `Forest`. Embrace evidence ledger, complexity witness, edge attributes. Architectural change — the instrument gains a graph-theoretic foundation.

2. **Algorithm extraction.** Implement `GraphView` on `Forest` directly (or on a thin `SimpleDiGraph`). Copy the ~500 lines of specific algorithms needed. Skip CGSE. No external dependency. Gains algorithms but not the structural model.

3. **Staged.** Start with path 2 (algorithms only). When typed edges and evidence ledger become needed (logbase, multi-participant, coaching modes), migrate to path 1.

Path 1 is the maximal adoption. It treats FrankenNetworkX not as an algorithm library but as the structural-relational substrate of the instrument.

---

## Medium-Term Possibilities

### FrankenPandas — DataFrame operations in Rust

**What it does:** Full pandas API in Rust. Columnar storage, group-by, pivot tables, rolling windows, statistical aggregation. 7 IO formats (CSV, JSON, Parquet, etc.). 1,500+ tests.

**Where it fits:**

- **Stats command.** Currently hand-computed aggregation. FrankenPandas could power richer analytics — pivot tables of mutations by time period, cross-tabulations of status × deadline × depth.
- **Trajectory analysis.** The projection engine (`sd-core/src/projection.rs`) estimates time-to-resolution from gap trends. A proper DataFrame with rolling windows and statistical functions would make this more rigorous.
- **Export.** `--csv` and `--json` output from stats could use FrankenPandas' IO layer for richer export formats including Parquet.
- **Business model (#36).** Revenue tracking, usage analytics, coach dashboards — all DataFrame territory.
- **Practitioner reports.** Weekly/monthly summaries of structural dynamics activity. "You resolved 12 tensions, your average resolution time decreased by 20%, your deepest active subtree is 4 levels."

**Integration considerations:** Adds significant dependency weight for what's currently simple aggregation. Justified once stats/analytics become a real product surface (coaching, business).

**Effort:** Medium.

#### Deep dive: internals and maximal adoption (2026-03-30)

**Crate architecture.** 12-crate workspace: fp-types (Scalar, DType, null semantics), fp-columnar (Column, ValidityMask with bitpacked nulls), fp-index (Index/MultiIndex), fp-frame (DataFrame + Series — 788 public methods), fp-expr (query()/eval() expression parser), fp-groupby (3 execution paths), fp-join (6 join types + asof), fp-io (7 formats), fp-runtime (policy + evidence ledger), fp-conformance (differential testing vs pandas oracle), fp-frankentui (experimental TUI dashboard), and the public facade. Edition 2024, nightly. Not on crates.io. Arrow 54.3.0 native.

**What was dismissed as "overkill for simple aggregation" — and what it actually is:**

**DatetimeAccessor makes temporal decomposition trivial.** `series.dt().year()`, `.month()`, `.day()`, `.dayofweek()`, `.hour()`, `.strftime("%Y-%m-%d")`. For mutations with timestamps, this means: "group mutations by day-of-week to find practice rhythms", "show monthly resolution rates", "detect hour-of-day patterns in engagement." The conceptual foundation says the trace shape is diagnostic — FrankenPandas gives you the tools to actually analyze trace shapes.

**Rolling windows for mutation pattern detection.** `series.rolling(7, min_periods=1).count()` gives 7-day rolling mutation frequency. `rolling(7).mean()` gives smoothed gap trajectory. `rolling(14).std()` gives volatility of engagement. These are the statistical foundations for `projection.rs`'s trajectory classification — currently using simple linear regression over gap samples. With FrankenPandas, you get proper rolling statistics, EWM (exponentially weighted moving average — weights recent activity more), expanding windows, and resampling.

**Pivot tables for the coaching dashboard.** `df.pivot_table("resolution_days", "tension_id", "status", "mean")` — average resolution time by tension and status. `pivot_table_with_margins` adds subtotals. `pivot_table_multi_agg` applies multiple aggregation functions. This is the analytical surface for coaches (#80, #81) — "your average resolution time is 12 days, but tensions under #10 average 3 days while those under #36 average 45 days."

**GroupBy with cardinality-optimized execution.** Dense int64 path (≤65K values): O(1) array indexing. Arena-backed (bumpalo): single malloc for groups. HashMap: fallback for high cardinality. For grouping mutations by tension_id (typically <1000 unique values), the dense path fires — fast enough for interactive stats.

**AACE with EvidenceLedger.** Every DataFrame computation records an evidence trail — which columns were aligned, which types were promoted, which nulls were propagated. This maps to the standard of measurement principle: every computed statistic is traceable to the data that produced it. Ground mode could show not just "your resolution rate is 0.7" but "computed from 47 resolved tensions out of 67 active, with 3 null values excluded."

**SQL IO reads directly from fsqlite.** `read_sql(conn, "SELECT * FROM mutations WHERE tension_id = ?")` — load mutations from the werk database directly into a DataFrame. No intermediate format conversion. The stats command becomes: open database → load to DataFrame → compute → display.

**Parquet export for external tools.** `write_parquet_bytes(&df)` — practitioners can export their structural dynamics data to Parquet and analyze it in any tool (Python pandas, DuckDB, Tableau). This is the data portability story for the business model (#36).

**Maximal adoption means FrankenPandas as the analytical engine for everything beyond simple counts:** ground mode statistics, trajectory classification, coaching dashboards, session analytics, pattern detection, and data export. The projection engine (`projection.rs`) could be rewritten as DataFrame operations — cleaner, more powerful, and benefiting from FrankenPandas' optimized execution paths.

### FrankenNumPy / FrankenSciPy — numerical computing

**What it does:** NumPy in Rust (9 crates, 2,282 tests, bit-exact RNG). SciPy port with adaptive solver selection.

**Where it fits:**

- **Trajectory extrapolation.** Currently uses simple trend lines in `estimate_time_to_resolution()`. SciPy-grade curve fitting or time-series analysis could improve accuracy.
- **Signal detection.** Statistical outlier detection for "something changed" — a tension that suddenly accelerates or stalls, detected by deviation from its historical pattern.
- **Urgency curve.** Currently a linear ratio (elapsed/total). A sigmoid or custom curve via numerical optimization could make urgency feel more natural — slow start, steep middle, plateau near deadline.

**Integration considerations:** Heavy dependencies for narrow current need. Justified if werk develops a serious analytics/prediction layer.

**Effort:** High relative to current need.

### FrankenEngine — deterministic runtime with cryptographic receipts

**What it does:** Native Rust runtime for extension workloads. Deterministic replay, cryptographic receipts, sandboxed execution.

**Where it fits:**

- **Hook system (#126).** werk already has event hooks and pre/post mutation concepts. FrankenEngine could sandbox user-defined hooks with deterministic replay — "this hook ran against this mutation and produced this result, provably."
- **Scripted batch mutations.** Currently YAML-based. Could become programmable with a sandboxed runtime — user-defined transformation scripts that are safe to run.
- **Cross-workspace sync (#96, #100).** Cross-workspace operations need trust guarantees. Cryptographic receipts on mutations would enable verified sync — "this mutation was applied in workspace A and can be replayed in workspace B with proof of origin."
- **Plugin architecture.** Third-party extensions to werk's gesture vocabulary, running in a sandboxed, deterministic environment.

**Integration considerations:** Architectural change, not incremental. Would reshape how hooks and extensions work. Worth designing for even if implementation is later.

**Effort:** High.

#### Deep dive: internals and maximal adoption (2026-03-30)

**Crate architecture.** 3 crates: frankenengine-engine (1,700+ source files, core VM + guardplane), frankenengine-extension-host (manifest validation, edition 2021), franken-metamorphic (testing framework). Plus fuzzing harness. Edition 2024 for engine, 2021 for host. Depends on franken-kernel, franken-decision, franken-evidence from the asupersync ecosystem.

**What was dismissed as "overkill for hooks" — and what it actually is:**

**A native Rust bytecode VM with a Bayesian risk inference guardplane.** Not a WASM runtime or V8 binding. FrankenEngine parses JavaScript/TypeScript through a deterministic IR pipeline (IR0→IR1→IR2→IR3→execution) in a baseline interpreter. Every extension runs in an isolated execution cell with bounded registers, heap limits, and instruction budgets. The trust model isn't binary (allow/deny) — it's a probabilistic posterior P(risk_state | evidence) updated per evidence atom, with expected-loss action selection.

**The capability lattice IS the permission model for third-party gestures.** Six capabilities: FsRead, FsWrite (implies FsRead), NetClient, HostCall, ProcessSpawn, Declassify. A hook that receives mutation events and returns actions needs only `HostCall`. A hook that writes to the filesystem needs `FsWrite`. A hook that makes network requests needs `NetClient`. The practitioner declares what each hook can do in its manifest. The engine enforces it at runtime.

**The 11-state lifecycle maps to session lifecycle.** Unloaded → Validating → Loading → Starting → Running ⇄ Suspending → Suspended → Resuming → Terminating → Terminated → Quarantined. Grace periods (5-30 seconds) for cooperative shutdown. An append-only transition log for deterministic replay. This is the extension version of werk's session concept.

**Information Flow Control prevents hooks from exfiltrating sensitive data.** IFC lattice: sensitive sources (credential files, env vars, key material) → sinks (network, subprocess, IPC). Flow rule: value label must be dominated by sink clearance, otherwise blocked. If a reality update contains an API key and a hook tries to send it over the network, IFC blocks it — without the practitioner needing to know the hook tried. Compile-time checks in IR2 discharge provable-safe flows; runtime checks on dynamic edges.

**Bayesian risk inference learns which extensions are trustworthy.** Evidence stream: hostcall sequence motifs, path/process/network intent deltas, permission-mismatch attempts, temporal anomaly scores, cross-session signature reoccurrence. Online Bayesian filtering updates P(risk_state) per evidence atom. The loss matrix concept: false positives (blocking legitimate hooks) are cheap, false negatives (allowing malicious hooks) are very expensive. Three presets: Balanced, Conservative, Permissive — mapping to practice modes.

**Deterministic replay for debugging.** Every nondeterministic decision (lane selection, timer reads, external API responses, thread scheduling) is captured in a `NondeterminismTrace`. Feed the same trace → reproduce the same behavior, bit-stable. "Why did this hook fire incorrectly? Here's the exact conditions, replayed." Counterfactual simulation: "What would have happened with a different policy?" — replay with alternate containment decisions.

**Cryptographic receipts for verified provenance.** Decision receipts with deterministic signatures: receipt ID, trace ID, decision ID, policy ID, verification timestamp, per-layer results (signature/transparency/attestation). The 3-layer verification pipeline: signature verification → transparency-log inclusion → attestation-chain evidence. This is the cross-workspace sync trust mechanism (#96, #100): "this mutation was applied in workspace A by this hook under this policy, verifiably."

**Maximal adoption means FrankenEngine as the trust substrate for all external code that touches the instrument:** user hooks, agent mutations via MCP, cross-workspace sync, plugin gestures, coaching automation scripts. The capability lattice ensures extensions can only do what the practitioner allows. The evidence ledger captures why every decision was made. The receipts prove it.

### FrankenFS — FUSE filesystem with MVCC and self-healing

**What it does:** ext4/btrfs in Rust via FUSE. Block-level MVCC, RaptorQ fountain codes for self-healing storage.

**Where it fits:**

- **Workspace as filesystem.** Instead of `.werk/sd.db`, tensions appear as files/directories in a mounted filesystem. External tools (editors, git, other CLIs) interact with tensions natively. `cat .werk/tensions/42/desired` returns the desired state.
- **Logbase (#89).** Design documents stored in a self-healing filesystem. Every version recoverable at the block level.
- **Version history.** Block-level MVCC means every state of every tension is recoverable without explicit snapshots.

**Integration considerations:** Very high cost. Platform-specific (FUSE on Linux/macOS). Changes the entire storage paradigm. Fascinating but speculative.

**Effort:** Very high.

---

## Long-Term / Speculative

### FrankenWhisper — speech recognition in Rust

**What it does:** Whisper speech-to-text, pure Rust, no Python dependency.

**Where it fits:**

- **Voice-driven practice.** Speak reality updates instead of typing. "The actual state of tension 42 is: I've finished the first draft but haven't sent it."
- **Coaching sessions (#80, #81).** Coach walks a client through their tension structure verbally. Whisper transcribes, a parser extracts structured mutations from natural language.
- **Mobile (#95).** Voice is the natural input on mobile. Whisper runs locally — no cloud dependency.
- **Session capture.** Record a thinking-out-loud session, transcribe it, extract structural observations as mutations or notes.

**Effort:** High. Requires audio capture pipeline and NLP layer to extract structured mutations from free-form speech.

#### Deep dive: internals and maximal adoption (2026-03-30)

**Crate architecture.** Single crate, monolithic: orchestrator.rs (9,000 lines), backend/mod.rs (9,900 lines), audio.rs (2,000 lines), streaming.rs (2,300 lines), model.rs, storage.rs (fsqlite), cli.rs, main.rs. Edition 2024, nightly. Not on crates.io. Depends on symphonia (native Rust audio decoding), fsqlite, asupersync, franken-kernel/decision/evidence, and optionally frankentorch/frankenjax for GPU acceleration.

**What was dismissed as "add speech input eventually" — and what it actually is:**

**Not native inference — an orchestrated ASR stack.** FrankenWhisper wraps three external backends: whisper.cpp (C++ CLI, GGML models, Metal/CUDA/CPU), insanely-fast-whisper (Python, HuggingFace Transformers, batched GPU), whisper-diarization (Python, pyannote + Whisper, speaker identification). Backend selection uses Bayesian adaptive routing — the engine learns which backend works best for the user's voice, accent, and hardware over time.

**10-stage composable pipeline.** Ingest → Normalize → VAD (voice activity detection) → Separate (source separation) → Backend (inference) → Accelerate (GPU confidence normalization) → Align (forced alignment for timestamps) → Punctuate → Diarize (speaker identification) → Persist (to fsqlite). Each stage is optional and composable via `PipelineBuilder`. The pipeline concept maps to werk's own composable architecture — gestures flow through a pipeline of validation, mutation, signal computation, and event emission.

**Speculative dual-model streaming.** Fast model (whisper-tiny) produces immediate partial results. Quality model (whisper-large) produces corrections in parallel. Sliding window with configurable overlap. Adaptive window sizing based on word error rate. This is the same two-tier pattern as FrankenSearch — fast results for responsiveness, quality results for accuracy.

**Audio via symphonia (native Rust).** Decodes MP3, AAC, FLAC, WAV, OGG Vorbis, ALAC, WavPack without ffmpeg. All input normalized to 16kHz mono PCM WAV. Microphone capture via ffmpeg (fixed duration via `--mic-seconds N`). Not yet continuous streaming — the main gap for real-time voice input.

**Uses fsqlite for persistence.** Transcription history, run reports, evidence from backend routing decisions — all stored in the same DB engine. Could share the werk database or use a sibling file.

**Maximal adoption means voice as a first-class input modality for structural dynamics practice:**
- **Reality updates by voice.** "The actual state of tension 42 is: I've finished the first draft but haven't sent it." Transcribed → parsed by an LLM → `werk reality 42 "first draft finished, not yet sent"`. The transcription pipeline handles audio; an AI agent handles the structural parsing.
- **Session narration.** Record verbal commentary during a TUI session. Whisper transcribes. Timestamps align with mutations. The logbase captures not just what was done but what was said while doing it.
- **Coaching session recording (#80, #81).** Coach and client walk through the tension structure. Whisper transcribes with speaker diarization. The transcript becomes queryable via FrankenSearch — "what did the coach say about tension #36?"
- **Thinking-out-loud capture.** Speak observations while reviewing the structure. Transcribed as notes on the active tension. Voice is faster than typing for unstructured observations.
- **Mobile (#95).** Voice is the natural input on mobile. Whisper runs locally on the device (whisper.cpp with Metal on iOS, GGML on Android). No cloud dependency.
- **TTY audio relay.** Low-bandwidth audio transport over lossy PTY links (mulaw+zlib+base64). Relevant for SSH-based practice — voice input from a remote terminal.

The pipeline stages map to a gesture pipeline: audio arrives (Ingest), gets cleaned (Normalize/VAD), gets recognized (Backend), gets aligned (Align/Punctuate), gets persisted (Persist). The evidence ledger captures which backend was chosen and why — the same decision-tracking pattern as FrankenEngine.

### FrankenRedis — Redis reimplementation

**What it does:** Full Redis protocol parity in Rust. Deterministic latency replication.

**Where it fits:**

- **Real-time collaboration (#4).** Multiple participants viewing/editing tensions simultaneously. Redis pub/sub for live synchronization of mutations.
- **Web sessions.** If the web surface becomes multi-user, session state and real-time update fanout.
- **Cache layer.** Computed signals (urgency, critical path, frontier) are currently recomputed on every read. A Redis-style cache with invalidation on mutation would improve responsiveness for large workspaces.

**Integration considerations:** Adds a server process. Overkill for local-first single-user. Justified only when multi-user becomes real.

**Effort:** High.

### FrankenNode — JS/TS runtime

**What it does:** Trust-native JS/TS runtime on FrankenEngine. Migration autopilot, per-extension trust cards.

**Where it fits:**

- **Plugin system.** User-defined dashboard widgets or custom views written in JS, running in a trusted runtime.
- **Coherence offering (#42).** "Agent prompt that produces a werk workspace from a conversation" — could be a JS-based agent.
- **Web frontend scripting.** If the embedded web frontend grows beyond vanilla JS, a proper runtime with trust guarantees.

**Effort:** Very high.

### FrankenJAX / FrankenTorch — ML frameworks

**What it does:** JAX/PyTorch transform semantics in Rust. Autodifferentiation, 110+ primitives.

**Where it fits:**

- **Pattern recognition.** Train on mutation histories to predict which tensions will stall, resolve quickly, or correlate with structural success.
- **Practitioner modeling.** After enough usage data across coaches/clients, model individual work patterns and suggest interventions.
- **Structural shape classification.** "This subtree looks like a pattern that historically leads to deadline drift."

**Effort:** Extreme. This is a different product. Only relevant if werk becomes a platform with enough usage data to train models.

---

## Prioritized Roadmap

| Priority | Tool | Unlocks | Effort | Relevant Tensions |
|----------|------|---------|--------|--------------------|
| **1** | FrankenNetworkX | Centrality signals, topological waves, bottleneck detection | Low | #13, #139 |
| **2** | FrankenMermaid | Visual tension trees (web, CLI, docs, public content) | Low-Medium | #82, #34, #37, #41, #51 |
| **3** | FrankenSearch | Logbase, semantic MCP queries, better TUI search | Medium | #89, #90 |
| **4** | fsqlite upgrade | Concurrent writers (remove file lock), self-healing WAL | Medium | #100, #96, #4 |
| **5** | ftui deeper | Fix rendering issues, replace custom render code, theming | Medium | #127, #101, #138, #15 |
| **6** | FrankenPandas | Richer stats, trajectory analysis, business analytics | Medium | #36, #69 |
| **7** | FrankenEngine | Sandboxed hooks, verified cross-workspace sync | High | #126, #96, #100 |
| **8** | FrankenWhisper | Voice input for mobile and coaching | High | #95, #80, #81 |

Items 1-3 are the sweet spot — they add capabilities werk visibly lacks (graph intelligence, visual output, semantic search) at reasonable integration cost. All three are pure Rust with no async runtime conflicts. FrankenNetworkX in particular is nearly free: werk already has the DAG, and the library hands you algorithms that would take weeks to implement correctly.
