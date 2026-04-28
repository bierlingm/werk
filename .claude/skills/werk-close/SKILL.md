---
name: werk-close
description: "Close a werk development session. Surveys the delta, builds a structured proposal for tension updates and version control, presents it for approval, then executes everything. Use at the end of any work session."
disable-model-invocation: true
allowed-tools: Bash, Read, Grep, Glob, Edit, Agent, AskUserQuestion
argument-hint: "[optional: notes about what was accomplished or specific tension IDs to update]"
---

# close-session — Session Closing

You are closing a werk development session. Your job is to look at what happened, propose a clean landing, get approval, then execute it.

**The cardinal rule: propose first, act second.** Never update tensions, commit, push, or merge without explicit approval.

## Step 1: Survey the Delta

Gather everything in parallel. You need the full picture before proposing anything.

### Code changes
```bash
git status
git diff --stat HEAD
git log --oneline HEAD~15..HEAD   # recent commits for context
```

### Tension state
```bash
cargo run --bin werk -- tree 2>/dev/null
cargo run --bin werk -- log --since 4h 2>/dev/null   # recent tension activity
```

### Build health
```bash
cargo test --workspace 2>&1 | tail -5
```

Capture the results. You need:
- Which files are modified/uncommitted
- Which branches exist and their commit state (pushed? unpushed? integrated?)
- Which tensions were touched this session (from `werk log`)
- Whether tests pass

If the user provided `$ARGUMENTS`, parse them:
- Bare numbers are tension IDs to focus on
- Quoted strings are notes about what was accomplished
- Use these to inform the proposal, not to skip the survey

## Step 2: Build the Proposal

Present a structured proposal with numbered sections. Each section is independently approvable/modifiable.

### Format

```
## Session Close Proposal

### 1. Tension Updates
For each tension that needs updating, show:

  #<id> <current-state> -> <proposed-state>
  <what to update: reality text / resolve / note / new child>
  Reason: <why this update>

If no tensions need updating, say so.

### 2. Code — Uncommitted Changes
For each uncommitted change, show:

  <branch>: <files> -> <proposed action: commit / amend / discard>
  Message: "<proposed commit message>"

If the workspace is clean, say so.

### 3. Code — Branch Cleanup
For each branch, show:

  <branch>: <proposed action: push / merge PR / leave / unapply>
  Reason: <why>

If nothing to do, say so.

### 4. PR Actions
For each branch that should become or update a PR:

  <branch> -> PR "<title>"
  Action: <create / merge / update>

If no PR actions, say so.

Approve all, or tell me what to change.
```

### Proposal Rules

- **Only propose tension updates for tensions touched this session.** Don't speculatively update tensions the user didn't work on.
- **Reality updates should be honest and specific.** State what exists now, not aspirational prose. Include concrete artifacts (PR numbers, function names, file counts).
- **Resolve only when the gap is genuinely closed.** If uncertain, propose a reality update instead and flag the question.
- **Commit messages should capture the structural change**, not just list files. Follow the existing commit style from `git log --oneline`.
- **Branch cleanup should be conservative.** Don't propose deleting or unapplying branches unless they're fully integrated.
- **If tests fail, do not propose committing.** Flag the failure and ask what to do.

## Step 3: Execute on Approval

Once the user approves (possibly with modifications), execute each section in order.

### Tension updates

Use the standard werk CLI commands:
```bash
cargo run --bin werk -- reality <id> "<text>"
cargo run --bin werk -- resolve <id>
cargo run --bin werk -- note <id> "<text>"
```

### Code commits

Plain `git`. Stage explicitly, commit on a feature branch (not directly on `main`):

1. `git status` and `git diff` to confirm what's about to be committed
2. `git checkout -b <topic>` if not already on a feature branch
3. `git add <paths>` (avoid `git add -A` — `.githooks/pre-commit` will flush `tensions.json` and stage it for you)
4. `git commit -m "<msg>"` — let the pre-commit hook run; don't `--no-verify`

### Branch operations

```bash
git push -u origin <branch>             # push and set upstream
gh pr create --fill                     # PR from single-commit branch (auto-message)
gh pr create -t "<title>" -b "<body>"  # PR with message
git branch -d <branch>                  # remove merged local branches
```

### PR merges

```bash
gh pr merge <number> --squash --delete-branch
git checkout main && git pull --ff-only
```

### Post-execution verification

After all actions complete:
```bash
git status                              # confirm clean state
cargo run --bin werk -- show <ids>      # confirm tension updates landed
```

Report the final state to the user: what was committed, what was pushed, what was merged, what tensions were updated.

## Error Handling

- **Merge conflicts during pull**: Do NOT auto-resolve. Show the conflict and ask.
- **Dependency-locked files**: Explain which branch owns the file and propose stacking.
- **Empty commits**: If `git commit` succeeds but produced no diff, flag it — don't retry blindly.
- **Failed tests**: Stop before committing. Show the failure and ask.
- **Integrated but unapplied branches**: Note them in the cleanup section but don't force-delete.

## What This Skill Does NOT Do

- Generate session prompts (use `/werk-session` for that)
- Triage what to work on next (use `/triage` for that)
- Make architectural decisions about code changes
- Write or modify source code
