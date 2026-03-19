# CLI Optimization for Agent Interaction

## Status: Research / Findings

**Date:** 2026-03-18
**Context:** The TUI has evolved significantly with a rich vocabulary (tensions, phases, horizons, gaze depths, structural tendency) while the CLI help text remains generic and clap-default. An agent encountering `werk` for the first time gets minimal orientation. This document analyzes the gap and proposes concrete improvements.

---

## 1. Current State: What an Agent Sees on First Contact

Running `werk --help` produces:

```
Operative instrument for structural dynamics

Usage: werk [OPTIONS] <COMMAND>

Commands:
  init        Initialize a workspace
  config      Get or set configuration values
  add         Create a new tension
  horizon     Set or display the temporal horizon
  show        Display tension details
  reality     Update the actual state
  desire      Update the desired state
  resolve     Mark a tension as resolved
  release     Release a tension
  reopen      Reopen a resolved or released tension
  snooze      Snooze a tension until a future date
  recur       Set or clear a recurrence interval
  rm          Delete a tension
  move        Reparent a tension
  note        Attach a narrative annotation
  notes       List notes
  health      Show system health summary
  insights    Show behavioral pattern insights
  diff        Show what changed in a time window
  list        List tensions with filtering
  tree        Display the tension forest as a tree
  trajectory  Show structural trajectory projections
  context     Output structural context for agent consumption
  run         Launch an agent with structural context
  watch       Monitor tension dynamics
  batch       Batch operations
  nuke        Destroy the current workspace

Options:
  -j, --json   Output in JSON format
  -h, --help   Print help
  -V, --version  Print version
```

### Problems for Agent Consumption

1. **No conceptual grounding.** "Operative instrument for structural dynamics" tells an agent nothing about what tensions are, what the data model looks like, or what workflow to follow. A human might read documentation; an agent needs it inline.

2. **Flat command list with no grouping.** 26 commands in alphabetical order. An agent cannot distinguish CRUD commands from analysis commands from agent-integration commands. The TUI groups interactions into clear acts (add, edit, resolve, release) vs. navigation vs. information depth — the CLI has no analogous structure.

3. **No workflow guidance.** The TUI enforces a natural flow: descend into a tension, gaze at it, study its dynamics, then act. The CLI presents all 26 commands as equally weighted options with no sense of sequence.

4. **`--json` is buried.** The most important flag for agent interaction is mentioned as one of two options at the bottom. An agent doesn't know that `--json` transforms every command into structured, parseable output.

5. **No mention of `context` or `batch` as the agent-facing commands.** These are the primary programmatic interfaces, but they sit in the same list as `nuke` and `snooze`.

6. **Help text uses terms without definition.** "Phase (G, A, C, M)" in `list --phase` — what do these mean? "Temporal horizon" — how does this differ from a deadline? The TUI has a complete visual language (glyphs, colors, temporal indicators) that maps to these concepts; the CLI just names them.

---

## 2. TUI Concepts Not Reflected in CLI

The TUI has matured a rich vocabulary that the CLI doesn't surface:

| TUI Concept | CLI Equivalent | Gap |
|---|---|---|
| **Tension** (desired ↔ actual gap) | `add`, `show` | CLI says "tension" but never explains the gap model |
| **Phase** (Germination → Assimilation → Completion → Momentum) | `list --phase G/A/C/M` | Single-letter codes with no explanation |
| **Tendency** (Advancing / Oscillating / Stagnant) | Only in `context` JSON | Not filterable or visible in `list` |
| **Horizon** (temporal aim, not deadline) | `horizon` command | Help says "temporal horizon" but doesn't explain the precision semantics (Year/Month/Day) |
| **Gaze** (three depths of information) | `show` (one depth) | No equivalent of Depth 1 (children + reality) vs Depth 2 (full dynamics + history) |
| **Tension chart** (positioned siblings bridging reality → desire) | `tree` (flat forest view) | No concept of positioned vs unpositioned ordering |
| **Temporal indicator** (six-dot window) | Only in TUI rendering | Not available in CLI human output |
| **Alerts** (neglect, oscillation, conflict, horizon past) | `health` (aggregate only) | No per-tension alert display |
| **Parent snapshots / divergence** | Not exposed | Only in TUI gaze |
| **Trunk line** (visual bridge reality → desire) | Not rendered | Only in TUI descended view |

