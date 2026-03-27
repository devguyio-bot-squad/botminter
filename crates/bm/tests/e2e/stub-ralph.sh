#!/bin/bash
# Stub ralph binary for E2E testing.
# Simulates ralph's basic behavior without making any API calls.
#
# Supports SIGTERM ignore mode: if .ralph-stub-ignore-sigterm exists in $PWD,
# SIGTERM is trapped and logged to .ralph-stub-sigterm.log instead of exiting.

case "$1" in
  run)
    echo $$ > "$PWD/.ralph-stub-pid"
    if [ -n "$RALPH_TELEGRAM_API_URL" ] && [ -n "$RALPH_TELEGRAM_BOT_TOKEN" ]; then
      curl -s "${RALPH_TELEGRAM_API_URL}/bot${RALPH_TELEGRAM_BOT_TOKEN}/getUpdates" \
        > "$PWD/.ralph-stub-tg-response" 2>&1
    fi
    if [ -n "$RALPH_MATRIX_ACCESS_TOKEN" ] && [ -n "$RALPH_MATRIX_HOMESERVER_URL" ]; then
      curl -s "${RALPH_MATRIX_HOMESERVER_URL}/_matrix/client/versions" \
        > "$PWD/.ralph-stub-matrix-response" 2>&1
    fi
    env | grep -E '^(RALPH_|GH_TOKEN|GH_CONFIG_DIR)' | sort > "$PWD/.ralph-stub-env"
    if [ -f "$PWD/.ralph-stub-ignore-sigterm" ]; then
      echo "$(date -u +%FT%TZ) SIGTERM trap set to ignore" >> "$PWD/.ralph-stub-sigterm.log"
      trap 'echo "$(date -u +%FT%TZ) SIGTERM received and ignored" >> "$PWD/.ralph-stub-sigterm.log"' SIGTERM
    else
      trap "rm -f \"$PWD/.ralph-stub-pid\"; exit 0" SIGTERM SIGINT
    fi
    while true; do sleep 1; done
    ;;
  loops)
    if [ "$2" = "stop" ]; then
      pid_file="$PWD/.ralph-stub-pid"
      if [ -f "$pid_file" ]; then
        kill "$(cat "$pid_file")" 2>/dev/null
        rm -f "$pid_file"
      fi
      exit 0
    fi
    ;;
  *)
    exit 0
    ;;
esac
