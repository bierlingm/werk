# `werk run` — the full design space

A brainstorm of what a `werk run` command (with various flags and arguments) could become. Every variant treats `run` as **time spent inside the instrument with a frame that holds the work**.

Right now werk is great at *recording* gestures. `run` would make werk great at *holding the space in which gestures happen*.

---

## I. Run as *focused session* (the tension as container)

### `werk run <id>`
Enter a focused session bound to one tension. The terminal prompt changes (`werk:#42 $`). A timer starts. Every gesture you make — git commits in adjacent repos, notes you jot, file edits captured by the UBS hook, even shell commands — gets attached as epochs on `#42` with provenance. At the end (`werk release` or Ctrl-D), you get a closure ritual:

```
Session on #42 "ship the auth refactor" — 47m
  reality moved: "design sketched" → "draft PR open"
  3 commits attached, 2 notes added, 1 child created (#43)

Did reality close on desire? [c]lose / [r]e-desire / [k]eep open
```

This makes work *legible to itself*. The tension isn't just a tracker — it's the frame the session pours into.

### `werk run @here`
Same as above but binds via cwd/shell PID instead of explicit id. The shell hook reads `~/.werk/session-{pid}.lock`; commits in this directory auto-attach. `werk release` unbinds. Lets you have one session per terminal pane — perfect for tmux-heavy workflows where each pane is already mentally a "thing I'm doing."

### `werk run --pomo 25`
Constrains the session to 25 minutes. At expiry, forces a micro-closure ("what shifted?") and asks whether to continue. Pomodoro for structural dynamics — but the *tension* is the unit, not the timer. The timer serves the tension.

---

## II. Run as *instrument-as-DJ* (let werk pick)

