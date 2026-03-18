# Hooks Integration Design: werk + Agent Sessions

Research document exploring how werk structural tension state integrates
with agent sessions (Claude Code and others) via hooks, focus clamping,
and bidirectional state synchronization.

## 1. `werk brief` — Compact Structural Chart

A new command producing a token-efficient, human-and-robot-readable
structural tension chart. Designed for context injection into agent
sessions.

### Output Format

```
Structural Tension Chart (2026-03-18)

◆ Ship the tool [advancing, 72% urgency, → 2026-04]
  ◇ TUI rebuild [stagnant, no horizon]
  ◈ Core engine tests [advancing, completion]

◆ Structural consultant cert [advancing, → 2026-04-30]
  ◇ Complete Foundations course [advancing]

◇ Cooperative dynamics design [germination]
```

Phase glyphs match the TUI. Brackets hold: tendency, urgency (if
horizoned), phase (if notable). Indentation shows hierarchy. Only
active tensions shown.

### Design Choices

- **Not JSON.** The session start hook injects text that Claude reads
  as natural language. JSON would work but wastes tokens on syntax.
  `werk context --all` already exists for structured agent consumption.
- **Compact.** A field of 20 tensions should fit in ~30 lines. This is
  orientation, not deep context.
- **Dynamics-aware.** The brief surfaces tendency and urgency — the two
  dynamics most useful for an agent deciding what to work on.

### Open Question: What Else to Include?

Candidates for optional brief sections:
- **Alerts:** neglected tensions, oscillation warnings, conflicts
- **Recent changes:** last 24h mutations (like a mini `werk diff`)
- **Focus indicator:** if a focus tension is set (see Layer 3 below)

These could be flags: `werk brief --alerts --recent --focus`
Or the default brief could include all of them and `--minimal` strips
to just the chart.

---

## 2. The `-g` (Global) Flag

### Current Workspace Resolution

`Workspace::discover()` walks up from CWD looking for `.werk/`, falls
back to `~/.werk/`. This means:
- Inside a project with `.werk/` → local workspace
- Anywhere else → global workspace
- No way to force global when a local exists

### Proposal: `werk -g <command>`

A global flag at the top level (like `--json`) that forces workspace
resolution to skip the walk-up and go straight to `~/.werk/`.

```rust
#[derive(Parser)]
struct Cli {
    #[arg(short, long, global = true)]
    json: bool,

    #[arg(short, long, global = true)]
    global: bool,

    #[command(subcommand)]
    command: Commands,
}
```

Implementation: `Workspace::discover()` gains a `force_global` param,
or we add `Workspace::global()` that only checks `~/.werk/`.

This means:
- `werk brief` → local chart (or global fallback)
- `werk -g brief` → always global chart
- `werk -g context --all` → global context for agent consumption
- The SessionStart hook can run `werk -g brief` to always get the
  global structural picture, regardless of which project directory
  Claude is in.

### Relationship Between Local and Global

This flag also opens the door to the question you raised: how do local
and global werkspaces relate?

**Current state:** They're completely independent. A local `.werk/`
shadows the global one entirely.

**Your intuition:** Local werkspaces should "extend out from" the
global one. A global tension could link to a local workspace.

**Possible models:**

**A. Parent-link model.** A local workspace has a config key
`parent_tension = "01ABCDEF"` pointing to a tension in the global
workspace. The local workspace's tensions are conceptually children
of that global tension.

- `werk brief` in a local workspace shows local tensions
- `werk -g brief` shows global tensions, with local workspaces
  appearing as collapsed children of their parent tensions
- This is the simplest model and maps cleanly to the existing
  parent/child hierarchy

**B. Federated model.** Local and global are peers. A tension in
either can reference a tension in the other via a qualified ID
(`global:01ABC` or `local:01ABC`).

- More flexible but more complex
- Raises sync questions: what happens when you're offline from
  the global workspace?

**C. Overlay model.** Local workspace *is* the global workspace,
filtered to a subtree. Mutations flow to the same database.

- Simplest conceptually (one source of truth)
- But means every project needs network access to `~/.werk/sd.db`
  or a sync mechanism
- Doesn't work well for truly independent project workspaces

**Recommendation:** Start with **A (parent-link)**. It's the least
invasive: local workspaces stay independent, with one config key
establishing the relationship. `werk -g brief` can annotate parent
tensions with "→ local workspace at /path/to/project" when a link
exists.

---

## 3. Stop Hook — Agent Activity Representation

### The Event

In Claude Code, the **Stop** hook fires when Claude finishes its
response turn and returns control to the user. This is the moment
between "Claude worked" and "user reviews."

### What the Hook Should Do

When Claude finishes a turn in a directory that has a `.werk/`
workspace (or when `WERK_FOCUS` is set), the Stop hook should:

1. Summarize what Claude did this turn
2. Determine if it materially affects any active tension's reality
3. If yes, update the reality of the affected tension(s)

### The Hard Problem: Mapping Activity to Tensions

Claude's activity is file edits, bash commands, test runs. Tensions
are about desired states and reality. The mapping isn't mechanical.

**Options:**

