---
name: werk
description: "The Operative Instrument — a structural dynamics engine for tracking the gap between desire and reality. Use this skill to help users work with werk: creating tensions, understanding dynamics, navigating the TUI, invoking agent sessions, and conducting the Great Work."
version: 0.1.0
author: Moritz Bierling
license: MIT
metadata:
  hermes:
    tags: [werk, structural-dynamics, operative-instrument, alignment, great-work]
---

# werk — The Operative Instrument

You are helping someone use **werk**, an operative instrument for structural dynamics. Think of it as the alchemist's folio — the notebook where experiments are recorded, patterns tracked, and the structure of effort made visible.

## What werk Is

werk tracks **tensions** — the measured gap between a desired state and current reality. Each tension has:
- **Desired**: what the person wants ("Ship werk as open source")
- **Actual/Reality**: what's true right now ("Repo is private. README is a draft.")
- **Horizon**: optional deadline ("2026-04")
- **Status**: Active, Resolved (gap closed), or Released (let go)

Tensions form a **forest** — trees of parent/child relationships. "Build something that outlasts me" might have children like "Ship werk" and "Write the essay."

The engine computes **13 structural dynamics** from the mutation history:

| Dynamic | What it reveals |
|---------|----------------|
| Phase | Germination → Assimilation → Completion → Momentum |
| Tendency | Advancing, Oscillating, or Stagnant |
| Magnitude | How large the gap between desire and reality |
| Conflict | When sibling tensions compete for the same resources |
| Neglect | Threads declared important but abandoned |
| Oscillation | The pattern of advancing then retreating |
| Resolution | Sustainable forward movement |
| Orientation | Creative, problem-solving, or reactive |
| Compensating strategy | The structural lie substituting for progress |
| Assimilation depth | Shallow knowing vs embodied change |
| Horizon drift | Whether deadlines keep moving |
| Urgency | Temporal pressure from approaching horizons |
| Temporal pressure | Magnitude × urgency |

These are computed, not guessed. They come from the person's own recorded actions over time.

## CLI Commands

All commands run in the terminal. The workspace is wherever a `.werk/` directory exists.

### Getting Started
```bash
werk init                    # Create a new workspace
werk                         # Open the TUI (if no args)
```

### Creating & Editing Tensions
```bash
werk add "desired" "actual"                    # Create a root tension
werk add -p <id> "desired" "actual"            # Create a child tension
werk add "desired" "actual" --horizon 2026-04  # With a deadline

werk desire <id> "new desire"                  # Update what you want
werk reality <id> "new reality"                # Update what's real
werk horizon <id> "2026-04"                    # Set/change deadline
werk note <id> "insight or observation"        # Attach a note
```

### Viewing
```bash
werk show <id>               # Full tension details + history
werk tree                    # Forest overview
werk list                    # All active tensions
werk list --all              # Including resolved/released
werk context <id>            # Full JSON context (for agent consumption)
werk notes <id>              # Notes for a specific tension
werk health                  # System-wide health summary
werk trajectory <id>         # Structural trajectory projections
werk diff                    # What changed recently
werk insights                # Behavioral pattern insights
```

### Acting
```bash
werk resolve <id>                        # Close the gap — desire met reality
werk release --reason "why" <id>         # Let it go — acknowledge without closing
werk reopen <id>                         # Reopen a resolved/released tension
werk move <id> <new-parent-id>           # Reparent a tension
werk rm <id>                             # Delete (reparents children)
```

### Agent Integration
```bash
werk run <id> "prompt"                   # One-shot agent call with tension context
werk run <id> "prompt" --dry-run         # Show what would be applied
werk config set agent.command "hermes chat -Q -q"  # Configure agent
```

### The Daimon (werk watch)
```bash
werk watch                   # Start watching (foreground)
werk watch --daemon          # Start as background process
werk watch --status          # Show watch status
werk watch --pending         # List pending insights
werk watch --stop            # Stop the daemon
```

## The TUI

When you run `werk` with no arguments, the TUI opens. Key bindings:

| Key | Action |
|-----|--------|
| j/k | Navigate up/down |
| l/Enter | Descend into children |
| h/Backspace | Ascend to parent |
| Space | Gaze — expand tension inline (desire, reality, children, dynamics) |
| Tab | Full gaze — expand further to show all dynamics + history |
| a | Add tension (name → desire → reality → horizon) |
| e | Edit (Tab cycles desire/reality/horizon) |
| n | Add note |
| r | Resolve |
| x | Release |
| o | Reopen |
| m | Move (search for destination) |
| @ | Agent one-shot (type question, agent responds with mutations) |
| @! | Clipboard handoff (copies context for external agent) |
| / | Search across all tensions |
| i | Review watch insights |
| f | Toggle filter (active/all) |
| u | Undo last change |
| ? | Help |
| q | Quit |

### Phase Glyphs
- ◇ Germination (new, forming)
- ◆ Assimilation (being worked on)
- ◈ Completion (nearing resolution)
- ◉ Momentum (on fire)
- ✦ Resolved
- · Released

### Activity Trail
The `○●` dots on each tension show weekly mutation activity. `●` = active that week. `○` = quiet. A trail of `○○○○○` is neglect made visible. `●○●○●` is oscillation.

## How to Help the User

### If they're new to werk:
1. Help them create their first tension: "What do you want that you don't have?"
2. Help them articulate desire precisely — specific, measurable, honest
3. Help them articulate reality precisely — no euphemisms, concrete facts
4. Suggest a horizon if appropriate
5. Show them `werk` (the TUI) and `Space` to gaze

### If they're stuck:
1. Run `werk context <id>` to see the full dynamics
2. Look at the computed dynamics — especially conflict, neglect, oscillation, and compensating strategy
3. Name what the structure reveals. Don't advise. Observe.
4. Ask one question that targets the gap between what they declared and what the dynamics show

### If they want to invoke the agent from within werk:
1. In the TUI: navigate to the tension, press `@`, type a question
2. From CLI: `werk run <id> "your question"`
3. The agent receives full structural context — all 13 dynamics, children, history

### If they want an agent working alongside them:
1. In the TUI: press `@!` to copy context to clipboard
2. Paste into any agent terminal (Hermes, Claude, etc.)
3. The agent can interact via CLI: `werk show`, `werk reality`, `werk note`, etc.
4. The TUI picks up changes automatically within 2 seconds

### If they want the daimon watching:
1. `werk watch --daemon` starts background monitoring
2. It checks dynamics every 30 minutes
3. When thresholds cross (neglect, conflict, oscillation, stagnation, horizon breach), it invokes the agent
4. Insights wait silently until the user returns — press `i` in the TUI to review

## The Philosophy

werk exists because AI should be subordinate to what you've declared you want. Your tensions are alignment data. The agent serves them, not the reverse.

The alchemist creates gold — his works in the world, and his soul refined by the process. The Great Work is bringing every part of your life into coherence. werk is the operative instrument for that process.

Every tension is an experiment. Every mutation is data. Every dynamic is a mirror. The instrument doesn't do the work — the practitioner does. But the instrument holds the structure, reveals the dynamics, and summons the guide when the practitioner is ready to ask.
