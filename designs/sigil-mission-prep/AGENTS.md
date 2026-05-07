# AGENTS.md — operational guidance

Workers must read this file at session start.

## Mission Boundaries (NEVER VIOLATE)

**Branch:** Stay on `sigil-engine-v1`. Do not branch off, rebase to main, or
push to other refs. Final PR to `main` happens at end of mission.

**Sacred core (off-limits to edits):**
- `designs/werk-conceptual-foundation.md`
- `designs/sigil-engine.md` (architectural authority — read it; do not edit it)

**Sigils are artifacts, not gestures.** Rendering does **not** enter werk's
gesture log. A sigil is a *view of werk-state*, not an event in it. Do not
add gesture-emission code paths in the engine or its consumers.

**Stable registries (additions are public API):**
- Attribute-name registry (in `werk-sigil`): names map to design Part IV
  semantics. Do not rename existing entries.
- Channel-name registry (in `werk-sigil`): adding a channel is fine; renaming
  one is a breaking change.
- Sigil short-code prefix `*` (in werk-core address parser): orthogonal to
  `#`, `g:`, `s:`. Do not change other prefix semantics.

**Tension model:** Do **not** modify `TensionStatus` enum variants
(`werk-core/src/tension.rs:35-44`). The "held" / "frozen" status names in the
design map to derived attributes, not enum variants — see
`library/sigil-engine-decisions.md` D1.

**Off-limits resources:**
- `tensions.json` at repo root: tracked snapshot, used as fixture data only;
  do not flush or rewrite.
- `.werk/` directory: gitignored runtime DB; safe to read for fixtures, do
  not commit.
- `target/`: build artifacts, gitignored.

## Coding Conventions

**Rust toolchain:** nightly, edition 2024 (see `rust-toolchain.toml`).

**Formatting:** `cargo fmt` is the lint floor. CI/UBS scans run on hooks; do
not bypass.

**Style:**
- `#![forbid(unsafe_code)]` at the top of every new lib crate (mirror
  `werk-core/src/lib.rs:1`).
- Errors: use `thiserror` for crate-local error enums; expose them via
  `pub use` at lib root.
- Public types: `#[derive(Debug, Clone)]` minimum. `#[derive(Serialize,
  Deserialize)]` if they cross the JSON surface (CLI output, web API).
- Avoid `unwrap()` / `expect()` outside tests. Bubble up `Result` or use
  `Option::ok_or`.

**Tests:**
- Unit tests live in `#[cfg(test)] mod tests` blocks at the bottom of each
  source file.
- Integration tests live in `<crate>/tests/`.
- In-memory store for fixtures: `Store::new_in_memory().unwrap()`.
- Filesystem fixtures: `tempfile::TempDir::new().unwrap()`.
- CLI tests: `assert_cmd::cargo_bin_cmd!("werk")` + `predicates`. See
  `werk-cli/tests/json.rs` for the pattern.
- Tests must not leave processes running. Kill any test daemon by PID, not
  by name.
- Time-sensitive tests: use `std::thread::sleep(Duration::from_millis(10))`
  to force monotonic timestamps when needed (precedent:
  `werk-core/tests/epoch_integrity.rs:55`).

**TDD discipline:** Write failing tests first, in one file edit. Then
implement until they pass, in a subsequent edit. Even when the test and
implementation live in the same file.

**Determinism in tests:** Tests must not depend on wall-clock time. The
engine receives `now: DateTime<Utc>` via `Ctx`; test fixtures pass a fixed
clock.

**Golden snapshots:**
- Live in `werk-sigil/tests/snapshots/<preset>.svg`.
- Committed.
- Regeneration via env var: `WERK_UPDATE_SNAPSHOTS=1 cargo test -p werk-sigil`.
- Updates **must be reviewed by orchestrator** before commit (eyeball the
  diff in an SVG viewer).

**JSON conventions:**
- All command output structs are `#[derive(Serialize)]`.
- `--json` mode suppresses all human chrome (no `println!` outside structured
  emit).
- Errors in `--json` mode go through `output.error_json(code, message)` with
  `WerkError::error_code()` mapping. See `werk-shared/src/error.rs`.

**CLI conventions:**
- Each command has 2-4 examples in `#[command(after_help = "...")]`.
- Mutating commands support `--dry-run` for preview without effect.
- `--json` is global; do not redefine on subcommands.
- Exit codes: 0 success, 1 user error, 2 internal — see
  `werk-shared/src/error.rs:97-115`.

## Mission-specific guidance

### Engine purity boundary
The function `engine.render(scope, logic) -> Sigil` performs **zero I/O**.
- No clock reads — `Ctx::now` is supplied by the caller.
- No DB writes — caller persists the resulting `Sigil`.
- No network calls — ever.
- Stage implementations may read from werk-core via the `Ctx` (which holds
  a `Store` reference) but do not write.

### Cache and archive responsibilities
The engine does not cache. The CLI and the web handler are the cache
boundary:
- CLI `--save` writes archive: `~/.werk/sigils/YYYY-MM-DD/<scope-slug>-<logic>-<seed>.svg`.
- Web handler computes cache key, hits `~/.werk/sigils/cache/<hash>.svg`,
  generates if miss, returns bytes.

### `Cargo.toml` updates
When adding `werk-sigil` to the workspace, update the **root**
`Cargo.toml` `[workspace] members` list (current value:
`["werk-core", "werk-cli", "werk-mcp", "werk-shared", "werk-tui",
"werk-web", "werk-app/src-tauri"]`).

Add `werk-sigil = { path = "../werk-sigil" }` to dependencies of
`werk-cli` and `werk-web` only when those features land (M4).

### `tensions.json` is the cross-machine fixture
Workers seeking realistic werk-state for tests can parse `tensions.json`
(at repo root) instead of standing up a `.werk/` database. The shape:
```json
{ "flushed_at": "...", "summary": {...}, "tensions": [ ...Tension... ] }
```

### Validator etiquette
- Scrutiny validator: it runs typecheck, test, lint commands from
  `services.yaml`. Make sure all three pass before declaring a feature
  complete.
- User-testing validator: it reads the `fulfills` field on completed
  features and runs assertions through real surfaces. Ensure CLI commands
  and web endpoints work end-to-end before marking M4/M5 features
  complete.

### Known pre-existing issues (do not fix)
None recorded yet. Validators that surface unrelated failing tests should
add an entry to a `## Known Pre-Existing Issues` section here, not create
fix features.

## Testing & Validation Guidance

**Validators must:**

- For Rust changes: run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` (note: project may not currently meet this bar — apply only to changes in this mission), `cargo fmt --check`.
- For werk-sigil: run snapshot tests with `WERK_UPDATE_SNAPSHOTS=0` (the
  default; ensures no accidental updates).
- For CLI features (M4): run `assert_cmd` integration tests; manually invoke
  `werk sigil 2 --logic contemplative --out /tmp/x.svg` and inspect.
- For web features (M4): run `cargo test -p werk-web`; manually
  `curl http://localhost:3749/api/sigil?scope=42 -o /tmp/x.svg` and inspect.
- For werk-tab (M5): use `agent-browser` to load the extension and
  visually verify the sigil mode renders.

**Snapshot review:** When a snapshot test fails, the validator surfaces
the diff as part of its handoff. The orchestrator decides whether to:
- Mark as a regression (failure stays).
- Approve the new snapshot (set `WERK_UPDATE_SNAPSHOTS=1` and commit).
- Spawn a fix feature.

**Eyeball-review milestones:** M2, M3, M5 require user eyeball review of
SVG output. Validators must produce SVGs and commit them under
`validation/<milestone>/` so the user can inspect.
