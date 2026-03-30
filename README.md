# werk

An operative instrument for structural dynamics practice.

## What werk is

werk holds **structural tensions** — the gap between what you want and what's true. You declare a desired outcome, state the current reality, and the instrument holds the pair. The gap between them is not a problem to solve — it is a force that, when held honestly, drives resolution.

Each tension carries a **theory of closure**: composed action steps that bridge from reality to desire. These are hypotheses — conjectured, ordered, revisable. As steps get resolved, the **frontier of action** advances. The instrument computes temporal facts (urgency, critical path, sequencing pressure) from your deadlines and ordering. It surfaces signals by exception: silence is the default.

werk does not tell you what to do. It does not interpret your patterns, diagnose your psychology, or compute dynamics like phase, tendency, or conflict. Those belong to the practice — the human (possibly aided by AI) or his coach reading the structure. The instrument holds the honest record. Interpretation is yours. Every computation the instrument produces is anchored to a standard of measurement you explicitly provide — a deadline, an ordering, an articulated desire. No user-supplied standard, no instrument-generated inference.

werk is operative, not managerial. It serves operations — closing gaps between where you are and where you aim. It does not track dependencies between tensions, enforce permissions, or manage coordination topology. It works best in high-trust, reality-facing contexts: individual practitioners, small teams, collaborators who share aims and are willing to face what's actually true.

