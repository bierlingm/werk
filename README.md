# werk

An operative instrument for the gap between desire and reality.

You want things. You have a reality. The gap between them is a **tension** — and tensions, when held honestly, drive resolution. werk is the instrument for holding them.

## What it does

werk tracks **structural tensions** — pairs of desired outcomes and current realities. You declare what you want and what's true. The gap between them generates energy for action. The theory of how to close each gap is composed of **action steps** — ordered, deadlined, held, resolved, or released over time.

The instrument computes temporal dynamics from your deadlines and ordering: urgency, sequencing pressure, critical path, overdue state. It does not guess at your psychology. It tells you what's structurally true about your commitments and their temporal relationships.

Then it hands that structure to an AI agent — and the agent serves your declared intentions. Not the reverse.

## Install

```bash
# With Rust nightly:
git clone https://github.com/bierlingm/werk
cargo install --path werk/werk-cli

# Without Rust:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly -y
source "$HOME/.cargo/env"
git clone https://github.com/bierlingm/werk
cargo install --path werk/werk-cli
```

## Quick start

```bash
werk init                                          # create a workspace
werk add "Ship the novel" "42,000 words. Stuck."   # declare a tension
werk                                               # open the instrument
```

## Core concepts

**Tension** — a desire-reality pair. The gap generates energy for creative action.

**Theory of closure** — the action steps you compose to bridge from reality to desire. A conjecture about how to cross the gap, subject to revision as you learn.

**Frontier of action** — where the present moment meets your theory. What's next, what's overdue, what's recently accomplished. The instrument centers on this.

**Gesture** — the unit of meaningful action. Creating, resolving, releasing, updating reality, noting, reordering. Each is a compression of intentionality into enacted form.

**Deadline** and **order of operations** — the two temporal primitives you set. Everything else (urgency, implied execution windows, sequencing pressure, critical path) is computed from these.

## The TUI

Run `werk` with no arguments to open the instrument.

The TUI is undergoing a fundamental rebuild around the **operating envelope** — a primary interaction surface centered on the frontier of action. See `designs/werk-conceptual-foundation.md` for the full architectural vision.

## CLI

```bash
werk add "desired" "actual"           # create a tension
werk add -p <id> "desired" "actual"   # create a child (action step)
werk reality <id> "what's true now"   # update reality (a narrative beat)
werk desire <id> "what you want"      # evolve desire
werk resolve <id>                     # mark as accomplished
werk release <id> --reason "why"      # let go
werk note <id> "observation"          # annotate
werk tree                             # see the whole structure
werk show <id>                        # full details on one tension
werk list                             # list with urgency and phase
werk health                           # structural health summary
```

## Agent integration

```bash
werk run <id> "prompt"                # one-shot agent with full structural context
werk context <id>                     # JSON export for any agent
werk watch                            # background daemon monitoring dynamics
```

Press `@` in the TUI to invoke an agent scoped to the selected tension. The agent receives the full structural context and proposes mutations the user reviews before applying.

## Architecture

```
sd-core          Structural dynamics engine (Rust)
  ├── tension    Core data model
  ├── mutation   Append-only change log
  ├── store      SQLite persistence
  ├── dynamics   Computed structural properties
  ├── engine     Orchestration + event emission
  └── horizon    Temporal horizons with variable precision

werk-cli         The practitioner's command-line toolkit
werk-tui         The operative instrument (terminal UI, built on ftui)
werk-shared      Shared configuration and workspace management
```

## Conceptual foundation

The instrument is being redesigned around four frameworks:

1. **Architecture of Space** — navigational dimensions, the one spatial law (desired above actual), stream and survey views
2. **Grammar of Action** — operative gestures, state machine, key bindings per state
3. **Calculus of Time** — deadline and order as primitives, urgency and critical path as computed properties
4. **Logic of Framing** — what's visible and actionable given context (operating envelope, thresholds, zoom)

Full specification: [`designs/werk-conceptual-foundation.md`](designs/werk-conceptual-foundation.md)

## Theoretical foundation

Based on [Robert Fritz's structural dynamics](https://www.robertfritz.com/resources/) — the principle that the gap between desired outcome and current reality creates structural tension that, when held honestly, drives genuine resolution. Influenced by Nicholas Catton's coaching methodology, Miguel A. Fernandez's operative traditions, and the Napoleonic field-of-opportunities strategy.

## Build & test

```bash
cargo build                     # full workspace
cargo test                      # all tests
cargo install --path werk-cli   # install to PATH
```

Requires [Rust nightly](https://rustup.rs/). TUI built on [ftui](https://crates.io/crates/ftui).

## License

MIT OR Apache-2.0
