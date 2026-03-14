#!/bin/bash
# Post-mutation hook: auto-commit .werk/sd.db changes.
# Configure: werk config set hooks.post_mutation "~/.werk/hooks/auto-commit.sh"
EVENT=$(cat)
FIELD=$(echo "$EVENT" | jq -r '.field // empty')
DESIRED=$(echo "$EVENT" | jq -r '.tension_desired')
# Find .werk directory relative to script
WERK_DIR=$(dirname "$(readlink -f "$0")")/../
cd "$WERK_DIR" 2>/dev/null || exit 0
git add .werk/sd.db 2>/dev/null
git commit -m "werk: $FIELD on '$DESIRED'" --no-verify 2>/dev/null
