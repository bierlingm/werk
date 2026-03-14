#!/bin/bash
# Post-resolve hook: send Slack notification when a tension is resolved.
# Configure: werk config set hooks.post_resolve "~/.werk/hooks/slack-notify.sh"
EVENT=$(cat)
DESIRED=$(echo "$EVENT" | jq -r '.tension_desired')
TIMESTAMP=$(echo "$EVENT" | jq -r '.timestamp')
if [ -n "$SLACK_WEBHOOK" ]; then
  curl -s -X POST "$SLACK_WEBHOOK" \
    -H 'Content-Type: application/json' \
    -d "{\"text\": \"Resolved: $DESIRED\"}" > /dev/null
fi
