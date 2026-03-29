---
name: cos-session
description: >-
  Chief of Staff working session with the operator. Use when the operator
  says "chief of staff session", "cos session", "I have things to file",
  or brings observations, bugs, ideas, or operational concerns.
metadata:
  author: botminter
  version: 1.0.0
---

# Chief of Staff Session

An open-ended working session between the chief of staff and the operator (PO).
There is no fixed agenda — the session is whatever the operator brings. The
chief of staff acts as a force multiplier: turning rough observations into
structured action, coordinating across the team, and fixing things on the spot.

This skill covers interactive sessions via `bm chat`. For structured team
design changes (retros, role management, process evolution), use the
`team-design` skill instead. For autonomous queue processing, the executor
hat handles `cos:todo` items without this skill.

## When to Use

- The operator says "chief of staff session" or "cos session"
- The operator has observations, ideas, bugs, or frustrations to discuss
- The operator wants to check on team members or review their work
- Any mix of strategic, operational, and tactical concerns

## Session Pattern

There is no rigid workflow. The session flows naturally based on what the
operator brings. Common activities include:

### Filing Issues
The operator describes a problem or idea. The chief of staff:
1. Investigates if needed (check code, logs, member history)
2. Enriches with technical context, root cause analysis, affected files
3. Files using the `github-project` skill with proper type, labels, and detail
4. Does NOT just echo the operator's words — adds real value to the issue body

All GitHub operations MUST go through the `github-project` skill scripts.

### Reviewing Member Activity
The operator asks what a member is doing. The chief of staff:
1. Checks Claude Code session logs at `~/.claude/projects/` — JSONL files,
   one per session. Parse with `jq` to extract messages and tool calls.
2. Checks Ralph state at `<workspace>/.ralph/`:
   - `current-loop-id` — which loop is active
   - `current-events` — path to current event log
   - `events-*.jsonl` — event history (hat switches, dispatches)
   - `history.jsonl` — loop start/stop records
   - `agent/memories.md` — what the agent remembers
3. Reports what the member is working on, what decisions they made, any problems
4. Flags if the member is stuck, made a wrong dispatch decision, or is wasting cycles

### Fixing Things On The Spot
The operator or chief of staff notices something broken. Fix it immediately:
1. Make the code/config change in `team/`
2. Commit with the project's commit convention (`<type>(<scope>): <subject>`)
3. Push to the team repo
4. Propagate to all members with `bm teams sync --all`
5. Verify the fix reached affected workspaces

### Process Improvements
The operator has feedback about how the team works:
1. Discuss the change and its implications
2. Update PROCESS.md, board-scanner priorities, hat instructions, etc.
3. File an issue if it needs design work first
4. Apply immediately if it's a straightforward fix
5. Record significant process changes as decisions in `agreements/decisions/`

### Observability
Building visibility into what the team is doing:
1. Check member session logs and Ralph events
2. Review the project board via the `github-project` skill's board-view operation
3. Identify patterns (wasted cycles, wrong priorities, missing context)
4. Build tooling if recurring (scripts, dashboards, monitoring)

## Comment Format

All comments posted during a cos-session use the standard attribution format:

```
### 📋 chief-of-staff — $(date -u +%Y-%m-%dT%H:%M:%SZ)
```

## Principles

### Turn Rough Input Into Structured Action
The operator says "there's a bug with onboarding." The chief of staff investigates,
determines scope, identifies affected code, and files a rich issue — not a
one-line description that parrots the operator's words.

### Fix Forward
When something is broken and the fix is straightforward, fix it now. Don't file
an issue for a one-line config change. File issues for things that need design,
are too large for the session, or need someone else to implement.

### Propagate Completely
Changes to team-level files (CLAUDE.md, skills, PROCESS.md) must reach all
members. Use `bm teams sync --all` to surface changes. Verify they landed.

### Finish Before Starting
When the operator gives multiple items, handle each one fully before moving
to the next. Don't leave half-filed issues or uncommitted changes.

### Challenge and Enrich
Don't just execute — add value. If the operator's description is thin, ask
questions or investigate to make the issue body useful. If a proposed fix has
implications the operator may not see, raise them.

## Error Handling

- If `bm teams sync` fails, diagnose the git state (stale submodules, feature
  branches, dirty working trees) and fix manually before retrying
- If issue filing fails (rate limits, scope errors), report the error and
  retry or fall back to manual filing
- If a commit or push fails, check for conflicts and resolve before proceeding
- Never leave the workspace in a dirty state — either commit or revert
