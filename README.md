# werk

A Rust workspace implementing Robert Fritz's structural dynamics as computational infrastructure for creative practice.

## Workspace Overview

This workspace contains two crates:

- **sd-core**: A pure Rust library implementing structural dynamics as a computational grammar. Fully implemented with 652 tests.
- **werk-cli**: Command-line interface for working with structural tensions and launching agent sessions. Fully implemented with 414 tests.

## sd-core

A faithful computational model of Robert Fritz's structural dynamics theory. The library treats structural tension — the gap between desired state and current reality — as the primitive. Everything else is computed from tensions and their mutation histories.

### Core Concept

Structural dynamics describes how humans navigate the gap between what they want (desired) and what is (actual). This gap creates structural tension, which generates movement toward resolution. sd-core captures this as data:

- Store tensions with `desired` and `actual` fields
- Record every change as an immutable mutation
- Compute dynamics from mutation history

### Data Model

The system uses two tables:

1. **tensions**: Stores the current state of each tension (id, desired, actual, parent_id, status)
2. **mutations**: Append-only log of all changes (tension_id, timestamp, field, old_value, new_value)

Everything else — structural conflicts, oscillation patterns, resolution detection — is computed on demand from this data.

### Modules

| Module | Purpose |
|--------|---------|
| `tension` | Core `Tension` struct with lifecycle (Active, Resolved, Released) |
| `mutation` | Immutable `Mutation` records and history replay |
| `store` | fsqlite-backed persistence with CRUD operations |
| `tree` | Forest topology: multiple roots, parent-child hierarchies |
| `dynamics` | All structural dynamics computations |
| `horizon` | Temporal horizon type with range computation, urgency, staleness, drift detection |
| `events` | Typed event system with subscription API |
| `engine` | `DynamicsEngine` for integrated dynamics computation |

### Key Design Decisions

- **fsqlite for storage**: Pure Rust SQLite implementation. No C dependencies, no unsafe code.
- **Forest topology**: Multiple root tensions allowed. Loose coupling — reparenting orphans when parents resolve.
- **Caller-injected thresholds**: All dynamics functions take threshold parameters. No hardcoded constants.
- **Horizon-aware dynamics**: All dynamics computations accept optional horizon context, enabling temporal pressure to influence conflict detection, oscillation analysis, and resolution tracking.
- **Hybrid gap magnitude**: Gap between desired and actual is measured using 60% normalized Levenshtein edit distance + 40% token-level Jaccard similarity (via `strsim` crate), replacing the naive character-level metric. Unicode-safe, always in [0.0, 1.0].
- **Deterministic time injection**: All time-dependent computations accept an explicit `now` parameter for testability. No hidden `SystemTime::now()` calls.
- **Events as extension boundary**: Grammar emits events; instruments subscribe and react. Clean separation.
- **Zero unsafe code**: `#![forbid(unsafe_code)]` at crate root.

### Dynamics (13 Total)

All dynamics functions are horizon-aware — they accept optional horizon context and configurable thresholds. The `DynamicsEngine` orchestrates all 13 dynamics computations together with event emission.

**Core Dynamics:**

| Dynamic | Function | Purpose |
|---------|----------|---------|
| StructuralTension | `compute_structural_tension` | Quantify desired/actual gap (hybrid Levenshtein+Jaccard metric) |
| StructuralConflict | `detect_structural_conflict` | Detect competing sibling tensions |
| Oscillation | `detect_oscillation` | Detect advance-then-regress patterns |
| Resolution | `detect_resolution` | Detect sustainable forward progress with velocity tracking |
| CreativeCyclePhase | `classify_creative_cycle_phase` | Classify into Germination/Assimilation/Completion/Momentum |
| Orientation | `classify_orientation` | Classify tension formation patterns |

**Secondary Dynamics:**

