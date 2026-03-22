#!/bin/bash
# Pre-mutation hook: example validation.
# Exit 0 to allow, exit 1 to block (stderr shown to user).
# Configure: werk config set hooks.pre_mutation "~/.werk/hooks/pre-validate.sh"
EVENT=$(cat)
EVENT_TYPE=$(echo "$EVENT" | jq -r '.event')
# Example: block mutations on weekends
DAY=$(date +%u)
if [ "$DAY" -ge 6 ]; then
  echo "Mutations blocked on weekends. Take a break!" >&2
  exit 1
fi
exit 0
