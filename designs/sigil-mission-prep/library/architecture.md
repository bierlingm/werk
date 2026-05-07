# Sigil Engine — architecture

The authority is `designs/sigil-engine.md`. This file is a worker-facing
distillation, focused on what to build and where.

## Components

### `werk-core` additions

Adds four canonical IRs and a metadata index:

- `werk_core::ir::TensionList`  — flat, attributed list. Each entry holds a
  `Tension` plus an `Attributes` map (computed via attribute-name registry).
- `werk_core::ir::TensionTree`  — wraps `Forest` with a side-table of node
  attributes.
- `werk_core::ir::AttributeGraph` — wraps `Forest::graph` (FNX `DiGraph`)
  with node attributes and edge type translation
  (`contains` → `parent_child`, etc.).
- `werk_core::ir::EpochSeries` — per-tension series backed by
  `Store::get_epochs(tension_id)`. v1 does not support cross-tension
  composition.

All four expose:
```rust
pub trait Ir { fn kind(&self) -> IrKind; }
```
where `IrKind` is the discriminant used for stage compatibility checks.

The `sigils` index table records metadata only (no SVG bytes). Schema:

```sql
CREATE TABLE IF NOT EXISTS sigils (
  id              INTEGER PRIMARY KEY,
  short_code      INTEGER UNIQUE NOT NULL,
  scope_canonical TEXT NOT NULL,
  logic_id        TEXT NOT NULL,
  logic_version   TEXT NOT NULL,
  seed            INTEGER NOT NULL,
  rendered_at     TEXT NOT NULL,        -- ISO-8601 UTC
  file_path       TEXT NOT NULL,        -- absolute path under ~/.werk/sigils/
  label           TEXT NULL
);
CREATE INDEX IF NOT EXISTS idx_sigils_short_code ON sigils(short_code);
CREATE INDEX IF NOT EXISTS idx_sigils_logic ON sigils(logic_id);
```

Address parsing extends `Address` with `Sigil(i32)` and recognizes the
`*N` and `space:*N` syntactic forms.

### `werk-sigil` (new crate)

Crate layout:

```
werk-sigil/
├── Cargo.toml
├── presets/
│   ├── contemplative.toml   # already authored; the reference preset
│   ├── glance.toml          # M3
│   ├── snapshot.toml        # M3
│   ├── identity.toml        # M3
│   └── oracle.toml          # M3
├── src/
│   ├── lib.rs               # re-exports + Engine entry point
│   ├── engine.rs            # `Engine::render(scope, logic) -> Sigil`
│   ├── ctx.rs               # `Ctx { now, store, workspace_name, ... }`
│   ├── scope.rs             # `Scope`, `ResolvedScope`
│   ├── logic.rs             # `Logic`, `LogicId`, `LogicVersion`
│   ├── sigil.rs             # `Sigil` (output), `SvgBytes`
│   ├── registry/
│   │   ├── mod.rs
│   │   ├── mark.rs          # Primitive enum
│   │   ├── channel.rs       # Channel enum + name table
│   │   ├── attribute.rs     # stable attribute names (Part IV)
│   │   └── glyph.rs         # Glyph family registry
│   ├── ir.rs                # IR trait + IrKind enum (mirrors werk-core)
│   ├── stages/
│   │   ├── mod.rs
│   │   ├── selector.rs      # Selector trait + Subtree, Space, Query, Union
│   │   ├── featurizer.rs    # Featurizer trait + TensionTree, TensionList,
│   │   │                    # AttributeGraph, EpochSeries
│   │   ├── encoder.rs       # Encoder trait + StructuralDefault,
│   │   │                    # ShapeByStatus, TomlDeclarative
│   │   ├── layouter.rs      # Layouter trait + RadialMandala,
│   │   │                    # FractalBranch, Constellation, Grid
│   │   ├── stylist.rs       # Stylist trait + InkBrush, MinimalLine,
│   │   │                    # Glyphic
│   │   └── renderer.rs      # SvgRenderer
│   ├── glyphs/
│   │   ├── mod.rs           # GlyphFamily trait
│   │   ├── alchemical.rs    # inline path data, ~16-50 glyphs
│   │   ├── geomantic.rs     # 16 binary patterns (deterministic)
│   │   └── handdrawn.rs     # ~12-30 brush atoms
│   ├── toml_schema.rs       # serde-driven schema + load_preset()
│   ├── expr.rs              # Rhai integration; ChannelExpr, evaluator
│   ├── error.rs             # SigilError enum
│   ├── archive.rs           # filesystem archive + cache helpers (used by
│   │                        # CLI and web — engine itself stays pure)
│   └── animation.rs         # render_animation() entry point (M6)
├── tests/
│   ├── snapshots/
│   │   ├── contemplative.svg     # pinned golden
│   │   ├── glance.svg
│   │   ├── snapshot.svg
│   │   ├── identity.svg
│   │   └── oracle.svg
│   ├── fixtures/
│   │   ├── small_tree.rs         # builds a Store + tensions for tests
│   │   └── ...
│   ├── pipeline_smoke.rs         # end-to-end render check
│   └── presets.rs                # one test per preset, golden compared
```

