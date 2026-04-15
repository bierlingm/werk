# Type Consolidation Audit

Scope: find duplicated / parallel type definitions across the werk workspace
(`werk-core`, `werk-shared`, `werk-cli`, `werk-mcp`, `werk-web`, `werk-tui`,
`werk-app`) and consolidate where the overlap is real and layering permits.

---

## 1. Domain types (core model)

These live in `werk-core` and are the single source of truth. No duplicates
found across crates; every consumer imports from `werk_core`.

| Type                     | Location                                     |
|--------------------------|----------------------------------------------|
| `Tension`                | `werk-core/src/tension.rs:58`                |
| `TensionStatus`          | `werk-core/src/tension.rs:34`                |
| `CoreError`              | `werk-core/src/tension.rs:13`                |
| `Horizon`, `HorizonKind` | `werk-core/src/horizon.rs`                   |
| `HorizonParseError`      | `werk-core/src/horizon.rs:41`                |
| `Edge`                   | `werk-core/src/edge.rs:26`                   |
| `Event`, `EventBuilder`  | `werk-core/src/events.rs:31`                 |
| `Mutation`               | `werk-core/src/mutation.rs`                  |
| `Forest`, `Node`         | `werk-core/src/tree.rs`                      |
| `Store`, `StoreError`    | `werk-core/src/store.rs`                     |
| `Frontier`               | `werk-core/src/frontier.rs`                  |
| `TreeError`              | `werk-core/src/tree.rs:17`                   |
| `Address`                | `werk-core/src/address.rs`                   |
| `ReplayError`            | `werk-core/src/mutation.rs:223`              |

Classification: **single-sourced, no action required**. `Tension` has no
newtype wrappers for `id`/`short_code` — both are plain `String` and
`Option<i32>`. That's a legitimate design choice (short codes are
workspace-scoped, IDs are ULIDs) and introducing newtypes would cascade
through the entire codebase without strengthening any invariant that isn't
already enforced at the constructor level.

---

## 2. Error types

| Type         | Location                              | Role                        |
|--------------|---------------------------------------|-----------------------------|
| `WerkError`  | `werk-shared/src/error.rs:13`         | Surface-level error (CLI, MCP, Web) — wraps `CoreError`, `StoreError`, `TreeError` via `#[from]` |
| `ErrorCode`  | `werk-shared/src/error.rs:72`         | JSON-wire error code enum   |
| `CoreError`  | `werk-core/src/tension.rs:13`         | Domain validation errors    |
| `StoreError` | `werk-core/src/store.rs:96`           | Storage-layer errors        |
| `TreeError`  | `werk-core/src/tree.rs:17`            | Forest-construction errors  |
| `ReplayError`| `werk-core/src/mutation.rs:223`       | Mutation replay errors      |
| `HorizonParseError` | `werk-core/src/horizon.rs:41`  | Horizon string parser       |

Classification: **cleanly layered, no action required**. One `WerkError` in
`werk-shared`, wrapping the four core-specific error kinds. The `werk-cli`
crate re-exports from `werk_shared` (`werk-cli/src/lib.rs:18`). The MCP
server also imports `WerkError` from `werk_shared` (`werk-mcp/src/tools.rs:18`).
The Tauri app previously did not depend on `werk-shared` at all and
converted errors to `String` at the boundary — that's still true for now
but the dep was added as part of this audit so future consolidations have
a path.

---

## 3. Configuration types

All live in `werk-shared/src/config.rs` and are re-exported (`Config`,
`AnalysisThresholds`, `SignalThresholds`). No duplicates.

`werk-tui/src/persistence.rs` defines serializable mirrors of TUI
internal enums (`PersistedCursorTarget`, `PersistedOrientation`,
`PersistedZoom`, `PersistedTimeBand`) via `From` impls. These are
**intentional anti-corruption boundary** between the TUI runtime types
and the on-disk state file: they let the internal enums evolve without
breaking the saved-session schema. **Leave as-is.**

---

## 4. DTOs for CLI `--json`, REST API, Tauri IPC  **(consolidation done)**

