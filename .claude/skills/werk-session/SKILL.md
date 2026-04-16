---
name: werk-session
description: "Generate a comprehensive session prompt for working on specific werk tensions. Reads the tension tree, maps code locations, grounds in the conceptual foundation, and produces a clipboard-ready prompt with theory of closure. Use when you've decided what to work on and need a thorough brief."
disable-model-invocation: true
allowed-tools: Bash, Read, Grep, Glob, Agent
argument-hint: "<tension-id> [additional tension IDs...] [\"custom instructions\"]"
---

# werk-session — Session Prompt Generator

You are generating a session prompt — a comprehensive brief that a fresh Claude Code session can consume cold to do focused werk development. You don't write code. You produce the prompt.

The prompt follows structural dynamics practice: it has a desired outcome (tensions to resolve), a current reality (what exists), a theory of closure (the plan), and it advances through gestures.

## Step 0: Parse Arguments

`$ARGUMENTS` contains tension IDs and optionally custom instructions in quotes.

Examples:
- `154` — single tension
- `46 129 130 131` — multiple tensions
- `154 "Focus on the schema migration and link gesture only"` — tension + custom instructions
- `140 154 "Skip community detection signals, just get edges working"` — multiple tensions + custom instructions

Parse rules:
- Bare numbers are tension IDs
- Quoted strings (single or double quotes) are **custom instructions** — the user's additional context, constraints, or focus directives for this session
- If no arguments: ask which tension(s) to target. Optionally show the tree (`werk tree`) to help them decide.
- If one ID: that's the target tension.
- If multiple IDs: determine relationship. If one is parent of the others, treat it as the umbrella. If they're siblings, find their common parent. The prompt addresses all of them.
- If custom instructions are present: they **shape the entire prompt**, not just a section. The instructions reframe what "resolved" means, which code to emphasize, which principles to foreground, and what the session task prioritizes. They may also appear verbatim in the prompt if relevant context for the receiving session.

## Step 1: Read the Tension Structure

For each tension ID:

```bash
werk show <id> --json
```

Then for each parent:

```bash
werk show <parent-id> --json
```

Build the parent chain up to root. Collect for each tension:
- Desired, actual (the gap)
- Children (if any)
- Signals (overdue, stale, blocked, etc.)
- Notes (prior session findings, design decisions)
- Deadline, order, position
- Status (active, resolved, etc.)

**If a tension is already resolved**, say so and suggest its siblings or parent instead. Stop and ask before continuing.

## Step 2: Read the Conceptual Grounding

