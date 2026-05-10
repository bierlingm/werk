# Mission: werk Sigil Engine v1

Build the sigil engine described in `designs/sigil-engine.md` — a pure-function
SVG renderer that compresses werk-state into operative artifacts across five
purposes (contemplative, glance, snapshot, identity, oracle).

## Plan Overview

The engine is a five-stage pipeline (`Selector → Featurizer → Encoder →
Layouter → Stylist → Renderer`) that materializes `(Scope, Logic) → Sigil`
deterministically. v1 ships:

- **werk-core foundation** — four canonical IRs (`TensionList`,
  `TensionTree`, `AttributeGraph`, `EpochSeries`), `sigils` index table,
  `*N` short-code parser.
- **werk-sigil crate** — stage traits, registries, SVG renderer, the v1
  vocabulary (4 layouters, 3 stylists, 3 encoders), 3 inline glyph
  families, 5 working presets, Rhai-backed E3 expression evaluator.
- **CLI** — `werk sigil <scope> [--logic …] [--save] [--seed …] [--out …]
  [--json] [--dry-run]`.
- **Web** — `GET /api/sigil` (one-shot) and `GET /api/sigil/stream` (SSE
  invalidation tied to werk's existing event channel).
- **werk-tab integration** — third mode rendering the `glance` preset on
  the existing SSE subscription.
- **Combinators** — Sheet (Grid layouter), Composite (Concentric),
  Animation (SeedSweep + ParamSweep, FrameSequence output).
- **Hardening** — error-handling discipline, cache retention, hot-reload
  of preset files.

## Expected Functionality (Milestones)

### M1 — werk-core foundation
- `TensionList`, `TensionTree`, `AttributeGraph`, `EpochSeries` IR types
  with attribute-join builders.
- `sigils` index table (metadata only) with insert/list/get/delete helpers.
- `*N` short-code parsing in `Address`.

### M2 — engine spine + first end-to-end render
- `werk-sigil` crate with stage traits, registries (`mark`, `channel`,
  `glyph-family`, `attribute-name`), `Ctx`, `Scope`/`Logic`/`Sigil` types.
- `SvgRenderer` with provenance metadata embedded as `<metadata>`.
- Selector::Subtree, Featurizer::TensionTree, Encoder::StructuralDefault,
  Layouter::RadialMandala, Stylist::InkBrush — wired end-to-end so
  `contemplative.toml` renders to a real SVG.
- TOML schema + loader (validates against the reference preset).
- First golden snapshot pinned for `contemplative` preset.

### M3 — v1 vocabulary complete
- Encoders: `ShapeByStatus`, `TomlDeclarative` (with Rhai expression
  evaluator).
- Layouters: `FractalBranch`, `Constellation`, `Grid`.
- Stylists: `MinimalLine`, `Glyphic`.
- Three inline glyph families (alchemical ~50, geomantic 16, hand-drawn
  primitives ~30) — composed as SVG path data (no external assets).
- Four remaining presets: `glance`, `snapshot`, `identity`, `oracle`.
  Each renders a distinct, recognizable sigil with golden snapshot pinned.

### M4 — CLI + Web surfaces
- `werk sigil` subcommand with full flag set + structured JSON output
  + `assert_cmd` integration tests.
- Filesystem archive at `~/.werk/sigils/YYYY-MM-DD/` and cache at
  `~/.werk/sigils/cache/<hash>.svg`.
- `GET /api/sigil` axum handler.
- `GET /api/sigil/stream` SSE handler with `sigil_invalidated` events
  emitted from existing mutation handlers.

### M5 — werk-tab integration
- Third mode toggle (alongside `space` and `field`) on the new-tab page.
- Glance preset render via `/api/sigil`.
- Sigil-specific SSE refresh.

### M6 — combinators + hardening
- `SheetLogic` (`Grid` layouter recursing into the engine).
- `CompositeLogic` with `Concentric` rule.
- `render_animation(scope, logic, axis, output)` with `SeedSweep` +
  `ParamSweep` axes and `FrameSequence` output.
- Error-handling discipline: loud at logic-construction; graceful at
  render-time.
- Cache retention: 7-day default for `cache/`, indefinite for archive.
- Logic-file hot-reload via `notify` watcher on
  `werk-sigil/presets/`.

## Environment Setup

- Rust nightly toolchain (per repository's `rust-toolchain.toml`,
  channel `nightly`, edition 2024).
- New crate `werk-sigil/` with `Cargo.toml` added to workspace members.
- New runtime dependency `rhai = "1"` with features `[]` (expression-mode
  uses default features minus `unchecked`; recommended trim — see
  `library/sigil-engine-decisions.md`).
- New runtime dependency `notify = "6"` (for hot-reload in M6 only).
- werk-tab dependency: none (vanilla JS extension; no toolchain change).
- Workspace members updated:
  ```
  members = ["werk-core", "werk-cli", "werk-mcp", "werk-shared",
             "werk-tui", "werk-web", "werk-sigil",
             "werk-app/src-tauri"]
  ```

## Infrastructure

**Services:**
- No new long-running services. The engine is a pure function; consumers
  (CLI, web) call it synchronously. The existing `werk serve` web server
  is the only daemon, and it is shared with prior work — workers do not
  start it for tests.

**Ports:**
- `werk-web` runs on its existing port (default 3749, see
  `werk-cli/src/commands/serve.rs`). No new ports introduced.

**Filesystem:**
- `~/.werk/sigils/YYYY-MM-DD/` (archive)
- `~/.werk/sigils/cache/` (transient, 7-day retention)

**Off-limits:**
- `designs/werk-conceptual-foundation.md` — sacred core; do not edit.
- `designs/sigil-engine.md` — architectural authority; do not edit.
- `werk-core/src/tension.rs` — do not change `TensionStatus` enum
  variants. The "held" / "frozen" status names in the design are
  resolved as derived attributes, not enum variants. See
  `library/sigil-engine-decisions.md`.
- `tensions.json` — tracked snapshot; do not commit changes that flush
  it.
- `.werk/` — gitignored runtime DB. Workers may write here for fixtures.

## Testing Strategy

- **Unit tests** in each crate (`#[cfg(test)] mod tests`) for stage
  implementations, attribute computation, scope resolution, expression
  evaluation. Conventions: in-memory store fixtures via
  `Store::new_in_memory()`; `tempfile::TempDir` for filesystem.
- **Integration tests** in `werk-sigil/tests/` for full pipeline runs
  against fixture werk-states.
- **CLI tests** in `werk-cli/tests/sigil.rs` using `assert_cmd` +
  `predicates` + `tempfile`, following the existing pattern in
  `werk-cli/tests/json.rs`.
- **Golden snapshot tests** pinned per preset in
  `werk-sigil/tests/snapshots/`. Stored as `.svg` files; committed.
  Determinism guaranteed by `seed = hash(canonical_inputs)`.
  Regeneration is a one-line `cargo test -- --update-snapshots` analogue
  (workers add a `WERK_UPDATE_SNAPSHOTS=1` env var convention; document
  in `library/architecture.md`).
- **Non-test runs** during development:
  - `cargo build -p werk-sigil`
  - `cargo run -p werk -- sigil 2 --logic contemplative --out /tmp/x.svg`
  - Visual inspection of `/tmp/x.svg` in any SVG viewer.

## User Testing Strategy

- **CLI surface** is the primary user-testing surface. Validators run
  `werk sigil ...` and inspect SVGs.
- **Web surface** is secondary: `curl http://localhost:3749/api/sigil?...`
  returns SVG bytes; `curl -N .../api/sigil/stream` confirms SSE.
- **werk-tab** is tertiary: visual inspection in a browser with the
  extension loaded (M5).
- Tooling: `tuistory` is not applicable (no TUI changes). `agent-browser`
  is applicable for werk-tab testing in M5 only.
- **Manual eyeball review** is required for milestone gate on M2 (first
  contemplative render), M3 (all five presets), M5 (werk-tab visual).
  Workers attach SVGs to handoffs and the orchestrator surfaces them to
  the user for review.

## Mission Readiness

To be verified at the start of the Zo session by readiness subagents (see
`kickoff-prompt.md` step 3):

- `cargo --version` and `cargo build -p werk-core` succeed on Zo.
- `cargo add --dry-run rhai = "1"` confirms registry access.
- `tensions.json` is present at the repo root.
- `~/.werk/` is writable (workers will write archive/cache files here).

## Non-functional Requirements

- **Determinism.** Same `(Scope, Logic, seed)` produces byte-identical
  SVG output. Test asserts.
- **Provenance.** Every rendered SVG embeds
  `(scope, logic_id, logic_version, seed)` in `<metadata>`. Renderable
  from a saved file alone.
- **Pure boundary.** The engine signature `(Scope, Logic) → Sigil`
  performs zero I/O at the boundary. No clock reads inside stages
  (clock supplied via `Ctx::now`). No DB writes. No network.
- **Loud at construction; graceful at render.** Schema parse, expression
  parse, IR-shape mismatch all fail loudly when the `Logic` is loaded.
  Per-element data gaps degrade gracefully (skip element, log warning,
  continue).
- **Stable public API.** Attribute-name and channel-name registries are
  public API once shipped. Additions in v1 are free; subsequent additions
  must follow a deprecation cycle.
- **Recursion limit.** Combinators may nest up to depth 4; depth-5
  attempts must error clearly.
- **Performance.** v1 target: a `contemplative` render of a 50-tension
  subtree completes in < 100ms on a modern machine. Benchmark in M6.

## Out of Scope (v1)

Per `designs/sigil-engine.md` Part IX:

- Terminal renderer.
- `EpochRange` animation axis (blocks on werk-core gaining historical-state
  queries).
- `AnimatedSvg` output (FrameSequence only in v1).
- Composite rules beyond `Concentric`.
- E4 full scripting / WASM plugins.
- Glyph asset hot-reload.
- Multi-host / multi-user concerns.
- Tween animation between Logics.

## Branch Strategy

All work lands on the `sigil-engine-v1` branch (already created and
checked out by the prep). The branch is opened as a PR to `main` only
after the M6 milestone seals and all validation contract assertions are
`passed`.