This is where the real duplication lived.

### Before

| DTO                     | `werk-web/src/lib.rs`          | `werk-app/src-tauri/src/main.rs` | `werk-cli/src/commands/tree.rs` |
|-------------------------|--------------------------------|----------------------------------|---------------------------------|
| `TensionJson`           | L357–392 (10 fields + `from_tension`) | L247–279 (identical 10 fields + `from_tension`) | L18–32 (10 fields + 4 tree-specific signal flags) |
| `SummaryJson`           | L415–421                       | L281–287 (identical)             | L41–47 (same fields, different field order)     |
| `TreeResponse`          | L408–413 (`tensions`, `roots`, `summary`) | L289–293 (`tensions`, `summary`) | N/A |
| `CreateTensionRequest` / `CreateTensionArgs` | L423–429 | L322–328 (identical) | N/A |
| `UpdateFieldRequest`    | L431–434                       | N/A                              | N/A |
| `ApiError`              | L436–439                       | N/A                              | N/A |

`werk-web`'s `TensionJson` and `werk-app`'s `TensionJson` had **byte-for-byte
identical** fields and `from_tension` constructors (web used
`chrono::Utc::now()` inline, app the same). The `CreateTension*` structs had
identical fields too — they were just named differently.

### After

New module `werk-shared/src/dto.rs` (file:
`werk-shared/src/dto.rs`):

- `TensionDto`            — formerly `TensionJson` (web + app), with `from_tension()` constructor
- `SummaryDto`            — formerly `SummaryJson` (web + app), with `from_tensions()` constructor
- `CreateTensionRequest`  — unified
- `UpdateFieldRequest`    — moved from web
- `ApiError`              — moved from web, `ApiError::new()` constructor

Re-exported from `werk-shared/src/lib.rs` so callers write
`werk_shared::dto::TensionDto`.

Call-site changes:
- `werk-web/src/lib.rs`: removed duplicate definitions, imported shared DTOs, replaced inline `SummaryJson { active, resolved, ... }` with `SummaryDto::from_tensions(&all)`.
- `werk-app/src-tauri/src/main.rs`: added `werk-shared` dep in its Cargo.toml, removed duplicate definitions, replaced the same way. `TreeResponse` is retained as a tauri-local envelope because the app's wire format does not include `roots` (tree is reconstructed on the frontend).

### What was NOT consolidated (and why)

**`TreeResponse` (web vs. app)** — they look similar but the **web** version includes a nested `roots: Vec<TreeNodeJson>` field (server-side tree construction) while the **app** version only ships the flat tensions list and lets the frontend construct the tree client-side. These are different wire contracts and merging them would either bloat the app response or break the web contract. Left split intentionally (classification: **should-stay-split**).

**`TreeJson` / `TensionJson` in `werk-cli/src/commands/tree.rs`** — the CLI tree command's `TensionJson` is a **superset** of `TensionDto` (adds `containment_violation`, `sequencing_pressure`, `closure_resolved`, `closure_total`; drops `position`) and its `TreeSummary` has the same *field set* as `SummaryDto` but a different serde key order (`total, active, resolved, released` vs. `active, resolved, released, total`). Changing either would change the CLI's `--json` wire format, which agents pattern-match on (per `CLAUDE.md`). Left split intentionally (classification: **should-stay-split-for-now** — could be re-evaluated under a coordinated CLI-JSON-schema revision).

**`ClosureJson` in `werk-web`** — only one instance, not a duplicate.

**`SseEvent` / `WorkspaceJson` in `werk-web`** — web-only wire types. `WorkspaceJson` is a serializable mirror of `werk_shared::daemon_workspaces::WorkspaceEntry`, which isn't itself `Serialize`. This is a thin surface adapter rather than a duplicate. Could be removed by deriving `Serialize` on `WorkspaceEntry` and serializing `path` via `#[serde(serialize_with = …)]` or changing the field to `String`, but the payload shape would shift (PathBuf vs. display-string). Classification: **unclear — revisit if the daemon/web-workspace surface grows**.

