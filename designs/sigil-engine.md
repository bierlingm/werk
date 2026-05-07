# Sigil Engine: Visualizing Werk-State as Operative Artifacts

**Drafted:** 2026-05-07, through dialogue.
**Status:** Architectural design. Captures 13 decisions reached through interview. Implementation pending.

---

## What This Document Is

The sigil engine renders werk-state as visual artifacts — SVG images compressed from the structural situation a practitioner is holding. Five purposes are served by one engine: **contemplative** (sit with), **glance** (read in a second), **snapshot/archive** (frozen-in-time, browsable as a series), **identity/signature** (the shape of a year, a project, a field), and **oracle** (read it to know where attention belongs).

It is not a charting library. It is not data viz. It is an operative instrument adjacent to werk's tension-tracking — the visual register matching werk's structural-dynamics register.

The doc captures the architecture; the implementation is a separate concern. `werk-conceptual-foundation.md` is the authority for *what werk is*; this doc is the authority for *what the engine is*. Both derive from the same lineage.

---

## Part I: The Sacred Core of the Engine

These are the invariants. Everything else derives from them.

### 1. The engine is a pure function

```
(Scope, Logic) → Sigil
```

No I/O at this boundary. No clock reads, no database writes, no network calls. Given the same `Scope` and `Logic`, the engine returns the same `Sigil`. Persistence, caching, live-update, and surface integration are responsibilities of *consumers* of the engine, not the engine itself.

### 2. Scope is a first-class abstraction

```rust
enum Scope {
    Tension(Id),
    Subtree { root: Id, depth: usize },
    Space(Name),
    Union(Vec<Scope>),
    Query(Predicate),
}

struct ResolvedScope { scope: Scope, at: Option<EpochId> }
```

The `at:` parameter is present from day one even if `Some(_)` is unsupported until werk-core grows historical-state queries. Animation and snapshot/archive depend on this.

### 3. Five-stage pipeline

```
Selector → Featurizer → Encoder → Layouter → Stylist → Renderer
```

Each stage is a pluggable trait. A *Logic* is a tuple of stage choices plus parameters. The Renderer is the backend swap-point (SVG primary; raster feature-gated; terminal deferred).

### 4. Seeded determinism

Default `seed = hash(scope_canonical, logic_canonical, logic_version)`. Same input → same SVG, byte-for-byte. Setting `seed = Some(n)` opts into stochastic variation. Every rendered sigil emits its provenance (`scope, logic_id, logic_version, seed`) as SVG metadata, so any sigil is regenerable from its file.

### 5. Four canonical IRs

```
TensionList   — flat, attributed list
TensionTree   — rooted tree with attributes
AttributeGraph — full typed graph (lossless universal)
EpochSeries   — time-indexed sequence of states
```

Live in `werk-core` (not `werk-sigil`). Featurizers declare their output IR; downstream stages declare their accepted input IR. Engine validates compatibility at logic-construction time.

### 6. The visual vocabulary is grammar-of-graphics-shaped

- **Encoder** fills geometry/stroke/fill channels on `MarkSpec` per element. *What* each thing looks like.
- **Layouter** fills position via `Frame` and emits `structural_marks`. *Where* things go and any structural drawing the layout needs.
- **Stylist** transforms a placed scene globally. Palette, line-weight, glyph-family, post-filters.
- **Renderer** materializes a `StyledScene` to bytes (SVG/PNG/etc).

Marks: `Circle, Ellipse, Path, Polygon, Glyph, Text, Group`. Channels are typed slots on a mark with a stable name registry.

### 7. The user surface has four tiers

| Tier | What | Form |
|---|---|---|
| 1 — preset | Pick a name | `werk sigil #42 --logic contemplative` |
| 2 — stage swap | Override individual stages | `--layouter constellation --stylist minimal` |
| 3 — fused logic | Author a Logic file | TOML, with E3 expression language for channel formulas |
| 4 — custom Rust | Add new stages or fully custom render | Rust crate depending on `werk-sigil` |