Based on [Robert Fritz's structural dynamics](https://www.robertfritz.com/resources/) with influence from Miguel A. Fernandez's work on operative traditions.

## Install

```bash
git clone https://github.com/bierlingm/werk && cd werk
cargo install --path werk-cli
```

Requires [Rust](https://rustup.rs/).

## Quick start

```bash
werk init                                            # create a workspace
werk add "Novel is drafted" "42,000 words. Stuck."   # declare a tension
werk                                                 # open the instrument
```

The instrument stores its data in a `.werk/` directory at your workspace root.

## Three interfaces

werk exposes every gesture through three surfaces. Same mutations, same facts — different modes of engagement.

### TUI

```bash
werk
```

The primary experience. You inhabit the structure. The TUI centers on the **operating envelope** — a window around the frontier of action showing what's overdue, what's next, what's held, and what was recently accomplished. This is where you land on opening.

### CLI

Every gesture available as a command. Human-readable text by default, structured JSON with `--json`.

All commands accept tension IDs as `#23` (shorthand), `01ARZ3N4` (ULID prefix, 4+ chars), or full ULID.

```bash
# Structure
werk add "desired" "actual"                    # declare a new tension
werk add -p <id> "desired" "actual"            # add a step to the theory of closure
werk compose <id> [<id>...]                    # create a parent above existing tensions
werk desire <id> "new desire"                  # evolve what you're aiming at
werk reality <id> "new reality"                # record what's actually true now
werk horizon <id> "2026-04"                    # set a deadline (day, month, quarter, or year)
werk note <id> "observation"                   # attach a note — context that isn't a state change

# State changes
werk resolve <id>                              # mark the gap as closed
werk release --reason "why" <id>               # let go of a tension (requires a reason)
werk reopen <id>                               # bring back a resolved or released tension

# Organizing
werk move <id> <new-parent-id>                 # reparent a tension
werk position <id> <pos>                       # set sequence position (1 = first)
werk hold <id>                                 # unposition — acknowledged but uncommitted
werk snooze <id> "+3d"                         # hide until a future date
werk recur <id> "+1w"                          # auto-reopen on an interval
werk rm <id>                                   # delete (children move to grandparent)

# Reading
werk show <id>                                 # full detail — state, children, signals, history
werk tree                                      # all active tensions as a hierarchy
werk list [--all|--urgent|--neglected]          # flat list with filtering and sorting
werk survey                                    # temporal view across all tensions
werk diff                                      # what changed recently
werk health                                    # structural statistics and alerts
werk ground                                    # field-wide debrief — engagement, epochs, gestures
werk insights                                  # behavioral facts — attention, postponement, activity
werk context <id>                              # structural context as JSON (for scripts/agents)
werk trajectory                                # projected completion and urgency collisions

# Epochs (structural snapshots)
werk epoch <id>                                # mark an epoch boundary (snapshots desire + reality)
werk epoch <id> --list                         # list all epochs for a tension
werk epoch <id> --show <n>                     # show what happened during epoch N
```

### MCP server

```bash
werk mcp
```

Protocol surface for AI agents. Starts an [MCP](https://modelcontextprotocol.io/) server on stdio transport, exposing every gesture as a typed tool. Direct library calls — no subprocess overhead.

31 tools organized as:
- **Read** (5 primary) — show (with full context), tree, list (rich filtering), stats (field aggregates), survey
- **Read** (6 legacy) — health, ground, diff, context, trajectory, insights — still functional, superseded by show/list/stats
- **Gesture** (14) — add, compose, reality, desire, resolve, release, reopen, move, hold, position, horizon, rm, snooze, recur
- **Note** (3) — note_add, note_rm, note_list
- **Epoch** (3) — epoch, epoch_list, epoch_show
- **Batch** (1) — apply mutations from YAML

#### Connecting from Claude Code

```bash
claude mcp add werk -- werk mcp
```

Or in `.claude/settings.json`:

```json
{
  "mcpServers": {
    "werk": {
      "command": "werk",
      "args": ["mcp"]
    }
  }
}
```

Any MCP client (Claude Desktop, Cursor, or custom harnesses) connects the same way — point at `werk mcp` on stdio.

## Core concepts

**Tension** — a desire-reality pair. The gap between them generates energy for creative action.

**Theory of closure** — the action steps composed to bridge from reality to desire. Each step is a hypothesis about what's needed. The theory is revisable — steps can be wrong, reordered, replaced.

**Frontier of action** — where accomplished meets remaining. The present moment's position in the order of operations.

**Operating envelope** — the window around the frontier containing everything action-relevant right now. The primary interaction surface.

**Gesture** — the unit of meaningful action. One gesture may involve multiple mutations. Gestures are the meaningful units for undo, history, and structural interpretation.

**Epoch** — a period of action within a stable desire-reality frame. When desire transforms or reality shifts significantly, the current epoch closes and a new one opens. The sequence of epochs forms the tension's **log**.

**Deadline** and **order of operations** — the two temporal primitives you set. Everything else (urgency, execution windows, sequencing pressure, critical path) is computed from these.

## Architecture

```
sd-core          Structural dynamics engine (Rust library)
  ├── tension    Desire-reality pairs, status, children
  ├── mutation   Append-only change log with gesture grouping
  ├── store      SQLite persistence
  ├── temporal   Urgency, horizon drift, critical path, sequencing pressure
  ├── frontier   Frontier of action, operating envelope computation
  ├── engine     Workspace operations + event emission
  └── horizon    Variable-precision temporal horizons

werk-shared      Configuration, workspace discovery, hooks, prefix resolution
werk-cli         Command-line interface
werk-tui         Terminal UI
werk-mcp         MCP server for AI agents
```

## Conceptual foundation

The instrument is organized around four frameworks:

1. **Architecture of Space** — the one spatial law (desired above actual), dimensions, positions, limits
2. **Grammar of Action** — gesture primitives, state machine, key bindings per state
3. **Calculus of Time** — two user-set primitives (deadline, order), six computed properties, two recorded facts
4. **Logic of Framing** — what's visible and actionable given context (envelope, zoom, thresholds)

Full specification: [`designs/werk-conceptual-foundation.md`](designs/werk-conceptual-foundation.md)

## Build and test

```bash
cargo build                     # full workspace
cargo test                      # all tests
cargo clippy                    # lint
cargo install --path werk-cli   # install to PATH
```

## Current state

Run `werk tree` for the live tension hierarchy. The instrument tracks its own development.

## License

MIT OR Apache-2.0
