# werk formal specifications

Executable specifications of werk's sacred core using [Quint](https://quint-lang.org).

## Modules

| Module | File | Specifies |
|--------|------|-----------|
| `types` | `types.qnt` | Core domain types (Tension, Edge, Mutation, Status, Horizon, Position) |
| `tension` | `tension.qnt` | Lifecycle state machine, field update guards, gap magnitude |
| `forest` | `forest.qnt` | Typed edges, acyclicity, sibling uniqueness, provenance |
| `timeCalculus` | `temporal.qnt` | Urgency, containment, sequencing, standard of measurement |
| `gestures` | `gestures.qnt` | Gesture atomicity, split/merge compound gestures |
| `concurrency` | `concurrency.qnt` | MVCC two-writer model, per-tension total order |
| `werk` | `werk.qnt` | Top-level composition with step relation and invariants |

## Invariants

**`systemInvariant`** (always holds, enforced by action guards):
- `desiredNeverEmpty` — no tension with empty desired state
- `singleParent` — at most one contains-edge per child
- `noSelfEdges` — no edge from a tension to itself
- `edgesValid` — edges reference existing tensions
- `statusValid` — status is always Active, Resolved, or Released
- `siblingPositionsUnique` — no two positioned siblings share a position

**`strongInvariant`** (includes containment — detected but not prevented in Rust):
- All of `systemInvariant` plus `noContainmentViolations`

## Usage

```bash
# Typecheck
quint typecheck specs/werk.qnt

# Simulate (1000 random traces, check core invariant)
quint run specs/werk.qnt --main=werk --max-samples=1000 \
  --invariant=systemInvariant --backend=typescript

# Find containment violations (expected to find them)
quint run specs/werk.qnt --main=werk --max-samples=1000 \
  --invariant=strongInvariant --backend=typescript

# Interactive REPL
quint -r specs/werk.qnt::werk

# Typecheck individual modules
quint typecheck specs/tension.qnt
quint typecheck specs/forest.qnt
```

## Relationship to Rust code

These specs are *above* the implementation — they define what must hold,
not how it's achieved. The mapping:

| Quint | Rust |
|-------|------|
| `types::Tension` | `sd_core::tension::Tension` |
| `types::TensionStatus` | `sd_core::tension::TensionStatus` |
| `types::Edge` | `sd_core::edge::Edge` |
| `types::Horizon` | `sd_core::horizon::Horizon` |
| `tension::canTransition` | `Tension::resolve/release/reopen` |
| `forest::singleParent` | `TreeError::CircularReference` check |
| `timeCalculus::urgency` | `temporal::compute_urgency` |
| `timeCalculus::hasContainmentViolation` | `temporal::ContainmentViolation` |
| `werk::systemInvariant` | Distributed across action guards in `engine.rs` |
