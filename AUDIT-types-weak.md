# Weak-Type Audit

Survey of weak-type sites in the `werk` Rust workspace, with proposed strong-type replacements. Wire format (SQLite schema, JSON on MCP/CLI `--json`, flushed `tensions.json`) is preserved throughout: all replacements either (a) stay internal, or (b) use `#[serde(from/into/transparent)]` to keep the on-disk/on-wire shape identical.

Branch: `worktree-agent-aeecce36`.

## Method

1. Surveyed for `Box<dyn Any>`, `serde_json::Value`, `HashMap<String, String>`, `Vec<String>`, bare `String` for domain IDs/enums, `i64/i32` for domain counts, `bool` flags.
2. Classified each site by whether strengthening is HIGH-confidence (narrow blast radius, wire-format safe) or LOW-confidence (needs wider trace through serde/SQLite/MCP/CLI consumers).
3. Implemented the HIGH set in this branch. LOW is flagged for follow-up.

## HIGH-confidence — implemented

### 1. `Mutation.field: String` → strongly-typed `MutationFieldKind` enum (as a view)

**File**: `werk-core/src/mutation.rs`
**Problem**: The `field` string encodes one of ~20 known mutation kinds (`"desired"`, `"actual"`, `"status"`, `"created"`, `"parent_id"`, `"horizon"`, `"position"`, `"note"`, `"note_retracted"`, `"deleted"`, `"split"`, `"merge"`, `"release_reason"`, `"reopen_reason"`, `"recurrence"`, `"recurrence_cleared"`, `"snoozed_until"`, `"snooze_cleared"`, `"cleared"`, `"none"`). Readers compare strings across ~40 sites with no compile-time check (see for example `m.field() == "status"` across `werk-core/src/store.rs`, `werk-mcp/src/tools.rs`, `werk-cli/src/commands/*.rs`). Silent typos produce dead code paths. `apply_mutation` in `mutation.rs` additionally returns `UnknownField` for valid but non-state-affecting fields (`"split"`, `"merge"`, `"release_reason"`, ...) — incomplete and brittle.

**Why HIGH**:
- The `Mutation` struct's `field: String` field and wire format (SQLite `mutations.field` TEXT column; JSON `"field":"desired"`) are preserved byte-for-byte.
- Strong enum added as a *view*: `Mutation::field_kind(&self) -> MutationFieldKind` and `MutationFieldKind::as_str()`. String storage is unchanged.
- `apply_mutation` is rewritten against the enum, closing the silent-rejection gap: known-but-non-state-affecting fields (`split`, `merge`, `release_reason`, `reopen_reason`, `recurrence`, `recurrence_cleared`, `snoozed_until`, `snooze_cleared`, `cleared`) are now explicitly noops during replay instead of returning `UnknownField`. The existing `UnknownField` error variant is preserved for truly unknown fields via `MutationFieldKind::Other(String)`.
- No public signature change to `Mutation::new` / `.field()`.

**Confidence**: HIGH.

### 2. TUI `UndoStack`: `Vec<String>` undo + redo stacks — documented intent via newtype `GestureId`

**File**: `werk-tui/src/undo.rs`, `werk-tui/src/app.rs`
**Problem**: Two `Vec<String>` fields, both named meaningfully but both `String`. The stack semantics mean "undo = gesture IDs that produced state"; "redo = IDs of undo gestures that undid those". They are distinct kinds but share a type — easy to push onto wrong stack.

**Fix**: Introduced a `GestureId(String)` newtype inside `werk-tui/src/undo.rs` (local, not part of core API). The stacks now hold `Vec<GestureId>`. Public methods take/return `String` at the boundary (no change to callers). Internal operations preserve the distinction.

**Confidence**: HIGH (single-file change).

## LOW-confidence — flagged, NOT implemented

Changing these requires tracing serde/SQLite/MCP/CLI/--json consumers; doing this confidently would exceed the scope of a single audit pass without a wider test campaign. Flagged for follow-up.

### L1. `Tension.id: String` (ULID) → `TensionId(Ulid)` newtype
Files: `werk-core/src/tension.rs:60`, threaded through **every** crate (50+ files). Wire format: JSON `"id":"01JX..."` and SQLite TEXT column. Serde-transparent newtype is the right shape, but every `&str` / `String` parameter in 150+ signatures would want to become `&TensionId`. Too broad for a single pass.

### L2. `Tension.short_code: Option<i32>` → `ShortCode(i32)` newtype
Files: `werk-core/src/tension.rs:83`, 52 files contain `Option<i32>` that may or may not be short codes. Same blast radius concern. Also: `Address::Tension(i32)` and `Address::Epoch { tension: i32, ... }` in `werk-core/src/address.rs` use the same `i32`, so the newtype should be introduced in `address.rs` first and threaded. Non-trivial because `&str` APIs often accept either an `id` (ULID) or a short-code string (`#42`); separating the two at the type level would ripple.

