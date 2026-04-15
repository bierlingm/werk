# AUDIT-legacy.md

Inventory of deprecated, legacy, and fallback code paths, classified for action.

Branch: `worktree-agent-a904af50`
Baseline: `cargo check --workspace` passes.

## Summary

| Target from CLAUDE.md | Status in code | Classification |
|---|---|---|
| Old CLI commands (`survey`, `diff`, `ground`, `insights`, `trajectory`, `health`, `context`) | **Already removed** from CLI and MCP. CLAUDE.md is stale. | Doc fix only |
| `parent_id` column "maintained for backward compat" while edges are source of truth | `parent_id` still primary reading path; `edges` backs only split/merge provenance and is populated on write for `contains`. Two edge-based helpers are defined and unused. | Narrow safely; column removal flagged (breaking, migration) |
| "Old advisory file lock removed" | Confirmed. No residue in any Rust file. | No action |

Only a small amount of unused/misleading code was found. The big CLI consolidation is actually already done — CLAUDE.md overstates what's left.

---

## 1. CLAUDE.md claims do not match code (safe doc fix)

**File:** `CLAUDE.md:25`
> Old commands (`survey`, `diff`, `ground`, `insights`, `trajectory`, `health`, `context`) still exist but are being consolidated.

Evidence: `werk-cli/src/commands/` contains no `survey.rs`, `diff.rs`, `ground.rs`, `insights.rs`, `trajectory.rs`, `health.rs`, `context.rs`. `Commands` enum in `werk-cli/src/commands/mod.rs` enumerates only the 5 reading commands (`show`, `list`, `tree`, `stats`, `log`) plus gestures. Nothing uses `#[command(hide = true)]`. MCP `WerkServer` in `werk-mcp/src/tools.rs` likewise exposes no tool by those names (search/show/tree/list/stats only on the read side).

"survey" persists as a TUI-only view name (`werk-tui/src/survey.rs`, `survey_tree.rs`) — that is a live TUI orientation, not a deprecated CLI command.
"trajectory" and "health" persist as `stats` **section flags** (`werk stats --trajectory --health`) and as internal domain types in `werk-core/src/projection.rs` (`Trajectory` enum) — legit, not legacy.
"ground" persists only in stale doc comments (`werk-core/src/graph.rs:32,61`) — harmless.

**Classification:** safe-to-remove (the stale sentence). Doc-only edit applied in this pass.

---

## 2. Unused `HookRunner` convenience wrappers (safe-to-remove)

**File:** `werk-shared/src/hooks.rs:699-724`

```rust
// === Legacy convenience methods (used by existing CLI pre-hook code) ===
pub fn pre_mutation(&self, event: &HookEvent) -> bool { ... }  // USED
pub fn post_mutation(&self, event: &HookEvent) { ... }          // UNUSED
pub fn post_resolve(&self, event: &HookEvent) { ... }           // UNUSED
pub fn post_release(&self, event: &HookEvent) { ... }           // UNUSED
pub fn post_create(&self, event: &HookEvent) { ... }            // UNUSED
```

Grep confirms `pre_mutation` has 15 callers across `werk-cli` and `werk-mcp`. The four `post_*` wrappers have zero callers outside `werk-shared/src/hooks.rs` itself. Post-hook firing is driven by the `HookBridge` subscription in `HookRunner::new`, which calls `run_post_hooks` directly — it does not go through these wrappers.

**Classification:** safe-to-remove. Applied in this pass.

---

## 3. Misleading "Legacy factory" doc comments (safe doc fix)

**File:** `werk-shared/src/hooks.rs:270, 294, 316`

```rust
/// Legacy factory: build a mutation HookEvent manually (for pre-hooks at command level).
pub fn mutation(...) -> Self { ... }
/// Legacy factory: build a status change HookEvent manually (for pre-hooks).
pub fn status_change(...) -> Self { ... }
/// Legacy factory: build a create HookEvent manually (for pre-hooks).
pub fn create(...) -> Self { ... }
```

These are not legacy — 20 callers use them across the CLI and MCP to assemble pre-hook payloads before the mutation runs. The "Legacy" label predates the EventBus→HookBridge split; it was meant to distinguish them from `HookEvent::from_event(&Event)` (the bus-driven path), not to mark them for removal. Label is actively misleading.

**Classification:** safe-to-remove (the label). Edit applied.

---

## 4. Unused edge-based parent helpers (safe-to-remove)

**File:** `werk-core/src/store.rs:3365-3399`

```rust
/// Get the parent ID for a tension (from contains edges).
/// This replaces direct parent_id column reads.
pub fn get_parent_id_from_edges(&self, tension_id: &str) -> Result<Option<String>, StoreError> { ... }

/// Get children IDs for a tension (from contains edges).
/// This replaces direct parent_id = ? queries.
pub fn get_children_ids_from_edges(&self, parent_id: &str) -> Result<Vec<String>, StoreError> { ... }
```

