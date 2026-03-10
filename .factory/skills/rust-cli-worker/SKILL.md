---
name: rust-cli-worker
description: Implements Rust CLI features for werk-cli with TDD, integration tests, and command verification
---

# Rust CLI Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the WORK PROCEDURE.

## When to Use This Skill

Use for all werk-cli features: CLI subcommands, argument parsing, output formatting, workspace resolution, config management, agent integration. This worker builds on sd-core (never reimplements its logic).

## Work Procedure

### 1. Understand the Feature

Read the feature description, preconditions, expectedBehavior, and verificationSteps carefully. Read AGENTS.md for conventions and boundaries. Check `.factory/library/architecture.md` for patterns.

If this feature adds new dependencies to Cargo.toml, do that first and run `cargo check -p werk` to verify compilation.

### 2. Check Preconditions

Verify all preconditions are met:
- Required sd-core types/methods exist
- Prior CLI commands this feature depends on are implemented
- Dependencies are in Cargo.toml (check `werk-cli/Cargo.toml`)

If preconditions are NOT met, return to orchestrator immediately.

### 3. Write Tests First (RED)

Write failing tests BEFORE any implementation:

- **Unit tests** in module files (`#[cfg(test)] mod tests`) for parsing, formatting, config logic
- **Integration tests** in `werk-cli/tests/` using `assert_cmd` + `predicates` crates for end-to-end command testing
- Cover ALL items in expectedBehavior
- Cover error paths: bad input, missing workspace, invalid IDs, status transition failures
- Test names: `test_add_creates_tension_with_valid_input`, `test_show_rejects_ambiguous_prefix`

Integration test pattern:
```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_init_creates_workspace() {
    let dir = TempDir::new().unwrap();
    Command::cargo_bin("werk").unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    assert!(dir.path().join(".werk").join("sd.db").exists());
}
```

Run `cargo test -p werk` to confirm tests fail.

### 4. Implement (GREEN)

Write the minimum implementation to make all tests pass:

- Use clap derive for command parsing (`#[derive(Parser)]`, `#[derive(Subcommand)]`)
- Use sd-core's Store and DynamicsEngine — never reimplement store/dynamics logic
- All output goes to stdout (human-readable by default, JSON with --json)
- All errors go to stderr with descriptive messages
- Exit codes: 0 success, 1 user error, 2 internal error
- No panics — handle all errors with Result and descriptive messages
- Color via `owo-colors` or similar, respecting NO_COLOR and --no-color
- ID prefix matching: resolve prefix to unique tension, error if ambiguous or not found

Run `cargo test -p werk` until all tests pass.

### 5. Lint and Format

Run these in order, fix any issues:

```
cargo fmt -p werk
cargo clippy -p werk -- -D warnings
cargo test -p werk
cargo test -p sd-core -- --test-threads=5
```

All four must pass. sd-core tests must not regress.

### 6. Manual Verification

Build and run the actual binary against a temp workspace:

```bash
cd $(mktemp -d)
cargo run -p werk -- init
cargo run -p werk -- add "test desired" "test actual"
cargo run -p werk -- show <id>
cargo run -p werk -- tree
```

Verify:
- Output is readable and well-formatted
- Colors appear correctly (when terminal supports them)
- Error messages are descriptive and helpful
- --json output is valid JSON (pipe to `jq .`)
- $EDITOR integration works if applicable

Record each manual check as an interactiveChecks entry.

### 7. Commit

Commit all changes with a descriptive message. Include all new and modified files.

## Example Handoff

