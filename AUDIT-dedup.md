# AUDIT: dedup and consolidation pass

Branch: `worktree-agent-a55b7cc2`
Base: `5f019d1d Bump to v1.6.0`
Commits:
1. `beaec531` — Add `format_short_code` and `format_datetime_compact` helpers to `werk-shared`
2. `c21fbd3f` — Use `format_short_code` helper in stats and prefix
3. `88374a7d` — Use `format_datetime_compact` in epoch and log commands
4. `a6d4c1f5` — Use `display_id` helper for ID-with-ULID-fallback call sites

Net diff (incl. tests): **+72 insertions, −67 deletions** across 8 files.
Duplication removed: **22 inline copies** of three specific patterns
collapsed to three helper calls.

Workspace status after final commit:
- `cargo check --workspace` clean
- `cargo test --workspace --lib` → 676 tests pass (23 werk-cli · 494 werk-core · 134 werk-shared · 25 werk-tui + 0 werk-web/werk-mcp lib tests)

## What was consolidated

### 1. `format_short_code(Option<i32>) -> String` (new helper)

Captures the exact inline pattern
`x.short_code.map(|c| format!("#{}", c)).unwrap_or_default()` — the
"chrome variant" of `display_id` that yields `#N` or an empty slot.

Sites replaced (10 total):
- `werk-cli/src/commands/stats.rs` × 9 — roots, branches, approaching,
  sequencing_pressure (+ predecessor), containment_violations (+ parent),
  critical_path (parent + child), and four more in
  drift/changes/trajectory readouts.
- `werk-shared/src/prefix.rs` × 1 — ambiguous-prefix match list.

Sites deliberately left alone:
- `werk-core/src/search.rs:160` — using `werk-shared` from `werk-core`
  would invert the crate dependency graph. `werk-core` is explicitly
  kept free of UI/display concerns per CLAUDE.md. The single copy stays.
- `werk-mcp/src/tools.rs:2809` — uses `.unwrap_or(0)` producing `"#0"`
  for missing short codes. Different (arguably broken) semantics — not
  a fit for `format_short_code`.
- `werk-cli/src/commands/stats.rs:972` — uses `.unwrap_or("?")` instead
  of empty string. Kept as-is.
- `werk-tui/src/update.rs` (four sites) — each uses a different string
  fallback (12-char ULID prefix, the literal word "tension"). Not a
  clean match.
- `werk-tui/src/logbase.rs` (two sites) — custom fallbacks with `…`
  suffix and different prefix lengths. Distinct formatting intent.

### 2. `format_datetime_compact(DateTime<Utc>) -> String` (new helper)

Captures `<dt>.to_rfc3339()[..19].replace('T', " ")`, yielding
`YYYY-MM-DD HH:MM:SS`.

Sites replaced (6 total):
- `werk-cli/src/commands/epoch.rs` × 3 — span_start, span_end,
  per-mutation timestamp.
- `werk-cli/src/commands/log.rs` × 4 — span_start, epoch.timestamp,
  per-mutation timestamp (in epoch show), and the gesture mutation loop.

Sites deliberately left alone:
- `werk-cli/src/commands/show.rs:671`, `werk-cli/src/commands/note.rs:364`
  — these operate on `m.timestamp[..19]` where `m.timestamp` is already
  a `String` from the JSON serialization layer. They appear as
  parse-failure fallbacks on a stringly-typed timestamp, which is a
  different data path than the `DateTime<Utc>` chain captured by the
  helper. Rewriting them would tangle the error-handling logic, not
  simplify it.
- `werk-cli/src/commands/log.rs:628` uses `[..16]` (minute precision)
  rather than `[..19]` (second precision), so it wouldn't share the
  helper cleanly.

### 3. Existing `display_id(Option<i32>, &str) -> String`

Replaces the inline pattern
`match sc { Some(c) => format!("#{}", c), None => id[..8.min(id.len())].to_string() }`
with a direct call.

Sites replaced (5 total):
- `werk-cli/src/commands/list.rs` — flat-row printer and tree-row
  printer.
- `werk-cli/src/commands/show.rs` — ancestors loop and siblings loop.
- `werk-cli/src/commands/compose_up.rs` — parent lookup fallback (now
  `display_id(None, pid)` instead of inline ULID slicing).
- `werk-tui/src/inspector.rs` — parent label fallback.

Sites deliberately left alone:
- `werk-cli/src/commands/show.rs:767` — the footer-hint path. That one
  produces `N` (no `#` prefix) for use as a CLI argument in the suggested
  commands, which is genuinely different from the display form.
