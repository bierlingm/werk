# werk

An operative instrument for structural dynamics practice.

## What werk is

werk holds **structural tensions** — the gap between what you want and what's true. You declare a desired outcome, state the current reality, and the instrument holds the pair. The gap between them is not a problem to solve — it is a force that, when held honestly, drives resolution.

Each tension carries a **theory of closure**: composed action steps that bridge from reality to desire. These are hypotheses — conjectured, ordered, revisable. As steps get resolved, the **frontier of action** advances. The instrument computes temporal facts (urgency, critical path, sequencing pressure) from your deadlines and ordering. It surfaces signals by exception: silence is the default.

werk does not tell you what to do. It does not interpret your patterns, diagnose your psychology, or compute dynamics like phase, tendency, or conflict. Those belong to the practice — the human (possibly aided by AI) or his coach reading the structure. The instrument holds the honest record. Interpretation is yours.

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

30 tools organized as:
- **Read** (11) — show, tree, list, survey, health, ground, diff, context, trajectory, insights, epoch_show
- **Gesture** (14) — add, compose, reality, desire, resolve, release, reopen, move, hold, position, horizon, rm, snooze, recur
- **Note** (3) — note_add, note_rm, note_list
- **Epoch** (2) — epoch, epoch_list
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

## Current tension tree

```
└── #2 [2026-06] werk is a mature tool for practicing st...  [6/13] (3 released)
    ├── #3 [2026-05-30] PRESSURE werk is a FrankenTUI-fir...  [4/8] (2 released)
    │   ├── #15 TUI rebuilt around the operating envelope as primary i...  [0/3]
    │   │   ├── #18 survey view designed and implemented — temporal framing,...
    │   │   ├── #19 threshold mechanics implemented — tap/hold navigation, l...
    │   │   └── #58 pathway palettes in TUI — inline option sets presented v...
    │   ├── #45 TUI yank — copy tension snapshots to clipboard for handoff t...
    │   ├── #51 werk has a proper documentation surface — reference docs for...
    │   └── #76 TUI architectural consolidation — unified cursor, sing...  [1/2]
    │       └── #94 focus zoom shows a detail card (full desire/reality text...
    ├── #13 [2026-06] the conceptual foundation is implemented — data...  [4/12]
    │   ├── #16 complete state machine specification — all states, transitio...
    │   ├── #20 data model extended — gesture grouping (gesture table)...  [2/3]
    │   │   └── #35 epoch and session lifecycle are fully operational
    │   ├── #30 epoch creation has a trigger path — CLI or TUI prompts 'star...
    │   ├── #46 threshold detection as independent structural signal logic —...
    │   ├── #49 structural transformation gestures (split, merge, restructur...
    │   ├── #65 observational analysis stance is decided — clear boundary be...
    │   ├── #87 test
    │   └── #88 test
    ├── #4 a clear structural model exists for how multiple participan...  [0/2]
    │   ├── #5 [2026-06] the Foundations for Structural Thinking course has ...
    │   └── #34 a public web surface exists where others can view (par...  [0/1]
    │       └── #93 werk ships as a Tauri desktop app — the web frontend wra...
    ├── #10 CLI is a complete, honest interface to the structure — fou...  [6/9]
    │   ├── #48 staging mechanism — propose/pending/confirm/reject for async...
    │   ├── #52 CLI is ergonomic, forgiving, and self-documenting — accepts ...
    │   └── #54 batch positioning exists — reorder multiple tensions in one ...
    ├── #36 werk has a sustainable business model that funds continued...  [0/7]
    │   ├── #37 werk is publicly known as the structural intent layer ...  [0/2]
    │   │   ├── #41 first public post about werk published — X post with Rem...
    │   │   └── #42 coherence offering designed — one-liner or agent prompt ...
    │   ├── #38 the open-format / proprietary-instrument split is implemente...
    │   ├── #39 at least one revenue stream is active and generating income
    │   ├── #40 werk integrates naturally with Statecraft services, Mo...  [0/3]
    │   │   ├── #66 Statecraft integration point is defined — what a client ...
    │   │   ├── #67 Modern Minuteman operations use werk as the planning ins...
    │   │   └── #68 Spikes feedback loops connect to werk tensions — how aud...
    │   ├── #62 Waterlight collaboration has a clear shape — what Mist gets ...
    │   ├── #69 first $1000 earned through practical means — proving s...  [0/3]
    │   │   ├── #70 setup calls offered — install werk, walk through structu...
    │   │   ├── #71 something obviously valuable is built and priced — a pac...
    │   │   └── #72 public presence exists sufficient for donations/sponsors...
    │   └── #78 werk is the standard practice tool for structural dyna...  [0/3]
    │       ├── #79 Fritz's discontinued tool is researched — what it was, w...
    │       ├── #80 SD practitioners and coaches have validated werk as usef...
    │       └── #81 werk is packaged and priced for coaches — per-seat, per-...
    ├── #82 werk has a graphical interface — browser or native Mac app...  [1/4]
    │   ├── #84 the GUI reaches feature parity with the core practice loop —...
    │   ├── #85 the GUI has visual design that expresses structural dynamics...
    │   └── #95 werk has a mobile app — iOS and/or Android — that brings the...
    └── #90 the root level is a command center — structural overview, ...  [0/1]
        └── #89 logbase — a searchable, queryable substrate of all prior des...

Total: 91  Active: 53  Resolved: 32  Released: 6
```

## License

MIT OR Apache-2.0
