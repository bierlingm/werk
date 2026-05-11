#!/usr/bin/env bash
# snapshot-capabilities.sh — R-013 golden-artifact capture.
#
# Refreshes werk-cli/tests/golden/capabilities.json from a freshly-built
# `werk doctor capabilities --json`. Run this AFTER any change that
# intentionally modifies the doctor's capabilities surface (new detector,
# new fixer, new exit code, schema_version bump). CI fails if the file
# drifts without this script being run.
#
# Usage:
#   ./scripts/snapshot-capabilities.sh           # update golden file
#   ./scripts/snapshot-capabilities.sh --check   # exit non-zero on drift
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
golden="$repo_root/werk-cli/tests/golden/capabilities.json"
mode="${1:-update}"

# Build once (debug is fine — schema is the same).
cargo build --quiet --package werk --bin werk

# Run from a throwaway workspace so capabilities is environment-pure.
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

actual="$tmp/capabilities.json"
( cd "$tmp" && "$repo_root/target/debug/werk" doctor capabilities --json ) > "$actual"

case "$mode" in
  --check)
    if ! diff -u "$golden" "$actual"; then
      echo "ERROR: capabilities surface drifted. Run scripts/snapshot-capabilities.sh to update." >&2
      exit 1
    fi
    echo "capabilities golden artifact matches."
    ;;
  *)
    cp "$actual" "$golden"
    echo "Updated $golden"
    ;;
esac
