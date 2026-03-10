# Environment

Environment variables, external dependencies, and setup notes.

**What belongs here:** Required env vars, external API keys/services, dependency quirks, platform-specific notes.
**What does NOT belong here:** Service ports/commands (use `.factory/services.yaml`).

---

## Rust Toolchain

- Edition: sd-core uses 2024, werk-cli uses 2021
- Stable toolchain via `rust-toolchain.toml`

## Dependencies

- sd-core: `strsim = "0.11"` for normalized Levenshtein distance (being added in this mission)
- werk-cli: `toon-format = "0.4"` for TOON serialization (being added in this mission)
- Storage: fsqlite (FrankenSQLite) in compatibility mode

## Database

- SQLite at `.werk/sd.db` per project or `~/.werk/sd.db` global
- Two tables: tensions (id, desired, actual, parent_id, created_at, status, horizon) and mutations (tension_id, timestamp, field, old_value, new_value)
- No migrations needed for dynamics changes (dynamics are computed, not stored)