```json
{
  "salientSummary": "Implemented `werk init` and `werk add` commands with workspace resolution, ID prefix matching, and --json support. 12 integration tests pass, clippy clean. Manually verified init creates .werk/sd.db, add stores tension correctly, --json outputs valid JSON.",
  "whatWasImplemented": "CLI skeleton with clap derive Parser/Subcommand. `werk init` creates .werk/sd.db (local) or ~/.werk/sd.db (--global). `werk add` creates tension with --desired/--actual/--parent flags, supports interactive prompt when flags omitted. Workspace resolution walks up directory tree, falls back to ~/.werk/. ID prefix matching resolves 4+ char prefixes, rejects ambiguous matches. --json flag on both commands. Exit codes: 0/1/2.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      { "command": "cargo test -p werk", "exitCode": 0, "observation": "12 tests passed, 0 failed" },
      { "command": "cargo test -p sd-core -- --test-threads=5", "exitCode": 0, "observation": "306 tests passed, no regressions" },
      { "command": "cargo clippy -p werk -- -D warnings", "exitCode": 0, "observation": "No warnings" },
      { "command": "cargo fmt -p werk --check", "exitCode": 0, "observation": "No formatting issues" }
    ],
    "interactiveChecks": [
      { "action": "cargo run -p werk -- init (in temp dir)", "observed": ".werk/sd.db created, exit code 0, message 'Workspace initialized at .werk/'" },
      { "action": "cargo run -p werk -- add 'write novel' 'have outline'", "observed": "Tension created with ULID, status Active, printed confirmation with ID" },
      { "action": "cargo run -p werk -- add 'write novel' 'have outline' --json", "observed": "Valid JSON output with id, desired, actual, status fields. Parsed by jq successfully." },
      { "action": "cargo run -p werk -- show <prefix> (4 chars)", "observed": "Resolved to correct tension, displayed all fields" },
      { "action": "Attempted show with ambiguous 2-char prefix", "observed": "Error: prefix too short, suggests using 4+ characters. Exit code 1." }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "werk-cli/tests/init.rs",
        "cases": [
          { "name": "test_init_creates_workspace", "verifies": "init creates .werk/sd.db" },
          { "name": "test_init_idempotent", "verifies": "re-init preserves data" },
          { "name": "test_init_global", "verifies": "--global creates ~/.werk/" }
        ]
      },
      {
        "file": "werk-cli/tests/add.rs",
        "cases": [
          { "name": "test_add_creates_tension", "verifies": "basic add with desired/actual" },
          { "name": "test_add_with_parent", "verifies": "--parent creates child" },
          { "name": "test_add_rejects_empty_desired", "verifies": "validation error on empty" },
          { "name": "test_add_handles_unicode", "verifies": "CJK and emoji preserved" },
          { "name": "test_add_json_output", "verifies": "--json produces valid JSON" }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## Mission-Specific Guidance

### Triple Copy-Paste Problem
The current main.rs (3607 lines) has dynamics computation duplicated in 3 places: cmd_show (~line 564), cmd_context (~line 2477), and cmd_run (~line 2954). Each has its own copy of the full dynamics computation block calling detect_oscillation, detect_resolution, etc. The shared dynamics module (src/dynamics.rs) must replace all 3 copies with a single function.

### Horizon Pass-Through Bugs
In cmd_context and cmd_run, several dynamics functions receive `None` for the horizon parameter instead of `tension.horizon.as_ref()`. grep for `None::<&Horizon>` or explicit `None` passed to dynamics functions. All must be fixed to pass the actual horizon.

### Output Module Pattern
The Output struct in output.rs controls format selection. Currently supports human-readable (default) and --json. TOON is being added as --toon flag. The pattern: each command calls output methods that switch on format.

### TOON Format
TOON (Token-Oriented Object Notation) is a structured format optimized for LLM token efficiency. Use the `toon-format` crate (v0.4). The format uses key-value blocks with indentation. See the toon-format crate docs for serialization API.

### Command Extraction Pattern
When extracting commands to src/commands/*.rs:
- Each file exports a `pub fn cmd_name(...)` function
- Common imports (Store, Output, WerkError, etc.) go in a commands/mod.rs prelude
- The dynamics computation function lives in src/dynamics.rs (imported by commands that need it)
- main.rs becomes: clap parsing + match dispatch + error handling

## When to Return to Orchestrator

- sd-core API doesn't have a method needed for this command (missing Store/Engine method)
- Workspace resolution logic unclear (ambiguity in .werk/ vs ~/.werk/ precedence)
- Color library doesn't support a required feature
- $EDITOR integration has platform-specific issues that can't be resolved
- A prior CLI command this feature depends on is broken or missing
- `cargo test -p sd-core` has regressions that weren't caused by this feature
