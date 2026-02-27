# Environment

Environment variables, external dependencies, and setup notes.

**What belongs here:** Required env vars, external API keys/services, dependency quirks, platform-specific notes.
**What does NOT belong here:** Service ports/commands (use `.factory/services.yaml`).

---

## Rust Toolchain

- Edition: 2024 (for sd-core, required by fsqlite)
- Channel: nightly (pinned via rust-toolchain.toml in workspace root)
- Minimum: rustc 1.85.0-nightly
- Current verified: rustc 1.92.0-nightly (f04e3dfc8 2025-10-19)

## Key Dependencies

- **fsqlite** 0.1.1 -- FrankenSQLite, pure Rust SQLite reimplementation. Connection::open(":memory:") for tests, Connection::open("path") for files. See `research/fsqlite_api_reference.md` for full API.
- **ulid** 1.x -- ULID generation for tension IDs
- **chrono** 0.4 with serde feature -- timestamps
- **serde** 1.x with derive feature -- serialization
- **serde_json** 1.x -- JSON serialization

## Platform

- macOS (darwin 25.4.0, aarch64)
- 10 CPU cores