### Pipeline

```
                                            +-----------+
Scope ──► Selector ──► ResolvedScope ──────►|           |
                                            | Featurizer|──► IR
                                            +-----------+
                                                          
+---------+   marks  +----------+   placed   +---------+    styled  +----------+
| Encoder | ───────► | Layouter | ─────────► | Stylist | ─────────► | Renderer |
+---------+          +----------+            +---------+            +----------+
                                                                          │
                                                                          ▼
                                                                       Sigil
                                                                    (SVG bytes
                                                                    + provenance)
```

- `MarkSpec` — `{ id, primitive, channels }`. Position channels (`cx`, `cy`)
  are not filled here; the layouter fills them.
- `Frame` — `{ position, rotation, scale, parent_frame }`. Frames nest.
- `Layout` — `{ frames, structural_marks }`.
- `PlacedScene` — `{ marks_with_frames, structural_marks }`.
- `StyledScene` — `{ placed_scene, palette, filters, background }`.

### `Ctx`

```rust
pub struct Ctx<'a> {
    pub now: DateTime<Utc>,
    pub store: &'a Store,
    pub workspace_name: String,
    pub seed: u64,
    pub rng: ChaChaRng,            // seeded; advances during render
    pub diagnostics: Diagnostics,  // collects render-time warnings
    // ...
}
```

Stages use `ctx.diagnostics.warn(...)` for render-time issues. The renderer
materializes accumulated diagnostics into `<metadata>` so saved SVGs are
self-describing (per design Part VIII).

### Determinism

`seed = u64::from_le_bytes(blake3(scope_canonical || logic_canonical || logic_version)[0..8])`
when not user-supplied. `rng = ChaChaRng::seed_from_u64(seed)`. Same input
→ byte-identical SVG. Tests assert this with byte-equality on the snapshot
file.

### Provenance metadata

Every SVG starts with:
```xml
<metadata>
  <werk-sigil>
    <scope>...canonical...</scope>
    <logic>{id}@{version}</logic>
    <seed>{n}</seed>
    <generated>{iso8601}</generated>
    <warnings count="0"/>
  </werk-sigil>
</metadata>
```

`generated` uses `ctx.now`, not the system clock. Tests using a fixed
`Ctx::now` get fixed metadata.

## Surfaces (M4)

### CLI

```
werk sigil [SCOPE...] [--logic NAME|PATH] [--seed N] [--out PATH]
           [--save] [--json] [--dry-run]
```

- Positional `SCOPE` accepts any address (`#42`, `*7`, `werk:#3`). Zero
  positionals = whole field (mirrors `werk list` / `werk tree`).
- `--logic` defaults to `"contemplative"`. Bare name → look up in
  `werk-sigil/presets/`. Path → load directly.
- `--seed` overrides deterministic default.
- `--out PATH` writes SVG to file.
- `--save` writes archive entry + records in the `sigils` table.
- `--json` emits a structured report (path, hash, scope, logic).
- `--dry-run` resolves and reports without rendering or persisting.

Module: `werk-cli/src/commands/sigil.rs`. Dispatch from
`werk-cli/src/main.rs` alongside `Commands::Field`. Tests in
`werk-cli/tests/sigil.rs` using `assert_cmd`.

### Web

`werk-web/src/lib.rs` adds:

```rust
.route("/api/sigil", get(get_sigil))
.route("/api/sigil/stream", get(sse_sigil_handler))
```

`get_sigil` accepts query params `scope`, `logic`, `seed`. Returns SVG bytes
with `Content-Type: image/svg+xml`. Computes cache key and serves from
`~/.werk/sigils/cache/<hash>.svg` if present; renders + caches if not.

