# Architecture

Architectural decisions, patterns, and conventions for sd-core.

**What belongs here:** ADRs, module boundaries, design patterns, extension points.

---

## Core Principle

sd-core is a pure grammar library implementing Robert Fritz's structural dynamics. Zero instrument dependencies. It computes dynamics and emits events. Instruments subscribe and react.

## Data Model

Two tables, everything else computed:

- **tensions**: id (ULID), desired, actual, parent_id (nullable), created_at, status
- **mutations**: tension_id, timestamp, field, old_value, new_value

Forest topology: multiple roots, loose tensions allowed.

## Module Structure

```
sd-core/src/
  lib.rs          -- Public API surface, re-exports
  tension.rs      -- Tension type, validation, status state machine
  mutation.rs     -- Mutation type, recording logic
  store.rs        -- fsqlite store, schema, CRUD, .werk/ discovery
  tree.rs         -- Forest construction, traversal, queries
  dynamics.rs     -- All dynamics computations (or dynamics/ directory)
  events.rs       -- Typed event system, subscription
```

## Key Patterns

- **Thresholds as parameters**: All dynamics functions take threshold params from callers. No hardcoded constants.
- **Events as extension boundary**: Grammar emits typed events. Instruments subscribe via callback.
- **Deterministic events**: Same data = same events regardless of subscribers.
- **Immutable mutations**: Append-only log. No modification or deletion.
- **Result everywhere**: No panics in library code. All fallible operations return Result.

## Error Handling

Use thiserror for error types. Single SdError enum (or per-module errors) with descriptive variants.

## Storage

fsqlite (FrankenSQLite) in compatibility mode (standard .db format). Store at .werk/sd.db per project scope. Discovery: walk up from CWD for .werk/, fall back to ~/.werk/.

## Dynamics (Full Fritz Model)

Core (6): StructuralTension, StructuralConflict, Oscillation, Resolution, CreativeCyclePhase, Orientation
Secondary (3): CompensatingStrategy, StructuralTendency, AssimilationDepth
Derived (1): Neglect

---

## werk-cli (Instrument Layer)

### Module Structure

```
werk-cli/src/
  main.rs          -- Entry point, clap Parser, top-level error handling
  commands/         -- One module per subcommand
    mod.rs
    init.rs, add.rs, show.rs, tree.rs, reality.rs, desire.rs,
    resolve.rs, release.rs, rm.rs, mv.rs, note.rs, config.rs,
    context.rs, run.rs
  workspace.rs     -- .werk/ discovery, Store creation
  prefix.rs        -- ID prefix resolution
  output.rs        -- Formatting (human + JSON), color control
  editor.rs        -- $EDITOR integration
```

### Key Patterns

- **Clap derive**: `#[derive(Parser)]` for CLI, `#[derive(Subcommand)]` for commands
- **Workspace resolution**: Walk up from CWD for `.werk/`, fall back to `~/.werk/`
- **Output module**: All formatting through output.rs — human text or JSON based on --json flag
- **Color via owo-colors**: Lightweight, supports NO_COLOR env var natively
- **ID prefix matching**: Case-insensitive, minimum 4 chars, must be unambiguous
- **Exit codes**: 0 success, 1 user error, 2 internal error
- **sd-core as consumer**: Never reimplement dynamics, store, or tree logic
