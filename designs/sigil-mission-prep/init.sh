#!/usr/bin/env bash
# init.sh — idempotent environment setup for the sigil engine v1 mission.
# Runs at the start of every worker session.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "[init] repo root: $REPO_ROOT"

# 1. Confirm rustup + nightly toolchain (the project pins to nightly via
#    rust-toolchain.toml; rustup will auto-install if missing).
if ! command -v cargo >/dev/null 2>&1; then
  echo "[init] ERROR: cargo not found in PATH; install rustup first" >&2
  exit 1
fi
echo "[init] cargo: $(cargo --version)"

# 2. Ensure ~/.werk/sigils/ tree exists (engine consumers will write here).
mkdir -p "$HOME/.werk/sigils/cache"
echo "[init] ensured ~/.werk/sigils/cache exists"

# 3. Confirm tensions.json is present (fixture data).
if [ ! -f "$REPO_ROOT/tensions.json" ]; then
  echo "[init] WARNING: tensions.json missing; some fixtures will not work" >&2
else
  TENSION_COUNT=$(grep -c '"id":' "$REPO_ROOT/tensions.json" || true)
  echo "[init] tensions.json present (~$TENSION_COUNT tensions)"
fi

# 4. Pre-build werk-core so subsequent worker tasks have a warm target/.
#    Skipped if already built (cargo handles incremental).
if [ "${SKIP_PREBUILD:-0}" != "1" ]; then
  echo "[init] pre-building werk-core (idempotent, fast on warm cache)"
  cargo build -p werk-core --quiet
fi

echo "[init] done"
