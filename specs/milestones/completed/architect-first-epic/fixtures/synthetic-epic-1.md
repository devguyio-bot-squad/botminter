---
number: 1
title: "[SYNTHETIC] Add health check endpoint to HCP controller"
state: open
labels:
  - kind/epic
  - status/po:triage
assignee: null
milestone: null
parent: null
created: "2026-02-16T10:00:00Z"
---

## Description

Add a `/healthz` endpoint to the HCP controller that reports reconciler
health. This is a synthetic test epic for M2 validation.

## Context

See `projects/hypershift/knowledge/hcp-architecture.md` for the
reconciler pattern used by HCP.

## Scope

- Add HTTP health check handler to the HCP controller
- Report reconciler loop health (last successful reconciliation timestamp)
- Include readiness and liveness probe support
- Expose metrics for monitoring integration
