# GitButler Agent Workflow

Operating manual for using GitButler (`but`) as an AI coding agent. Derived from the werk `#242` session failure (2026-04-15), the exported session transcript (`3fcc8dc3-c69b-4222-9f86-6d0769786416.jsonl`), the existing `.claude/skills/gitbutler/` skill, and GitButler CLI 0.19.7 observable behavior.

## Executive summary

GitButler lets N virtual branches coexist in one working tree, merged into `gitbutler/workspace`. Each branch has its own staging area, and every uncommitted hunk has a tracked dependency on the commit chain that introduced its surrounding lines. When you try to commit a hunk to a branch that doesn't contain its dependency chain, GitButler silently drops that hunk from the commit. The command exits 0, reports `✓ Created commit <hash>`, prints `Warning: Some selected changes could not be committed.`, and leaves the hunks unassigned. Retrying the same command produces another empty commit. The fix is to stack your branch on the branch that contains the dependency: `but move <your-branch> <dependency-branch>`, then recommit.

**Three highest-cost failure modes observed:**

1. **Empty-commit dependency lock.** ~15 empty commits accumulated in the `#242` session before the agent noticed. Burn: 90 minutes. Fix: `but absorb --dry-run <file-id>` as preflight + `but move` to stack + verify non-empty post-commit.
2. **`but uncommit --discard` rebase cascade.** Discarding what looks like an empty follow-up commit actually discards a good commit whose hash shifted under a prior rebase. Hashes captured earlier in the session are stale. Fix: never carry commit hashes across mutations — re-read from `but status -fv` every time.
3. **`but teardown` escape hatch lands on stale tree.** Tearing down to reach a branch whose fork predates recent edits leaves the working tree full of changes that don't belong to that branch; stash-pop conflicts follow. Fix: don't escape — fix the lock via `but move`.

**Three highest-leverage fixes:**

1. **Symptoms → Fix ledger at the top of SKILL.md** (done) — the recipes already exist in the skill but were buried. Making them the first thing an agent reads ends the "I never consulted the skill until the end" pattern.
2. **Mandatory non-empty verification step** in the commit recipe (done) — every `but commit` is followed by an assertion that the commit contains the expected files. Turns silent failure into loud failure.
3. **Preflight with `but absorb --dry-run`** when commit target isn't obvious (done) — 0.5s of cost to avoid a 90-minute spiral.

## GitButler model (agent-oriented)

You don't need a theory of how it's implemented. You need a theory of *what can block you*. Five mechanisms matter.

**1. Virtual branches + workspace merge commit.** `gitbutler/workspace` is a synthetic git branch holding an octopus merge of all *applied* virtual branches. Your working tree reflects this merge, not any single branch. Committing to branch A adds a commit on A's underlying git ref; the workspace merge is rebuilt. Consequence: `git status`, `git checkout`, raw `git commit` all see the merge — not what you think they see. Use `but status -fv`.

**2. Per-branch staging areas.** Each virtual branch has its own staging area. Files are assigned to a branch via `but stage`, `but commit --changes`, or automatic rules. Files with no assignment sit in `zz` (unassigned). Consequence: "staged" is per-branch. The same file cannot be staged to two branches simultaneously.

