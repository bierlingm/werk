# werk-cli conventions (research artifact)

## CLI structure (clap derive)

- Entry: `Cli` in `werk-cli/src/main.rs:30` with `#[derive(Parser)]`.
- Subcommands: `Commands` enum at `werk-cli/src/commands/mod.rs:118` with
  `#[derive(Subcommand)]`.
- Dispatch: giant `match` in `main.rs:113-298`; each arm calls a
  free function `cmd_<name>(output: &Output, ...) -> Result<(), WerkError>`.
- Modules: `pub mod <name>;` lines at `commands/mod.rs:5-37`.
- Global flags on `Cli` itself (`main.rs:53-72`) using `global = true`:
  - `-j` / `--json` (bool)
  - `-w` / `--workspace <NAME>`
  - `-g` / `--global-space`

## --json convention

- Set on root `Cli` as global flag.
- `Output` constructed once at `main.rs:99`; threaded into every command.
- Each command defines its own `#[derive(Serialize)]` JSON struct;
  `output.print_structured(&result)` emits pretty-printed JSON.
- In `--json` mode: no human chrome (no println outside structured emit).
- Errors via `output.error_json(code, message)` with
  `WerkError::error_code()` mapping (`werk-shared/src/error.rs:117-130`).
- Codes: `NOT_FOUND`, `INVALID_INPUT`, `AMBIGUOUS`, `NO_WORKSPACE`,
  `PERMISSION_DENIED`, `IO_ERROR`, `CONFIG_ERROR`, `INTERNAL_ERROR`.

## --dry-run convention

- Per-command `#[arg(long)]`, never global.
- Help text: "Preview without making changes."
- Execution: same validation/resolution path; no gesture, no write.
- JSON: `#[serde(skip_serializing_if = "std::ops::Not::not")] dry_run: bool`
  → omitted in normal runs, present as `true` in previews.
- Human: "Would <verb> tension #N" + closing "No changes made."

## Help examples

- 2-4 examples per command in `#[command(after_help = "Examples:\n  ...")]`.
- Format: `werk <verb> <args>` + inline trailing comment for non-trivial
  flags (two-space gap before comment).
- Top-level `--help` framework groupings live in `Cli`'s
  `after_long_help` string (`main.rs:32-51`).

## Error handling / exit codes

- Every command: `Result<(), WerkError>`.
- Variants: NoWorkspace, TensionNotFound, PrefixTooShort, AmbiguousPrefix,
  InvalidInput, PermissionDenied, IoError, ConfigError, CoreError,
  StoreError, TreeError.
- `exit_code()` (werk-shared/src/error.rs:97-115): 0 success, 1 user
  error, 2 internal/IO/core.

## Recommended sigil scope-spec

Reuse existing address grammar via `werk_core::parse_address` for
positional IDs (#42, *7, werk:#3). Reuse `werk list`'s flag set for
"scope as filters": `--root`, `--parent`, `--changed`, `--has-deadline`,
`--signals`, `--search`, `--status`, `--tree`. Workspace is global
(`-w`). Cross-space via the address itself. Avoid `--space` (would
shadow `-w`).

## Sigil command dispatch placement

1. New module `werk-cli/src/commands/sigil.rs` (mirrors `commands/serve.rs`
   pattern).
2. `pub mod sigil;` line in `commands/mod.rs:5-37`.
3. `Commands::Sigil { ... }` variant in `commands/mod.rs:118` with
   `#[command(after_help = "...")]` examples.
4. Dispatch arm in `werk-cli/src/main.rs` next to `Commands::Field`.
5. `werk-cli/Cargo.toml`: add `werk-sigil = { path = "../werk-sigil" }`.

## Test patterns

- `assert_cmd::cargo_bin_cmd!("werk")` (precedent: `werk-cli/tests/skeleton.rs:9`).
- `tempfile::TempDir` per test, `cargo_bin_cmd!("werk").arg("init")
  .current_dir(...)` to bootstrap.
- `.assert().success().get_output().stdout` + `String::from_utf8_lossy` for
  capture.
- JSON: parse to `serde_json::Value`, assert keys present.
- Files organised per feature: `add_show.rs`, `lifecycle.rs`, `json.rs`,
  `tree.rs`, etc. Sigil tests go in `werk-cli/tests/sigil.rs` and
  `werk-cli/tests/sigil_lifecycle.rs`.
