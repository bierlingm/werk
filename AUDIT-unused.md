# Unused Code Audit

**Branch:** `worktree-agent-aa78cfce`
**Baseline:** `5f019d1d Bump to v1.6.0`
**Date:** 2026-04-15

## Tooling Notes

The task originally referenced `knip` and `madge` (JS tools). For this Rust
workspace the equivalent coverage comes from:

| Tool | Available? | Used |
|------|------------|------|
| `cargo clippy --workspace --all-targets -- -W dead-code -W unused` | yes | yes |
| `cargo machete` (installed for this audit, v0.9.2) | yes | yes |
| `cargo +nightly udeps` | no (`cargo install` not attempted — machete covers the same surface) | no |
| `rustc -W unused-crate-dependencies` (via `RUSTFLAGS`) | yes | checked; no additional signal |

- Nightly toolchain is available (`nightly-aarch64-apple-darwin`) but `cargo-udeps`
  is not installed. `cargo-machete` was installed via `cargo install cargo-machete`
  and used instead. Its false-positive rate on proc-macro-only deps (e.g. `thiserror`)
  was hand-verified per dep below.

## Compiler Dead Code Warnings

`cargo clippy --workspace --all-targets -- -W dead-code -W unused` emitted
**zero warnings in the `dead_code`, `unused_imports`, `unused_variables`,
`unused_must_use`, `unused_assignments`, `unused_mut`, `unused_macros`, or
`unused_attributes` families**. The Rust compiler already enforces these as
warnings in this workspace and the tree is clean. The 218 clippy warnings that
do fire are all *stylistic* (`collapsible_if`, `needless_borrow`,
`map_or`-simplification, `sort_by_key`, function-arg-count, etc.), not
dead-code. Those are out of scope for this audit.

## `#[allow(dead_code)]` annotations — FLAGGED BUT KEPT

Four `#[allow(dead_code)]` sites exist. Each is intentional and documented
as either transitional or about-to-be-used. Per the task constraint
(“Don't remove `#[allow(dead_code)]` gated items without checking why the
allow exists” and “Don't remove deprecated-but-documented-as-transitional
code”), all four are left as-is:

1. `werk-core/src/search.rs:212` — `pub fn persist_disk_index`. Doc-comment
   says *“Call only when the disk index is needed (e.g., before upgrading to
   hybrid search). Not needed for the current in-memory search path.”*
   Transitional; leave.

2. `werk-mcp/src/tools.rs:59` — `pub struct ListParam`. A schemars-derived
   MCP tool parameter struct. Even though the struct itself may not be named
   explicitly, the MCP tool-router pathway consumes it through macro-generated
   code and the `schemars::JsonSchema` derive. Leave.

3. `werk-tui/src/deck.rs:519` — `ZoomLevel::Orient` variant. Comment
   immediately above says *“Zoom level (V1: Normal only)”*. Roadmap variant;
   leave.

4. `werk-cli/tests/cross_area.rs:78` — `fn extract_ulids`. Test helper;
   harmless; leave.

## Unused Dependencies — REMOVED

`cargo machete` found 11 unused declared dependencies across 6 crates. Each
was cross-verified by grepping `werk-*/src`, `werk-*/tests`, and
`werk-*/examples` for `use <crate>`, `extern crate <crate>`, `<crate>::`,
and macro-flavoured references (e.g. `#[tokio::main]`, `#[derive(thiserror::Error)]`).
After removal, `cargo check --workspace --all-targets` and
`cargo test --workspace -- --test-threads=1` both pass.