### L3. `Tension.position: Option<i32>` → `SiblingPosition(i32)` newtype
Files: `werk-core/src/tension.rs:74`. Same pattern as short_code. Unit confusion: a bare `i32` can mean position, short_code, or ordinal — a newtype per concept would prevent a class of bugs but requires global rename discipline.

### L4. `Mutation.tension_id: String` and `Mutation.gesture_id: Option<String>`
Depend on L1. Same wire-format concern. When L1 lands, these slot in naturally.

### L5. `HookEntry.commands: Vec<String>` is fine — it really is a list of shell command strings.
No action.

### L6. `Address::Gesture(String)`, `Address::Session(String)`
`werk-core/src/address.rs:28, 30`. These are opaque ID strings. A `GestureId` / `SessionId` newtype in `werk-core` would be the right home (subsuming the TUI-local one from HIGH #2). Deferred because `Gesture` IDs are ULIDs and `Session` IDs are date-seq (`20260328-1`) — two different internal shapes that happen to both serialize as a string. A sum-type or two newtypes would both work. Decision deferred pending L4.

### L7. `ReplayError::InvalidStatus(String)` and the replay parser
`werk-core/src/mutation.rs:184-189`: matches `"Active" | "Resolved" | "Released"` against strings to reconstruct `TensionStatus`. `TensionStatus` already has serde `Serialize/Deserialize`; `serde_json::from_str(&format!(r#""{}""#, s))` or `str::parse` would let us reuse the enum directly. Narrow, but the wire format for the mutation `new_value` string must be "Active"/"Resolved"/"Released" exactly — so the parsing code is fine where it is, just could delegate. Flagged but cosmetic.

### L8. `HookEvent.field: Option<String>`
`werk-shared/src/hooks.rs:40`. Takes values like `"actual"`, `"desired"`, `"status"`, `"parent_id"`, `"horizon"`, `"urgency"`, `"horizon_drift"`, `"note"`, `"gesture"`. Overlaps partially with `MutationFieldKind` but adds non-mutation concepts (`urgency`, `horizon_drift`, `gesture`). Could be `HookField` enum; deferred because `HookFilter::parse` accepts arbitrary strings from TOML config.

### L9. `HookFilter::Status(String)` should probably be `HookFilter::Status(TensionStatus)`
`werk-shared/src/hooks.rs`. The `Status` variant matches case-insensitively against `HookEvent.new_value` strings. Could delegate to the enum. Small, but crosses the config-parsing boundary — deferred.

### L10. `parent_snapshot_json: Option<String>` — JSON-in-string
`werk-core/src/tension.rs:81`. Holds a JSON blob describing the parent's descended view at child creation time. A structured type (`ParentSnapshot { children: Vec<ChildSnapshot>, ... }`) with serde + `#[serde(with = "json_string")]` would strengthen this without changing storage. Deferred — semantics of the blob vary by writer (CLI epoch.rs, CLI desire.rs, core/store.rs each produce slightly different shapes).

### L11. MCP `serde_json::Value` usage
`werk-mcp/src/tools.rs` — 30+ sites. All are at the MCP wire boundary (JSON-RPC), where `serde_json::Value` is legitimate. No action, per task constraints.

### L12. `AddressParseError.input: String, reason: String` — these are fine.
Diagnostic free-form.

### L13. `CoreError::ValidationError(String)` — fine, diagnostic.

### L14. `HashMap<String, String>` for `logbase_id_to_desire`
`werk-tui/src/app.rs:145`, `werk-tui/src/logbase.rs:150,199,519`. Maps tension id → desired text. Both sides are domain-meaningful strings, but neither is an ad-hoc "config". Would become `HashMap<TensionId, DesiredText>` after L1. Deferred.

## Summary

| ID | Site | Confidence | Status |
|----|------|------------|--------|
| 1  | `Mutation.field` → `MutationFieldKind` view | HIGH | Implemented |
| 2  | TUI undo stacks → `GestureId` newtype | HIGH | Implemented |
| L1 | `Tension.id` → `TensionId` | LOW | Flagged |
| L2 | `short_code` → `ShortCode` | LOW | Flagged |
| L3 | `position` → `SiblingPosition` | LOW | Flagged |
| L4 | `Mutation.tension_id`, `gesture_id` newtypes | LOW | Flagged (depends on L1) |
| L6 | `Address::Gesture/Session` newtypes | LOW | Flagged |
| L7 | Replay parsing uses enum directly | LOW | Cosmetic |
| L8 | `HookEvent.field` → `HookField` | LOW | Flagged |
| L9 | `HookFilter::Status(String)` → `TensionStatus` | LOW | Flagged |
| L10| `parent_snapshot_json: Option<String>` → struct | LOW | Flagged |
| L11| MCP `serde_json::Value` | N/A | Wire boundary — no action |
| L14| `HashMap<String, String>` (TUI) | LOW | Flagged (depends on L1) |

Post-commit:
- `cargo check --workspace`: passes
- `cargo test --workspace`: passes (run after each implementation commit)