- `werk-cli/src/commands/list.rs:743, 868` — these use `format!("#{:<4}", c)`
  / `format!("{:<8}", ...)` for fixed-width column alignment. Padding is
  a display-layer concern that doesn't belong in the shared helper.
- `werk-cli/src/commands/undo.rs:69, 93, 94` — operate on gesture/undo
  IDs, not tension IDs, and only take the ULID slice. Different shape.
- `werk-tui/src/update.rs` gesture-description strings — use the slice
  to build gesture labels for undo history, not for display. Not a
  semantic match.

## What I looked at and chose NOT to consolidate

### CLI/MCP/Web handler dispatch

I surveyed the command dispatch paths in `werk-cli/src/commands/*`,
`werk-mcp/src/tools.rs`, and `werk-web/src/lib.rs`. There *is* surface
repetition (e.g. "parse id → call core mutation → return JSON") but each
surface has meaningfully different concerns: CLI output shapes for human
vs. JSON and per-command narrative, MCP tools wrap responses in the
rmcp protocol envelope, and Web returns axum `Response` types. A shared
dispatcher would thread so many generic parameters (Output vs.
ContentValue vs. axum::Json) that it would obscure more than it saves.
The shared pieces *already live* in `werk-core` (the mutations
themselves) and `werk-shared` (the display helpers). This is the right
seam; collapsing it further would violate locality.

### MCP tools.rs shape

`werk-mcp/src/tools.rs` is 3256 lines with lots of similar-looking JSON
envelope construction, but each tool's arguments and response shape is
genuinely unique. The repetition is at the level of `CallToolResult::success(vec![ContentValue::text(json_string)])`
kind of boilerplate, and untangling it would require a macro that
obscures what each tool actually does. I left it alone.

### SQL fragments in `werk-core/src/store.rs`

The store has 5762 lines with several repeated query patterns (`SELECT
... FROM tensions WHERE ... AND status != 'Released'`), but they differ
in ORDER BY, LIMIT, and joins in ways where extracting a common
fragment would require string-concat or a query builder — both worse
than the status quo.

### Short-code formatting in `werk-tui/src/update.rs` and `logbase.rs`

Every TUI site I inspected uses a distinct fallback (different ULID
prefix length, or a literal word, or `…` decoration). A helper that
took fallback-shape as a parameter would have as many parameters as the
inline code and save nothing.

### `format_timestamp` + `relative_time` in parse-failure fallbacks

`show.rs` and `note.rs` parse a string timestamp, use the rich
`format_timestamp`/`relative_time` on success, and fall back to a raw
slice on parse failure. The fallback could be rewritten around the
helper if we moved the parse step into a shared function, but that
function would be used only twice and would obscure the obvious data
flow. Not worth it.

### `werk-core/src/search.rs:160` `format!("#{}", c)` site

This is the one place where the `format_short_code` pattern appears
inside `werk-core`. Moving it to the helper would require `werk-core`
to depend on `werk-shared`, which is the wrong direction per
`CLAUDE.md` ("`werk-core` stays pure, no UI concerns leak in"). The one
copy stays.

## Things I noticed but did not touch (per task rules)

These are kept out of the dedup commits to keep the scope clean:

1. **`werk-mcp/src/tools.rs:2809`** — the `.unwrap_or(0)` that produces
   `"#0"` on missing short codes. This is arguably a bug: when a child
   has no short code yet, the error message says "source has children
   that need assignment: #0" which is misleading. Would suggest fixing
   separately.
2. **`werk-cli/src/commands/epoch.rs:286`** — the comment says "Try to
   find short code" but the code just truncates the ULID to 8 chars.
   The comment is outdated — there's no lookup happening. A near-term
   opportunity to either implement the lookup or fix the comment.
3. **`werk-cli/src/commands/epoch.rs:327`** and **`werk-cli/src/commands/log.rs`**
   both define their own private `truncate(&str, usize) -> &str` that
   returns a `&str` while `werk_shared::truncate` returns an owned
   `String` (with a `…` suffix). The private versions have different
   semantics (byte-slice, no ellipsis) and are used for their return-
   type convenience in format macros. Consolidating would change output
   (adding ellipsis where there is none today). Intentionally not
   touched.

## Confidence

High:
- All three helper extractions replace byte-identical expansions (I
  verified each site produces the same string for the same inputs).
- All tests pass before and after each commit.
- Each commit is isolated and can be reverted independently.

The deliberate exclusions are low-risk — in every case I verified that
the surface patterns differ in a way that a shared helper would paper
over.
