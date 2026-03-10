---
name: rust-library-worker
description: Implements Rust library features with TDD, fsqlite storage, and comprehensive testing
---

# Rust Library Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the WORK PROCEDURE.

## When to Use This Skill

Use for all sd-core library features: types, storage, dynamics computations, events, and integration tests. This worker handles pure Rust library code with no CLI/TUI/server concerns.

## Work Procedure

### 1. Understand the Feature

Read the feature description, preconditions, expectedBehavior, and verificationSteps carefully. Read AGENTS.md for conventions and boundaries. Check `.factory/library/architecture.md` for patterns.

If this feature modifies Cargo.toml dependencies, do that first and run `cargo check -p sd-core` to verify compilation.

### 2. Check Preconditions

Verify all preconditions are met:
- Required modules/types from earlier features exist
- Dependencies are in Cargo.toml
- Schema matches expectations (if store-related)

If preconditions are NOT met, return to orchestrator immediately.

### 3. Write Tests First (RED)

Write failing tests BEFORE any implementation:

- Unit tests in the module file (`#[cfg(test)] mod tests { ... }`)
- Integration tests in `sd-core/tests/` for cross-module behavior
- Cover ALL items in expectedBehavior
- Cover edge cases: empty inputs, Unicode, large data, error paths
- Test names should be descriptive: `test_tension_creation_with_empty_desired_fails`

Run `cargo test -p sd-core` to confirm tests fail (compile errors are acceptable at this stage if types don't exist yet -- that counts as "red").

### 4. Implement (GREEN)

Write the minimum implementation to make all tests pass:

- Follow patterns in `.factory/library/architecture.md`
- Use fsqlite (NOT rusqlite) for storage. See AGENTS.md for API reference.
- All public types: derive Debug, Clone, Serialize, Deserialize
- All errors via thiserror, return Result
- No panics (no unwrap in library code)
- No hardcoded thresholds in dynamics

Run `cargo test -p sd-core` until all tests pass.

### 5. Lint and Format

Run these in order, fix any issues:

```
cargo fmt -p sd-core
cargo clippy -p sd-core -- -D warnings
cargo test -p sd-core
```

All three must pass with zero warnings/errors.

### 6. Manual Verification

Since this is a library, "manual verification" means:

- Read through the public API you created. Is it ergonomic? Are types well-named?
- Check that error messages are descriptive
- Verify doc comments exist on public items
- Run `cargo doc -p sd-core --no-deps` and check it builds

### 7. Commit

Commit all changes with a descriptive message. Include all new and modified files.

## Example Handoff

```json
{
  "salientSummary": "Implemented Tension type with ULID id, validation (non-empty desired/actual), status state machine (Active->Resolved|Released), and serde serialization. All 18 tests pass, clippy clean, fmt clean.",
  "whatWasImplemented": "sd-core::tension module: Tension struct (id, desired, actual, parent_id, created_at, status), TensionStatus enum (Active/Resolved/Released), Tension::new() with validation, update_desired/update_actual/update_parent with validation, resolve()/release() with state machine enforcement. All types derive Debug, Clone, PartialEq, Serialize, Deserialize.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      { "command": "cargo test -p sd-core", "exitCode": 0, "observation": "18 tests passed, 0 failed" },
      { "command": "cargo clippy -p sd-core -- -D warnings", "exitCode": 0, "observation": "No warnings" },
      { "command": "cargo fmt -p sd-core --check", "exitCode": 0, "observation": "No formatting issues" }
    ],
    "interactiveChecks": [
      { "action": "Reviewed public API surface for ergonomics", "observed": "Types are well-named, error messages descriptive, doc comments present on all public items" },
      { "action": "cargo doc -p sd-core --no-deps", "observed": "Documentation builds successfully" }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "sd-core/src/tension.rs",
        "cases": [
          { "name": "test_tension_new_valid", "verifies": "Construction with valid inputs succeeds" },
          { "name": "test_tension_new_empty_desired_fails", "verifies": "Empty desired rejected" },
          { "name": "test_tension_new_empty_actual_fails", "verifies": "Empty actual rejected" },
          { "name": "test_tension_ulid_uniqueness", "verifies": "1000 tensions have unique ids" },
          { "name": "test_tension_status_transitions", "verifies": "State machine Active->Resolved, Active->Released, invalid transitions fail" },
          { "name": "test_tension_serialization_roundtrip", "verifies": "JSON roundtrip preserves all fields" },
          { "name": "test_tension_unicode", "verifies": "Unicode in desired/actual preserved" }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## Mission-Specific Guidance

### Dynamics Functions Pattern
All dynamics functions in dynamics.rs follow a consistent pattern:
- Accept threshold structs as parameters (never hardcoded constants)
- Use `effective_recency(absolute_recency, horizon, now)` for horizon-scaled time windows
- Accept `now: DateTime<Utc>` to avoid internal `Utc::now()` calls
- Return result structs with `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`

### Gap Magnitude
The `compute_gap_magnitude` function (line ~651 in dynamics.rs) is called from many places. When its algorithm changes, ALL tests that assert specific magnitude values will need their expected values updated. Search for `compute_gap_magnitude` and all test assertions that compare magnitude floats.

### Event System Pattern
New events follow the pattern in events.rs:
1. Add variant to `Event` enum with serde `tag = "type", rename_all = "snake_case"`
2. Add `EventBuilder::event_name()` static method returning `Event`
3. Add match arms to `tension_id()` and `timestamp()` methods
4. Add `test_event_name_serialization_roundtrip` test
5. Wire transition detection in `engine.rs::compute_and_emit_for_tension()`

### Engine Wiring
`DynamicsEngine::compute_and_emit_for_tension()` (engine.rs) tracks previous state via `PreviousDynamics` and emits events on state transitions. Several TODO comments mark where new events should be emitted. The engine currently never calls `detect_compensating_strategy` — this must be added.

## When to Return to Orchestrator

- Feature depends on a module or type that doesn't exist yet (precondition not met)
- fsqlite API behaves unexpectedly (not matching documented behavior in AGENTS.md)
- Unclear how a dynamics computation should work (ambiguous Fritz concept)
- Cargo.toml dependency conflict that can't be resolved locally
- Test infrastructure issue (cargo test itself broken)
