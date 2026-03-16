# werk watch — The Daimon

## What It Is

`werk watch` is a background daemon that monitors your tension dynamics and automatically invokes the configured agent when something structurally significant changes. It turns werk from a tool you pull up when you remember into an instrument that watches while you work.

The agent becomes a genuine daimon — the Socratic inner voice that notices things about your structural dynamics before you do.

## How It Works

### The Command

```
werk watch                    # start watching (foreground, Ctrl-C to stop)
werk watch --daemon           # start as background process
werk watch --stop             # stop the background daemon
werk watch --status           # show what's being watched and last check
```

### The Loop

Every N minutes (default: 30, configurable via `werk config set watch.interval 30`):

1. Recompute dynamics for all active tensions
2. Compare against the last snapshot
3. For each tension where a threshold was crossed, queue an agent check
4. For queued tensions, call the configured agent with context + a system prompt explaining what changed
5. Write the agent's response + any structured mutations to a pending file
6. Next time the user opens the TUI, the lever shows: `3 insights waiting — press i to review`

### What Triggers a Check

Threshold crossings — not every change, only structurally significant ones:

| Trigger | Condition | Why It Matters |
|---------|-----------|----------------|
| **Neglect onset** | `neglect` transitions from None to Some | A thread you declared you care about is being abandoned |
| **Conflict detected** | `conflict` transitions from None to Some | Two things you want are fighting each other |
| **Oscillation spike** | `oscillation.reversals` increases by 2+ | You're going back and forth — the ouroboros |
| **Horizon breach** | Horizon passes without resolution | You missed your own deadline |
| **Phase transition** | Phase changes (germination → assimilation, etc.) | Something structural shifted |
| **Stagnation** | Tendency transitions to Stagnant for 7+ days | Nothing is moving |
| **Resolution** | Tension is resolved | A gap closed — worth noting |

Each trigger has a cooldown (default: 24h per tension per trigger type) to prevent spam.

### What the Agent Receives

```
SYSTEM: You are monitoring structural dynamics for the user's tensions.
A threshold was crossed. Analyze what changed and suggest one action.

TRIGGER: neglect_onset
TENSION: "Write the novel"
PREVIOUS STATE: 3 active children, all advancing, no neglect
CURRENT STATE: 3 children, 2 stagnant for 14 days, neglect detected

[full context from `werk context <id>`]

Respond with a brief observation (2-3 sentences) and optionally suggest one mutation. Use the YAML format if suggesting a change.
```

### What the Agent Returns

Stored in `.werk/watch/pending/`:

```yaml
# .werk/watch/pending/2026-03-16T14:30:00_01KJNAA8.yaml
tension_id: "01KJNAA84ANHXCF58M8K24YZV5"
trigger: "neglect_onset"
timestamp: "2026-03-16T14:30:00Z"
response: |
  Two of three children haven't moved in two weeks. The novel's
  structure work is done but the actual writing has stalled.
  Consider whether "Research the setting" is still needed or
  should be released.
mutations:
  - action: add_note
    tension_id: "01KJNAA84ANHXCF58M8K24YZV5"
    text: "Watch: neglect detected — 2/3 children stagnant for 14d"
reviewed: false
```

### TUI Integration

When pending insights exist:

**Lever**: `3 insights waiting` (replaces breadcrumb, cyan color)

**Press `i`**: Opens insight review mode (similar to mutation review):

```
  ─ watch insight ───────────────────────────────────

  neglect detected on "Write the novel"
  2 of 3 children stagnant for 14 days

  Two of three children haven't moved in two weeks...

  suggested: add note "Watch: neglect detected..."

  [x] accept    Esc dismiss    @ follow up
```

**After review**: The pending file is marked `reviewed: true`. The insight is stored as a note mutation if accepted. The lever returns to normal.

### CLI Integration

```
werk watch --pending          # list pending insights (for agents to read)
werk watch --history          # show recent watch activity
werk watch --triggers         # show configured triggers and cooldowns
```

Any agent (Claude, Hermes, or any tool with terminal access) can:
```
werk watch --pending          # see what the daimon noticed
werk context <id>             # get full context on a flagged tension
werk reality <id> "..."       # act on the insight
```

## What It Changes

### The Relationship with the Instrument

Without `werk watch`, the instrument is passive — it waits for you. With it, the instrument holds you accountable to your own declarations. You said you wanted to write the novel. You haven't touched it in two weeks. The daimon noticed.

This isn't a notification system. There are no push alerts, no badges, no sounds. The insights wait silently until you return. The weight is in the returning — opening werk and seeing "3 insights waiting" is the mirror showing you what you've been avoiding.

### The Agent's Role

The agent transforms from a tool you invoke to a presence that watches. It's still subordinate — it can only suggest, never act autonomously. But it sees patterns you can't see because you're inside them. The 7-day stagnation that feels like "I'll get to it tomorrow" is visible to the daimon as a structural pattern.

### The Alignment Thesis

This is individual alignment made concrete:
- You declare what matters (tensions)
- The system computes structural truth (dynamics)
- The agent monitors for divergence between declaration and behavior
- The agent surfaces observations, never commands
- You decide what to do

The agent is aligned to YOU — to your declared intentions, not to corporate norms or safety theater. This is exactly the Nous Research thesis.

## What It Requires

### Technical

- A simple timer loop (every N minutes, recompute dynamics)
- A snapshot store (`.werk/watch/snapshots/` — last known dynamics per tension)
- A diff function (compare current dynamics to snapshot, identify threshold crossings)
- Agent invocation (reuse existing `execute_agent_oneshot`)
- Pending file storage (`.werk/watch/pending/` — YAML files)
- TUI integration (lever message + insight review mode)
- CLI subcommand (`werk watch` with flags)

### Configuration

```toml
[watch]
interval = 30          # minutes between checks
cooldown = 1440        # minutes between alerts per tension per trigger (24h)
triggers = ["neglect", "conflict", "oscillation", "horizon_breach", "stagnation"]
```

### Effort Estimate

- Core loop + snapshot + diff: ~200 lines
- Agent invocation + pending files: ~100 lines
- TUI insight review: ~150 lines
- CLI subcommand: ~80 lines
- Total: ~530 lines, roughly 1 day of focused work

## The Name

"The Daimon" — from the Greek δαίμων. Not a demon. The inner guiding spirit. Socrates described his daimon as a voice that said "not that" rather than "do this." It constrained rather than commanded. That's exactly what `werk watch` does.
