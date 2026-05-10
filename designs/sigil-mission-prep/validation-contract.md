# Validation Contract — werk Sigil Engine v1

This contract is the definition of "done" for the mission. Each assertion
is testable through one of the surfaces listed in
`library/user-testing.md`.

The orchestrator on Zo should run **one review pass** on this contract
before `propose_mission`. See `kickoff-prompt.md` step 2.

---

## Area: Foundation (werk-core IRs, sigils table, address parsing)

### VAL-FOUND-001: TensionList builds with all registry attributes
Building a `TensionList` over a fixture of 5 tensions populates each entry
with the full design-Part-IV attribute set (identity, text, status, time,
computed, trajectory, structure). Trajectory attributes default sensibly
when no projection is available.
Tool: cargo test
Evidence: passing test `werk_core::ir::tension_list::tests::builds_full_attribute_set`

### VAL-FOUND-002: TensionTree wraps Forest with attributes
`TensionTree::build(store, forest, ctx)` returns a tree where every node has
a populated `Attributes` map; root depths are 0; nested children depths
match `Forest::depth(...)`.
Tool: cargo test
Evidence: passing test `tension_tree::tests::depths_and_attributes_match_forest`

### VAL-FOUND-003: AttributeGraph translates edge type names
`AttributeGraph::build(...)` exposes edges with sigil-registry names:
`contains` translates to `parent_child`; `merged_into` to `merge_into`;
`split_from` is unchanged. Unknown edge types fall through unchanged
with a `ctx.diagnostics.warn(...)` entry.
Tool: cargo test
Evidence: passing test `attribute_graph::tests::translates_edge_type_names`

### VAL-FOUND-004: EpochSeries replays per-tension epochs
`EpochSeries::for_tension(store, id)` returns a chronologically ordered
series matching `Store::get_epochs(id)` length and timestamps; each point
carries the snapshot from `EpochRecord`.
Tool: cargo test
Evidence: passing test `epoch_series::tests::matches_get_epochs_chronologically`

### VAL-FOUND-005: TensionStatus untouched
`werk-core/src/tension.rs` `TensionStatus` enum still has exactly three
variants: `Active`, `Resolved`, `Released`. No `Held` or `Frozen` variants
added.
Tool: grep / inspection
Evidence: `git diff main -- werk-core/src/tension.rs` shows no enum
changes

### VAL-FOUND-006: `is_held` derived via Frontier
A featurizer attaches `is_held=true` to an Active tension whose horizon
range starts in the future and is currently held by `Frontier::compute`;
`status` categorical attribute is `"held"` for the same tension.
Tool: cargo test
Evidence: passing test `tension_tree::tests::derives_is_held_for_unstarted_horizon`

### VAL-FOUND-007: `urgency` clamped to 0..1; `urgency_raw` exposed unclamped
For a tension whose horizon is past, `urgency` attribute is `1.0`;
`urgency_raw` is the unclamped value (>1.0).
Tool: cargo test
Evidence: passing test `tension_tree::tests::urgency_clamped_and_raw_exposed`

### VAL-FOUND-008: `gap_magnitude` is binary 0/1
The featurizer-emitted `gap_magnitude` attribute is exactly `0.0` if
`desired == actual` else `1.0` (matches `werk_core::temporal::gap_magnitude`).
Tool: cargo test
Evidence: passing test `tension_list::tests::gap_magnitude_is_binary`

### VAL-FOUND-009: `space` attribute populated from Ctx
A featurizer run with `Ctx { workspace_name: "journal", .. }` produces
entries each with `space = "journal"`.
Tool: cargo test
Evidence: passing test `tension_list::tests::space_attribute_from_ctx`

### VAL-FOUND-010: `*N` short code parses
`parse_address("*7")` returns `Address::Sigil(7)`; `parse_address("werk:*7")`
returns `CrossSpace { space: "werk", inner: Sigil(7) }`. Round-trip via
`Display` is stable.
Tool: cargo test
Evidence: passing tests `address::tests::test_sigil`, `test_cross_space_sigil`,
`test_display_roundtrip` (extended)

