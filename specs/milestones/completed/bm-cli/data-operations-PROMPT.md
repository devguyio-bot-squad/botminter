# Milestone Leap: Data Operations

## Objective

Extend `bm` so that teams can manage knowledge interactively, wake members only when there's work, and deploy members to targets beyond the local machine — validated with full E2E and integration tests.

## Prerequisites

- M3 complete (`bm` CLI: init, hire, sync, start, stop, status, profiles, teams, members, roles)
- Profiles exist with `botminter.yml`, `.schema/v1.yml`, and self-contained content
- E2E test harness established (TempRepo, TgMock, stub ralph)

## Scope

**In scope:** knowledge/invariant management via Claude skill bridge, event-driven daemon with webhook and polling modes, pluggable formations (local and k8s), formation-aware status, kind-based k8s testing.

**Deferred:** per-member daemon routing, cloud VM formations, `bm upgrade`, production k8s.

## Requirements

1. **Knowledge management** — `bm knowledge` MUST launch an interactive Claude Code session pre-loaded with a skill that understands the knowledge/invariant hierarchy (team → project → member → member+project), file formats, and scoping rules. The skill MUST be embedded in the profile under a new `skills/` directory. `bm knowledge list` and `bm knowledge show` MUST be deterministic (no Claude session). Changes produced by the skill session propagate via existing `bm teams sync`.

2. **Reusable skill session abstraction** — the mechanism that launches Claude Code sessions for knowledge management MUST be reusable for other domains. It MUST also support launching one-shot headless Ralph sessions (used by the formation manager). Future skill-based commands plug into the same abstraction.

3. **Profile schema v2** — profiles MUST gain `skills/` and `formations/` directories. Schema version MUST bump to v2. v1 teams attempting v2-dependent operations MUST receive a schema mismatch error.

4. **Event-driven daemon** — `bm daemon start/stop/status` MUST provide a long-running process that receives GitHub events and starts team members only when there's work. MUST support webhook mode (preferred, via tunnel) and polling mode (fallback). Members started by the daemon MUST run one-shot (`persistent: false`) — scan all available work, process it, exit. The daemon handles restart responsibility. This eliminates idle token burn.

5. **One-shot execution model** — `specs/design-principles.md` Section 2 MUST be updated to distinguish poll-based members (`persistent: true`, existing model) from event-triggered members (`persistent: false`, daemon model). The board scanner remains the universal entry point. In one-shot mode, it MUST process ALL matching work before exiting.

6. **Team formations** — `bm start` MUST accept a `--formation` flag. A formation defines WHERE members run. `bm` MUST NOT contain deployment logic for any formation type. Instead, a formation manager — a one-shot Ralph session — handles deployment and produces a topology file describing where members ended up. `bm` reads that topology file for all subsequent operations (status, stop). Endpoints in the topology file MUST be structured data, not shell commands.

7. **Local formation** — the default. Current behavior, plus a topology file is written so all formations share the same operational model.

8. **K8s formation** — a formation manager with hat decomposition (deploy, verify, write topology) MUST deploy members as pods to a lightweight, local Kubernetes cluster on the operator's machine (e.g., kind). MUST be idempotent — re-running detects healthy pods and skips redeployment. MUST reconcile stale resources. Secrets MUST be provisioned in the target environment by the formation manager. The local k8s deployment MUST be a first-class operational target, not just a test fixture.

9. **Formation-aware status** — `bm status` MUST work across formation types, performing live health checks via the topology file. When the target is unreachable, MUST fall back to cached topology with a warning. SHOULD include daemon status when running.

10. **Testing** — E2E and integration tests are the TOP PRIORITY. Every feature MUST have corresponding tests. k8s formation tests MUST use kind (Kubernetes in Docker). kind tests MUST be gated behind a feature flag. Webhook tests MUST use simulated payloads. Interactive Claude sessions MUST NOT be tested in CI — test the session abstraction, not the AI interaction.

## Acceptance Criteria

```
Given a team with schema v2 and knowledge files at multiple scopes
When the operator runs `bm knowledge list`
Then all knowledge and invariant files are shown grouped by scope, with no Claude session launched
```

```
Given a team with schema v1
When the operator runs `bm knowledge` or `bm start --formation k8s`
Then bm exits with a schema version mismatch error
```

```
Given no daemon running
When the operator runs `bm daemon start`, then `bm daemon status`, then `bm daemon stop`
Then the daemon lifecycle completes cleanly — started, reported running, stopped, cleaned up
```

```
Given a running daemon
When a GitHub event arrives (webhook or polled)
Then the daemon starts all members for the matching team one-shot
And members exit after processing all available work
And state is cleaned up after member exit
```

```
Given synced workspaces
When the operator runs `bm start --formation local`
Then a topology file is written and `bm stop` removes it after teardown
```

```
Given a reachable kind cluster
When the operator runs `bm start --formation k8s`
Then members are deployed as pods, verified healthy, and a topology file is written
And a second run detects healthy pods and skips redeployment
```

```
Given an active formation
When the operator runs `bm status`
Then output reflects the formation type with live health per member
```

```
Given the full test suite
When `cargo test -p bm` and `cargo test -p bm --features e2e` are run
Then all tests pass with no clippy warnings
```

```
Given a kind cluster
When `cargo test -p bm --features e2e,kind-tests` is run
Then k8s formation deploy, teardown, idempotency, and reconciliation tests pass
```