**A. Ask Claude to self-report.** The Stop hook injects a prompt:
"Review what you just did. If any active tensions' reality changed,
output `werk reality <id> <new-reality>` commands."

This is the most flexible but relies on Claude having the tension
chart in context (which the SessionStart hook provides).

Implementation: The Stop hook adds `additionalContext` with
instructions, and Claude's next response includes the werk commands.
Problem: the Stop hook fires *after* Claude's response, not before
its next one. You'd need the context injection to carry into the
next turn.

**B. UserPromptSubmit hook.** Instead of Stop, inject a reminder on
every user prompt: "Before responding, check if your previous work
affected any structural tensions." This is lighter but noisier.

**C. Dedicated wrap-up command.** The user types `werk sync` or
`/werk-sync` at the end of a work session. This is the simplest and
most intentional, but requires the user to remember.

**D. PreCompact hook.** When context is about to compact, ask Claude
to update werk state first. This catches the "long session" case
naturally.

**Recommendation:** Start with **C** — a manual `werk sync` or
equivalent slash command that asks Claude to review the session
transcript and propose reality updates. Layer A on top later via a
Stop hook once the pattern is proven. D is a good safety net
regardless.

### What "Material" Means

Not every file edit matters to structural tension. A typo fix in a
README doesn't change the reality of "Ship the tool." But completing
the test suite does.

This is inherently a judgment call. The agent needs:
1. The tension chart (from SessionStart)
2. A sense of what changed this session
3. The ability to distinguish "this advances tension X" from "this
   is maintenance"

The `werk brief` output with dynamics gives enough context for an
agent to make this judgment. The instruction in the sync command can
be specific: "Only update reality when the *structural gap* between
desired and actual meaningfully narrowed or widened."

### `werk touch <id>`

A minimal alternative to full reality updates. `werk touch` would:
- Record that work occurred on this tension
- Update the "last activity" timestamp (prevents neglect detection)
- Not change the reality text

This is useful for "I worked on this but can't articulate how reality
changed yet." It keeps the dynamics accurate without requiring a
narrative update.

---

## 4. Focus / Clamping Mechanism

### The Concept

A "focus" is a persistent pointer to a tension that scopes subsequent
work. When focus is set:

- SessionStart hook injects deep context for that tension (not just
  the brief chart, but full `werk context <id>` output)
- The agent knows what it's working toward
- Subsequent sessions in the same workspace inherit the focus
- Moving focus is a deliberate act

### Where Focus Lives

**Option A: File-based.** `.werk/focus` contains a tension ID.

```bash
# Set focus
echo "01ABCDEF" > .werk/focus

# Clear focus
rm .werk/focus

# CLI
werk focus 01AB        # set
werk focus --clear     # clear
werk focus             # show current
```

**Option B: Config-based.** `focus = "01ABCDEF"` in `.werk/config.toml`.
Same API but stored alongside other config. Slightly more principled.

**Recommendation:** File-based (Option A). Focus is ephemeral state,
not configuration. It changes frequently. A dedicated file is simpler
to read from hooks and doesn't pollute the config namespace.

### Focus in the TUI

The TUI already has navigation (descend into a tension, view children).
Focus could mean:
- TUI opens with the focused tension already descended-into
- Or: focus is shown in the status line but doesn't change navigation

The TUI's gaze/descend model is about *viewing*. Focus is about
*working intent*. They're related but distinct. You might be focused
on "Ship the tool" but navigating around the field to check other
tensions.

### Focus in the CLI

```
$ werk focus
◆ Ship the tool [advancing, 72% urgency]

$ werk focus 01AB
Focus set: ◆ Ship the tool

$ werk brief
Structural Tension Chart (2026-03-18)
→ FOCUS: Ship the tool [advancing, 72% urgency, → 2026-04]
    ◇ TUI rebuild [stagnant, no horizon]
    ◈ Core engine tests [advancing, completion]

  ◆ Structural consultant cert [advancing, → 2026-04-30]
  ◇ Cooperative dynamics design [germination]
```

The brief output reorders to put the focused tension and its subtree
first, with an arrow indicator.

### The Multi-Session Problem

You raised this: if focus is in `.werk/focus` and you run multiple
Claude sessions in the same directory, they all see the same focus.

**Options:**

1. **Accept it.** Usually you're working on one thing at a time in
   one workspace. Multiple sessions in the same workspace working on
   different things is the exception, not the rule.

2. **Session-scoped override.** `WERK_FOCUS=01XYZ` as an env var
   overrides the file-based focus for that session only. The
   SessionStart hook could be:
   ```bash
   if [ -n "$WERK_FOCUS" ]; then
     werk context "$WERK_FOCUS"
   elif [ -f .werk/focus ]; then
     werk context "$(cat .werk/focus)"
   fi
   werk brief
   ```

3. **Per-session focus in Claude's env.** The SessionStart hook
   writes to `CLAUDE_ENV_FILE` to set `WERK_FOCUS` for the session,
   reading from `.werk/focus` at start. The user can then change it
   mid-session without affecting other sessions.

**Recommendation:** Option 2. The file is the default. The env var
is the escape hatch. This handles 95% of cases cleanly and the 5%
with a simple override.

