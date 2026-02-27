# User Testing

Testing surface: tools, URLs, setup steps, isolation notes, known quirks.

**What belongs here:** How to manually test the library, testing tools, setup steps.

---

## Testing Surface

sd-core is a pure Rust library with no user-facing surface (no CLI, no TUI, no server, no browser). All validation is through automated tests.

## Tools

- `cargo test -p sd-core` -- run all tests
- `cargo clippy -p sd-core -- -D warnings` -- lint
- `cargo fmt -p sd-core --check` -- format check

## Testing Approach

For user-testing validation, the "user" is a Rust developer calling the API. Assertions are verified by running the test suite and checking that all tests pass. Each validation contract assertion maps to one or more test cases in the sd-core test suite.

## Known Quirks

- fsqlite requires nightly Rust (edition 2024)
- fsqlite error types use FrankenError, not standard SQLite error types
- In-memory databases via Connection::open(":memory:")

## Flow Validator Guidance: rust-library-api

- Surface: Rust library API behavior validated through test execution (no browser/TUI/API server).
- Isolation strategy for parallel validators:
  - Use separate assertion groups by domain area (`tension`, `mutation`, `store`, `tree`).
  - Run scoped tests with filters for assigned area only; avoid running full-suite concurrently in all subagents.
  - Use unique data namespace labels in reports (`foundation-tension`, `foundation-mutation`, etc.) for traceability.
- Shared-state boundaries:
  - Do not modify production library code while validating.
  - If temporary artifacts are created, keep them under repo-local test outputs only.
  - Do not write outside assigned flow report file.
- Off-limits:
  - No edits to `werk-cli/`.
  - No mocking sd-core internals; validate against real crate behavior via tests.
