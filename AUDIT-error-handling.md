# Error handling audit

Branch: `worktree-agent-a0929327`
Scope: Rust workspace (`werk-core`, `werk-shared`, `werk-cli`, `werk-mcp`, `werk-tui`, `werk-web`, `werk-app`).

Goal: remove defensive error handling that serves no purpose (the Rust equivalents of try/catch that eat errors), while preserving graceful handling at real system boundaries (IO, DB, subprocess, user input, UI surfaces).

## Method

Scanned for these patterns across the workspace via codedb + ripgrep:

- `unwrap_or_default()` / `unwrap_or(...)` — 116 + ~200 hits
- `if let Ok(...) = ...` with silent-discard arms — 63 hits
- `.ok();` (throwing away the error) — 26 hits
- `.expect(...)` — 137 hits (overwhelmingly in test files; 3 in production code)
- `unreachable!()` — 2 hits in production code
- `anyhow::Context` chains — 0 hits (crate not used)
- Functions returning `Result` with infallible bodies — no systematic offenders found; most `Result`-returning functions genuinely propagate `StoreError`, `CoreError`, `WerkError`, or `serde_json::Error`.
- Overly-broad `match err { _ => ... }` — 0 hits in production code.

The bulk of the workspace follows a deliberate, consistent pattern:

- **Store boundaries** (`werk-core/src/store.rs`) raise `StoreError`/`CoreError` for real failure modes (SQLite transaction failure, schema violations, commit contention).
- **CLI surface** (`werk-cli/src/commands/*.rs`) maps those to `WerkError` and exits with structured JSON on `--json`.
- **Hooks** (`werk-shared/src/hooks.rs`) deliberately fail-open (`pre_mutation()` returning `true` on internal error is the documented fail-open policy — a broken user hook must not prevent mutations).
- **Persistence** for TUI feedback/workspace state is intentionally fail-soft — a corrupt state file shouldn't kill the session.
- **Config loading** uses a chain `Config::load(...).unwrap_or_default()` because a missing/broken config file should degrade to defaults, not fatal-error the CLI.

This pattern is consistent with `CLAUDE.md` (MCP/CLI/Web surfaces: "user-facing errors must be graceful").

---

## Findings

### Classification key

- **remove**: defensive handling that papers over a real bug or is equivalent to `!`.
- **keep-with-reason**: boundary or UX-graceful handling that is intentional; flagged so future readers see the reasoning.
- **clarify**: handling that is correct but under-documented; small improvement warranted (comment, `.expect(msg)` with invariant, etc.).
- **needs-deeper-refactor**: would be nice to tighten but requires a signature change affecting many callers.

### High-signal findings

#### 1. `werk-cli/src/commands/show.rs:551` — bare `unreachable!()` (clarify)

```rust
HorizonDriftType::Stable => unreachable!(),
```

The enclosing block is guarded at L189: `horizon_drift` is only `Some(_)` when `drift_type != Stable`. The `unreachable!()` is technically correct but bare — if the invariant is ever broken (e.g. `drift_type` field added or `detect_horizon_drift` refactored), the crash is unhelpful.

**Action**: upgraded to an annotated `unreachable!` with an explanatory message pointing to the guard. Applied in commit on this branch.

Confidence: high. Pure documentation/safety improvement, no behavior change.

#### 2. `werk-cli/src/commands/note.rs:67` — `unreachable!("arg2 without arg1")` (keep-with-reason)

```rust
(None, Some(_)) => unreachable!("arg2 without arg1"), // ubs:ignore positional args guarantee this
```

Already has a clear invariant message. Clap's positional arg parsing guarantees arg2 can't appear without arg1. Keep as-is.

#### 3. `werk-shared/src/hooks.rs:703` — `pre_mutation` fail-open (keep-with-reason)

```rust
pub fn pre_mutation(&self, event: &HookEvent) -> bool {
    self.run_pre_hook("mutation", event).unwrap_or(true)
}
```

Fail-open is intentional: a broken user-provided hook command must not prevent a mutation. This is the documented policy. Do not change.

#### 4. `werk-shared/src/hooks.rs:686-691, 696` — `self.log.lock().ok()` patterns (keep-with-reason)

```rust
if let Ok(mut log) = self.log.lock() { ... }
self.log.lock().map(|l| l.clone()).unwrap_or_default()
```

`Mutex::lock` only returns `Err` on poisoning (another thread panicked while holding the lock). Poisoned hook log should not cascade into a hard failure in user code — silently dropping a log entry is acceptable. Keep.

#### 5. `werk-tui/src/persistence.rs:191, 203, 214, 226` — silent discard on workspace state save/load (keep-with-reason)

```rust
if let Ok(data) = serde_json::to_vec(state) { registry.set(...) }
serde_json::from_slice(&entry.data).ok()
```

TUI workspace state and palette feedback are ephemeral assists. A corrupt file should degrade cleanly to "no stored state" rather than crash the session. Keep.

#### 6. `werk-core/src/store.rs:334-358 (backup_db)` — chain of `let _ =` on filesystem ops (keep-with-reason)

Best-effort housekeeping during `Store::init`. If backups can't be written/pruned (disk full, permissions), we still want the store to open. Keep.

#### 7. `werk-core/src/store.rs:1641,1644 (update_parent)` — `let _ = remove_edge/create_edge` (needs-deeper-refactor)

```rust
if let Some(ref old_pid) = old_parent {
    let _ = self.remove_edge(old_pid, id, crate::edge::EDGE_CONTAINS);
}
if let Some(ref new_pid) = new_parent {
    let _ = self.create_edge(new_pid, id, crate::edge::EDGE_CONTAINS);
}
```

