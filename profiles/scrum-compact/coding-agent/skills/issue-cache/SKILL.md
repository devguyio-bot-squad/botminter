---
name: issue-cache
description: >-
  Reads pre-fetched issue data cached by the board scanner.
  Use instead of querying GitHub for issue reads.
metadata:
  author: botminter
  version: 1.0.0
---

# Issue Cache

The board scanner pre-fetches full issue data for all actionable issues
and caches each one as a Ralph Task before dispatching to hats.

## Reading Cached Data

1. Check the event payload for a task reference: `task: <task-id>`.
2. Read the task and parse its `description` field as JSON.
3. The JSON contains the full issue data:

| Field | Type | Description |
|-------|------|-------------|
| `number` | int | Issue number |
| `title` | string | Issue title |
| `state` | string | `OPEN` or `CLOSED` |
| `body` | string | Issue description (markdown) |
| `issueType` | object | `{ "name": "Epic" \| "Task" \| "Bug" }` |
| `labels` | object | `{ "nodes": [{ "name": "..." }] }` |
| `assignees` | object | `{ "nodes": [{ "login": "..." }] }` |
| `milestone` | object/null | `{ "title": "..." }` or null |
| `subIssues` | object | `{ "nodes": [{ "number", "title", "state", "issueType" }] }` (up to 50) |
| `comments` | object | `{ "nodes": [{ "author": { "login" }, "body", "createdAt" }] }` (last 20) |

## When to Query GitHub Instead

Use the `github-project` skill for:
- **Write operations**: posting comments, changing status, creating PRs
- **Missing cache**: no task reference in event payload (e.g., direct-chain events between hats)
- **Fresh sub-issue status**: monitors that need current completion state should
  query `subtask-ops.sh --action status` directly

Do NOT re-query GitHub for data already available in the cache.