### `werk run --next`
The autopilot. Werk computes a "what most needs a gesture right now" score across the forest:
- urgency (deadline proximity ÷ slack)
- staleness (time since last reality movement)
- inherited pressure (parent's deadline propagated down)
- structural fragility (Quint witness: tensions one gesture from violating an invariant)
- envelope fit (current zoom, current framing)

…and drops you into a session on the winner. Optionally `--explain` to see *why* it chose. Structural-dynamics version of "what should I work on" — but grounded in your own model of the work, not arbitrary priority numbers.

### `werk run --next --hands-off 5`
Picks the next tension, but you've got 5 seconds to veto before it commits. Cron-mode autopilot for daily standups: "every morning at 9, pick what to start." Combined with `--pomo 90` it becomes a self-driving workday.

### `werk run --mood scattered|deep|small`
Hint to the picker. `--mood scattered` favors many small tensions in quick succession (clear inbox debt). `--mood deep` picks one tension with high structural weight and locks the session for 90+ min. `--mood small` finds tensions where the gap between reality and desire is one gesture wide. The instrument adapts to *you*, not the other way around.

---

## III. Run as *bounded loop* (the tension as accumulator)

### `werk run <id> --until reality=='shipped'`
Loop on the tension until a predicate matches reality. Each iteration prompts you for one gesture; the loop exits when the condition holds or you abort. Useful for "I will not stop until this is done" sprints.

### `werk run <id> --until 2026-05-01`
Time-bounded version. Werk pings you at adaptive intervals (closer to deadline = more frequent) asking "what's the smallest gesture that moves reality?" Each ping logs an epoch even if the answer is "nothing today, blocked on X" — making invisible work visible.

### `werk run --until forest-clean`
Exits when no tension has urgency > threshold and no parent contains a child with deadline > parent's deadline (the containment invariant from `temporal.qnt`). A "do whatever it takes to restore structural sanity" mode. The Quint specs become *operational targets*.

---

## IV. Run as *gesture script* (the tension as program)

### `werk run script.werk`
Execute a small DSL of gestures atomically:

```werk
let auth = add "ship auth refactor" --deadline 2026-05-15
split $auth "design" "build" "test"
desire $auth.children[1] "PR merged with 2 reviews"
note $auth "kicked off in planning meeting"
```

Single epoch group. Undoable as a unit (`werk undo-gesture <group-id>`). Composable. Lets you encode *patterns* — "the way I always start a client engagement" becomes a script you `werk run` once.

### `werk run --pipe`
Read gestures from stdin. Pipes well: `cat meeting-notes.md | llm extract-tensions | werk run --pipe`. The instrument becomes a sink for upstream tooling.

### `werk run --template client-kickoff <name>`
Parameterized scripts. Templates live in `~/.werk/templates/`. Each one is a gesture-script with `{{name}}`, `{{deadline}}` slots. "Kickoff" creates the parent tension, three standard children, sets deadlines relative to today, attaches the canonical kickoff note.

---

## V. Run as *time travel* (the tension as projection)

### `werk run --simulate 14d`
Dry-run the forest forward 14 days based on:
- explicit deadlines
- historical gesture cadence per tension
- Quint specs (which witnesses fire when)
- recurring tensions (`werk recur`)

Produces a projected `werk tree` for two weeks out. Shows which tensions will go red, which deadlines will compound, which parents will breach containment. Planning by *shadow-casting* — you see the forest you're heading toward before you arrive.

### `werk run --simulate --what-if "resolve #42"`
Branch-and-project. "What does the forest look like if I close this today?" Useful for triage: see the second-order effects of a closure before committing. The Quint spec becomes a what-if engine.

### `werk run --replay 2026-01-01..2026-04-01`
Walk the logbase forward through historical gestures, frame by frame. Slow scrub or fast-forward. Watch your forest *grow*. Useful for retrospectives — you see *where the structure broke*, not just where the work happened.

---

## VI. Run as *sweep / maintenance* (the instrument as janitor)

### `werk run --resolve-stale`
Walk every tension where reality hasn't moved in >X days. Prompt one decision per: resolve / re-desire / snooze / split / delete. Inbox-zero for structural debt. Defaults: 30d for leaves, 90d for parents.

### `werk run --reframe`
Walk tensions where the desire was set >60d ago and ask: "is this still what you want?" Half the value of werk is keeping the *desire* fresh — reality moves on its own; desire requires intentional renewal.

### `werk run --rebalance`
Find structural anti-patterns (parents with one child = collapse, children whose desires don't compose toward parent's desire = misalignment, orphan branches with no recent activity = abandoned subtrees) and walk them one by one.

### `werk run --garden 15m`
Time-boxed sweep mode. "15 minutes of forest gardening." Picks a mix of stale, misaligned, and overdue tensions; you get to one decision each before moving on. Daily ritual.

---

## VII. Run as *binding* (the tension as ambient context)

### `werk run @here` (extended)
Beyond cwd-binding: every shell command in this pane gets a one-line epoch. Every file written gets attached. Every git commit's message gets parsed for tension references (`#42 in commit msg → epoch on #42`). The pane *becomes* the tension for as long as you're in it.

### `werk run --watch <path>`
Bind to a filesystem path instead of a shell. Any change under `<path>` while the session is alive attaches to the tension. Good for "I'm working on the auth module — anything that happens in `src/auth/` is part of this work."

### `werk run --bind-branch`
Bind to the current GitButler branch. Every commit on this branch attaches to the tension. When you `but pr new`, the PR description auto-pulls the tension's desire as context. The branch *is* the tension's reality.

---

## VIII. Run as *multi-agent dispatch* (the tension as work order)

### `werk run <id> --agent claude`
Spawn an agent in a fresh session, hand it the tension's full context (desire, current reality, parent chain, recent epochs, attached notes), and let it propose gestures. Agent's gestures arrive as MVCC writes; you see them in `werk log` as a different "writer." You stay the human approver; the agent does the legwork.

### `werk run --swarm 3 --tension-pool stale`
Dispatch three agents in parallel, one per stale tension, each in its own worktree. Each agent attempts the smallest gesture that moves reality. Results land as proposed epochs you review in a unified diff.

### `werk run <id> --pair`
Two-writer mode. You and an agent (or you and another human via mcp-agent-mail) work on the same tension simultaneously. MVCC handles conflicts; the session UI shows both cursors. Pair programming for tensions.

---

## IX. Run as *ritual* (the tension as ceremony)

### `werk run --morning`
Composite ritual. Runs in sequence: `--reframe` (5 stale desires), `--next` (pick today's primary), `stats --health` (forest temp check). Three minutes, you start the day oriented.

### `werk run --evening`
Mirror. Walks today's epochs, asks "did reality move?" per touched tension, prompts closures, generates a one-line journal entry. Werk as the *bookend* of your work, not a side-tab you forget about.

### `werk run --weekly-review`
GTD-style weekly review, but native to tensions. Walks every parent, asks "still relevant?", "still composing toward something?", "still yours?" Followed by `--reframe` on the survivors. Followed by `--simulate 7d` so you see the week ahead.

---

## X. Run as *meta* (the instrument observing itself)

### `werk run --teach`
Tutorial mode. Generates a sandbox forest in a temp store, walks you through gestures, explains the four frameworks as you encounter them. Live instructor for new users.

### `werk run --debug-spec`
Run gestures against a Quint-checked sandbox. Every gesture you attempt is first replayed in the spec; if it would violate `systemInvariant` or `strongInvariant`, werk refuses with a witness counter-example. The spec becomes a guard rail at the gesture surface, not just CI.

### `werk run --sing`
Werk reads your forest aloud as structured prose. "You hold 23 open tensions across 4 parents. The oldest desire is from 89 days ago: 'understand what shipping means here.' Today, three things are due." Useful while driving, walking, or staring at the ceiling.

---

## The unifying read

Every variant treats `run` as **time spent inside the instrument with a frame that holds the work**. The frame can be:

- **a single tension** (`<id>`, `@here`, `--bind-branch`)
- **a chosen tension** (`--next`, `--mood`)
- **a predicate** (`--until`, `--resolve-stale`)
- **a script** (`script.werk`, `--template`, `--pipe`)
- **a projection** (`--simulate`, `--replay`)
- **a ritual** (`--morning`, `--weekly-review`)
- **a delegation** (`--agent`, `--swarm`)

Across all of them, the constant is: **a session is a bounded interval where gestures accumulate against a frame, and at the end something gets closed.**

The deepest version is probably `werk run --next --pomo 90 --evening-after`: you tell werk "I have 90 minutes — pick the work, hold the container, close it for me." The instrument becomes the operator.