---

## 5. Store-actor pattern (web + app)

Both `werk-web/src/lib.rs:40–188` and `werk-app/src-tauri/src/main.rs:19–242`
implement the "store on a dedicated OS thread, talk via channels" pattern,
because `werk_core::Store` is `!Send` (fsqlite uses `Rc`). Parallel
definitions:

| Type          | Web                                  | App                                    |
|---------------|--------------------------------------|----------------------------------------|
| `StoreCmd`    | 8 variants, `tokio::sync::oneshot` reply channels, includes `ComputeVitals` / `ComputeAttention` | 7 variants, `std::sync::mpsc::SyncSender` reply channels, no Vitals/Attention |
| `StoreHandle` | `async` wrappers returning `Future`  | blocking `fn` wrappers                 |
| `StoreResult` | `type StoreResult<T> = Result<T, String>` | *same* (also named `StoreResult`) |

Classification: **should-stay-split**. Consolidating would require:
1. Generalising the reply channel over sync/async (doable with a small `Responder` trait, but it buys little)
2. Reconciling the variant sets (app would need no-op vitals/attention commands or a feature flag)
3. Generalising the wrappers as both sync and async flavours

The actor boundary lives at the Tauri/axum layer for a reason: the web
server is async throughout (tokio broadcast for SSE) and the Tauri invoke
handler is synchronous. Merging them means one layer grows an adapter
shim for the other; the duplication is small and the abstraction is
questionable. **Left as-is, noted for future redesign if a third surface
(e.g. a gRPC daemon) appears.**

`StoreResult<T>` is a 1-line alias; reducing it to a shared alias in
`werk-shared` would save literally one line per crate and would couple the
app crate to a trivial helper. Not worth it.

---

## 6. Display / format wrappers

`werk-shared` owns display helpers (`display_id`, `display_id_named`,
`format_timestamp`, `relative_time`, `truncate` — `util.rs`) and
palette / glyph tables (`cli_display/`, `palette.rs`). No duplicate
definitions in other crates; everyone imports from `werk_shared`.

The CLI has `werk-cli/src/palette.rs` which is **presentation code for
palettes**, not a parallel type — it re-exports `werk_shared::palette::{Palette, PaletteChoice}` and provides stdin-reading interaction logic.

CLI-local serialize helpers (`werk-cli/src/serialize.rs`:
`HorizonRangeJson`, `TensionInfo`, `MutationInfo`) are show-command-specific
richer DTOs that don't match the Web/Tauri shape. They include
`urgency: Option<f64>`, `staleness_ratio`, `horizon_range` — fields the
minimal cross-surface DTO doesn't carry. Classification: **CLI-local rich
DTO, intentionally richer than shared wire format.**

---

## Summary

| Category            | Classification           | Action          |
|---------------------|--------------------------|-----------------|
| Domain types        | single-sourced in `werk-core` | none            |
| Error types         | single-sourced in `werk-shared` | none         |
| Config types        | single-sourced in `werk-shared` | none         |
| TUI persisted enums | intentional wire-format boundary | none      |
| **Web/Tauri DTOs**  | **previously duplicated**   | **consolidated into `werk-shared::dto`** |
| CLI `TensionJson` (tree)  | JSON schema superset    | left split — changing breaks CLI wire format |
| `TreeResponse` (web/app) | diverged contracts     | left split     |
| `StoreCmd`/`StoreHandle` | parallel sync vs. async patterns | left split |
| Display helpers     | single-sourced in `werk-shared` | none         |

**Lines removed across `werk-web` + `werk-app`:** ~95 (duplicate struct
declarations + `from_tension` bodies + manual summary builders).
**Lines added in `werk-shared/src/dto.rs`:** ~115 (new module +
constructors + docs).

Net: slight increase in LOC because of shared module docs; but one source
of truth for the wire types that flow over REST, SSE, and Tauri IPC
instead of three. Adding a field (e.g. a new signal-surfacing field
like `overdue`) now requires one edit in `werk-shared/dto.rs` instead
of three parallel edits.