| Dynamic | Function | Purpose |
|---------|----------|---------|
| CompensatingStrategy | `detect_compensating_strategy` | Detect avoidance patterns (GivingUp, ReducingVision, BusyWork) |
| StructuralTendency | `predict_structural_tendency` | Predict likely outcomes from patterns |
| AssimilationDepth | `measure_assimilation_depth` | Measure depth of reality engagement |

**Derived Dynamics:**

| Dynamic | Function | Purpose |
|---------|----------|---------|
| Neglect | `detect_neglect` | Detect tensions abandoned despite opportunity |

**Horizon Dynamics:**

| Dynamic | Function | Purpose |
|---------|----------|---------|
| Urgency | `compute_urgency` | Temporal pressure from approaching horizon |
| TemporalPressure | `compute_temporal_pressure` | Combined magnitude × urgency weighting |
| HorizonDrift | `detect_horizon_drift` | Detect shifting horizons (postponement, tightening, oscillation) |

### Event System (12 Event Types)

The `DynamicsEngine` emits typed events when dynamics change state. Subscribers receive events through a channel-based API.

**Original Events:**

| Event | Emitted When |
|-------|-------------|
| `TensionCreated` | New tension is created |
| `TensionResolved` | Tension reaches resolved state |
| `TensionReleased` | Tension is released |
| `OscillationDetected` | Advance-then-regress pattern emerges |
| `ResolutionAchieved` | Sustained forward progress detected |
| `NeglectDetected` | Tension abandoned despite opportunity |

**New Transition Events:**

| Event | Emitted When |
|-------|-------------|
| `UrgencyThresholdCrossed` | Urgency crosses configured threshold (up or down) |
| `HorizonDriftDetected` | Horizon drift pattern first detected or changes type |
| `CompensatingStrategyDetected` | Avoidance pattern detected (GivingUp, ReducingVision, BusyWork) |
| `OscillationResolved` | Previously detected oscillation ceases |
| `NeglectResolved` | Previously neglected tension receives attention |
| `ResolutionLost` | Resolution progress regresses |

## Setup

Requires nightly Rust (edition 2024), pinned via `rust-toolchain.toml`:

```bash
rustup show  # Installs toolchain from rust-toolchain.toml
```

### Dependencies

| Crate | Version | Used By | Purpose |
|-------|---------|---------|---------|
| `strsim` | 0.11 | sd-core | Normalized Levenshtein distance for gap magnitude computation |
| `toon-format` | 0.4 | werk-cli | TOON serialization for token-efficient LLM output |

Build:

```bash
cargo build -p sd-core      # Library only
cargo build -p werk-cli     # CLI binary
```

Run tests:

```bash
cargo test -p sd-core       # 652 tests
cargo test -p werk          # 414 tests
cargo test                  # All 1066 tests
```

## Test Suite

- **1066 tests total**: 652 sd-core + 414 werk-cli (integration and unit tests)
- **Zero unsafe code**
- **Deterministic**: Same operations always produce same results (deterministic time injection throughout)

Run the full test suite:

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## Quick Example

```rust
use sd_core::{Horizon, Store, Tension, TensionStatus};

// Create an in-memory store
let store = Store::new_in_memory()?;

// Create a tension with a temporal horizon
let horizon = Horizon::new_month(2026, 6)?;
let tension = store.create_tension_full(
    "write a novel", "have an outline", None, Some(horizon),
)?;

// Update actual as progress is made
store.update_actual(&tension.id, "have a draft chapter")?;

// Resolve when desired state is achieved
store.update_status(&tension.id, TensionStatus::Resolved)?;
```

## werk-cli

The operative instrument. Command-line interface to sd-core for working with structural tensions and launching agent sessions.

### Commands

