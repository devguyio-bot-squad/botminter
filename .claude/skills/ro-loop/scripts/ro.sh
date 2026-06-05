#!/usr/bin/env bash
set -euo pipefail

RALPH_FILE="${RALPH_FILE:-ralph.yml}"

_usage() {
  cat >&2 <<'EOF'
Usage: ro.sh <command> [args...]

Commands:
  hats                      List all hats (JSON array of {id, name, description})
  hat <id>                  Full hat config (JSON)
  hat <id> instructions     Hat instructions (plain text)
  hat <id> triggers         Hat trigger events (newline-separated)
  hat <id> publishes        Hat publishable events (newline-separated)
  resolve <event>           Find which hat handles an event (hat ID or exit 1)
  resolve-status <status>   Map project board status to hat (reads board-scanner dispatch table)
  chain <event>             Trace all reachable hats from an event (JSON tree)
  graph                     Full event routing adjacency list (JSON)
  deps <hat1> <hat2> ...    Check if hats can run in parallel
  config                    Non-hat config (JSON)

Options:
  --file <path>             Ralph config file (default: ralph.yml in CWD)
EOF
  exit 1
}

# Parse --file flag from anywhere in args
_args=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --file)
      [[ $# -lt 2 || -z "${2:-}" || "${2:-}" == --* ]] && {
        echo "Error: --file requires a path" >&2
        exit 1
      }
      RALPH_FILE="$2"
      shift 2
      ;;
    *) _args+=("$1"); shift ;;
  esac
done
set -- "${_args[@]+"${_args[@]}"}"

[[ $# -lt 1 ]] && _usage

if [[ ! -f "$RALPH_FILE" ]]; then
  echo "Error: $RALPH_FILE not found" >&2
  exit 1
fi

# Build event→hat trigger map (JSON object). Warns on ambiguous routing to stderr.
_build_trigger_map() {
  local raw
  raw=$(yq -o=json '.hats | to_entries[] | {"hat": .key, "triggers": (.value.triggers // [])} | .triggers[] as $t | {"event": $t, "hat": .hat}' "$RALPH_FILE" | jq -s '.')
  echo "$raw" | jq 'group_by(.event) | map(select(length > 1)) | .[] | "Warning: ambiguous trigger \(.[0].event) claimed by: \([.[].hat] | join(", "))"' -r >&2 2>/dev/null || true
  echo "$raw" | jq 'map({(.event): .hat}) | add // {}'
}

cmd_hats() {
  yq -o=json '.hats | to_entries | map({"id": .key, "name": .value.name, "description": .value.description})' "$RALPH_FILE"
}

cmd_hat() {
  local hat_id="$1"
  local field="${2:-}"

  local exists
  exists=$(yq ".hats | has(\"$hat_id\")" "$RALPH_FILE")
  if [[ "$exists" != "true" ]]; then
    echo "Error: hat '$hat_id' not found" >&2
    exit 1
  fi

  case "$field" in
    instructions)
      yq -r ".hats.\"$hat_id\".instructions // \"\"" "$RALPH_FILE"
      ;;
    triggers)
      yq -r ".hats.\"$hat_id\".triggers // [] | .[]" "$RALPH_FILE"
      ;;
    publishes)
      yq -r ".hats.\"$hat_id\".publishes // [] | .[]" "$RALPH_FILE"
      ;;
    "")
      yq -o=json ".hats.\"$hat_id\"" "$RALPH_FILE"
      ;;
    *)
      echo "Error: unknown field '$field' (use: instructions, triggers, publishes)" >&2
      exit 1
      ;;
  esac
}

cmd_resolve_status() {
  local status="$1"

  # Find dispatch-table.json from board-scanner skill (first match in skills.dirs order)
  local skills_dirs
  skills_dirs=$(yq -r '.skills.dirs[]' "$RALPH_FILE" 2>/dev/null)

  local dispatch_table=""
  while IFS= read -r dir; do
    local candidate="$dir/board-scanner/dispatch-table.json"
    if [[ -f "$candidate" ]]; then
      dispatch_table="$candidate"
      break
    fi
  done <<< "$skills_dirs"

  if [[ -z "$dispatch_table" ]]; then
    echo "Error: board-scanner/dispatch-table.json not found in skills.dirs" >&2
    exit 1
  fi

  local event
  event=$(jq -r --arg s "$status" '.[$s] // empty' "$dispatch_table")

  if [[ -z "$event" ]]; then
    echo "Error: no dispatch entry for status '$status'" >&2
    exit 1
  fi

  cmd_resolve "$event"
}

cmd_resolve() {
  local event="$1"
  local matches
  matches=$(yq -r ".hats | to_entries[] | select(.value.triggers // [] | map(. == \"$event\") | any) | .key" "$RALPH_FILE")

  if [[ -z "$matches" ]]; then
    echo "Error: no hat triggers on '$event'" >&2
    exit 1
  fi

  local count
  count=$(echo "$matches" | wc -l)
  if [[ "$count" -gt 1 ]]; then
    echo "Warning: ambiguous routing — $count hats trigger on '$event': $(echo "$matches" | tr '\n' ', ')" >&2
  fi

  echo "$matches" | head -1
}

