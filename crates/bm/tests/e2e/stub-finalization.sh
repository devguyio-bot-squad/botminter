#!/bin/bash
# Stub finalization subagent for E2E testing.
# Simulates the finalization process that runs after graceful session stop
# when dirty repos are detected.
#
# Behavior:
#   - Records its PID to .stub-finalization-pid
#   - Records all env vars to .stub-finalization-env
#   - Exits with BM_STUB_FINALIZATION_EXIT_CODE (default 0) after recording

echo $$ > "$PWD/.stub-finalization-pid"
env > "$PWD/.stub-finalization-env"

exit "${BM_STUB_FINALIZATION_EXIT_CODE:-0}"