---

## 3. Agent Integration Surface: Current vs. Ideal

### What Exists

| Feature | Status | Notes |
|---|---|---|
| `--json` on all commands | Working | Every command outputs structured JSON |
| `context <id>` | Working | Rich JSON with tension, ancestors, siblings, children, dynamics, mutations, projection |
| `context --all` | Working | Bulk context for all active tensions |
| `context --urgent` | Working | Filtered to urgency > 0.75 |
| `run <id> "prompt"` | Working | One-shot agent invocation with context |
| `run <id> -- <cmd>` | Working | Interactive agent with context piped to stdin + env vars |
| `run --system "prompt"` | Working | System-wide agent invocation |
| `run <id> --decompose` | Working | Auto-decomposition into sub-tensions |
| `batch apply <file>` | Working | YAML mutation application |
| `batch validate <file>` | Working | Dry-run validation |
| `WERK_TENSION_ID` env var | Working | Set in interactive mode |
| `WERK_CONTEXT` env var | Working | Full JSON context in interactive mode |
| `WERK_WORKSPACE` env var | Working | Path to .werk directory |
| Structured YAML response parsing | Working | Agents can return mutations in YAML |
| Exit codes (0/1/2) | Working | Success / user error / internal error |
| JSON error output | Working | `--json` errors include `code` and `message` |

### What's Missing for Robust Agent Workflows

1. **Machine-readable schema discovery.** No `werk schema` or `werk help --json` that would let an agent understand the full command surface programmatically. Currently an agent must parse human help text.

2. **Idempotency information.** An agent doesn't know which commands are safe to retry vs. which create duplicates. `add` creates a new tension every time; `reality` is idempotent if the value hasn't changed.

3. **No `--quiet` / `--silent` mode.** Some commands emit human-friendly messages ("✓ Tension created") that pollute stdout when an agent is parsing JSON output. The `--json` flag handles this, but the agent must know to use it.

4. **No completion/verification loop.** After `batch apply`, the agent gets a count of applied/failed mutations but no way to verify the resulting state without a second `context` call.

5. **Agent-specific help.** `werk --help` is oriented toward a human at a keyboard. An agent needs: "Here is the data model. Here are the commands grouped by function. Here is the typical workflow. Here is how to read/write data programmatically."

---

## 4. Proposed Improvements

### 4.1 Restructured Help Text with Command Groups

Replace the flat command list with grouped sections using clap's `help_heading`:

```
Operative instrument for structural dynamics.

werk manages structural tensions — gaps between desired and actual states.
Each tension has a lifecycle (Active → Resolved/Released), a creative phase
(Germination → Assimilation → Completion → Momentum), and optional temporal
horizons that create urgency.

Usage: werk [OPTIONS] <COMMAND>

Create & Modify:
  add         Create a new tension (desired + actual states)
  desire      Update the desired state of a tension
  reality     Update the actual state (confront reality)
  horizon     Set temporal aim (year/month/day precision)
  move        Reparent a tension to a new parent
  rm          Delete a tension (reparents children)

Lifecycle:
  resolve     Mark a tension as resolved (gap closed)
  release     Release a tension (consciously abandon)
  reopen      Reopen a resolved or released tension
  snooze      Snooze until a future date
  recur       Set a recurrence interval

Observe:
  show        Display tension details and dynamics
  list        List tensions with filtering and sorting
  tree        Display the tension forest hierarchy
  health      System health summary (phases, alerts)
  diff        Show what changed in a time window
  insights    Behavioral pattern analysis
  trajectory  Structural trajectory projections
  notes       List annotations

Annotate:
  note        Attach a narrative annotation

Agent Integration:
  context     Output structural context as JSON
  run         Launch an agent with structural context
  watch       Monitor dynamics, invoke agent on thresholds
  batch       Apply/validate mutations from YAML

Workspace:
  init        Initialize a workspace (.werk/ directory)
  config      Get or set configuration values
  nuke        Destroy the current workspace

Options:
  -j, --json   Output ALL commands as structured JSON (essential for agents)
  -h, --help   Print help
  -V, --version  Print version

Agent Quick Start:
  werk context --all --json    Read the full structural field
  werk batch apply - --json    Apply mutations from stdin (YAML)
  werk list --json             List all active tensions as JSON
```