1. Read `designs/werk-conceptual-foundation.md`.
2. Identify which sacred core principles are relevant to this tension's domain:
   - **Desired Above Actual** — always relevant for display/hierarchy work
   - **Theory of Closure** — always relevant (it's the session structure itself)
   - **Signal by Exception** — relevant for temporal/health/signal work
   - **Standard of Measurement** — relevant for temporal properties, computed fields
   - **Gesture as Unit of Change** — relevant for action/mutation work
   - **Locality** — relevant for multi-participant, agent, or context work
   - **Structure Determines Behavior** — relevant for hierarchy, parenting, position work
3. Search `designs/` for related design documents:
   ```bash
   ls designs/
   ```
   Then grep for keywords from the tension's desired/actual text across design docs.
4. Extract any settled design decisions that constrain this session's work.

## Step 3: Map the Code

Based on the tension's domain, read the relevant source files and build a precise code locations table. Don't guess line numbers — read the files.

Domain to file mapping:

| Domain | Key files |
|--------|-----------|
| Signals / temporal | `werk-core/src/temporal.rs`, `werk-core/src/frontier.rs`, `werk-core/src/projection.rs` |
| Data model / store | `werk-core/src/store.rs`, `werk-core/src/tension.rs`, `werk-core/src/types.rs` |
| CLI commands | `werk-cli/src/commands/*.rs`, `werk-cli/src/main.rs` |
| TUI | `werk-tui/src/app.rs`, `werk-tui/src/update.rs`, `werk-tui/src/render.rs` |
| MCP tools | `werk-mcp/src/tools.rs` |
| Web surface | `werk-web/src/lib.rs`, `werk-web/static/` |
| Display / formatting | `werk-shared/src/display.rs`, `werk-shared/src/lib.rs` |
| Desktop app | `werk-app/src/`, `werk-app/src-tauri/` |

Read enough of each relevant file to produce accurate line references. Build a table:

```
| Concern | File | Lines | What's there |
|---------|------|-------|--------------|
```

## Step 4: Check Prior Sessions

```bash
cass search "<keywords from tension desired/actual>" --workspace . --agent claude_code --limit 3 --max-content-length 300
```

If prior sessions touched this area, note:
- What they accomplished
- What they left open
- Any decisions or discoveries recorded

If `cass` is not available, skip this step silently.

## Step 5: Assemble the Session Prompt

Generate a prompt with this exact structure:

### Section 1: Context Block
```
## Context

Read `designs/werk-conceptual-foundation.md` for the sacred core. Read `CLAUDE.md` for conventions.

Run `werk tree` to see the full tension structure.
Run `werk show <id>` for each target tension.

Note: use `werk` (the installed binary) for reading structure. Use `cargo run --bin werk --` only if you are modifying werk itself and need to test against your changes.

## Session Branch

Your vbranch: `close/<id>-<slug>` (already applied in the workspace).

All commits name the branch explicitly:
  but commit close/<id>-<slug> -m "..."

Do not commit to any other vbranch. Do not create new vbranches without
first surfacing why — see Drift Protocol below.
```

### Section 2: Session Identity
```
## Session Identity

This session attacks [tension #ID: desired text].

Parent chain: #ID (desired) -> #ID (desired) -> ... -> root
Related tensions: #ID (sibling, relevant because...), #ID (depends on this)
```

### Section 3: Current Reality
```
## What Already Exists

[Thorough, honest accounting of what's implemented and what's not.
File paths, line numbers, function names. What works, what's partial,
what's missing entirely. This must be accurate — read the code, don't guess.]
```

### Section 4: Conceptual Grounding
```
## Conceptual Grounding

The following sacred core principles constrain this work:
- [Principle]: [how it applies to this specific tension]

Design decisions already made:
- [Decision from foundation doc or design docs, with reference]

Relevant design documents:
- `designs/[filename]` — [what it covers]
```

### Section 5: Key Code Locations
```
## Key Code Locations

| Concern | File | Lines | What's there |
|---------|------|-------|--------------|
| [specific concern] | `path/to/file.rs` | L42-87 | [what this code does] |
```

### Section 5b: Scope Declaration
```
## Scope Declaration

This session edits files in the Key Code Locations table above. Treat
that list as a fence — additions require acknowledging the drift
(see Drift Protocol) before touching a new file.

Out of scope for this session: everything not in the table.
```

### Section 6: Session Task with Theory of Closure

If custom instructions were provided in Step 0, they have already shaped all prior sections — the code locations emphasize what the instructions asked about, the conceptual grounding foregrounds the relevant principles, the current reality section addresses the questions raised. Now anchor the session task to those instructions as the primary framing:

```
## Session Task

### Target tensions
- #ID: [desired] — currently: [actual summary]

### Before writing any code

Define your theory of closure:

1. **What does "resolved" look like?** For each tension, state the concrete condition that closes the gap.
2. **Sequence.** What comes first? What depends on what?
3. **Scope boundaries.** What's in for this session, what's explicitly out.
4. **Risks.** What could go wrong? What needs human input before proceeding?

Present this plan for approval before writing code.

### After approval

- Implement the approved plan
- Pause at logical checkpoints for review
- Update tension reality as ground is covered: `werk reality <id> "new state"`
- Resolve tensions when done: `werk resolve <id>`
- Run `werk flush` before commits
- Commit at logical boundaries: `but commit close/<id>-<slug> -m "..."`

### Drift Protocol

If the work requires touching a file outside the Scope Declaration:

- **Trivial drift** (a version bump in Cargo.toml, a type import, a one-line config): make the edit, log it on the tension with `werk note <id> "touched <file>: <reason>"`, continue.
- **Meaningful drift** (new module, cross-cutting refactor, touching a path another session might own): stop, surface the discovery to the user, re-negotiate scope before proceeding. Don't silently expand.
- **Blocking drift** (the task cannot proceed without crossing into contested territory): stop, write a note on the tension describing what's needed, hand back to the user.

Sticking to scope is the default; breaking it is a deliberate act.

### Note-taking

Use `werk note <id> "..."` as you go — not for every step, but for:
- **Major learnings** that future sessions should know about
- **Cross-tension discoveries** — if working on #X reveals something about #Y, note it on #Y
- **Design decisions** made during the session that aren't captured elsewhere
- **Surprises** — when reality differs significantly from what was expected

Notes are the session's memory. A good note on a sibling tension can save the next session an hour of rediscovery.
```

### Section 7: Prior Session Context (if any)
```
## Prior Session Context

[What previous sessions accomplished in this area, what they left open,
any decisions or caveats recorded. Omit this section if no prior context found.]
```

## Step 6: Set up the branch and copy to clipboard

### 6a. Derive the branch name

Branch format: `close/<id>-<slug>`

- `<id>` — the primary tension's short code, without the `#` (shell-hostile). For multi-tension sessions, use the umbrella/parent tension's short code.
- `<slug>` — 2–4 content words, kebab-case, lowercase.
  - If custom instructions were provided in Step 0, derive the slug from them (they reframe what "closed" means).
  - Else, derive from the target tension's desired text. Strip filler words (a/the/and/of/to/for/in/on). Take the most salient 2–4 content words.
  - Cap total branch name at ~50 chars; truncate the slug tail if needed.

Examples:
- tension #42 "Extract display helpers into werk-shared" → `close/42-extract-display-helpers`
- tension #46, custom instr "focus on schema migration" → `close/46-schema-migration`
- tensions #10, #11, #12 under parent #9 "Signals cleanup" → `close/9-signals-cleanup`

### 6b. Create or apply the branch

```bash
BRANCH="close/<id>-<slug>"
if but branch list 2>&1 | grep -q " ${BRANCH}\b"; then
  but apply "$BRANCH"
else
  but branch new "$BRANCH"
fi
```

`but branch new` creates the branch applied in the workspace. `but apply` brings an existing branch back if a prior session left it unapplied. Either way, the session starts with its branch ready to receive commits.

### 6c. Copy the prompt

```bash
echo "<the assembled prompt>" | pbcopy
```

### 6d. Summarize for the user

Tell the user, under 10 lines:
- Which tensions the prompt targets
- The branch name (so they know where this session's commits will land)
- Which code areas it maps
- Which sacred core principles it highlights
- How many prior sessions touched this area

The prompt itself is the deliverable.

## Quality Checks

Before copying to clipboard, verify:

- [ ] Every file path in the prompt actually exists
- [ ] Every line number reference is current (you read the file this session)
- [ ] The "current reality" section matches what the code actually does, not what the tension says it should do
- [ ] Sacred core principles are specific to this tension's domain, not generic boilerplate
- [ ] The theory-of-closure section asks for concrete resolution conditions, not vague "improve X"
- [ ] Tension IDs are correct short codes (#42 format)
- [ ] The prompt is self-contained — a cold session can consume it without prior context