Edge mutations happen AFTER the tension update is already committed. If they fail, we have a partial state: the tension's `parent_id` changed but the edge table is stale. Per `CLAUDE.md`, "edges are the source of truth for new relationships", so this is a real inconsistency risk.

**Why not touched in this pass**: fixing this properly means moving edge mutations inside the same transaction as the tension update, which is a non-trivial refactor. Flagged for a dedicated tension.

#### 8. `werk-core/src/store.rs:827 (create_tension_full)` — `.ok()` on parent-snapshot serde (keep-with-reason)

```rust
tension.parent_snapshot_json = serde_json::to_string(&snapshot).ok();
```

`parent_snapshot_json: Option<String>` is inherently nullable (no parent → `None`). Using `.ok()` here fuses two "snapshot absent" reasons — no parent vs. serialization failure — but serialization of a JSON `Value` we just constructed is infallible in practice. `.ok()` is fine.

#### 9. `werk-cli/src/commands/mod.rs:58-96` — nested config loading with `unwrap_or_default` (keep-with-reason)

```rust
pub fn load_signal_thresholds() -> SignalThresholds {
    Workspace::discover().ok()
        .and_then(|ws| Config::load(&ws).ok())
        .map(|c| SignalThresholds::load(&c))
        .unwrap_or_default()
}
```

Intentional: CLI flag defaults must be resolvable before a command knows where it is. Falling back to hardcoded defaults if workspace/config can't be read is correct.

#### 10. `werk-shared/src/config.rs:362-379, 416-431` — `.and_then(|v| v.parse().ok()).unwrap_or(default)` (keep-with-reason)

Every numeric config key has this shape. If a user types `signals.approaching.days = "sometime"`, we silently fall back to the default rather than crashing. The config-registry surface validates on write; this is the read-side belt-and-braces. Keep.

#### 11. `werk-core/src/frontier.rs:92, 144, 152` — `.unwrap_or_default()` / `.unwrap_or(false)` (keep-with-reason)

```rust
let children = forest.children(tension_id).unwrap_or_default();  // Option<Vec<...>>
positioned.sort_by_key(|t| t.position.unwrap_or(i32::MAX));
let is_overdue = t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false);
```

All three model real business logic:
- Forest with an unknown tension id → treat as leaf (no children).
- Unpositioned tension → sort after any positioned one.
- Tension with no horizon → not overdue.

These are correct semantic choices. Keep.

#### 12. `werk-shared/src/prefix.rs:43` — `if let Ok(code) = prefix.parse::<i32>()` (keep-with-reason)

User input parse — `Err` means "not a number, try ULID matching". Intentional fall-through, not silent discard.

#### 13. `werk-app/src-tauri/src/main.rs:440,455` — `.expect("...")` at Tauri startup (keep-with-reason)

Startup failure should panic — there's no useful fallback and the user will see the Tauri error dialog. Keep.

#### 14. `werk-tui/src/app.rs:348` — `Engine::new_in_memory().expect(...)` (keep-with-reason)

```rust
let engine = Engine::new_in_memory().expect("failed to create in-memory engine"); // ubs:ignore in-memory SQLite cannot fail
```

Already has a `ubs:ignore` annotation. In-memory SQLite cannot actually fail to open. Keep.

#### 15. Test files — `.expect(...)` throughout (keep)

All `.expect(...)` calls inside `werk-cli/tests/*.rs`, `werk-core/src/search.rs` test module, `werk-core/examples/import_json.rs` are test scaffolding — `.expect()` with a descriptive message is idiomatic for tests, and a panicking test fail is the desired behavior. Keep.

#### 16. `werk-cli/src/commands/hooks.rs` — many `Config::load(&workspace).unwrap_or_default()` (keep-with-reason)

Every `hooks` subcommand needs a config. Missing config → treat as empty config → user can add hooks. The UX requires this. Keep.

#### 17. `werk-core/src/store.rs:2122` — `parse_mutation_rows: ...unwrap_or(None)` (clarify — low priority)

```rust
.unwrap_or(None),
```

This is in the row→Mutation parser; `unwrap_or(None)` on a `Result<Option<T>, _>` collapses a parse error to "field absent". In a row-parser fed from our own schema this is defensible (only we write these rows). Flagged for a future pass that could upgrade this to a hard error, since a parse failure here means our schema and our writer disagree — a real bug.

---

## Overall summary

- **Removals**: none. No pattern I found on this pass is unambiguously redundant and safe to delete without a callsite-level behavior change.
- **Clarifications**: 1 applied (`unreachable!()` → `unreachable!("...")` in show.rs) with explanatory comment.
- **Deeper refactors flagged**: 1 (finding #7, transactional edge updates in `update_parent`).

The defensive patterns in this codebase are almost entirely deliberate and correspond to real boundaries:
- Config (fail-soft with defaults),
- Persistence (fail-soft with "no stored state"),
- Hooks (fail-open per policy),
- Store boundaries (propagate real errors as `StoreError`/`CoreError`),
- User input parse (`Err` arms are meaningful fall-throughs).

A second pass with behavior-preserving signature tightening (e.g., making `Forest::children` return `&[Node]` instead of `Option<Vec<&Node>>` to eliminate `.unwrap_or_default()` at call sites) would be valuable but is out of scope for a focused error-handling sweep.

---

## Commits on this branch

1. `Clarify unreachable!() in show.rs with invariant message` — upgrades the bare `unreachable!()` at `werk-cli/src/commands/show.rs:551` to an annotated variant that documents the L189 guard.

## Verification

- `cargo check --workspace` — PASS
- `cargo test --workspace` — see branch CI log