Grep across the workspace: no callers. The doc comment claims they "replace direct parent_id column reads", but every producer and consumer in `store.rs` still reads `parent_id`. These two methods are a false start toward the migration described in CLAUDE.md and are currently dead code.

**Classification:** safe-to-remove. Applied in this pass. Re-introduce when/if the `parent_id` → `edges` reader migration is actually undertaken.

Note: `store.rs` **does** write `contains` edges alongside `parent_id` (see `create_tension_full`, `update_parent`, `delete_tension`, and the one-time population at L712). So the data is there when a later pass wants to flip the reader over.

---

## 5. parent_id column itself (FLAGGED — breaking, migration required)

`parent_id` is still:
- The column read by every `get_tension*` and `list_tensions` query (`werk-core/src/store.rs` L1082, L1336, L1346, L1357, L1967, L2296).
- The primary in-memory field on `Tension` (`werk-core/src/tension.rs`).
- Consumed by TUI, CLI, MCP, and web-UI read paths (`werk-tui/src/app.rs`, `werk-cli/src/commands/move_cmd.rs`, `werk-mcp/src/tools.rs`, `werk-app/src-tauri/src/main.rs`).

Per CLAUDE.md the long-term direction is "edges are the source of truth". That is a meaningful refactor: every read path would need to fetch `contains` edges, reconstruct `parent_id`, and keep the column in sync (or drop it). Dropping the SQLite column requires a migration.

**Classification:** user-visible-breaking / needs-migration-plan. **NOT implemented.**

Proposed migration plan:
1. Add `Tension::parent_id_from_edges(store)` helper. Internal only.
2. For a release cycle, compute `parent_id` from edges on load and assert it equals the column value (sanity check). Fail loudly on divergence.
3. Flip all read paths to the edge-based helper while continuing to write both.
4. Next release: stop writing `parent_id` column. Add a migration that drops it. Bump schema version.
5. Remove the column from `Tension` struct, update serializers, update `tension.parent_id` tests.

Each step is its own PR. I'd suggest keeping CLAUDE.md's "maintained for backward compat" wording honest by adding "reader has not yet been migrated" until step 3 ships.

---

## 6. `migrate_legacy_db` sd.db→werk.db rename (FLAGGED — user-visible timing decision)

**File:** `werk-core/src/store.rs:156-170`

Renames an old `sd.db` (from before the werk rename) to `werk.db` on open. If no user still has a pre-rename workspace on disk, this is safe to remove; if any user's workspace predates the rename, removing it loses their data silently.

**Classification:** needs-migration-plan. Not implemented. Low effort to remove after a release note. Keep until you're confident no `.werk/sd.db` remains in the wild.

---

## 7. Hook name back-compat mapping (FLAGGED — user-visible contract)

**File:** `werk-shared/src/hooks.rs:565-578`

```rust
fn legacy_hook_names(prefix: &str, event_name: &str) -> Vec<String> { ... }
// Maps user configs with "post_resolve" / "post_release" / "post_create"
// onto current event names "tension_resolved" / "tension_released" / "tension_created".
```

This is a deliberate back-compat bridge for users whose `config.toml` still uses the old hook names. Removal would silently disable user hooks.

**Classification:** keep-as-deliberate-alias. No change. Optionally: emit a one-time warning on load when `post_resolve` etc. match, and plan removal after N releases.

---

## 8. Old advisory file lock

CLAUDE.md: "The old advisory file lock has been removed."
Grep (`flock`, `advisory.*lock`, `file.*lock`, `lock.*file` — case-insensitive, Rust only): no matches.

**Classification:** confirmed clean. No action.

---

## What was NOT found

- No `#[deprecated]` attributes anywhere in the workspace.
- No `// TODO: remove when ...` / `TODO: clean` / `TODO: delete` comments.
- No `FIXME`, `XXX:`, or `HACK:` comments.
- No `_v2` / `_new` / `_old` / `OldImpl` / `NewImpl` type or function names (only unrelated incidental uses like `old_value` locals in diff code, and an `old_to_new` HashMap in `examples/import_json.rs`).
- No feature-flag / runtime-branch guards for legacy vs new implementations (`if use_new_impl`, `cfg(legacy)`, etc.).
- Web API routes (`werk-web/src/lib.rs:463-476`): clean — no deprecated endpoints.

---

## Commits on this branch

1. Remove four unused `HookRunner` wrappers (`post_mutation`, `post_resolve`, `post_release`, `post_create`), remove two unused edge-based store helpers (`get_parent_id_from_edges`, `get_children_ids_from_edges`), fix misleading "Legacy factory" doc comments, update CLAUDE.md reading-surface section to reflect actual state.

Net: cleaner hook API, cleaner store API, truthful CLAUDE.md. `cargo check --workspace` and `cargo test --workspace` pass.

## Flagged, not implemented

- `parent_id` reader migration (item 5) — breaking, migration required.
- `sd.db` → `werk.db` rename shim removal (item 6) — timing decision, low effort once decided.
- `legacy_hook_names` back-compat mapping (item 7) — user-contract, keep-as-alias unless you want to ship a removal with release notes.
