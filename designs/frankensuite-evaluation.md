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

### FrankenTUI (ftui) — terminal interface

werk uses ftui v0.2 with crossterm backend. The TUI hand-rolls significant rendering logic.

**Untapped potential:**

- **Widget library.** ftui ships 106 widgets across 20 crates. werk's TUI custom-builds tree views, tables, temporal indicators, and survey layouts. Some of these likely have ftui equivalents.
- **Layout primitives.** The Bayesian diff renderer was disabled due to sync issues. ftui's built-in layout system may handle this correctly.
- **Theme infrastructure.** #101 (dark/light theme support) requires dual palettes. ftui likely has theme machinery that could replace the hand-built `theme.rs`.

**Near-term tickets this affects:** #127 (rendering corruption), #138 (ID formatting inconsistency), #101 (dark/light backgrounds), #15 (envelope-based TUI rebuild).

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
