#!/bin/bash
# Stub coding-agent binary for E2E and integration testing.
# Simulates a coding agent (e.g., claude) without making any API calls.
#
# Behavior:
#   - Records its PID to .stub-agent-pid
#   - Records all env vars to .stub-agent-env
#   - Traps SIGINT and logs to .stub-agent-sigint.log
#   - Exits with BM_STUB_AGENT_EXIT_CODE (default 0) after recording

echo $$ > "$PWD/.stub-agent-pid"
env > "$PWD/.stub-agent-env"

# Set up SIGINT trap to record signal receipt
trap 'echo "$(date -u +%FT%TZ) SIGINT received" >> "$PWD/.stub-agent-sigint.log"' INT

# Optional: stay alive until signaled if BM_STUB_AGENT_INTERACTIVE is set
if [ -n "$BM_STUB_AGENT_INTERACTIVE" ]; then
    while true; do sleep 0.1; done
fi

exit "${BM_STUB_AGENT_EXIT_CODE:-0}"