| Command | Flags | Purpose |
|---------|-------|---------|
| `init` | `--global` | Initialize workspace (local `.werk/` or global `~/.werk/`) |
| `nuke` | `--confirm`, `--global` | Delete workspace (.werk/ directory). Requires --confirm for safety. --global targets ~/.werk/ |
| `config set` | | Set configuration key=value |
| `config get` | | Get configuration value |
| `add` | `--parent <id>`, `--horizon <date>` | Create tension (desired state, actual state) |
| `horizon` | `<id> [value]` | Set, clear, or display temporal horizon with urgency |
| `show` | `--verbose` | Display tension with computed dynamics |
| `tree` | `--open`, `--all`, `--resolved`, `--released` | Forest tree with dynamics indicators |
| `reality` | | Update actual state of a tension |
| `desire` | | Update desired state of a tension |
| `resolve` | | Mark tension as resolved |
| `release` | `--reason <text>` | Release tension with optional reason |
| `rm` | | Delete tension (automatically reparents children) |
| `move` | `--parent <id>` | Reparent tension to new parent |
| `note` | | Add annotation to workspace |
| `notes` | | List all workspace notes |
| `context` | | Output rich JSON context for agent consumption |
| `run` | `-- <command>` | Launch agent with full werk context |

### Global Flags

- `--json`: Output raw JSON (for piping to agents)
- `--toon`: Output in TOON format (Token-Oriented Object Notation — token-efficient, LLM-optimized). Mutually exclusive with `--json`.
- `--no-color`: Disable colored output

### Workspace Discovery

werk-cli walks up the directory tree to find `.werk/` directories. Supports both local (project-specific) and global (`~/.werk/`) workspaces. The `--global` flag forces global workspace operations.

### Agent Integration

When launching agents via `werk run`, the CLI sets environment variables:

- `WERK_TENSION_ID`: Active tension ID (if applicable)
- `WERK_CONTEXT`: Path to JSON context file
- `WERK_WORKSPACE`: Absolute path to workspace root

Agents receive full structural context via stdin piping, including tension forest, dynamics state, and computed tendencies.

### TOON Output Format

TOON (Token-Oriented Object Notation) is a structured output format optimized for LLM token efficiency. It uses key-value blocks with indentation — roughly 40-60% fewer tokens than equivalent JSON for the same data.

```bash
# Show tension dynamics in TOON format
werk show <id> --toon

# Export context for LLM consumption (token-efficient)
werk context --toon | pbcopy

# Tree view in TOON format
werk tree --open --toon
```

TOON and JSON are mutually exclusive. If both flags are provided, JSON takes precedence. TOON output is machine-parseable via the `toon-format` crate (v0.4).

### JSON Output Enhancements

The `--json` (and `--toon`) output now includes additional dynamics fields:

- `horizon_drift`: Drift type, change count, and net shift in seconds
- `resolution.required_velocity` and `resolution.is_sufficient`: Whether current progress rate is sufficient to meet the horizon
- `staleness_ratio`: How stale a tension is relative to its horizon

### CLI Architecture

The CLI is structured as a thin dispatch layer with extracted command modules:

- `src/main.rs` (~100 lines): Argument parsing and command dispatch
- `src/commands/`: 17 command handler files (init, add, show, horizon, reality, desire, resolve, release, rm, move_cmd, note, notes, tree, context, run, nuke, config)
- `src/dynamics.rs`: Shared dynamics computation module using `DynamicsEngine` from sd-core
- `src/output.rs`: Output formatting (Human, JSON, TOON)

### Quick Example

```bash
# Initialize workspace
werk init

# Create a tension with a temporal horizon
werk add --horizon 2026-06 "ship v1.0" "have working prototype"

# Update reality as progress happens
werk reality <id> "have alpha build"

# Set or display horizon with urgency
werk horizon <id>
werk horizon <id> 2026-05-15

# View forest with dynamics indicators
werk tree --open

# Export context for agent consumption
werk context | jq '.tensions | length'

# Export context in TOON format (LLM-optimized)
werk context --toon

# Launch agent with full context
werk run -- claude --analyze-tensions
```

## License

MIT OR Apache-2.0