**3. Hunk dependency tracking (the load-bearing mechanism for #242).** For every uncommitted hunk, GitButler computes which existing commit introduced the surrounding lines. That hunk is *dependency-locked* to the chain ending in that commit. If you try to commit it on a branch whose history doesn't contain that commit, GitButler drops the hunk (because applying it would require cherry-picking the dependency first, which it won't do silently). This is where the empty-commit failure mode lives. **Locks survive across session and process boundaries** — they're recomputed from the diff against the applied branches' tips.

**4. Stacks.** A stack is an ordered chain of virtual branches where each is rebased on the previous. `but status -fv` shows stacks as nested branches in the tree view. `but move <branch> <target-branch>` stacks `<branch>` on top of `<target-branch>`, which *pulls <target-branch>'s commits into <branch>'s history* — making any hunks locked to `<target-branch>` commitable on `<branch>`. `but move <branch> zz` tears off a branch from a stack.

**5. Oplog.** Every mutation is recorded as a snapshot. `but undo` reverts one step; `but oplog restore <snapshot-id>` jumps back further. Lost work from an errant `uncommit --discard` is usually recoverable via `but oplog` — check there *before* reaching for `git reflog`.

**Two things that surprise agents:**

- **`but commit` without `--changes` sweeps in every uncommitted change.** Unlike `git commit` (which commits staged changes), `but commit <branch>` with no `--changes` commits *everything currently assigned to that branch plus* — depending on auto-assignment — *everything unassigned too*. Always pass `--changes` explicitly. Reading `reference.md` line 210 confirms: "Without `--only`, ALL uncommitted changes are committed to the branch, not just staged files."
- **Commit hashes are ephemeral.** `but move`, `but squash`, `but uncommit`, and `but pull` all rebase the downstream chain. A hash captured ten commands ago is probably stale. GitButler's own error: `Failed to uncommit. Source '<hash>' not found. If you just performed a Git operation (squash, rebase, etc.), try running 'but status' to refresh the current state.` Never cache hashes across mutations.

## Failure taxonomy

Ranked by cost-per-incident, derived from the `#242` session transcript and pattern-matching into what GitButler's error paths can produce. Dates show when each was observed.

### F1 — Empty-commit dependency lock (90 min, 2026-04-15)

**Signature**

```
$ but commit aggregate-field -m "..." --changes ab,cd --status-after
✓ Created commit c56e894
Warning: Some selected changes could not be committed.
...(status still shows ab,cd as unassigned)...
$ git show --stat c56e894
(no files)
```

**Cause.** Hunks are locked to commits on a branch that isn't in `aggregate-field`'s history. `but commit` silently drops them; the resulting tree is identical to the parent; git creates an empty commit.

**Recovery.**

1. `but absorb --dry-run <file-id>` — confirm the lock target.
2. `but move <your-branch> <dependency-branch> --status-after` — stack your branch on the lock's owner.
3. `but commit <your-branch> -m "..." --changes <ids> --status-after` — retry; hunks are now in-scope.
4. `git show --stat <new-hash>` — verify non-empty.
5. `but clean --dry-run` then `but clean` — prune any stranded empty commits/branches.

**Documented?** Yes, in SKILL.md under "Stacked dependency / commit-lock recovery" — but only discoverable if the agent scrolls past the happy-path recipes. The Symptoms → Fix ledger (added in the skill revision) fixes this.

### F2 — Uncommit cascade through rebased hashes (20 min, 2026-04-15)

**Signature**

```
$ but uncommit --discard 00df593 ...
✓ Uncommitted
$ but uncommit --discard fd2b534 ...
Failed to uncommit. Source 'fd2b534' not found. If you just performed
a Git operation (squash, rebase, etc.), try running 'but status' to
refresh the current state.
```

What got lost in #242: the user queued 6 sequential `but uncommit --discard` calls. The first discarded a good commit (hash had shifted from a prior rebase), destroying working copies of `field.rs` and `aggregate.rs`. The subsequent calls failed with "source not found" because the stack kept rebasing under them.

**Cause.** Treating captured commit hashes as stable references across mutations. `but uncommit`, `but squash`, `but pull`, `but move` all rebase. Hashes are versioned by the tip of their branch; after rebase, the same logical commit has a new hash.

**Recovery.**

1. Stop. Don't issue more destructive operations.
2. `but oplog` — find the snapshot just before the first destructive call.
3. `but oplog restore <snapshot-id>` — roll back to that snapshot.
4. If oplog is no help: `git reflog`, then `git show <reflog-hash>:<path>` to extract file contents, write them out, and recommit.

**Prevention.** Between every mutation, run `but status -fv` and re-read IDs. Never issue more than one mutation per "re-inspect state" cycle.

**Documented?** Not explicitly. The skill revision's "What NOT to do when stuck" block addresses this.

### F3 — `but teardown` lands on stale tree (20 min, 2026-04-15)

**Signature**

```
$ but teardown
$ git checkout aggregate-field
$ ls werk-tab/   # doesn't exist on this branch's tip
$ git stash pop   # conflicts against tree that doesn't have the directories
```

**Cause.** `aggregate-field` was created off the workspace merge, but its underlying ref tracks only its own commits. After teardown + checkout, git's working tree resets to that branch's tip — which in `#242` was forked from a `main` predating `werk-tab/` and `werk-web/src/lib.rs`. The in-flight edits don't belong to that tree.

**Recovery (if already in this state).**

1. Do not stash-pop into the conflicts.
2. `git checkout gitbutler/workspace` — return to the merge.
3. `but setup` — re-enter GitButler mode.
4. Recover edits from stash via `git stash show -p` → save as patch → `git stash drop` → re-apply by hand.
5. Now apply F1 recovery.

**Prevention.** Never use `but teardown` as an escape hatch for a failed commit. It doesn't solve the problem and adds a new one.

**Documented?** Addressed in the revised skill's "What NOT to do when stuck" block.

### F4 — `but commit` without `--changes` sweeps in unrelated files (potential, not observed in #242)

**Signature.** A commit with 20 file changes when you meant 3.

**Cause.** `but commit <branch> -m "..."` with no `--changes` commits every uncommitted hunk assigned to (or auto-assignable to) that branch. On a busy workspace this can pull in config files, cache files, or other agents' in-flight work.

**Recovery.** `but uncommit <commit-id>` — moves changes back to unstaged. Then recommit with explicit `--changes`.

**Prevention.** Always use `--changes <ids>` for commits in an agent context. Treat bare `but commit <branch> -m "..."` as an anti-pattern.

**Documented?** `reference.md:210` says so clearly. The Non-Negotiable Rules in the revised skill reinforce it.

### F5 — `but pr new` fails because forge auth isn't configured (potential, observed as warning)

**Signature**

```
$ but pr new aggregate-field
Error: Forge not configured. Run `but config forge auth` to authenticate.
```

**Cause.** `but config` shows `Forge: ✗ Not configured`.

**Recovery.** `but config forge auth` → follow prompts.

**Prevention.** `but config` as part of session-start check if PR creation is in scope.

### F6 — Hook-blocked raw `git commit` (observed 2026-04-15)

**Signature**

```
$ git commit -m "..."
GITBUTLER_ERROR: Cannot commit directly to gitbutler/workspace branch.
```

**Cause.** `.githooks/pre-commit` explicitly blocks direct commits on `gitbutler/workspace`. By design.

**Recovery.** Use `but commit <branch> --changes <id>` — the hook is protecting you from committing to the synthetic merge branch. There is no good reason to bypass it with `--no-verify`.

**Documented?** Added to "What NOT to do when stuck" in the revised skill.

### F7 — `but absorb --new` lands changes on the wrong branch (observed 2026-04-15)

**Signature.** You run `but absorb --new <your-branch>` hoping to force hunks onto `<your-branch>`, but they end up on whichever branch the hunk-range-lock points to.

**Cause.** `--new` creates new commits instead of amending existing ones, but it still respects the dependency lock — changes are placed on the branch whose commit-chain contains their dependency, not on `<your-branch>`.

**Recovery.** Apply F1: `but move` to stack, then commit normally.

**Documented?** Implicit in the existing skill's description of absorb's smart matching; the revised Symptoms → Fix table makes it explicit.

## Multi-session workflow playbook

Most of the failure modes above are single-session problems. When N agents run against the same repo concurrently, the additional concerns are: branch naming collisions, competing auto-assignment, and oplog churn.

### Branch naming

One convention for the whole fleet:

```
<agent-slug>/<short-topic>-<YYYYMMDD>
```

Examples: `cc-moritz/aggregate-field-20260415`, `cod-alice/refactor-types-20260415`. Rationale: eliminates collisions when N agents create branches named `feature` within the same workspace; date suffix makes stale branches easy to `but clean` later; `<agent-slug>` makes the owning session obvious from `but status -fv`.

### Base selection

Order of preference when picking a base to stack on:

1. **A specific branch whose files you're editing.** If you're touching files another branch just committed, stack on that branch from the start — you avoid F1 proactively. `but branch new <name> -a <anchor>`.
2. **The latest integrated branch.** If your work should merge after branch X, stack on X. Avoids having to rebase later.
3. **Bare `but branch new <name>` (stacks on `main`).** Default for genuinely parallel, independent work.

When in doubt, scan `but status -fv` for existing applied branches that touched the same files you're about to edit. Use `git log --oneline -- <path>` to see recent history per-file.

### Pre-flight checks (run before every mutation-heavy phase)

```bash
but status -fv                 # 1. state of applied branches, IDs
but config 2>&1 | head         # 2. forge config, target branch
but pull --check               # 3. target branch drift (no mutation)
```

If any check surfaces something unexpected (stale branches applied that aren't yours, conflicted commits, forge auth missing), fix before making edits. Each check is free.

### Per-commit pre-flight

For anything beyond a one-file one-line commit:

```bash
but status -fv                     # re-read IDs; they change between mutations
but absorb --dry-run <file-id>     # where would this file actually go?
```

If `absorb --dry-run` targets a branch that isn't the one you intended, stack first.

### Post-commit verification (mandatory)

```bash
# Immediately after but commit:
git show --stat <new-hash>   # must list the expected files with expected line counts
but status -fv               # files previously at zz or on another branch must now be gone
```

If either check fails, do not retry. Apply F1 recovery.

### Concurrent-session coordination

**Isolation options, in order of preference:**

1. **Separate worktrees.** `git worktree add ../werk-alice ...`, each agent operates its own worktree. GitButler's config and virtual-branch state live per worktree. This is the only fully-isolated option and the right default when agents are likely to touch overlapping files. Caveat: worktrees share the object store, so disk pressure remains shared; `but setup` must be run in each.
2. **Shared worktree, disjoint branches.** Each agent creates its own branch; `but rub` auto-assignment routes each file to its owner. Works if agents touch different files. Fails loudly (dependency locks, empty commits) if they don't — treat this as an early-warning signal, not a disaster.
3. **Shared worktree, explicit file ownership.** Agents agree via external coordination (agent-mail, tmux locks) on which files they touch. Risk-mitigating but coordination-heavy.

**When another agent's lock is blocking you.** Treat it like F1: `but absorb --dry-run` identifies the owning branch. If the other agent is still active, do not `but move` onto their branch — coordinate first (pause the other agent, or wait for their commit). If they've stopped, stack your branch on theirs.

### Handoff

When a session ends mid-work:

1. `but status -fv --json > handoff-status.json` — current state snapshot.
2. `werk note <id> "HANDOFF: <summary of what's committed, what's uncommitted, what the next agent should do>"` — durable narrative for #242-style tensions. (Repo-specific; use the project's issue tracker otherwise.)
3. Do **not** teardown. Leave GitButler active so the next agent inherits the workspace merge.
4. `but push` pushed branches that should be visible to a human reviewer. `but pr new` if the work is ready.

### Known-safe commands for concurrent sessions

- Read-only: `but status`, `but show`, `but diff`, `but branch list`, `but oplog`.
- Non-destructive mutation: `but commit` (on your own branch, with `--changes`), `but stage` (to your own branch), `but branch new`, `but push`.
- Destructive: `but uncommit --discard`, `but branch delete`, `but undo`, `but squash`, `but move`, `but absorb` (without `--dry-run`). Only issue these when no other agent is mid-mutation.

## Tooling gaps, ranked

### 1. `but commit --assert-nonempty` — upstream feature request (highest impact)

**Problem.** Silent empty-commit is the #1 failure mode by cost. GitButler has the knowledge (the Warning string proves it) but returns exit 0 anyway.

**Proposal.** Add `--assert-nonempty` flag that makes `but commit` exit non-zero if any selected change was dropped or if the resulting commit has zero file diffs. Equivalent could be an environment variable `BUT_AGENT_MODE=1` that promotes all such warnings to errors.

**Where.** File a feature request with GitButler (`gh repo view gitbutlerapp/gitbutler`).

**Interim local wrapper.** Post-commit check in `sanity-check.sh verify-commit <branch> <expected-file-count>`. Ships today.

### 2. `but doctor` — upstream feature request (high impact)

**Problem.** No single command summarizes workspace health. Agents need to inspect state across `status`, `config`, `oplog`, `pull --check`, `absorb --dry-run` to be confident.

**Proposal.** `but doctor` reports: forge auth, target branch drift, applied-but-empty branches, dependency-locked uncommitted files (with their lock targets), conflicted commits, oplog depth since last push. Exit 0 healthy, 1 needs attention, 2 broken.

**Interim local wrapper.** The sanity-check script covers the common subset.

### 3. Promote warnings to errors in agent mode — local wrapper (high impact)

**Problem.** GitButler emits warnings on stderr that agents don't parse. "Some selected changes could not be committed" is the canonical example.

**Proposal.** Shell wrapper `but-strict` that runs `but "$@"` and greps stderr for `^Warning:`; if any warning matched, exits non-zero with the warning text. Installed as alias in agent shells.

```bash
# ~/.local/bin/but-strict
#!/usr/bin/env bash
out=$(but "$@" 2>&1)
ec=$?
echo "$out"
if echo "$out" | grep -qE '^Warning:'; then
  exit $((ec + 10))
fi
exit $ec
```

Then the agent CLAUDE.md / skill directs agents to `but-strict commit ...` for commits where empty-commit would be a serious problem.

### 4. `.claude/hooks/` PostToolUse hook for commit verification — local config (medium impact)

**Problem.** Even with the skill teaching `git show --stat <hash>`, an agent may skip the verification step.

**Proposal.** A PostToolUse hook that, when it sees a Bash tool call matching `but commit`, inspects the most recent commit on the branch and aborts the turn if it's empty. Implementation in `.claude/hooks/post-but-commit.sh`, wired via `settings.json`.

Trade-off: adds latency to every commit; prone to false-positives on intentional empty commits (`but commit empty --before`). Ship only after F1 recurs despite the skill revision.

### 5. Skill-level pre-push `but clean --dry-run` — skill guidance (low impact, zero cost)

**Problem.** Sessions with F1 churn leave stranded empty branches and empty commits that clutter `but status` and sometimes the remote.

**Proposal.** Add to SKILL.md's push recipe: `but clean --dry-run` before `but push`; run `but clean` if it reports prunable branches. Already covered by the `reference.md` `but clean` entry — just needs promotion into the standard flow.

## Sanity-check script

`.claude/skills/gitbutler/sanity-check.sh` — run at session start and after every commit. See that file for invocation details. Exit codes: 0 healthy, 1 needs attention, 2 broken.

## What was deliberately not done in this pass

- **Reading GitButler's full Rust source tree.** The observable behavior documented here (via CLI help, error messages, the `#242` transcript) is sufficient for the agent-facing skill. Source-level accuracy about hunk-range-lock internals would change none of the recovery recipes.
- **Filing the upstream feature requests (gaps 1 and 2).** Skill-level fixes and the sanity-check script cover the 90% case. Upstream filings should be a separate, lower-urgency batch once the skill revision has been validated across a few sessions.
- **Mining cass for more F2/F3 instances.** The cass corpus covers 2025-10-30 through 2026-04-09 (the `#242` session was re-indexed to surface it). Targeted searches for `hunk-range-lock`, `uncommit --discard`, `gitbutler/workspace` returned only this session's failures — the corpus simply doesn't contain more. Future sessions that hit these modes should add instances to the F* taxonomy here.