cmd_chain() {
  local entry_event="$1"

  # Build trigger and publish maps as temp files for the recursive function
  local trigger_map publish_map
  trigger_map=$(mktemp)
  publish_map=$(mktemp)
  trap "rm -f '$trigger_map' '$publish_map'" EXIT

  _build_trigger_map > "$trigger_map"

  # publish_map: hat_id -> [events]
  yq -o=json '.hats | to_entries[] | {"hat": .key, "publishes": (.value.publishes // [])}' "$RALPH_FILE" \
    | jq -s 'map({(.hat): .publishes}) | add // {}' > "$publish_map"

  _chain_recurse "$entry_event" "$trigger_map" "$publish_map" ""
}

_chain_recurse() {
  local event="$1"
  local trigger_map="$2"
  local publish_map="$3"
  local visited="$4"

  local hat_id
  hat_id=$(jq -r --arg e "$event" '.[$e] // empty' "$trigger_map")

  if [[ -z "$hat_id" ]]; then
    jq -n --arg e "$event" '{"event": $e, "hat": null, "terminal": true}'
    return
  fi

  # Cycle detection
  if echo "$visited" | grep -qF "|$hat_id|"; then
    jq -n --arg e "$event" --arg h "$hat_id" '{"event": $e, "hat": $h, "cycle": true}'
    return
  fi

  local new_visited="${visited}|${hat_id}|"

  local publishes
  publishes=$(jq -r --arg h "$hat_id" '.[$h] // [] | .[]' "$publish_map")

  if [[ -z "$publishes" ]]; then
    jq -n --arg e "$event" --arg h "$hat_id" '{"event": $e, "hat": $h, "branches": []}'
    return
  fi

  local branches="[]"
  while IFS= read -r pub_event; do
    local child
    child=$(_chain_recurse "$pub_event" "$trigger_map" "$publish_map" "$new_visited")
    branches=$(echo "$branches" | jq --argjson c "$child" '. + [$c]')
  done <<< "$publishes"

  jq -n --arg e "$event" --arg h "$hat_id" --argjson b "$branches" \
    '{"event": $e, "hat": $h, "branches": $b}'
}

cmd_graph() {
  local trigger_map
  trigger_map=$(_build_trigger_map)

  yq -o=json '.hats | to_entries | map({
    "id": .key,
    "triggers": (.value.triggers // []),
    "publishes": (.value.publishes // [])
  })' "$RALPH_FILE" | jq --argjson tm "$trigger_map" '
    map(. + {
      "downstream": [.publishes[] as $p | $tm[$p] // empty] | unique
    }) | map({(.id): {triggers, publishes, downstream}}) | add // {}
  '
}

cmd_deps() {
  local hats=("$@")
  [[ ${#hats[@]} -lt 2 ]] && { echo "Error: need at least 2 hat IDs" >&2; exit 1; }

  local trigger_map
  trigger_map=$(_build_trigger_map)

  for src in "${hats[@]}"; do
    local exists
    exists=$(yq ".hats | has(\"$src\")" "$RALPH_FILE")
    if [[ "$exists" != "true" ]]; then
      echo "Error: hat '$src' not found" >&2
      exit 1
    fi

    local publishes
    publishes=$(yq -r ".hats.\"$src\".publishes // [] | .[]" "$RALPH_FILE" 2>/dev/null)
    [[ -z "$publishes" ]] && continue

    while IFS= read -r event; do
      local target
      target=$(echo "$trigger_map" | jq -r --arg e "$event" '.[$e] // empty')
      [[ -z "$target" ]] && continue

      for dst in "${hats[@]}"; do
        if [[ "$target" == "$dst" && "$src" != "$dst" ]]; then
          echo "sequential: $src publishes $event which triggers $dst"
          return 0
        fi
      done
    done <<< "$publishes"
  done

  echo "independent"
}

cmd_config() {
  yq -o=json '. | del(.hats)' "$RALPH_FILE"
}

case "${1:-}" in
  hats)    cmd_hats ;;
  hat)     [[ $# -lt 2 ]] && { echo "Error: hat requires an ID" >&2; exit 1; }; cmd_hat "$2" "${3:-}" ;;
  resolve) [[ $# -lt 2 ]] && { echo "Error: resolve requires an event" >&2; exit 1; }; cmd_resolve "$2" ;;
  resolve-status) [[ $# -lt 2 ]] && { echo "Error: resolve-status requires a status" >&2; exit 1; }; cmd_resolve_status "$2" ;;
  chain)   [[ $# -lt 2 ]] && { echo "Error: chain requires an event" >&2; exit 1; }; cmd_chain "$2" ;;
  graph)   cmd_graph ;;
  deps)    shift; cmd_deps "$@" ;;
  config)  cmd_config ;;
  *)       _usage ;;
esac