Tier 3 supports E2 (declarative field/channel/scale references) and E3 (small expression language for channel formulas). E4 (full scripting, e.g. Lua/Rhai) is **not** in v1; WASM plugins are the future path if needed.

### 8. Sigils are artifacts, not gestures

A sigil is a *view of werk-state*, not an event in it. Rendering a sigil does not enter the gesture log. A `sigils` index table in `werk-core` records metadata only (no SVG bytes); SVGs live on the filesystem at `~/.werk/sigils/`. Sigils get short codes `*N` parallel to tensions (`#42`) and notes (`n3`).

---

## Part II: The Five Stages

### Selector

```rust
trait Selector {
    fn select(&self, ctx: &mut Ctx, scope: Scope) -> ResolvedScope;
}
```

Resolves the abstract `Scope` (CLI argument, query, tension reference) into a concrete subset of werk-state at the requested epoch. Hands off to the Featurizer. Selectors are usually thin — most logic is "fetch tensions matching the scope at time T."

### Featurizer

```rust
trait Featurizer {
    type Output: IR;
    fn featurize(&self, ctx: &mut Ctx, scope: ResolvedScope) -> Self::Output;
}
```

Reads werk-state via `werk-core` APIs and produces one of the four IRs. Computes/exposes attributes from the **stable attribute registry** (Part IV). Featurizers may attach *custom attributes* beyond the registry; downstream stages that don't recognize them ignore them.

### Encoder

```rust
trait Encoder<I: IR> {
    fn encode(&self, ctx: &mut Ctx, ir: &I) -> Vec<MarkSpec>;
}

struct MarkSpec {
    id: ElementId,
    primitive: Primitive,        // Circle | Ellipse | Path | …
    channels: HashMap<Channel, Value>,  // r, stroke_color, fill_opacity, …
    // Position channels (cx, cy) are NOT filled here; Layouter fills them.
}
```

Per-element decision: what shape, what color, what stroke. Reads from the IR's attribute map. Channel values can be literals, field references, or E3 expressions over fields.

### Layouter

```rust
trait Layouter<I: IR> {
    fn layout(&self, ctx: &mut Ctx, ir: &I, marks: &[MarkSpec]) -> Layout;
}

struct Layout {
    frames: HashMap<ElementId, Frame>,    // position, rotation, scale, parent_frame
    structural_marks: Vec<MarkSpec>,      // marks the layout itself wants to draw
}
```

Places each Encoder-produced mark in 2D space via Frames (which can nest via `parent_frame`). May add structural marks of its own — connecting curves between parent and child, concentric guide rings, mandala spokes, halos. These are conceptually *part of the layout*, not the data.

### Stylist

```rust
trait Stylist {
    fn style(&self, ctx: &mut Ctx, scene: PlacedScene) -> StyledScene;
}
```

Transforms the placed scene globally. Palette substitution, line-weight scaling, glyph-family substitution, post-render filter application (SVG `<filter>` chains for ink-bleed, watercolor, hand-drawn jitter), background washes. Mostly transformation, not generation — but may add ambient marks (paper textures, vignettes).

### Renderer

```rust
trait Renderer {
    fn render(&self, scene: StyledScene) -> Box<dyn SigilOutput>;
}
```

Backend-specific materialization. `SvgRenderer` produces an SVG document; `RasterRenderer` (feature-gated) produces a PNG via `resvg` or `tiny-skia`. The Renderer is where the cross-backend abstraction terminates — non-portable features (SVG filters, CSS animations) get materialized here or substituted.

---

## Part III: Combinators

The engine signature `(Scope, Logic) → Sigil` covers single-image rendering. Three combinators sit on top.

### Sheet (a Logic)

A *SheetLogic* has a Layouter (`Grid` in v1) that recurses: for each sub-scope in `Union(...)`, calls `engine.render(sub_scope, inner_logic)`, then composes the resulting sub-sigils on a single canvas. Use case: contact-sheet of all top-level tensions, side by side.

### Composite (a Logic)

