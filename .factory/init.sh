#!/bin/bash
set -e

# Ensure Rust toolchain is available
rustup show active-toolchain || rustup default stable

# Build workspace to verify everything compiles
cargo build --workspace