### Focus and Local/Global

With the `-g` flag and parent-link model:
- `werk focus` sets focus in the local workspace
- `werk -g focus` sets focus in the global workspace
- The SessionStart hook could inject both:
  - Global brief (the big picture)
  - Local deep context (the focused tension)

This naturally separates "where am I in life" from "what am I
working on right now."

---

## 5. Dynamics-Triggered Hooks and Authority Sources

### Programmatic Authority

You raised an interesting question: being able to specify an
objective source/authority for a tension's state.

Currently, reality is a text string updated by the practitioner.
But some realities are objectively measurable:

- "All tests pass" — `cargo test` can verify this
- "CI is green" — a GitHub API call can check
- "Inbox is at zero" — an email API can count
- "Deployed to production" — a deploy status check

### Tension Authorities

A tension could optionally have an `authority` — a command or URL
that, when queried, returns the current objective state:

```toml
# .werk/config.toml
[authorities]
"01ABCDEF" = { command = "cargo test --quiet 2>&1 | tail -1" }
"01XYZABC" = { url = "https://api.github.com/repos/owner/repo/actions/runs?status=success&per_page=1" }
"01QWERTY" = { command = "werk -g context 01QWE --json | jq '.tension.actual'" }
```

Or as a field on the tension itself:
```
werk authority 01AB "cargo test --quiet 2>&1 | tail -1"
```

When `werk brief` runs, it could optionally poll authorities and
show freshness:
```
◆ Ship the tool [advancing, 72% urgency]
  ◈ All tests pass [authority: ✓ 652 passed, 0 failed] (checked 2m ago)
  ◇ TUI rebuild [stagnant, no authority]
```

### Dynamics-Triggered Actions

The engine already emits events: `PhaseTransition`, `NeclectOnset`,
`OscillationDetected`, etc. These could drive hooks:

```toml
[hooks]
on_neglect_onset = "echo 'Neglect detected: {tension_desired}' >> ~/.werk/alerts.log"
on_phase_transition = "werk brief > ~/.werk/latest-brief.txt"
on_oscillation = "notify 'Oscillation detected in: {tension_desired}'"
```

But the harder question is: **who receives these?**

In a single-user, single-agent setup, the answer is simple: log them
and surface them at session start. In a cooperative/multi-agent
setup, this becomes a pub/sub system.

### The Orchestrator Question

You noted this gets into territory that agent harnesses/orchestrators
also occupy. That's exactly right. The question is whether werk
*is* the orchestrator or *informs* an orchestrator.

**werk as orchestrator:** Dynamics events trigger agent invocations.
"This tension is neglected → launch an agent to investigate." This
is what `werk watch` already does in prototype form.

**werk as context provider:** An external orchestrator (Claude Code,
a custom harness, a multi-agent framework) queries werk for state
and makes its own decisions. werk is the structural awareness layer.

**Recommendation:** werk should be the context provider, not the
orchestrator. Orchestration decisions depend on too many factors
outside werk's domain (what tools are available, what the user is
doing right now, cost constraints, etc.). Werk's job is to hold
structural truth and surface it when asked.

The hook system then becomes:
1. **Inbound hooks:** External events update werk state (agent
   completed work, CI passed, deploy succeeded)
2. **Outbound hooks:** Dynamics changes notify interested parties
   (session start injection, alert logs, webhook calls)
3. **Query interface:** Agents pull state on demand (CLI, MCP, API)

---

## 6. Implementation Sequence

### Phase 1 — Now

1. `werk brief` command (compact structural chart)
2. `-g` flag on `Cli` struct, threaded through `Workspace::discover`
3. SessionStart hook: `werk brief` (or `werk -g brief`)
4. PreCompact hook: same (re-inject after compaction)
5. Clean up stale hooks (br prime, others per user decision)

### Phase 2 — Focus

6. `werk focus <id>` / `werk focus --clear` / `werk focus`
7. `.werk/focus` file
8. SessionStart hook enhanced: brief + deep context for focus
9. `WERK_FOCUS` env var override

### Phase 3 — Bidirectional

10. `werk touch <id>` command
11. `werk sync` concept (manual or slash command)
12. Stop/PreCompact hook for activity → werk state flow
13. Local-to-global parent-link in config

### Phase 4 — Authority & Dynamics Hooks

14. `authority` field or config for tensions
15. Outbound dynamics hooks (on_neglect, on_phase_transition, etc.)
16. MCP server for agent-native access

---

## Open Questions

1. Should `werk brief` be the default when no subcommand is given
   and stdout is not a terminal? (i.e., `werk | cat` gives brief
   instead of launching TUI)

2. How verbose should the brief be by default? Minimal (just chart)
   vs. annotated (chart + alerts + recent changes)?

3. Should focus affect `werk brief` output ordering, or should brief
   always show the natural tree order?

4. For the Stop hook / sync mechanism: should the agent propose
   `werk reality` commands that the user approves, or should it
   apply them directly?

5. For authorities: polling frequency? On-demand only (when brief
   runs)? Periodic (via watch daemon)?