### VAL-FOUND-011: `*N` does not collide with other prefixes
`parse_address("g:*7")` is rejected (gestures don't accept `*`).
`parse_address("#*7")` is rejected. `parse_address("*")` is rejected.
Tool: cargo test
Evidence: passing test `address::tests::sigil_prefix_collision_rejected`

### VAL-FOUND-012: `sigils` table created via migration
A fresh in-memory store has the `sigils` table with the schema documented
in `library/architecture.md`. Indexes on `short_code` and `logic_id` exist.
Tool: cargo test
Evidence: passing test `store::sigils::tests::table_and_indexes_present`

### VAL-FOUND-013: `Store::record_sigil` persists metadata only
`record_sigil(SigilRecord { ... })` inserts a row; `list_sigils()`
returns it in chronological order; `get_sigil_by_short_code(n)` returns
exactly one record. The DB does not store SVG bytes — only the
`file_path`.
Tool: cargo test
Evidence: passing test `store::sigils::tests::insert_list_get_roundtrip`

### VAL-FOUND-014: `Store::delete_sigil` removes the row
`delete_sigil(short_code)` removes the metadata row and returns
`Ok(true)`; calling it again returns `Ok(false)`. The associated
filesystem path is **not** removed (caller's responsibility).
Tool: cargo test
Evidence: passing test `store::sigils::tests::delete_returns_existence`

---

## Area: Engine spine (M2 — first end-to-end render)

### VAL-ENGINE-001: `Engine::render` is pure
A unit test invokes `Engine::render` with a fixed `Ctx::now` and `seed`,
captures the SVG bytes, runs again, and asserts byte-equality. No
filesystem writes occur during render (test runs against a read-only
tempdir).
Tool: cargo test
Evidence: passing test `engine::tests::render_is_pure_and_deterministic`

### VAL-ENGINE-002: Default seed derives from canonical inputs
With `seed = None`, two renders of the same `(Scope, Logic)` produce
byte-identical SVGs. Changing the scope changes the seed (different SVG
bytes).
Tool: cargo test
Evidence: passing test `engine::tests::default_seed_from_canonical_inputs`

### VAL-ENGINE-003: Provenance metadata embedded
Every rendered SVG contains a `<metadata><werk-sigil>` block with
`<scope>`, `<logic>`, `<seed>`, `<generated>`, and `<warnings count="N"/>`.
Values match the inputs.
Tool: cargo test
Evidence: passing test `engine::tests::metadata_contains_provenance`

### VAL-ENGINE-004: contemplative.toml renders end-to-end
Loading `werk-sigil/presets/contemplative.toml`, resolving against a
fixture 7-node subtree, and rendering produces a non-empty SVG with at
least 7 `<circle>` or `<glyph>` data marks plus structural ring guides
and parent-child curves (per the preset's `[layouter.radial_mandala.structural_marks]`).
Tool: cargo test (golden snapshot)
Evidence: `werk-sigil/tests/snapshots/contemplative.svg` matches byte-for-byte

### VAL-ENGINE-005: Selector::Subtree resolves correctly
`Selector::Subtree { root: id, depth: 4 }.select(...)` returns the root
plus all descendants up to depth 4 inclusive; depth-5 descendants are
excluded.
Tool: cargo test
Evidence: passing test `selector::subtree::tests::respects_depth_limit`

### VAL-ENGINE-006: Featurizer::TensionTree attribute compatibility
`Featurizer::TensionTree.featurize(...)` returns a `TensionTree` whose
attributes are the union of the design Part IV registry plus any custom
attributes declared in `[featurizer.tension_tree.attributes]`. Unknown
custom attributes pass through with `AttributeValue::Unknown`.
Tool: cargo test
Evidence: passing test `featurizer::tension_tree::tests::respects_requested_attributes`

### VAL-ENGINE-007: Encoder::StructuralDefault basic mapping
StructuralDefault produces a `MarkSpec` per IR node where:
- `primitive = circle` for active, `glyph` for resolved, `polygon` for released
- `r` channel is monotone with `urgency`
- `stroke_width` is monotone with `gap_magnitude`
- `fill_opacity` is monotone with `depth` (decreasing)
Tool: cargo test
Evidence: passing test `encoder::structural_default::tests::channel_mappings`

### VAL-ENGINE-008: Layouter::RadialMandala places marks
RadialMandala places the root at the canvas center; depth-1 children at
ring radius 80; depth-2 at radius 160; angular share follows
`child_weighted` mode (children with more descendants get larger arcs).
Structural marks include parent-child curves and ring guides per
preset config.
Tool: cargo test
Evidence: passing test `layouter::radial_mandala::tests::places_root_and_rings`

### VAL-ENGINE-009: Stylist::InkBrush applies palette + filters
After InkBrush, the rendered SVG contains: `fill="#1a1818"` on data marks,
`#f5efe1` background, and a `<filter>` element if `filter_mode = "filter"`
in the preset.
Tool: cargo test
Evidence: passing test `stylist::ink_brush::tests::applies_palette_and_filter`

### VAL-ENGINE-010: TOML schema rejects malformed presets
Loading a TOML missing required `[meta]` or `[pipeline]` returns a
`SigilError::Construction` with location info pointing at the missing
section.
Tool: cargo test
Evidence: passing test `toml_schema::tests::rejects_missing_meta`,
`rejects_missing_pipeline`

### VAL-ENGINE-011: TOML schema reads contemplative.toml without warning
Loading the bundled `presets/contemplative.toml` succeeds with zero
warnings in `ctx.diagnostics`.
Tool: cargo test
Evidence: passing test `toml_schema::tests::contemplative_loads_clean`

### VAL-ENGINE-012: Render-time gracefully skips bad elements
A featurized tree where one node has `urgency = None` (synthetic), passed
through Encoder + Layouter + Renderer, produces an SVG with N-1 data
marks and a `<warnings count="1">` block; no panic.
Tool: cargo test
Evidence: passing test `engine::tests::render_skips_missing_data_with_warning`

### VAL-ENGINE-013: Eyeball review of contemplative render
The orchestrator and user have visually inspected
`validation/m2/eyeball/contemplative.svg` and confirmed it looks like a
recognizable mandala for the fixture subtree.
Tool: manual / orchestrator presents to user
Evidence: explicit user approval recorded in `validation/m2/eyeball/REVIEW.md`

---

## Area: Vocabulary (M3 — remaining stages, glyph families, four more presets)

### VAL-VOCAB-001: Encoder::ShapeByStatus per-status primitives
ShapeByStatus produces:
- `circle` for `status="active"`
- `ellipse` for `status="held"`
- `glyph` (alchemical, deterministic index) for `status="resolved"`
- `polygon` for `status="released"`
Tool: cargo test
Evidence: passing test `encoder::shape_by_status::tests::per_status_primitives`

### VAL-VOCAB-002: Encoder::TomlDeclarative parses literal channels
A preset declaring `[encoder.channels.r] literal = 18.0` produces marks
with `r = 18.0`.
Tool: cargo test
Evidence: passing test `encoder::toml_declarative::tests::literal_channel`

### VAL-VOCAB-003: Encoder::TomlDeclarative E2 field references
`r = { field = "urgency", scale = "sqrt", range = [4, 36] }` maps each
node's urgency through a sqrt scale into [4, 36].
Tool: cargo test
Evidence: passing test `encoder::toml_declarative::tests::e2_field_with_sqrt_scale`

### VAL-VOCAB-004: Encoder::TomlDeclarative E3 expression evaluation
`r = { expr = "sqrt(urgency + 0.1) * 26 + ln(child_count + 1) * 4 + 4" }`
evaluates correctly per node, with `sqrt`, `ln`, `+`, `*` resolved by
Rhai expression mode.
Tool: cargo test
Evidence: passing test `encoder::toml_declarative::tests::e3_expression_eval`

### VAL-VOCAB-005: Encoder::TomlDeclarative E3 categorical mapping
A `primitive = { field = "status", kind = "categorical", mapping = { active = "circle", held = "ellipse", ... } }`
choice produces the right primitive per node.
Tool: cargo test
Evidence: passing test `encoder::toml_declarative::tests::categorical_mapping`

### VAL-VOCAB-006: E3 parse error is loud at logic-construction
A preset containing `r = { expr = "sqrt(urgency +" }` (unclosed) returns
a `SigilError::Construction { line, col, message }` at
`Engine::compile(logic)`. No render runs.
Tool: cargo test
Evidence: passing test `expr::tests::malformed_expression_loud_at_construct`

### VAL-VOCAB-007: E3 eval error is graceful at render time
A preset whose expression references an attribute name that doesn't exist
on a particular node skips that node with a warning; other nodes
render.
Tool: cargo test
Evidence: passing test `expr::tests::missing_attribute_on_one_node_graceful`

### VAL-VOCAB-008: Layouter::FractalBranch places nodes recursively
For a 4-level tree, FractalBranch places leaves at decreasing scales;
no two marks overlap; root frame is at canvas center.
Tool: cargo test
Evidence: passing test `layouter::fractal_branch::tests::recursive_placement`

### VAL-VOCAB-009: Layouter::Constellation runs force-directed
For a 10-node `AttributeGraph` with mixed edge types, Constellation
produces a stable layout (deterministic from seed) where edge bundling
clusters nodes connected by `parent_child` edges.
Tool: cargo test
Evidence: passing test `layouter::constellation::tests::deterministic_force_layout`

### VAL-VOCAB-010: Layouter::Grid tiles sub-sigils
For a 4-element `Union` scope with `inner_logic = "glance"`, Grid
produces a 2x2 tiled SVG with four sub-sigils at expected positions.
Tool: cargo test
Evidence: passing test `layouter::grid::tests::tiles_2x2`

### VAL-VOCAB-011: Stylist::MinimalLine produces clean linework
After MinimalLine, the SVG has no `fill` attributes on data marks
(stroke-only); no SVG `<filter>` references; palette is monochrome.
Tool: cargo test
Evidence: passing test `stylist::minimal_line::tests::stroke_only_palette`

### VAL-VOCAB-012: Stylist::Glyphic uses heavy glyph substitution
After Glyphic, a majority of data marks use `<g class="glyph">` groups
(primitive = glyph). Symmetric layouts get a `<g class="glyph-mirror">`
applied where the preset enables it.
Tool: cargo test
Evidence: passing test `stylist::glyphic::tests::glyph_majority`

### VAL-VOCAB-013: Alchemical glyph family has at least 16 distinct glyphs
`AlchemicalFamily::glyph(idx)` returns distinct path data for at least
16 indices; the same idx returns the same path twice.
Tool: cargo test
Evidence: passing test `glyphs::alchemical::tests::sixteen_distinct_glyphs`

### VAL-VOCAB-014: Geomantic family has exactly 16 binary patterns
`GeomanticFamily::glyph(idx)` returns one of the 16 canonical geomantic
figures (Acquisitio, Amissio, Albus, ..., Via). Patterns match the
classical mapping (4 rows of 1 or 2 dots).
Tool: cargo test
Evidence: passing test `glyphs::geomantic::tests::canonical_sixteen_figures`

### VAL-VOCAB-015: Hand-drawn primitive family has at least 12 atoms
`HandDrawnFamily::glyph(idx)` returns distinct, non-empty path data for
at least 12 indices.
Tool: cargo test
Evidence: passing test `glyphs::handdrawn::tests::twelve_atoms_present`

### VAL-VOCAB-016: glance.toml renders successfully
`glance.toml` loads, resolves against fixture, renders; golden snapshot
pinned.
Tool: cargo test
Evidence: `werk-sigil/tests/snapshots/glance.svg` matches byte-for-byte

### VAL-VOCAB-017: snapshot.toml renders successfully
`snapshot.toml` loads, resolves against fixture, renders; golden snapshot
pinned. Snapshot uses `Glyphic` stylist visibly (glyph element majority).
Tool: cargo test
Evidence: `werk-sigil/tests/snapshots/snapshot.svg` matches byte-for-byte

### VAL-VOCAB-018: identity.toml renders successfully
`identity.toml` loads, resolves against `Union` of fixture spaces,
renders; golden snapshot pinned.
Tool: cargo test
Evidence: `werk-sigil/tests/snapshots/identity.svg` matches byte-for-byte

### VAL-VOCAB-019: oracle.toml renders successfully
`oracle.toml` loads, resolves against `Query(active)` fixture,
renders; golden snapshot pinned. Visibly uses Constellation layouter
(force-directed positioning).
Tool: cargo test
Evidence: `werk-sigil/tests/snapshots/oracle.svg` matches byte-for-byte

### VAL-VOCAB-020: Eyeball review of all five presets
Orchestrator + user have visually inspected
`validation/m3/eyeball/{contemplative,glance,snapshot,identity,oracle}.svg`
and confirmed each is a recognizable, distinct sigil for its purpose.
Tool: manual / orchestrator presents to user
Evidence: explicit user approval in `validation/m3/eyeball/REVIEW.md`

---

## Area: Surfaces (M4 — CLI, Web, archive, cache)

### VAL-SURF-001: `werk sigil <id>` renders to stdout
`werk sigil 2 --logic contemplative` prints SVG bytes to stdout (no
file written), exits 0.
Tool: assert_cmd
Evidence: passing test `werk-cli/tests/sigil.rs::renders_to_stdout`

### VAL-SURF-002: `werk sigil <id> --out PATH` writes file
`werk sigil 2 --out /tmp/x.svg` writes a valid SVG file at the given
path; stdout is empty in human mode (or a one-line summary), exits 0.
Tool: assert_cmd
Evidence: passing test `werk-cli/tests/sigil.rs::writes_to_out_path`

### VAL-SURF-003: `werk sigil --json` produces structured output
`werk sigil 2 --json` prints a JSON object with fields `scope`,
`logic`, `logic_version`, `seed`, `path` (if --out or --save was
given), `svg` (inline string if neither), `warnings` (array).
Tool: assert_cmd + serde_json
Evidence: passing test `werk-cli/tests/sigil.rs::json_output_shape`

### VAL-SURF-004: `werk sigil --dry-run` does not write
`werk sigil 2 --dry-run --out /tmp/x.svg` does not create the file;
JSON output includes `"dry_run": true`.
Tool: assert_cmd
Evidence: passing test `werk-cli/tests/sigil.rs::dry_run_does_not_write`

### VAL-SURF-005: `werk sigil --save` writes archive + records metadata
`werk sigil 2 --save` writes an SVG under `~/.werk/sigils/YYYY-MM-DD/`
and records a row in the `sigils` table. The new sigil has a `*N`
short code.
Tool: assert_cmd + filesystem inspection
Evidence: passing test `werk-cli/tests/sigil.rs::save_archives_and_records`

### VAL-SURF-006: `werk sigil --seed N` overrides default
`werk sigil 2 --seed 7` and `werk sigil 2 --seed 8` produce different
SVG bytes for the same scope. Repeating `--seed 7` produces identical
bytes.
Tool: assert_cmd
Evidence: passing test `werk-cli/tests/sigil.rs::seed_override_changes_output`

### VAL-SURF-007: Errors in --json mode use error_code
Invalid scope (`werk sigil 99999 --json`) returns exit 1, prints a JSON
error object with `code = "NOT_FOUND"`.
Tool: assert_cmd
Evidence: passing test `werk-cli/tests/sigil.rs::not_found_json_error_shape`

### VAL-SURF-008: `werk sigil --help` shows examples
`werk sigil --help` includes at least 3 worked examples in `Examples:`
block.
Tool: assert_cmd
Evidence: passing test `werk-cli/tests/sigil.rs::help_includes_examples`

### VAL-SURF-009: `GET /api/sigil` returns SVG bytes
`curl http://localhost:3749/api/sigil?scope=2&logic=contemplative` returns
HTTP 200 with `Content-Type: image/svg+xml` and a body starting with
`<?xml`.
Tool: curl + cargo test (web integration)
Evidence: passing test `werk-web/tests/sigil.rs::get_returns_svg`

### VAL-SURF-010: `GET /api/sigil` caches transparently
First call renders + writes to `~/.werk/sigils/cache/<hash>.svg`. Second
call with same params returns identical bytes faster, served from cache.
Filesystem inspection confirms a cache file with the expected name.
Tool: cargo test
Evidence: passing test `werk-web/tests/sigil.rs::caches_on_second_call`

### VAL-SURF-011: `GET /api/sigil` 400 on bad scope
`curl http://localhost:3749/api/sigil?scope=` returns HTTP 400 with a
JSON error body.
Tool: cargo test
Evidence: passing test `werk-web/tests/sigil.rs::missing_scope_returns_400`

### VAL-SURF-012: `GET /api/sigil/stream` emits sigil_invalidated on mutation
A client connected to `/api/sigil/stream` receives a `sigil_invalidated`
SSE event within 2 seconds of a `PATCH /api/tensions/<id>/desired`
request completing.
Tool: cargo test (uses futures + tokio test)
Evidence: passing test `werk-web/tests/sigil_stream.rs::invalidates_on_mutation`

### VAL-SURF-013: Cache key includes werk_state_revision
After rendering and caching, mutating a tension in the resolved scope
produces a different cache hash for the same `(scope, logic, seed)` —
two distinct cache files exist after both renders.
Tool: cargo test
Evidence: passing test `werk-sigil/tests/archive.rs::cache_key_invalidates_on_state_change`

### VAL-SURF-014: Archive paths follow YYYY-MM-DD discipline
`archive_path(scope, logic, seed)` returns
`~/.werk/sigils/YYYY-MM-DD/<scope-slug>-<logic>-<seed>.svg` where
`YYYY-MM-DD` is derived from `Ctx::now`.
Tool: cargo test
Evidence: passing test `werk-sigil/tests/archive.rs::archive_path_uses_now_date`

### VAL-SURF-015: Eyeball: CLI render of all 5 presets
`werk sigil 2 --logic <each_preset> --out /tmp/m4-<name>.svg` produces
visually correct SVGs for all 5 presets. Orchestrator + user inspect.
Tool: manual / orchestrator presents to user
Evidence: `validation/m4/eyeball/REVIEW.md`

---

## Area: werk-tab (M5)

### VAL-TAB-001: Sigil toggle present in header
After loading the extension, the new-tab page has three buttons in the
header: `space`, `field`, `sigil`.
Tool: agent-browser
Evidence: screenshot showing all three buttons

### VAL-TAB-002: Clicking sigil shows SVG section
Clicking the `sigil` toggle hides space and field sections, shows a new
`#sigil` section containing one inline `<svg>` element with at least one
data mark inside.
Tool: agent-browser
Evidence: screenshot showing sigil section with rendered SVG

### VAL-TAB-003: Sigil refreshes on mutation via SSE
With sigil mode active, triggering
`curl -X PATCH http://localhost:3749/api/tensions/<id>/desired -d ...`
causes the inline SVG to change within 2 seconds (verified by comparing
two screenshots before and after).
Tool: agent-browser + curl
Evidence: two screenshots and a hash comparison

### VAL-TAB-004: Reuses single EventSource (no second connection)
After loading sigil mode, network inspector shows exactly one open
`EventSource` connection to `/api/events` (the existing connection
gets a new event listener; no new connection is opened).
Tool: agent-browser DevTools network panel
Evidence: network log screenshot

### VAL-TAB-005: Offline state when daemon down
Stopping `werk serve` causes the sigil section to show an "offline,
reconnecting…" banner consistent with the existing offline patterns
in space and field modes.
Tool: agent-browser
Evidence: screenshot

### VAL-TAB-006: Mode toggle preserves SSE connection
Toggling between space → sigil → field → sigil keeps the same single
EventSource open the entire time.
Tool: agent-browser DevTools
Evidence: network log persistence

### VAL-TAB-007: Eyeball review of werk-tab
Orchestrator + user view a screenshot of werk-tab in sigil mode and
confirm the rendered glance preset looks like a coherent compact sigil.
Tool: manual / orchestrator presents to user
Evidence: `validation/m5/eyeball/REVIEW.md`

---

## Area: Combinators (M6)

### VAL-COMB-001: SheetLogic tiles inner sigils
Loading a `SheetLogic` with `inner_logic = "glance"` and a 4-element
Union scope produces an SVG with four sub-sigils tiled in a grid.
Tool: cargo test
Evidence: passing test `combinators::sheet::tests::tiles_four_sub_sigils`

### VAL-COMB-002: Composite Concentric stacks rings
A `CompositeLogic { rule: Concentric }` with two
`(scope, logic)` pairs produces an SVG where the outer pair occupies the
outer ring and the inner pair occupies the inner ring; element count
matches the sum of children.
Tool: cargo test
Evidence: passing test `combinators::composite::tests::concentric_stacks_rings`

### VAL-COMB-003: Recursion-depth limit at 4
Nesting 4 deep is allowed; nesting 5 deep returns
`SigilError::RecursionLimit { depth: 5 }`.
Tool: cargo test
Evidence: passing test `combinators::tests::recursion_limit_at_four`

### VAL-COMB-004: SeedSweep produces a frame sequence
`render_animation(scope, logic, SeedSweep { 0..5, step: 1 },
FrameSequence { dir })` writes 5 SVG files to `dir`, named in seed
order; each is a valid SVG with the correct seed in metadata.
Tool: cargo test
Evidence: passing test `animation::tests::seed_sweep_writes_5_frames`

### VAL-COMB-005: ParamSweep interpolates a numeric stage param
`ParamSweep { stage: layouter, param: "ring_step", from: 60.0, to: 100.0,
frames: 5 }` produces 5 SVGs with `ring_step` values
[60.0, 70.0, 80.0, 90.0, 100.0]; each render is deterministic against
its sweep value.
Tool: cargo test
Evidence: passing test `animation::tests::param_sweep_linear_interp`

### VAL-COMB-006: AnimatedSvg output is rejected
`render_animation(..., AnimatedSvg { .. })` returns
`SigilError::Unsupported { feature: "AnimatedSvg" }`.
Tool: cargo test
Evidence: passing test `animation::tests::animated_svg_unsupported`

### VAL-COMB-007: EpochRange axis is rejected
`render_animation(..., EpochRange { .. }, ..)` returns
`SigilError::Unsupported { feature: "EpochRange" }`.
Tool: cargo test
Evidence: passing test `animation::tests::epoch_range_unsupported`

---

## Area: Hardening (M6)

### VAL-HARD-001: Loud schema parse error
Loading `Logic::from_toml("not a valid pipeline = section")` returns
`SigilError::Construction` with line/col info. No render runs.
Tool: cargo test
Evidence: passing test `error_handling::tests::schema_parse_loud`

### VAL-HARD-002: Loud expression parse error
A preset with `r = { expr = "(((" }` returns
`SigilError::Construction` at logic-construction time, naming the
encoder name and channel name.
Tool: cargo test
Evidence: passing test `error_handling::tests::expr_parse_loud`

### VAL-HARD-003: Loud channel-name mismatch
A preset with `[encoder.channels.bogus]` returns
`SigilError::UnknownChannel { name: "bogus" }` at construction.
Tool: cargo test
Evidence: passing test `error_handling::tests::unknown_channel_loud`

### VAL-HARD-004: Loud IR-shape mismatch
Pairing a `tree`-requiring layouter with an `AttributeGraph` featurizer
returns `SigilError::IrIncompatible { stage, expected, actual }` at
construction.
Tool: cargo test
Evidence: passing test `error_handling::tests::ir_shape_mismatch_loud`

### VAL-HARD-005: Graceful render-time skip with warning
A featurizer that emits a node with `urgency = None` (synthetic) lets
the encoder skip that node and adds a warning. Final SVG metadata
`<warnings count="1">` includes the skip reason.
Tool: cargo test
Evidence: passing test `error_handling::tests::missing_field_graceful`

### VAL-HARD-006: cache retention removes stale entries
`cleanup_cache(retention_days = 7)` removes files in
`~/.werk/sigils/cache/` whose mtime is >7 days old; newer files are
kept; archive (YYYY-MM-DD/) is untouched.
Tool: cargo test (uses tempfile + filetime)
Evidence: passing test `archive::tests::cleanup_cache_removes_stale`

### VAL-HARD-007: Logic file hot-reload triggers invalidation
With the M6 watcher running, modifying
`werk-sigil/presets/contemplative.toml` causes the loaded logic's
`logic_version` to change, and any `/api/sigil/stream` subscriber gets
a `sigil_invalidated` event within 1 second.
Tool: cargo test (long-running with notify mock)
Evidence: passing test `hot_reload::tests::file_change_invalidates`

### VAL-HARD-008: render of 50-tension subtree under 100ms
Performance benchmark: `Engine::render` over a 50-node fixture subtree
with `contemplative` preset completes in <100ms on a modern machine
(measured median of 20 runs).
Tool: cargo test --release (criterion-style or simple Instant::now)
Evidence: passing test `bench::tests::contemplative_50_node_under_100ms`

---

## Cross-Area Flows

### VAL-CROSS-001: CLI render → archive → list via short code
1. Run `werk sigil 2 --save`
2. Run `werk list --kind sigil` (or equivalent if a sigil filter exists)
3. The new sigil appears with a `*N` short code
4. Run `werk show *N`
5. Output includes the file path, logic, seed, scope
Tool: assert_cmd, end-to-end CLI flow
Evidence: passing test `werk-cli/tests/sigil_lifecycle.rs::save_then_show`

### VAL-CROSS-002: Web render same-as CLI render
With identical scope, logic, seed, and werk-state revision, the SVG
returned by `GET /api/sigil` is byte-identical to the SVG produced by
`werk sigil <scope> --logic <name> --seed N`.
Tool: cargo test (cross-binary)
Evidence: passing test `werk-web/tests/parity.rs::web_matches_cli`

### VAL-CROSS-003: werk-tab refresh round-trip
1. Open werk-tab in sigil mode (M5)
2. Mutate a tension via `werk sigil` (or any path that triggers
   `sigil_invalidated`)
3. werk-tab's SVG visibly updates within 2 seconds
Tool: agent-browser + cargo run
Evidence: screenshots before/after, agent-browser run log

### VAL-CROSS-004: Determinism across machines
Given the same `tensions.json` fixture and same werk-core version, the
SVG produced for a fixed `(scope, logic, seed)` is byte-identical
between Zo and any other machine. (Validated locally if possible; flag
as deferred otherwise.)
Tool: manual / cargo test against fixture data
Evidence: documented hash comparison in `validation/m6/cross-machine.md`

### VAL-CROSS-005: Sigil rendering does NOT enter the gesture log
Running `werk sigil 2 --save`, then running `werk log #2`, shows no
new gesture entry triggered by the sigil. The gesture log is unchanged
by sigil rendering.
Tool: assert_cmd + log inspection
Evidence: passing test `werk-cli/tests/sigil_lifecycle.rs::no_gesture_emitted`

### VAL-CROSS-006: All 5 presets are visually distinct
Side-by-side visual inspection of the five preset golden snapshots
shows clearly distinct sigils — different aesthetic register, layouter,
and density. Not just the same shape with different colors.
Tool: manual / orchestrator presents to user
Evidence: `validation/m6/eyeball/distinct-presets.md` with five SVGs
embedded