A *CompositeLogic* layers multiple `(Scope, Logic)` pairs into one image with a composition rule. v1 ships `Concentric` (outer scope = outer ring, inner scope = inner). Future: `Overlay`, `SideBySide`, `Masked`.

### Animation (a separate combinator)

Different output type — multi-frame instead of single — so it gets its own entry point:

```rust
fn render_animation(
    scope: Scope,
    logic: Logic,
    axis: AnimationAxis,        // SeedSweep | ParamSweep | EpochRange
    output: AnimationOutput,    // FrameSequence | AnimatedSvg
) -> AnimatedSigil;
```

v1: `SeedSweep` and `ParamSweep` axes; `FrameSequence` output. `EpochRange` and `AnimatedSvg` deferred (the former blocks on werk-core time-travel; the latter on portability concerns).

**Recursion-depth limit.** Logics can nest Logics (sheet of composites of sheets…). Depth limit = 4 with a clear error.

---

## Part IV: The Stable Attribute Registry (v1)

These attribute names appear in user-authored TOML and *must remain stable*. Future additions go through deprecation cycles.

**Identity.** `id`, `short_code`, `space`.

**Text.** `desire`, `actual`.

**Status.** `status` (categorical: `active`, `held`, `resolved`, `released`, `frozen`), `is_held`, `is_resolved`, `is_released`.

**Time.** `created_at`, `updated_at`, `deadline`, `last_pulse_at`, `age_seconds`, `time_to_deadline_seconds`.

**Computed.** `urgency` (0..1), `staleness` (0..1), `gap_magnitude`.

**Trajectory** (from `werk-core::projection`). `frequency_per_day`, `frequency_trend`, `gap_trend`, `mutation_count`, `is_projectable`.

**Structure.** `depth`, `child_count`, `descendant_count`, `parent_id`, `parent_short_code`, `note_count`, `has_children`.

**Edge attributes.** `edge_type` (categorical: `parent_child`, `split_from`, `merge_into`, `references`), `edge_weight`.

---

## Part V: The v1 Starter Library

**Aesthetic center: sigilic with mandala affordances.** Privileges hand-drawn / glyph-heavy / contemplative-occult registers; reaches generative-art and iconographic registers via additional Stylists later.

**Layouters (4):**
1. `RadialMandala` — concentric rings, root at center, depth → radius. The default for `Tension` and `Subtree` scopes.
2. `FractalBranch` — recursive branching tree. For organic, asymmetric shapes.
3. `Constellation` — force-directed scatter with edge bundling. Default for `AttributeGraph` scopes.
4. `Grid` — sheet combinator backbone.

**Stylists (3):**
1. `InkBrush` — black-on-cream, brush textures, optional jitter via SVG turbulence.
2. `MinimalLine` — clean monochrome linework, no fills, no textures. Doubles as approximate terminal-fallback when that backend lands.
3. `Glyphic` — heavy use of glyph fonts, formal symmetry.

**Encoders (3):**
1. `StructuralDefault` — sensible defaults for any IR. Status → primitive, urgency → stroke_width, gap → fill_opacity, depth → scale.
2. `ShapeByStatus` — categorical primitive choice driven by status.
3. `TomlDeclarative` — workhorse. Reads channel mappings from the Logic's TOML. The expression-language (E3) lives inside it.

**Glyph families (3, embedded as assets):**
1. **Alchemical** — planetary, elemental, processual symbols (~50).
2. **Geomantic** — the 16 geomantic figures (binary patterns of 4 dots).
3. **Hand-drawn primitives** — curated brush-stroke atoms (~30) the engine assembles into composite forms.

**Presets (5):**
- `contemplative.toml` — `Subtree(root, 4)` + `RadialMandala` + `InkBrush` + `StructuralDefault`. Sit-with.
- `glance.toml` — `Space(active)` + `RadialMandala` + `MinimalLine` + `ShapeByStatus`. Compact, fits in werk-tab.
- `snapshot.toml` — `Space(all)` + `RadialMandala` + `Glyphic` + `StructuralDefault`. Auto-saves with date stamp.
- `identity.toml` — `Union(all_spaces)` + `FractalBranch` + `Glyphic` + `TomlDeclarative`. Rich, signature-flavored.
- `oracle.toml` — `Query(active)` + `Constellation` + `InkBrush` + `ShapeByStatus`. Surfaces overdue/stale via color emphasis.

