# Decisions resolved during prep

These decisions were made during mission preparation. They resolve
ambiguities between `designs/sigil-engine.md` (the architectural authority)
and the current state of werk-core. Workers must follow them; deviations
require orchestrator approval.

---

## D1 — Status taxonomy: 3 + derived

**Tension:** The design lists status values `active`, `held`, `resolved`,
`released`, `frozen` (Part IV). werk-core's `TensionStatus` enum has only
`Active`, `Resolved`, `Released` (`werk-core/src/tension.rs:35-44`). "Held"
is currently a *frontier classification* of an Active tension whose horizon
has not yet started or has gaps (`werk-core/src/frontier.rs:62, 100, 118,
137, 188`). "Frozen" has no analogue.

**Decision:** Do **not** expand `TensionStatus`. The sacred core is
authoritative on werk's structural model.

**How attributes are derived:**
- `status` (categorical, in registry): emits one of `active`, `held`,
  `resolved`, `released`. Computed as:
  - if `tension.status == Resolved` → `"resolved"`
  - if `tension.status == Released` → `"released"`
  - else if active and `Frontier::compute(...).held` for this id → `"held"`
  - else → `"active"`
- `is_held` is a derived bool: `Frontier::compute` membership.
- `is_resolved`, `is_released` are direct enum checks.
- `frozen` is **dropped** from the v1 attribute registry. Presets that
  reference it will get a load-time warning, not an error.

**Where this is enforced:** Featurizer implementations (M1) compute
`status` and `is_held` once per IR build using a scope-bounded
`Frontier::compute(...)`.

**If `frozen` ever lands in werk-core:** add it to the `status` mapping in
the featurizer; the registry treats new categorical values as additive.

---

## D2 — Edge type name translation

**Tension:** Design names edge types `parent_child`, `split_from`,
`merge_into`, `references`. werk-core uses constants
`EDGE_CONTAINS = "contains"`, `EDGE_SPLIT_FROM = "split_from"`,
`EDGE_MERGED_INTO = "merged_into"` (`werk-core/src/edge.rs:23-25`). No
`references` edge type exists.

**Decision:** The featurizer is the translation boundary.

**Mapping:**
| werk-core      | sigil registry |
|----------------|----------------|
| `contains`     | `parent_child` |
| `split_from`   | `split_from`   |
| `merged_into`  | `merge_into`   |
| (none yet)     | `references`   — drop from v1 registry; add when werk-core adds it |

**Where this is enforced:** `AttributeGraph` IR builder in M1. Translation
is one-way (werk-core → sigil); no need for a reverse map in v1.

---

## D3 — `gap_magnitude` is binary

**Tension:** `werk-core/src/temporal.rs:32-34` returns `0.0` if
`desired == actual` else `1.0`. The design implies a continuous quantity
(0..1).

**Decision:** Document as binary in the registry (this file plus
`library/architecture.md`). Do not modify `gap_magnitude` in werk-core
in v1 — the binary semantic is intentional per werk's grammar (gap is
either present or absent; structural dynamics doesn't grade it).

If presets want a continuous proxy, they should use:
- `urgency` (continuous, 0..)
- `staleness` (continuous, 0..1)
- `mutation_count`, `frequency_per_day` (continuous trajectory signals)

---

## D4 — `urgency` clamping

**Tension:** `compute_urgency` can return values >1.0 once the horizon is
past. Design says urgency is `0..1`.

**Decision:** The featurizer clamps `urgency` to `[0.0, 1.0]` *only when
emitted as the registry attribute named `urgency`*. The unclamped value
is exposed as a separate attribute `urgency_raw` for presets that want
the overdue signal.

---

## D5 — `last_pulse_at` = last mutation timestamp

**Tension:** Design names `last_pulse_at` (Part IV). werk-core has no
"pulse" concept.

**Decision:** Map `last_pulse_at` to the latest timestamp in the
`mutations` table for that tension. `Store::get_last_mutation_timestamps`
already exists (`werk-core/tests/frontier_epochs.rs:54`).

---

## D6 — `space` attribute via Selector

**Tension:** Design lists `space` as an identity attribute (Part IV).
`Tension` has no `space` field; spaces are workspace-level.

**Decision:** The Selector receives the active workspace name from
`Ctx`. The Featurizer attaches `space = ctx.workspace_name()` to every
node it emits. This keeps tensions structurally pure while letting
presets render space-aware sigils.

---

## D7 — Expression library: Rhai (expression mode)

**Tension:** Design Part X.3 lists candidates: `evalexpr`, `cel-rust`,
`rhai`, or a small Pratt parser.

