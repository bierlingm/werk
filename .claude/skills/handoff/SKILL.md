---
name: handoff
description: "Session closing ritual. Summarize work done, update the werk tension tree, commit, and generate a handoff prompt for the next session. Use at the end of any werk development session."
disable-model-invocation: true
allowed-tools: Bash, Read, Grep, Glob, Edit
argument-hint: "[optional: notes about what was accomplished]"
---

# Handoff — Session Closing

You are closing a werk development session. Your job: capture what happened, update the structure, commit cleanly, and prepare the next session.

## Step 1: Take Stock

1. Run `git diff --stat` and `git log --oneline HEAD~10..HEAD` to see what changed this session.
2. Run `cargo test --workspace 2>&1 | tail -10` to confirm the build is healthy.
3. Run `cargo run --bin werk -- tree` to see the current tension structure.

Summarize in 3-5 lines what was accomplished. If the user provided `$ARGUMENTS`, use that as a starting point.

## Step 2: Update the Tension Tree

Based on what was accomplished, propose tension updates. Common moves:

- **Reality update** (`cargo run --bin werk -- reality <id> "new reality"`): When progress was made but the tension isn't closed
- **Resolve** (`cargo run --bin werk -- resolve <id>`): When the gap is genuinely closed
- **Add child** (`cargo run --bin werk -- add -p <id> "desired" "actual"`): When new work was discovered
- **Note** (`cargo run --bin werk -- note <id> "observation"`): When there's something worth recording that isn't a state change

Present the proposed updates and wait for confirmation before executing. Do not update tensions the user didn't work on.

## Step 3: Commit

If there are uncommitted changes:

1. Stage relevant files (be specific, don't `git add -A`)
2. Write a commit message that captures the structural change, not just the code change. Follow the existing commit style (look at `git log --oneline -10`).
3. Commit and confirm.

If there's nothing to commit, say so and move on.

## Step 4: Generate Handoff Prompt

Write a prompt that the next session can consume to continue work. This prompt should:

1. Start with orientation instructions: which parts of the foundation doc to read, which tensions to look at
2. State what was just completed (so the next session doesn't redo it)
3. State what's open / what the natural next step is
4. Reference specific tension IDs, file paths, and foundation sections

Format:

```
Read designs/werk-conceptual-foundation.md [specific sections if relevant] and run `cargo run --bin werk -- tree` to orient. Then `cargo run --bin werk -- show <id>` on [relevant tensions].

Recently completed:
- [what was done, with tension IDs]

What's open:
- [next work, with tension IDs and file paths]

[Any specific context the next session needs]
```

Copy this prompt to the clipboard:
```bash
echo "the prompt" | pbcopy
```

Confirm to the user that the handoff is on their clipboard.
