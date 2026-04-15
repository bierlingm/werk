---
name: but
version: 0.19.7
description: "Commit, push, branch, and manage version control with GitButler. Use for: commit my changes, check what changed, create a PR, push my branch, view diff, create branches, stage files, edit commit history, squash commits, amend commits, undo commits, pull requests, merge, stash work. Replaces git - use 'but' instead of git commit, git status, git push, git checkout, git add, git diff, git branch, git rebase, git stash, git merge. Covers all git, version control, and source control operations."
author: GitButler Team
---

# GitButler CLI Skill

Use GitButler CLI (`but`) as the default version-control interface.

## Non-Negotiable Rules

1. Use `but` for all write operations. Never run `git add`, `git commit`, `git push`, `git checkout`, `git merge`, `git rebase`, `git stash`, or `git cherry-pick`. If the user says a `git` write command, translate it to `but` and run that.
2. Always add `--status-after` to mutation commands.
3. Use CLI IDs from `but status -fv` / `but diff` / `but show`; never hardcode IDs.
4. Start with `but status -fv` before mutations so IDs and stack state are current.
5. Create a branch for new work with `but branch new <name>` when needed.
6. **Verify every commit lands non-empty.** `but commit` can exit 0 and report `✓ Created commit` while actually producing an empty commit (see Symptoms → Fix: empty commit). After every commit, either inspect `--status-after` output for the new commit's file list, or run `git show --stat <hash>`. If empty, stop and recover — do not retry the same command.

## Symptoms → Fix

Consult this table *first* when something looks wrong. Each entry points to the full recipe below.

