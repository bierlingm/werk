# Environment

## Toolchain

- **Rust:** nightly (pinned via `rust-toolchain.toml`, channel = `"nightly"`,
  profile minimal, components rustfmt + clippy).
- **Edition:** 2024 (per existing crate `Cargo.toml` files).

## Workspace

`Cargo.toml` at repo root. Members listed:
```
["werk-core", "werk-cli", "werk-mcp", "werk-shared",
 "werk-tui", "werk-web", "werk-app/src-tauri"]
```
Workers must add `"werk-sigil"` to this list when M2 starts.

## Dependencies introduced in this mission

| Crate                  | Version                         | Used by      | Milestone |
|------------------------|---------------------------------|--------------|-----------|
| `rhai`                 | `1`                             | werk-sigil   | M3        |
| `quick-xml` (or `xml-rs`) | TBD by worker                | werk-sigil   | M2        |
| `blake3`               | `1`                             | werk-sigil   | M2        |
| `notify`               | `6`                             | werk-sigil   | M6        |
| `rand_chacha`          | `0.3`                           | werk-sigil   | M2        |

(Workers select an XML writer crate based on the current ecosystem at
the time of implementation. `quick-xml` is recommended; lightweight,
maintained, supports reading and writing.)

`werk-cli` keeps existing deps; `werk-sigil = { path = "../werk-sigil" }` is
added when M4 lands.

`werk-web` similarly adds `werk-sigil = { path = "../werk-sigil" }` when M4
lands.

`werk-tab` is a Chrome MV3 extension — pure JS, no toolchain changes.

## External services / APIs

**None.** The sigil engine is fully local. No third-party APIs, no network
calls.

## Environment variables

| Variable                  | Purpose                                       | Set by     |
|---------------------------|-----------------------------------------------|------------|
| `WERK_UPDATE_SNAPSHOTS`   | When `1`, snapshot tests rewrite the .svg files instead of asserting equality | Worker     |
| `WERK_SIGIL_PRESETS_DIR`  | Override default preset search path (testing) | Worker, optional |

`tensions.json` at repo root is a tracked snapshot of werk-state, available
on Zo via git. No env var needed to reach it.

## Filesystem

| Path                                     | Used for                                  |
|------------------------------------------|-------------------------------------------|
| `~/.werk/sigils/YYYY-MM-DD/`             | Archive (`--save`); persists indefinitely |
| `~/.werk/sigils/cache/`                  | Cache (web handler); 7-day retention     |
| `werk-sigil/presets/`                    | Bundled presets; embedded via `include_str!` |
| `werk-sigil/tests/snapshots/`            | Golden SVG fixtures; committed           |

`init.sh` ensures `~/.werk/sigils/cache/` exists before any worker runs.

## Platform notes

- **macOS / Linux:** primary targets. Both supported by `notify` (used in
  M6 hot-reload).
- **Windows:** untested. Don't introduce Windows-specific paths.
- **Zo (BYOM):** the mission target. Standard Linux box. All deps install
  cleanly via `cargo build` with crates.io access. No allowlisting required
  for any of the named crates.

## Verified during readiness check

(To be filled in by the readiness subagent on Zo.)

- [ ] `cargo --version` returns nightly.
- [ ] `cargo build -p werk-core` succeeds.
- [ ] `cargo add --dry-run rhai = "1"` succeeds (registry reachable).
- [ ] `~/.werk/sigils/cache/` is writable.
- [ ] `tensions.json` exists at repo root.
