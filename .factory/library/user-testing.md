# User Testing

Testing surface: tools, URLs, setup steps, isolation notes, known quirks.

**What belongs here:** How to manually test the library, testing tools, setup steps.

---

## Testing Surface

### sd-core (library)

sd-core is a pure Rust library. Validation through automated tests.
- `cargo test -p sd-core` -- run all tests (306 tests)
- `cargo clippy -p sd-core -- -D warnings` -- lint
- `cargo fmt -p sd-core --check` -- format check

### werk-cli (CLI application)

werk-cli is a CLI application. Testing via direct command execution.

**How to test:**
1. Create temp directory: `mktemp -d`
2. Init workspace: `cargo run -p werk -- init` (from temp dir)
3. Run commands under test
4. Verify stdout, stderr, exit code

**Build:** `cargo build -p werk` (binary at `target/debug/werk`)
**Run:** `cargo run -p werk -- <subcommand> [args]`

**Test isolation:** Each flow uses its own temp directory. Never use the user's real `~/.werk/`.
Set `HOME` to temp dir for global workspace tests.

## Known Quirks

- fsqlite requires nightly Rust
- fsqlite error types use FrankenError, not standard SQLite types
- ULID generation is time-based — rapid creation produces sequential IDs
- `$EDITOR` tests need mock editor (`EDITOR=cat` to verify, `EDITOR=true` to skip)
- Color auto-disabled when piping (`werk tree | cat` = plain text)

## Flow Validator Guidance: werk-cli-cli

- Use only CLI user-surface commands (`cargo run -p werk -- ...` or `target/debug/werk ...`).
- Isolation boundary: each flow validator must use its own temp workspace root and its own `HOME` directory.
- Never touch the real user home workspace (`~/.werk`) directly; always override `HOME` to your assigned namespace path.
- Keep all files inside your assigned namespace prefix (e.g. `/tmp/werk-foundation-<group>`).
- Do not modify source code while validating; only execute commands and inspect outputs/exit codes.
- Validate assertions by observable user behavior (stdout/stderr, exit code, and command-visible state).

## Flow Validator Guidance: sd-core-library

- Use only user-surface validation commands for the library (`cargo test -p sd-core -- <filter>`, `cargo test -p sd-core -- --test-threads=5`, `cargo clippy -p sd-core -- -D warnings`, `cargo fmt --all --check` when assigned).
- Isolation boundary: each flow validator must run from the repo root but keep any temporary artifacts in its assigned namespace path (e.g. `/tmp/horizon-core-<group>`).
- Do not modify production source code during validation. Only execute tests/checks and collect observable outcomes.
- If a test command fails, capture the first relevant failing assertion/test name and stderr fragment in the flow report.
- Treat test output and command exit codes as the source of truth for assertion pass/fail/blocked status mapping.
