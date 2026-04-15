#!/usr/bin/env bash
# GitButler agent sanity-check
#
# Modes:
#   start                      Session preflight. Assert GitButler workspace is healthy and usable.
#   verify-commit <branch>     Post-commit check. Assert the topmost commit on <branch> is non-empty.
#   verify-commit <branch> <n> Assert the topmost commit on <branch> contains exactly <n> files.
#
# Exit codes:
#   0   healthy / check passed
#   1   needs attention (non-fatal — investigate before proceeding)
#   2   broken (stop; take manual recovery action)
#
# Not intended to run as a hook. Call from Bash tool when useful.

set -u

die()      { echo "ERROR: $*" >&2; exit 2; }
warn()     { echo "WARN:  $*" >&2; }
info()     { echo "ok:    $*"; }

require_tool() {
  command -v "$1" >/dev/null 2>&1 || die "$1 not found on PATH"
}

mode="${1:-start}"

case "$mode" in

  start)
    require_tool but
    require_tool git

    # 1. Inside a git repo?
    git rev-parse --git-dir >/dev/null 2>&1 || die "not inside a git repository"

    # 2. GitButler set up? (workspace branch exists)
    git show-ref --quiet refs/heads/gitbutler/workspace \
      || { warn "gitbutler/workspace ref missing — run 'but setup'"; exit 1; }

    # 3. Currently on gitbutler/workspace? (required for most but commands)
    current_branch=$(git symbolic-ref --short HEAD 2>/dev/null || echo "")
    if [ "$current_branch" != "gitbutler/workspace" ]; then
      warn "HEAD is on '$current_branch', not gitbutler/workspace — may need 'but setup'"
      exit 1
    fi

    # 4. but status runs cleanly
    if ! but status >/dev/null 2>&1; then
      die "but status failed — GitButler workspace state is broken"
    fi

    # 5. Forge configured? (non-fatal warning)
    if but config 2>/dev/null | grep -q "Forge:.*Not configured" \
       || but config forge 2>/dev/null | grep -qi "No forge accounts"; then
      warn "forge not configured — 'but pr new' will fail; run 'but config forge auth' for GitHub OAuth, or use 'gh pr create' as fallback"
    fi

    # 6. Any conflicted commits? (fatal — must resolve before new work)
    status_out=$(but status 2>&1)
    if echo "$status_out" | grep -qi "conflict"; then
      die "conflicted commits detected — run 'but resolve <commit-id>' before further work"
    fi

    # 7. Surface: applied branch count + unpushed commit summary
    applied_count=$(echo "$status_out" | grep -cE '^┊╭┄|^┊├┄' || true)
    info "GitButler workspace healthy ($applied_count applied branches)"
    exit 0
    ;;

  verify-commit)
    require_tool but
    require_tool git

    branch="${2:-}"
    expected_count="${3:-}"

    if [ -z "$branch" ]; then
      die "usage: sanity-check.sh verify-commit <branch> [<expected-file-count>]"
    fi

    # Find the branch's tip commit hash. Use git rev-parse against the local ref.
    if ! top_hash=$(git rev-parse --verify "$branch" 2>/dev/null); then
      die "branch '$branch' not found (git rev-parse failed)"
    fi

    # Count files in that commit.
    file_count=$(git show --stat --format="" "$top_hash" 2>/dev/null \
      | grep -vE '^\s*$|files? changed' \
      | wc -l | tr -d ' ')

    if [ "$file_count" -eq 0 ]; then
      echo "EMPTY COMMIT: $branch tip ($top_hash) has zero files" >&2
      echo "Likely cause: dependency lock. See SKILL.md 'Stacked dependency / commit-lock recovery'." >&2
      echo "Recovery: but absorb --dry-run <file-id>  then  but move $branch <dep-branch>" >&2
      exit 2
    fi

    if [ -n "$expected_count" ] && [ "$file_count" -ne "$expected_count" ]; then
      warn "$branch tip ($top_hash) contains $file_count files; expected $expected_count"
      git show --stat "$top_hash" | tail -20 >&2
      exit 1
    fi

    info "$branch tip ($top_hash) is non-empty ($file_count files)"
    exit 0
    ;;

  *)
    echo "usage: sanity-check.sh start"
    echo "       sanity-check.sh verify-commit <branch> [<expected-file-count>]"
    exit 2
    ;;
esac
