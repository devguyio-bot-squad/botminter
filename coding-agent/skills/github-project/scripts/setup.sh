#!/bin/bash
# Common setup for all github-project skill operations
# Source this file at the start of each script

set -euo pipefail

# ── Persistent config helpers ────────────────────────────────────────────────
# Project metadata is resolved once and persisted. No TTLs — mutating
# operations (e.g., process-evolution) update the files when they change
# upstream state. Delete the directory to force a full re-resolve.

METADATA_DIR="$HOME/.botminter/cache/github-project"

_meta_get() {
  local file="$METADATA_DIR/$1"
  [ -f "$file" ] && cat "$file" && return 0
  return 1
}

_meta_set() {
  mkdir -p "$METADATA_DIR" 2>/dev/null
  echo "$2" > "$METADATA_DIR/$1" 2>/dev/null || true
}

# Board state cache — volatile, per-repo, written by board-view.sh
_board_cache_path() {
  echo "$METADATA_DIR/board-state.json"
}

# ── Detect team repo ─────────────────────────────────────────────────────────
# Uses git remote directly — no API call.

TEAM_REPO=$(_meta_get "team_repo" 2>/dev/null || true)
if [ -z "$TEAM_REPO" ]; then
  TEAM_REPO=$(cd team && git remote get-url origin 2>/dev/null | sed 's|.*github.com[:/]||;s|\.git$||')
  if [ -z "$TEAM_REPO" ]; then
    echo "❌ ERROR: Could not detect team repository from git remote"
    exit 1
  fi
  _meta_set "team_repo" "$TEAM_REPO"
fi

OWNER=$(echo "$TEAM_REPO" | cut -d/ -f1)

# ── Member identity ─────────────────────────────────────────────────────────

if [ -f .botminter.yml ]; then
  ROLE=$(grep '^role:' .botminter.yml | awk '{print $2}')
  EMOJI=$(grep '^comment_emoji:' .botminter.yml | sed 's/comment_emoji: *"//' | sed 's/"$//')
else
  ROLE="superman"
  EMOJI="🦸"
fi

# Minimal mode: only detect team repo, owner, and identity
# (for scripts that don't need project IDs or field data)
if [ "${SETUP_MODE:-}" = "minimal" ]; then
  export TEAM_REPO OWNER ROLE EMOJI METADATA_DIR
  export -f _meta_get _meta_set _board_cache_path
  return 0 2>/dev/null || exit 0
fi

# ── Scope check (persisted) ─────────────────────────────────────────────────
# Checked once and saved. Uses REST API (separate rate limit from GraphQL).
# Delete ~/.botminter/cache/github-project/scope_ok to force re-check.

if ! _meta_get "scope_ok" &>/dev/null; then
  TOKEN_SCOPES=$(gh api -i user 2>/dev/null | grep -i "x-oauth-scopes:" || true)
  if [ -n "$TOKEN_SCOPES" ] && ! echo "$TOKEN_SCOPES" | grep -qi "project"; then
    echo "❌ ERROR: Missing 'project' scope on GH_TOKEN"
    echo "Run: gh auth refresh -s project -h github.com"
    exit 1
  fi
  _meta_set "scope_ok" "1"
fi

# ── Project number (from config) ─────────────────────────────────────────────

BM_CONFIG="$HOME/.botminter/config.yml"
PROJECT_NUM=""
if [ -f "$BM_CONFIG" ]; then
  PROJECT_NUM=$(awk -v repo="$TEAM_REPO" '
    /^- name:/ || /^  - name:/ { in_team=1; found_repo=0; pn="" }
    in_team && /github_repo:/ && $0 ~ repo { found_repo=1 }
    in_team && /project_number:/ { pn=$2 }
    in_team && found_repo && pn { print pn; exit }
  ' "$BM_CONFIG" 2>/dev/null)
fi

if [ -z "$PROJECT_NUM" ]; then
  echo "❌ ERROR: No project_number found in $BM_CONFIG for team repo: $TEAM_REPO"
  echo "Ensure 'bm init' was run and project_number is set in config.yml"
  exit 1
fi

# ── Project ID (persisted) ───────────────────────────────────────────────────

PROJECT_ID=$(_meta_get "project_id" 2>/dev/null || true)
if [ -z "$PROJECT_ID" ] || [ "$PROJECT_ID" = "null" ]; then
  PROJECT_ID=$(gh project view "$PROJECT_NUM" --owner "$OWNER" --format json 2>&1 | jq -r '.id')
  if [ -z "$PROJECT_ID" ] || [ "$PROJECT_ID" = "null" ]; then
    echo "❌ ERROR: Could not get project ID for project #$PROJECT_NUM"
    exit 1
  fi
  _meta_set "project_id" "$PROJECT_ID"
fi

# ── Field data (persisted) ───────────────────────────────────────────────────
# Updated by process-evolution skill when status options change.

FIELD_DATA=$(_meta_get "field_data" 2>/dev/null || true)
if [ -z "$FIELD_DATA" ]; then
  if ! FIELD_DATA=$(gh project field-list "$PROJECT_NUM" --owner "$OWNER" --format json 2>&1); then
    echo "❌ ERROR: Could not fetch project field list"
    echo "$FIELD_DATA"
    exit 1
  fi
  if [ -z "$FIELD_DATA" ]; then
    echo "❌ ERROR: Empty response from project field list"
    exit 1
  fi
  _meta_set "field_data" "$FIELD_DATA"
fi

# Extract Status field ID with validation
STATUS_FIELD_ID=$(echo "$FIELD_DATA" | jq -r '.fields[] | select(.name=="Status") | .id')
if [ -z "$STATUS_FIELD_ID" ] || [ "$STATUS_FIELD_ID" = "null" ]; then
  echo "❌ ERROR: No 'Status' field found in project #$PROJECT_NUM"
  echo "Available fields:"
  echo "$FIELD_DATA" | jq -r '.fields[] | .name'
  exit 1
fi

# ── Repo ID (persisted — immutable) ──────────────────────────────────────────

REPO_ID=$(_meta_get "repo_id" 2>/dev/null || true)
# Exported for scripts that need it (create-issue, subtask-ops)
# Resolved lazily — only fetched when a script actually uses it.

# ── Issue type IDs (persisted — rarely changes) ──────────────────────────────

ISSUE_TYPES_JSON=$(_meta_get "issue_types" 2>/dev/null || true)
# Same as repo_id — exported for create-issue.sh and subtask-ops.sh

echo "✓ Setup complete: $TEAM_REPO, project #$PROJECT_NUM"

# Export variables for use in calling scripts
export TEAM_REPO OWNER PROJECT_NUM PROJECT_ID FIELD_DATA STATUS_FIELD_ID ROLE EMOJI
export METADATA_DIR REPO_ID ISSUE_TYPES_JSON
export -f _meta_get _meta_set _board_cache_path