(Generative-seed is not its own preset — it's `seed = None` + `--reroll` on any preset.)

---

## Part VI: Crate Layout and Dependencies

**`werk-core`** grows:
- The four canonical IRs (`TensionList`, `TensionTree`, `AttributeGraph`, `EpochSeries`).
- A `sigils` index table (id, scope_canonical, logic_id, logic_version, seed, rendered_at, file_path, label?).
- Address-parser support for `*N` short codes.

**`werk-sigil`** (new crate) contains:
- Stage traits, registries (mark, channel, glyph-family, attribute-name).
- Built-in stages (4 Layouters, 3 Stylists, 3 Encoders).
- Glyph assets (embedded via `include_bytes!`).
- TOML schema and loader.
- E3 expression evaluator.
- Preset library (5 TOML files, embedded).
- SVG renderer (default).
- Raster renderer (feature `raster`).

**`werk-cli`** gains a `sigil` subcommand.

**`werk-web`** gains:
- `GET /api/sigil?scope=…&logic=…&seed=…` (one-shot SVG response).
- `GET /api/sigil/stream?scope=…&logic=…` (SSE; emits invalidation events when relevant werk-state or Logic file changes).

**`werk-tab`** integrates the glance preset on its existing event-stream subscription.

**`werk-tui`** is deferred — terminal sigil rendering is a separate design problem.

Dependency direction: `werk-cli`, `werk-web`, `werk-tab` depend on `werk-sigil`; `werk-sigil` depends on `werk-core`; `werk-core` depends on nothing internal.

---

## Part VII: Persistence, Caching, Live Update

**Engine = pure.** Persistence belongs to consumers.

**Filesystem archive.** Saved sigils at `~/.werk/sigils/YYYY-MM-DD/<scope-slug>-<logic>-<seed>.svg`. Cache-only sigils at `~/.werk/sigils/cache/<hash>.svg`. SVG metadata embeds full provenance — every file is self-describing.

**`sigils` index in `werk-core`.** Metadata only. Lets `werk list --kind sigil`, `werk show *7`, `werk log` see saved sigils. Cache files do *not* enter the index.

**Cache key:** `(scope_canonical, logic_canonical, logic_version, seed, werk_state_revision)`. The `werk_state_revision` is `max(updated_at)` over the resolved scope's elements. Any state change → key changes → cache miss → re-render.

**Live update via SSE.** `werk serve`'s existing event stream (mutations, epoch closes) drives sigil invalidation. `GET /api/sigil/stream` opens an SSE connection per `(scope, logic)`; the server emits `invalidate` events that prompt the client to re-fetch.

**Hot-reload of Logic files** via fsevents/inotify on `werk-sigil/presets/` and any user-supplied logic paths. File change → `logic_version` recomputes → invalidate event on every active stream using that logic.

**Glyph asset hot-reload: not in v1.** Embedded via `include_bytes!`; iteration on glyphs requires recompile.

**Cache retention.** `cache/` entries deleted after 7 days unused. Archive (`YYYY-MM-DD/`) entries persist; user-controlled cleanup.

---

## Part VIII: Error Handling Surface

**Loud at logic-construction time.** Schema parse error, expression parse error, mark/channel mismatch, IR-shape incompatibility (e.g. tree-requiring layouter handed a flat scope).

**Graceful at render time.** Missing data on individual elements (e.g. a referenced field is `None`) → that element is skipped or rendered with a fallback; the rest of the sigil renders. Log warnings; never panic.

**SVG metadata records errors.** Any non-fatal render-time issue is captured in `<metadata>` so that a saved sigil carries its own caveats.

---

## Part IX: What Is Deferred

These are *not* in v1. They have shapes that don't block v1 architecturally; they wait for need to materialize.

- **Terminal renderer.** Character-grid sigil rendering is its own design problem.
- **`EpochRange` animation axis.** Blocks on werk-core gaining historical-state queries.
- **`AnimatedSvg` output.** v1 is `FrameSequence` only.
- **Composite rules beyond `Concentric`.** `Overlay`, `SideBySide`, `Masked` later.
- **Layouters beyond the four.** `Geomantic`, `Stratigraphy`, `Spiral` etc. land as logics demand them.
- **Stylists beyond the three.** `WatercolorWash`, `NeoArtDeco`, `Engraving`, `Risograph` etc.
- **E4 scripting / WASM plugins.** When TOML+E3 demonstrably can't reach a common case.
- **Tween animation between Logics.** Hard to do well; defer.
- **Glyph asset hot-reload.** Recompile is acceptable for v1.
- **Multi-host / multi-user concerns.** SSE-per-tab is fine for one user; revisit if scenarios change.

---

## Part X: Open Implementation Items

Architectural decisions are settled. These are implementation choices to make during the build, not before.

1. **Scope-spec syntax for the CLI.** `werk sigil #42`, `werk sigil --space all --depth 4`, `werk sigil --query "active overdue"`. Bikeshed against actual use.
2. **TOML schema details.** Drive from `contemplative.toml` (the reference preset) and let the schema crystallize against it.
3. **Expression-language choice for E3.** Candidates: `evalexpr`, `cel-rust`, `rhai` in expression-mode, or a small Pratt parser. Pick when starting `TomlDeclarative`.
4. **Glyph-family curation pipeline.** Sources, license/provenance discipline, cleaning workflow. Public-domain alchemical/geomantic sources exist; hand-drawn primitives need original work.
5. **Sigil short-code allocator.** Integrate with werk's existing address parser. Conflict-free with `#`, `n`, `g:`.

---

## Part XI: Sequencing

Ordered build path. Each step is independently checkpoint-able.

1. **Foundation.** This doc + `contemplative.toml` reference preset (handwritten before code, to surface schema). `AttributeGraph` and projections in `werk-core`. `sigils` index migration.
2. **Engine spine.** `werk-sigil` crate skeleton, stage traits, `Ctx`, registries, SVG renderer. No stages implemented yet.
3. **One end-to-end logic.** `RadialMandala` + `MinimalLine` + `StructuralDefault` walking the full pipeline against `contemplative.toml`. First rendered sigil.
4. **TOML loader + E3.** Pick the expression library, wire up `TomlDeclarative` encoder, validate against the reference preset.
5. **Vocabulary expansion.** Remaining 3 Layouters, 2 Stylists, 1 Encoder. Glyph families.
6. **5 presets working.** Each preset rendered against a real werk space. Visual review.
7. **CLI surface.** `werk sigil <scope> [--logic …] [--save] [--seed …] [--out …] [--json]`.
8. **Web surface.** Endpoints, SSE invalidation stream, archive-as-cache.
9. **werk-tab integration.** Embed glance preset.
10. **Combinators.** Sheet (Grid), Composite (Concentric), Animation (SeedSweep + ParamSweep, FrameSequence output).
11. **Hardening.** Error handling, retention policies, performance pass on Constellation layout.

---

## Lineage

The engine inherits from:

- **Grammar of graphics** (Wilkinson, Wickham, Bostock, Heer) — the marks/channels/scales decomposition.
- **Sigil traditions** — chaos magic sigils, alchemical seals, geomantic figures, occult glyph composition. The aesthetic register the engine privileges.
- **Generative art lineage** — Tarbell, Tatsuya Saito, Casey Reas, plotter art / vsketch. The seeded-deterministic discipline.
- **Werk's own visualization research** (`designs/visualization-research.md`) — Sankey, Marey, strata, fisheye, conductor's score, Labanotation. Sources for future Layouters.
- **Werk's conceptual foundation** — desire above actual, theory of closure, signal by exception, gesture as unit of change. The engine renders *into* this lineage; it does not invent against it.

The sigil is not the work. The sigil is a way of seeing the shape of the work.