**Decision:** **`rhai = "1"`** in expression-only mode
(`Engine::compile_expression`). Rationale:

- `evalexpr` is now AGPL-3.0 (incompatible with werk's MIT/Apache stack).
- `cel-interpreter` lacks built-in `sqrt`/`log`/`abs`/etc. (would need
  ~10 custom functions; minor friction).
- `rhai` has all required built-ins, dual MIT/Apache, expression-mode
  enforces no-statements-no-loops at parse time, excellent error
  messages with line/col, sandboxed via `set_max_operations` /
  `set_max_expr_depths`.

**Trim:** Disable Rhai features `metadata`, `serde`, `decimal`, `f32`,
`internals`. Keep defaults otherwise. See `research/expr-library.md` for
the full evaluation.

---

## D8 — Glyph sourcing: inline SVG paths only

**Tension:** Design Part V mentions three glyph families embedded via
`include_bytes!`. User decision during prep was: no external assets in
v1.

**Decision:** Workers compose SVG path data **inline as Rust constants**
(e.g., `const ALCHEMICAL_SUN: &str = "M ... L ...";`). No
`include_bytes!`, no asset directory. This keeps the mission
self-contained.

**Family sizes for v1:**
- Alchemical: minimum 16 distinct glyphs (target ~50 if budget allows).
- Geomantic: exactly 16 (the geomantic figures are a fixed set of
  binary 4-row dot patterns; deterministic from index).
- Hand-drawn primitives: minimum 12 (target ~30 if budget allows).

**`glyph_index` selection** is deterministic per design
(`(short_code_hash % family_size)`). If a preset references an index
beyond the family size, it wraps modulo and emits a render-time
metadata note.

---

## D9 — `EpochSeries` is per-tension only in v1

**Tension:** Design lists `EpochSeries` as an IR. Cross-tension
time-aligned series requires historical-state queries werk-core does
not yet have.

**Decision:** v1 ships `EpochSeries` as a per-tension IR only, backed by
`Store::get_epochs(tension_id)`. Cross-tension `EpochSeries` is
explicitly deferred (matches design Part IX: `EpochRange` animation axis
deferred for the same reason).

Featurizers that consume `EpochSeries` in v1 only operate on `Scope::Tension`
or `Scope::Subtree { root, depth: 1 }`. Other scope kinds error at
logic-construction time when paired with an `EpochSeries`-consuming
featurizer.

---

## D10 — `notify` (M6 hot-reload) is opt-in

**Tension:** Hot-reload is in M6. `notify = "6"` is a non-trivial dep
(transitively pulls fsevents on macOS, inotify on Linux).

**Decision:** Hot-reload landed behind the workspace-default `hot-reload`
feature flag in `werk-sigil`. Disable it for any test that doesn't
exercise reload behaviour. Feature is **on** by default for the
production binary.

---

## D11 — werk-tab is a Chrome MV3 extension, not a Rust crate

**Tension:** `werk-tab/` lives in the repo but is **not** a Cargo
workspace member. Its `manifest.json` declares it as a Chromium MV3
extension. It uses vanilla JS (no bundler, no framework).

**Decision:** Frontend changes (M5) are pure JS/HTML/CSS. The
`frontend-integrator` worker handles them. Test commands relevant to
Rust crates do **not** apply to `werk-tab`. Manual visual verification
in a browser is the validation surface; agent-browser is the testing
tool.

---

## D12 — SSE invalidation: additive, not bus bridge

**Tension:** werk-web has its own broadcast channel `Sender<SseEvent>`,
not subscribed to werk-core's `EventBus`.

**Decision:** v1 takes the **additive route**: each existing mutation
handler (`create_tension`, `update_desired`, etc., in
`werk-web/src/lib.rs`) gets a second `state.tx.send(SseEvent { kind:
"sigil_invalidated".into() })` next to its current emit. The new
`/api/sigil/stream` handler is a clone of `sse_handler` that filters
its `BroadcastStream` to only `kind == "sigil_invalidated"`.

The richer "bridge to `EventBus`" path (so CLI/MCP/TUI mutations also
trigger sigil refreshes) is acknowledged but **not done in v1**. It is
captured as a follow-up issue at the end of the mission.

---

## D13 — Cache key includes `werk_state_revision`

**Definition:** `werk_state_revision` for a `ResolvedScope` =
`MAX(updated_at)` over all elements in the scope, where `updated_at`
for a tension is `MAX(mutations.timestamp WHERE tension_id = ?)` (no
`updated_at` column exists in `tensions` table).

**Cache key (full):** `hash(scope_canonical, logic_canonical,
logic_version, seed, werk_state_revision)`.

Any state change → key changes → cache miss → re-render.
