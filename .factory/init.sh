#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

# Ensure nightly toolchain with required components
rustup install nightly --profile minimal 2>/dev/null || true
rustup component add clippy rustfmt --toolchain nightly 2>/dev/null || true

# Fetch dependencies
cargo fetch 2>/dev/null || true
