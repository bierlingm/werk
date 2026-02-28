# werk

A Rust workspace implementing Robert Fritz's structural dynamics as computational infrastructure for creative practice.

## Workspace Overview

This workspace contains two crates:

- **sd-core**: A pure Rust library implementing structural dynamics as a computational grammar. Fully implemented, tested, and ready for use.
- **werk-cli**: Command-line interface for the structural dynamics system. Planned but not yet implemented.

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
| `events` | Typed event system with subscription API |
| `engine` | `DynamicsEngine` for integrated dynamics computation |

### Key Design Decisions

- **fsqlite for storage**: Pure Rust SQLite implementation. No C dependencies, no unsafe code.
- **Forest topology**: Multiple root tensions allowed. Loose coupling — reparenting orphans when parents resolve.
- **Caller-injected thresholds**: All dynamics functions take threshold parameters. No hardcoded constants.
- **Events as extension boundary**: Grammar emits events; instruments subscribe and react. Clean separation.
- **Zero unsafe code**: `#![forbid(unsafe_code)]` at crate root.

### Dynamics (10 Total)

**Core Dynamics:**

| Dynamic | Function | Purpose |
|---------|----------|---------|
| StructuralTension | `compute_structural_tension` | Quantify desired/actual gap |
| StructuralConflict | `detect_structural_conflict` | Detect competing sibling tensions |
| Oscillation | `detect_oscillation` | Detect advance-then-regress patterns |
| Resolution | `detect_resolution` | Detect sustainable forward progress |
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

## Setup

Requires nightly Rust (edition 2024), pinned via `rust-toolchain.toml`:

```bash
rustup show  # Installs toolchain from rust-toolchain.toml
```

Build the library:

```bash
cargo build -p sd-core
```

Run tests:

```bash
cargo test -p sd-core
```

## Test Suite

- **306 tests total**: 277 unit tests + 18 integration tests + 10 store discovery tests + 1 doctest
- **Zero unsafe code**
- **Deterministic**: Same operations always produce same results

Run the full test suite:

```bash
cargo test -p sd-core
cargo clippy -p sd-core -- -D warnings
cargo fmt -p sd-core --check
```

## Quick Example

```rust
use sd_core::{Store, Tension, TensionStatus};

// Create an in-memory store
let store = Store::new_in_memory()?;

// Create a tension: gap between "write a novel" and "have an outline"
let tension = store.create_tension("write a novel", "have an outline")?;

// Update actual as progress is made
store.update_actual(&tension.id, "have a draft chapter")?;

// Resolve when desired state is achieved
store.update_status(&tension.id, TensionStatus::Resolved)?;
```

## License

MIT OR Apache-2.0