| Commit | Crate | Dependency | Evidence |
|--------|-------|------------|----------|
| `5460a675` | werk-cli | `owo-colors` | `grep -r 'owo_colors\|owo-colors\|OwoColorize' werk-cli` → 0 hits |
| `5460a675` | werk-cli | `regex` | `grep -rE '\bregex\b' werk-cli` → only Cargo.toml |
| `5460a675` | werk-cli | `thiserror` | `grep -r 'thiserror' werk-cli` → only Cargo.toml (no `#[derive(thiserror::Error)]` anywhere) |
| `5460a675` | werk-cli | `toml` | `grep -rE '\btoml\b' werk-cli/src` → only string literals (`"config.toml"`) and doc comments; no `toml::` code |
| `287ddf64` | werk-tui | `notify` | `grep -r 'notify' werk-tui/src` → 0 hits |
| `287ddf64` | werk-tui | `serde_yaml` | `grep -rE 'serde_yaml\|serde_yml' werk-tui/src` → 0 hits |
| `287ddf64` | werk-tui | `which` | `grep -rE '\bwhich\b' werk-tui/src` → only English-word hits in doc comments; no `which::` code |
| `5465d541` | werk-core | `fsqlite-error` | `grep -rE 'fsqlite_error\|fsqlite::error' werk-core` → 0 hits. Error types flow through the `fsqlite` crate directly. |
| `6219a0eb` | werk-web | `serde_json` | `grep -r 'serde_json' werk-web/src` → 0 hits. `axum::Json` extractor pulls serde_json transitively. |
| `459e938d` | werk-app | `serde_json` | `grep -r 'serde_json' werk-app/src-tauri/src` → 0 hits. Tauri's `generate_handler!` macro resolves serde_json through the `tauri` crate, not via the consumer's own dep. |
| `372401e6` | werk-mcp | `tokio` | `grep -r 'tokio\|#\[tokio::' werk-mcp/src` → 0 hits. werk-mcp is lib-only; the async runtime is owned by the caller (`werk-cli/src/main.rs` has `#[tokio::main]`) and `rmcp` itself depends on tokio. |

### Risk Rationale for the Two Most-Risky Removals

- **werk-mcp tokio.** The lib exposes `pub async fn run_server()`. Even without
  a direct tokio dep, callers get a working `Future` because (a) the return
  type `Result<(), Box<dyn Error>>` doesn't name tokio types, and (b) rmcp
  (the only consumer of tokio inside werk-mcp via `transport-io`) pulls tokio
  through its own `Cargo.toml`. Feature unification means the workspace-wide
  `tokio/full` feature set requested by werk-cli still applies to the
  transitive dependency graph. Full workspace `cargo check --all-targets`
  and `cargo test` pass post-removal.

- **werk-app serde_json.** Tauri's `#[tauri::command]` / `generate_handler!`
  macros emit paths of the form `::tauri::<something>` and use tauri's own
  re-exports internally; they do not assume a top-level `serde_json` crate in
  the consumer's dep tree. `cargo check` re-compiled the full Tauri macro
  expansion successfully with serde_json removed.

## Unused Public Items — NONE REMOVED

The compiler's `dead_code` lint fires on pure-private-scope dead items. For
`pub` items in library crates it stays silent (the compiler can't know what
external consumers reference). I skimmed the largest exported surfaces
(`werk-core`'s `Store` / `Engine`, `werk-shared`'s config/registry, the MCP
tool set, the CLI command set) and every `pub fn` / `pub struct` I checked
has at least one intra-workspace caller. Given werk's four dispatch surfaces
(CLI, TUI, MCP, Web + Tauri app), many items have non-obvious callers — I
chose to leave `pub` items alone unless they have literally zero references
anywhere, which is a harder claim than I can make with high confidence in
the time available. This keeps the audit conservative per the
“when in doubt, LEAVE IT” instruction.

## Stylistic Clippy Warnings — OUT OF SCOPE

The 218 stylistic clippy warnings (`collapsible_if`, `needless_borrow`,
`manual_map`, etc.) are real and worth a separate cleanup pass, but they are
*not* unused-code findings. Left untouched.

## Verification

After all commits:

```
$ cargo machete
cargo-machete didn't find any unused dependencies in this directory. Good job!

$ cargo check --workspace --all-targets
Finished `dev` profile ...

$ cargo test --workspace -- --test-threads=1
(all 900+ tests pass; failures that appear under default parallelism are pre-existing
fsqlite test contention unrelated to this audit — reproducible on the pre-audit tree.)
```

## Summary

- **6 commits**, one per crate cleanup, each independently revertable.
- **11 unused dependencies removed**, 0 source-code items removed.
- **4 `#[allow(dead_code)]` items flagged and kept** (all transitional/roadmap).
- **Zero compiler dead_code warnings** in the tree — the codebase was already
  clean on that front.
- Full workspace check and full test suite green after each commit.