| Symptom | Likely cause | Jump to |
|---|---|---|
| `Warning: Some selected changes could not be committed.` + empty commit hash | Dependency (hunk-range) lock — changes depend on a commit on another branch | [Dependency lock recovery](#stacked-dependency--commit-lock-recovery) |
| File stays in `unassignedChanges` / `zz` after `but commit ... --changes <id>` | Same as above | [Dependency lock recovery](#stacked-dependency--commit-lock-recovery) |
| `'<hunk-id>' is assigned to a different stack. Use 'but rub <id> zz' to unassign it first.` | You used `-p <hunk-id>`; hunk already claimed by another stack | Run `but rub <id> zz` then retry, *or* use [Dependency lock recovery](#stacked-dependency--commit-lock-recovery) |
| `Failed to uncommit. Source '<hash>' not found. ... try running 'but status'` | Previous `uncommit`/`squash`/`move` rebased the stack; hash you captured is stale | Re-read the new hash from `but status -fv` before the next mutation. Never carry commit hashes across mutations. |
| `but commit` on `gitbutler/workspace` directly via `git commit` → hook blocks with `GITBUTLER_ERROR: Cannot commit directly to gitbutler/workspace branch.` | You bypassed `but`; the hook is doing its job | Use `but commit <branch> --changes <id>` instead. Never use raw `git commit` in the workspace. |
| `but teardown` + `git checkout <branch>` shows a tree missing recent files | Branch forked from a `main` predating those files; working-tree changes don't belong to that tree | Do **not** teardown to escape. Use [Dependency lock recovery](#stacked-dependency--commit-lock-recovery) instead. |
| `but commit -m ...` without `--changes` swept in files you didn't want | Bare `but commit` commits *all* uncommitted changes on the branch | Always pass `--changes <id>` (or `--only` after explicit `but stage`) for precise commits |
| Merge conflict markers appear after `but move`/`but pull` | Rebase produced a conflicted commit | [Resolve conflicts after reorder/move](#resolve-conflicts-after-reordermove) |
| `but absorb` placed changes somewhere unexpected | Absorb follows hunk-range locks to their anchor commits, not to your active branch | Run `but absorb <file-id> --dry-run` first; if the target is wrong, use [Dependency lock recovery](#stacked-dependency--commit-lock-recovery) |

## What NOT to do when stuck

These moves look tempting but deepen the hole:

- **Do not `but uncommit --discard` to "reset"** — if a prior op rebased the stack, the hash you think is the bad commit may actually be your good commit. Recovery is possible via `git reflog` but costly.
- **Do not `but teardown` + `git checkout`** — you'll land on a raw git branch whose tree may not match the workspace you just left. Stash-pop conflicts follow.
- **Do not raw `git commit` (or `git commit --no-verify`)** — the pre-commit hook blocks it on `gitbutler/workspace` by design. If you're tempted, the correct move is `but move <branch> <dependency-branch>`.
- **Do not retry the same `but commit` expecting a different result** — if it produced an empty commit once with no input change, it will again. Stop, diagnose, apply the dependency-lock recipe.

## Forge configuration (for `but pr new`)

`but pr new` delegates to GitButler's forge integration (GitHub/GitLab OAuth), which is separate from `gh auth`. If `but config forge` shows `No forge accounts configured`, `but pr new` will fail.

**One-time setup (interactive, opens a browser):**

```bash
but config forge auth
```

**Fallback when you can't run interactive commands (e.g., non-TTY agent sessions):** use `gh pr create --base <base> --head <head> --title "..." --body "..."`. The PR is real and functions identically; `but branch list --review` will enrich it once forge auth is later configured.

Prefer `but pr new` when forge is configured — it keeps `but status` and `but branch list --review` in sync.

## Core Flow

**Every write task** should follow this sequence.

```bash
# 1. Inspect state and gather IDs
but status -fv

# 2. If new branch needed:
but branch new <name>

# 3. Edit files (Edit/Write tools)

# 4. Refresh IDs if needed
but status -fv

# 5. Perform mutation with IDs from status/diff/show
but <mutation> ... --status-after
```

## Command Patterns

- Commit: `but commit <branch> -m "<msg>" --changes <id>,<id> --status-after`
- Commit + create branch: `but commit <branch> -c -m "<msg>" --changes <id> --status-after`
- Amend: `but amend <file-id> <commit-id> --status-after`
- Reorder commits: `but move <source-commit-id> <target-commit-id> --status-after` (**commit IDs**, not branch names)
- Stack branches: `but move <branch-name-or-id> <target-branch-name-or-id> --status-after` (**branch names or branch CLI IDs**)
- Tear off a branch: `but move <branch-name-or-id> zz --status-after` (`zz` = unassigned; branch name or branch CLI ID)
- Equivalent branch subcommand syntax remains available: `but branch move <branch-name> <target-branch-name>` and `but branch move --unstack <branch-name>`
- Push: `but push` or `but push <branch-id>`
- Pull: `but pull --check` then `but pull --status-after`

## Task Recipes

### Commit files

1. `but status -fv`
2. Find the CLI ID for each file you want to commit.
3. `but commit <branch> -m "<msg>" --changes <id1>,<id2> --status-after`
   Use `-c` to create the branch if it doesn't exist. Omit IDs you don't want committed.
4. **Verify the commit is non-empty.** Read two things from the `--status-after` output:
   (a) the target files left `unassignedChanges` / `zz` (they're now committed); (b) no `Warning: Some selected changes could not be committed.` message appears. If either check fails, do NOT retry the same command — jump to [Dependency lock recovery](#stacked-dependency--commit-lock-recovery). For extra safety, `git show --stat <hash>` confirms the commit contains real diffs.

### Preflight: detect dependency locks before committing

If you're unsure whether your changes will land on the branch you want (e.g., you're committing to a new branch while unrelated branches were recently active), run this one-liner *before* `but commit`:

```bash
but absorb --dry-run <file-id>
```

If the dry-run target commit sits on a branch other than the one you intended to commit to, those hunks are dependency-locked to that other branch. Either stack your branch (preferred — see recipe below) or commit to the locked branch. This check costs nothing and prevents the empty-commit spiral.

### Amend into existing commit

1. `but status -fv` (or `but show <branch-id>`)
2. Locate file ID and target commit ID.
3. `but amend <file-id> <commit-id> --status-after`

### Reorder commits

`but move` supports both commit reordering and branch stack operations. Use commit IDs when reordering commits.

1. `but status -fv`
2. `but move <commit-a> <commit-b> --status-after` — uses commit IDs like `c3`, `c5`
3. Refresh IDs from the returned status, then run the inverse: `but move <commit-b> <commit-a> --status-after`

### Stack existing branches

To make one existing branch depend on (stack on top of) another, use top-level `move`:

```bash
but move feature/frontend feature/backend
```

This moves the frontend branch on top of the backend branch in one step.

Equivalent subcommand syntax:

```bash
but branch move feature/frontend feature/backend
```

**DO NOT** use `uncommit` + `branch delete` + `branch new -a` to stack existing branches. That approach fails because git branch names persist even after `but branch delete`. Always use `but move <branch> <target-branch>` (or the equivalent `but branch move ...`).

**To unstack** (make a stacked branch independent again):

```bash
but move feature/logging zz
```

Equivalent subcommand syntax:

```bash
but branch move --unstack feature/logging
```

**Note:** branch stack/tear-off operations use branch **names** (like `feature/frontend`) or branch CLI IDs, while commit reordering uses commit **IDs** (like `c3`). Do NOT use `but undo` to unstack — it may revert more than intended and lose commits.

### Stacked dependency / commit-lock recovery

A **dependency lock** occurs when a file was originally committed on branch A, but you're trying to commit changes to it on branch B. Symptoms:
- `but commit` succeeds but the file still appears in `unassignedChanges` in the `--status-after` output
- The file shows as "unassigned" instead of being staged to any branch

**Recovery:** Stack your branch on the dependency branch, then commit:

1. `but status -fv` — identify which branch originally owns the file (check commit history).
2. `but move <your-branch-name> <dependency-branch-name>` — stack your branch on the dependency. Uses full branch **names**, not CLI IDs.
3. `but status -fv` — the file should now be assignable. Commit it.
4. `but commit <branch> -m "<msg>" --changes <id> --status-after`

**If `but move <branch> <target-branch>` fails:** Do NOT try `uncommit`, `squash`, or `undo` to work around it — these will leave the workspace in a worse state. Instead, re-run `but status -fv` to confirm both branches still exist and are applied, then retry with exact branch names from the status output.

### Resolve conflicts after reorder/move

**NEVER use `git add`, `git commit`, `git checkout --theirs`, `git checkout --ours`, or any git write commands during resolution.** Only use `but resolve` commands and edit files directly with the Edit tool.

If `but move` causes conflicts (conflicted commits in status):

1. `but status -fv` — find commits marked as conflicted.
2. `but resolve <commit-id>` — enter resolution mode. This puts conflict markers in the files.
3. **Read the conflicted files** to see the `<<<<<<<` / `=======` / `>>>>>>>` markers.
4. **Edit the files** to resolve conflicts by choosing the correct content and removing markers.
5. `but resolve finish` — finalize. Do NOT run this without editing the files first.
6. Repeat for any remaining conflicted commits.

**Common mistakes:** Do NOT use `but amend` on conflicted commits (it won't work). Do NOT skip step 4 — you must actually edit the files to remove conflict markers before finishing.

## Git-to-But Map

| git | but |
|---|---|
| `git status` | `but status -fv` |
| `git add` + `git commit` | `but commit ... --changes ...` |
| `git checkout -b` | `but branch new <name>` |
| `git push` | `but push` |
| `git rebase -i` | `but move`, `but squash`, `but reword` |
| `git rebase --onto` | `but branch move <branch> <new-base>` |
| `git cherry-pick` | `but pick` |

## Notes

- Prefer explicit IDs over file paths for mutations.
- `--changes` accepts comma-separated values (`--changes a1,b2`) or repeated flags (`--changes a1 --changes b2`), not space-separated.
- Read-only git inspection (`git log`, `git blame`, `git show --stat`) is allowed.
- After a successful `--status-after`, don't run a redundant `but status -fv` unless you need new IDs.
- Use `but show <branch-id>` to see commit details for a branch, including per-commit file changes and line counts.
- **Per-commit file counts**: `but status` does NOT include per-commit file counts. Use `but show <branch-id>` or `git show --stat <commit-hash>` to get them.
- Avoid `--help` probes; use this skill and `references/reference.md` first. Only use `--help` after a failed attempt.
- Run `but skill check` only when command behavior diverges from this skill, not as routine preflight.
- For command syntax and flags: `references/reference.md`
- For workspace model: `references/concepts.md`
- For workflow examples: `references/examples.md`
- For session preflight + post-commit verification: `sanity-check.sh` (run `.claude/skills/gitbutler/sanity-check.sh start` to assert the workspace is healthy, `... verify-commit <branch>` after any commit)
- For the agent-centric mental model, failure taxonomy, and multi-session playbook: `../../../designs/gitbutler-agent-workflow.md`
