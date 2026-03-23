---
name: triage
description: "Session opening ritual. Orient in the werk codebase and tension structure, check health, and recommend what to work on next. Use at the start of any werk development session."
disable-model-invocation: true
allowed-tools: Bash, Read, Grep, Glob, Agent
argument-hint: "[optional: tension ID or topic to focus on]"
---

# Triage — Session Opening

You are opening a werk development session. Your job: orient fast, present the state honestly, and recommend what to work on.

## Step 1: Orient

Read the authority document and get the current structure:

1. Read `designs/werk-conceptual-foundation.md` — this is the constitutional authority. Note any vocabulary or concepts you'll need for this session.
2. Run `cargo run --bin werk -- tree` to see the full tension structure.
3. Run `cargo test --workspace 2>&1 | tail -20` to check build/test health.
4. Run `git status` and `git log --oneline -5` to see repo state and recent work.

If the user provided a tension ID or topic as `$ARGUMENTS`, focus your orientation there: run `cargo run --bin werk -- show <id>` for the specific tension and its children.

## Step 2: Assess

Present a concise status report:

- **Tree health**: How many active tensions, what's the completion picture at the top levels
- **Build health**: Tests passing? Clippy clean? Any compilation issues?
- **Repo state**: Clean? Uncommitted changes? Stale worktrees?
- **Recent momentum**: What do the last 5 commits suggest about where work has been happening?

If there's a specific focus (`$ARGUMENTS`), dive deeper there: read the relevant source files, check what's implemented vs. what the foundation specifies.

## Step 3: Recommend

Based on the tree structure, recommend what to work on next. Consider:

- **Dependency order**: What unblocks other work?
- **Momentum**: What was recently active and could be completed with one more push?
- **Urgency**: Any horizons approaching?
- **Neglect**: Anything declared important but untouched?

Present 1-3 options with brief reasoning. Don't over-explain. The user knows the project.

## Step 4: Search Prior Sessions (if relevant)

If the recommendation touches a topic that might have prior session context, run:
```
cass search "*relevant-topic*" --workspace . --agent claude_code --limit 3 --max-content-length 300
```

Mention any relevant findings briefly — don't dump search results.

## Output Format

Keep the whole output under 40 lines. Structure:

```
## State
[tree summary, build health, repo state — 5-8 lines]

## Momentum
[what recent work suggests — 2-3 lines]

## Recommendation
[what to work on, why — 3-5 lines per option]
```

Then ask: "What do you want to work on?"
