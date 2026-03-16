# werk — The Operative Instrument

A structural dynamics engine for the gap between desire and reality.

```
  ◇ Ship a tool that 1000 people use weekly              ○○●●○●●
  ◆ Complete a 50-mile ultramarathon                      ○○○○●●●
  ◇ Be engaged by end of year                             ○○○○○○○
  ◈ Have 48k in liquid savings                            ●○●○●○●
  ◉ Call mom every Sunday                                 ●●●●●●●
```

## What is this?

You want things. You have a reality. The gap between them is a **tension** — and tensions, when held honestly, drive resolution.

werk computes **13 structural dynamics** from the history of your declared tensions. Not vibes. Not sentiment analysis. Structural truth, computed from your own recorded actions over time.

Then it hands those dynamics to an AI agent — and the agent serves your declared intentions. Not the reverse.

## Install

Requires Rust nightly (edition 2024):

```bash
git clone https://github.com/bierlingm/werk-public && cd werk-public
cargo install --path werk-cli
```

## Quick start

```bash
werk init                                          # create a workspace
werk add "Ship the novel" "42,000 words. Stuck."   # declare a tension
werk                                               # open the instrument
```

## The TUI

Run `werk` with no arguments to open the Operative Instrument.

```
  ◇ Ship a tool that 1000 people use weekly...        ○○●●○●● ▸
  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  desire   Ship a tool that 1000 people use weekly to close
           the gap between what they want and what's real
  reality  werk has 652 passing tests, a TUI rewritten from
           scratch. Zero users besides me.
  horizon  2026-04 (18d)

    ◆ Ship werk as open source                        ○●●
    ◇ Write the essay on operative instruments        ○○○
    ◈ Win the Nous Research Hackathon                 ●●●

  gap      ████████████░░░░                           large
  conflict competing tensions
  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  ◆ Complete a 50-mile ultramarathon                  ○○○○●●●
  ◇ Be engaged by end of year                         ○○○○○○○
```

**Navigation** — descent through a forest of tensions:

| Key | Action |
|-----|--------|
| `j`/`k` | Move up/down |
| `l`/Enter | Descend into children |
| `h`/Backspace | Ascend to parent |
| Space | **Gaze** — expand inline (desire, reality, children, dynamics) |
| Tab | **Full gaze** — all 13 dynamics + mutation history |

**Acts** — every modification is deliberate:

| Key | Action |
|-----|--------|
| `a` | Add tension (name → desire → reality → horizon) |
| `e` | Edit (Tab cycles desire/reality/horizon) |
| `n` | Add note |
| `r` | Resolve (desire met reality) |
| `x` | Release (let go) |
| `o` | Reopen |
| `m` | Move (search for destination) |
| `@` | **Agent** — one-shot, tension-scoped |
| `y` | Copy tension ID to clipboard |
| `/` | Search across all tensions |
| `i` | Review watch insights |
| `?` | Help |

### Phase glyphs

- ◇ Germination — new, still forming
- ◆ Assimilation — being actively worked on
- ◈ Completion — nearing resolution
- ◉ Momentum — sustained forward motion
- ✦ Resolved — desire met reality
- · Released — let go

### Activity trail

The `○●` dots show weekly mutation activity. `●` = something changed that week. A trail of `○○○○○` is neglect made visible. `●○●○●` is oscillation — back and forth.

## The 13 Dynamics

All computed from mutation history. No self-reporting. No vibes.

| Dynamic | What it reveals |
|---------|----------------|
| **Phase** | Germination → Assimilation → Completion → Momentum |
| **Tendency** | Advancing, oscillating, or stagnant |
| **Magnitude** | How large the gap between desire and reality |
| **Conflict** | When sibling tensions compete for the same life |
| **Neglect** | Threads declared important but abandoned |
| **Oscillation** | The pattern of advancing then retreating |
| **Resolution** | Sustainable forward movement |
| **Orientation** | Creative, problem-solving, or reactive |
| **Compensating strategy** | The structural lie substituting for progress |
| **Assimilation depth** | Shallow knowing vs embodied change |
| **Horizon drift** | Whether deadlines keep moving |
| **Urgency** | Temporal pressure from approaching horizons |
| **Temporal pressure** | Magnitude × urgency |

## Agent Integration

### One-shot (`@` in TUI)

Press `@` on any tension, type a question. The agent receives the full structural context — all 13 dynamics, children, history — and responds within scope. Structured mutations (update reality, add children, resolve) come back as reviewable cards.

```bash
# From CLI:
werk run <id> "What pattern do you see in my oscillation?"
```

### Clipboard handoff (`@!` in TUI)

Copies tension context + CLI reference to clipboard. Paste into any agent terminal. The agent can interact with werk directly:

```bash
werk show <id>                    # full details
werk context <id>                 # JSON with all dynamics
werk reality <id> "new reality"   # update
werk note <id> "observation"      # annotate
```

### Hermes Agent skill

```bash
cp -r skills/werk ~/.hermes/skills/
hermes chat -s werk
```

## The Daimon (`werk watch`)

A background daemon that monitors your dynamics and invokes the agent when structurally significant thresholds cross.

```bash
werk watch              # foreground
werk watch --daemon     # background
werk watch --pending    # list waiting insights
werk watch --status     # check daemon state
```

**Triggers**: neglect onset, conflict detected, oscillation spike, horizon breach, phase transition, stagnation, resolution.

When you return to the TUI, the lever shows "3 insights waiting." Press `i` to review. Space expands each insight. `a` to apply, `d` to dismiss.

The daimon doesn't interrupt. It watches while you're away, and waits for you to come back.

## The Philosophy

The alchemist creates gold — his works in the world, and his soul refined by the process. The Great Work is bringing every part of your life into coherence. werk is the operative instrument for that process.

AI should be subordinate to what you've declared you want. Your tensions are alignment data. The agent serves them, not the reverse. This is individual alignment — not through fine-tuning or system prompts, but through structure.

Every tension is an experiment. Every mutation is data. Every dynamic is a mirror. The instrument doesn't do the work. The practitioner does. But the instrument holds the structure, reveals the dynamics, and summons the guide when the practitioner is ready to ask.

## Architecture

```
sd-core          Structural dynamics engine (Rust, 652 tests, zero unsafe)
  ├── tension    Core data model — desired, actual, status, horizon
  ├── mutation   Append-only change log
  ├── store      SQLite persistence (fsqlite, pure Rust)
  ├── dynamics   All 13 computations
  ├── engine     Orchestration + event emission
  └── horizon    Temporal horizons with variable precision

werk-cli         30+ commands — the practitioner's toolkit
  ├── add, show, reality, desire, resolve, release, reopen
  ├── tree, list, health, insights, trajectory, diff
  ├── context    Full JSON export for agent consumption
  ├── run        One-shot agent invocation
  └── watch      The Daimon — background dynamic monitoring

werk-tui         The Operative Instrument — terminal interface
  ├── descent    Navigate by going deeper, not wider
  ├── gaze       Progressive inline expansion
  ├── agent      Tension-scoped AI interaction
  └── vlist      Variable-height virtual list

skills/werk      Hermes Agent skill for onboarding + assistance
```

## Build & Test

```bash
cargo build                 # full workspace
cargo test                  # 1066 tests
cargo install --path werk-cli   # install to PATH
```

Requires Rust nightly. Pinned via `rust-toolchain.toml`.

## License

MIT OR Apache-2.0

---

*Built for the [Nous Research](https://nousresearch.com) Hermes Agent Hackathon.*
*The agent serves your declared intentions. Not the reverse.*