`sse_sigil_handler` clones the existing `BroadcastStream` from
`AppState.tx`, filters to `kind == "sigil_invalidated"`, and emits SSE
events. Each existing mutation handler (create/update/resolve/release/
reopen) gets a second `state.tx.send(SseEvent { kind:
"sigil_invalidated".into() })` next to its current emit.

### werk-tab (M5)

Vanilla JS extension. Adds a third mode toggle (`sigil`) in
`werk-tab/index.html` next to `field`. New section renders the result
of `GET /api/sigil` (glance preset) and refreshes on
`sigil_invalidated` events from `/api/sigil/stream`.

No Rust changes for this milestone. No bundler. No framework.

## Combinators (M6)

### Sheet (`Grid` layouter recursing)

`SheetLogic` has `pipeline.layouter = "grid"` and `pipeline.recursive_logic
= "<inner_preset>"`. The Grid layouter, given a `Union` scope, calls
`engine.render(sub_scope, inner_logic)` for each sub-scope and tiles the
results. Recursion-depth limit = 4 (per design Part III); depth-5
attempts return `SigilError::RecursionLimit`.

### Composite (`Concentric` only in v1)

`CompositeLogic` carries multiple `(scope, logic)` pairs and a composition
rule. `Concentric`: outermost pair on the outer ring, inner pair on the
inner ring. Implemented as a special selector that produces a stacked
`MarkSpec` set with frame `parent_frame` pointing at concentric anchors.

Other rules (`Overlay`, `SideBySide`, `Masked`) are deferred per design.

### Animation

```rust
pub fn render_animation(
    scope: Scope,
    logic: Logic,
    axis: AnimationAxis,
    output: AnimationOutput,
    ctx: &mut Ctx,
) -> Result<AnimatedSigil, SigilError>;

pub enum AnimationAxis {
    SeedSweep { start: u64, end: u64, step: u64 },
    ParamSweep { stage: StageRef, param: String,
                 from: Value, to: Value, frames: usize },
    // EpochRange { from: EpochId, to: EpochId } — DEFERRED
}

pub enum AnimationOutput {
    FrameSequence { dir: PathBuf },
    // AnimatedSvg — DEFERRED
}
```

`SeedSweep` re-renders with each seed in `[start, end)` step `step`.
`ParamSweep` interpolates a single stage param (numeric only in v1) across
`frames` steps. Output is one SVG per frame in `dir`.

## Error Handling Discipline

Per design Part VIII:

- **Logic-construction time** (loud): schema parse error, expression parse
  error, mark/channel name not in registry, IR-shape mismatch (e.g., a
  tree-requiring layouter receives an `AttributeGraph`), recursion depth
  exceeded. These return `SigilError::Construction { ... }` with
  precise location.

- **Render time** (graceful): missing data on individual elements (e.g., a
  referenced field is `None` for a tension), unrecognized glyph index in
  family, expression eval error on one element. Stages emit
  `ctx.diagnostics.warn(...)`, skip the element, and continue. The
  finished SVG `<metadata>` includes a `<warnings count="N">` block with
  details so the saved file is self-describing.

The renderer **never panics**. Internal invariant violations return
`SigilError::Internal` with a descriptive message; tests assert this.

## Cache and Archive

The engine itself does not cache (purity). `werk-sigil::archive::*`
provides:

- `archive_path(scope, logic, seed) -> PathBuf` — under
  `~/.werk/sigils/YYYY-MM-DD/`.
- `cache_path(scope, logic, seed, werk_state_revision) -> PathBuf` — under
  `~/.werk/sigils/cache/`. Filename is `blake3(...)`-derived.
- `cleanup_cache(retention_days: u32)` — removes `cache/*.svg` older than
  `retention_days` (default 7). Called by the CLI and web start-up.
- The archive is never auto-cleaned.

CLI `--save` uses `archive_path` + records in `sigils` index.
Web handler uses `cache_path` for transparent caching.

## Test approach

- **Determinism** is the primary test invariant. Same `(scope, logic, seed)`
  → byte-identical SVG.
- **Provenance metadata** asserted as a structural check (regex on
  `<scope>`, `<logic>`, `<seed>`).
- **Element counts** asserted (e.g., a 5-tension subtree produces ≥5 `<circle>`
  or `<glyph>` elements).
- **Golden snapshots** pinned per preset against a fixed fixture werk-state
  (built in `tests/fixtures/small_tree.rs`).

Snapshot regeneration: `WERK_UPDATE_SNAPSHOTS=1 cargo test -p werk-sigil`.
The orchestrator must approve any snapshot change before commit.