### 4.2 `werk guide` Command (Agent Orientation)

A new command that outputs the conceptual model as structured text or JSON — the equivalent of the TUI's design document, but machine-readable:

```
werk guide              # Human-readable overview
werk guide --json       # Machine-readable schema + concepts
werk guide --agent      # Compact prompt suitable for agent system messages
```

The `--agent` variant would output a single block of text (< 2000 tokens) that an agent can include in its system prompt, covering:
- What tensions are (desired/actual gap)
- The three statuses (Active, Resolved, Released)
- The four phases (G, A, C, M) and what they mean
- Horizons and urgency
- The mutation types available
- The YAML response format
- The key commands for reading and writing

This replaces the need for agents to discover the model by trial and error.

### 4.3 Align `show` with TUI Gaze Depths

Currently `show` outputs a fixed set of fields. Align it with the TUI's three depths:

```
werk show <id>              # Depth 0: one-line summary (glyph + name + horizon + temporal indicator)
werk show <id> --gaze       # Depth 1: children preview + reality + phase
werk show <id> --study      # Depth 2: full dynamics + mutation history
werk show <id> --json       # All depths as structured JSON (same as context, essentially)
```

This makes the CLI's information architecture match the TUI's, and gives agents three levels of detail to choose from.

### 4.4 Surface Per-Tension Alerts in `list` and `show`

The TUI computes alerts (neglect, oscillation, conflict, horizon_past, multiple_roots) on every load. The CLI's `health` command shows aggregate health but doesn't surface per-tension alerts.

Add alert information to:
- `list --json` output (include alerts array per tension)
- `show` output (display alerts)
- `context` output (already has dynamics, but alerts are computed separately in TUI)

### 4.5 `--porcelain` Flag for Stable Machine Output

Inspired by git's `--porcelain` flag: a stable, parseable output format that won't change between versions, suitable for scripts and agents. Distinct from `--json` in that it's line-oriented and minimal:

```
werk list --porcelain
# Output: TAB-separated fields, one tension per line
# id\tstatus\tphase\ttendency\turgency\tdesired\tactual
```

This is lighter than full JSON for agents that just need to scan the field.

### 4.6 Vocabulary Consistency

Align CLI terminology with TUI throughout:

| Current CLI Term | TUI Term | Proposed CLI Change |
|---|---|---|
| "actual state" | "reality" | Use "reality" in help text |
| "movement" (in context JSON) | "tendency" | Rename to "tendency" |
| "name" (in list output) | "desired" | Already correct, but `list` human output calls it "name" |
| No term | "the field" | Root-level view should be called "the field" in help |
| No term | "descended view" | Entering a parent's children should use this language |

### 4.7 Explicit `--json` Documentation Per Command

Each command's `--help` should note what its JSON output shape looks like:

```
werk list --help
...
  -j, --json   Output as JSON array of objects:
               [{id, desired, actual, status, phase, tendency, urgency, horizon, children_count}]
```

This removes guesswork for agents.

---

## 5. Priority Ranking

| Priority | Change | Effort | Impact for Agents |
|---|---|---|---|
| **P0** | `werk guide --agent` command | Medium | Eliminates cold-start problem entirely |
| **P0** | Grouped help text with command headings | Low | Immediate orientation improvement |
| **P1** | Vocabulary alignment (tendency, reality, field) | Low | Reduces confusion between CLI and TUI |
| **P1** | Per-tension alerts in `list --json` and `show` | Medium | Agents can identify what needs attention |
| **P2** | `show --gaze` / `show --study` depth levels | Medium | Matches TUI information architecture |
| **P2** | JSON shape documentation in `--help` | Low | Removes guesswork |
| **P3** | `--porcelain` flag | Medium | Useful for scripts, less critical with `--json` |

---

## 6. `werk guide --agent` Draft Content

Below is a draft of what the compact agent orientation text might contain:

```
werk is a structural dynamics instrument. It manages tensions — gaps between
a desired state and current reality.

CORE MODEL:
- Tension: { id, desired, actual, status, horizon, parent_id }
- Status: Active (gap exists) | Resolved (gap closed) | Released (abandoned)
- Phase: Germination (new) → Assimilation (working) → Completion (closing) → Momentum (post-resolution energy)
- Tendency: Advancing (moving toward resolution) | Oscillating (back-and-forth) | Stagnant (no movement)
- Horizon: temporal aim with precision levels (Year/Month/Day). Not a deadline — the time scale the tension lives in.
- Hierarchy: tensions can have children (sub-tensions). Parent + children form a "tension chart" bridging reality to desire.

READ STATE:
  werk list --json                    # All active tensions
  werk list --json --all              # Including resolved/released
  werk context <id> --json            # Full context for one tension (dynamics, mutations, projection)
  werk context --all --json           # Full context for all active tensions
  werk tree --json                    # Hierarchical forest view
  werk health --json                  # System-wide health summary
  werk diff --json --since yesterday  # Recent changes

WRITE STATE:
  werk add "desired" "actual"                    # Create tension
  werk add "desired" "actual" -p <parent_id>     # Create child tension
  werk reality <id> "new reality"                # Update reality
  werk desire <id> "new desire"                  # Update desire
  werk horizon <id> "2026-05"                    # Set temporal aim
  werk resolve <id>                              # Mark resolved
  werk release <id> -r "reason"                  # Release (abandon)
  werk move <id> -p <new_parent>                 # Reparent
  werk note <id> "annotation text"               # Add note

BATCH MUTATIONS (preferred for agents):
  echo '<yaml>' | werk batch apply - --json

  YAML format:
  ---
  mutations:
    - action: update_actual
      tension_id: <id>
      new_value: "new reality"
      reasoning: "why"
    - action: create_child
      parent_id: <id>
      desired: "sub-goal"
      actual: "current state"
      reasoning: "why"
    - action: update_desired
      tension_id: <id>
      new_value: "revised desire"
      reasoning: "why"
    - action: update_status
      tension_id: <id>
      new_status: "Resolved"
      reasoning: "why"
    - action: set_horizon
      tension_id: <id>
      horizon: "2026-05"
      reasoning: "why"
    - action: move_tension
      tension_id: <id>
      new_parent_id: <id>
      reasoning: "why"
    - action: add_note
      tension_id: <id>
      text: "annotation"
  response: "summary of changes"
  ---

EXIT CODES: 0=success, 1=user error, 2=internal error
JSON ERRORS: {"error": {"code": "NOT_FOUND|INVALID_INPUT|...", "message": "..."}}
PREFIX MATCHING: tension IDs can be shortened to unique 4+ char prefixes
```

---

## 7. Implementation Notes

### Clap Command Groups
Clap supports `help_heading` on individual variants:
```rust
#[derive(Subcommand)]
enum Commands {
    #[command(help_heading = "Create & Modify")]
    Add { ... },

    #[command(help_heading = "Lifecycle")]
    Resolve { ... },

    #[command(help_heading = "Agent Integration")]
    Context { ... },
}
```

This is the lowest-effort change with the highest immediate impact.

### Guide Command
A simple command that prints static text (no workspace needed). Three variants:
- Default: human-readable markdown
- `--json`: structured schema document
- `--agent`: compact prompt text (the draft above)

### Show Depth Alignment
`show` already computes most of this data. The change is presentation:
- Default: add phase glyph, temporal indicator to the existing output
- `--gaze`: add children list and reality prominently
- `--study`: add all 12 dynamics + mutation history

---

## 8. Relationship to Hooks Integration

The [hooks integration design](../research/hooks-integration-design.md) envisions agents being invoked by `watch` when thresholds are crossed. For this to work well, the agent must understand the data it receives. The `guide --agent` command directly supports this: the watch daemon can include the guide text in its agent prompts, ensuring any invoked agent has full conceptual grounding.

The batch interface is already the right shape for hook-invoked agents to write back mutations. The missing piece is the conceptual orientation — which this document addresses.
