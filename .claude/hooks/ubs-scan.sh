#!/bin/bash
# UBS (Ultimate Bug Scanner) — Claude Code PostToolUse hook
# Runs on file writes/edits for Rust files in this project

# Read the tool result JSON from stdin to get the file path
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // .tool_input.path // empty' 2>/dev/null)

if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

# Only scan Rust files
if [[ ! "$FILE_PATH" =~ \.rs$ ]]; then
  exit 0
fi

# Skip if ubs not installed
if ! command -v ubs >/dev/null 2>&1; then
  exit 0
fi

# Run UBS on the specific file, quiet mode, fail on critical only
RESULT=$(ubs --only=rust --files="$FILE_PATH" --quiet 2>&1)
CRITICALS=$(echo "$RESULT" | grep -c "CRITICAL" || true)

if [[ "$CRITICALS" -gt 0 ]]; then
  echo "$RESULT" | grep -B1 -A2 "CRITICAL" | head -30
fi
