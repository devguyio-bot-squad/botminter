# Data Operations Milestone — Specification

> **Summary:** Extend the `bm` CLI with knowledge management, event-driven daemon, pluggable
> formations (local and k8s), and formation-aware status — all validated with comprehensive
> E2E and integration tests.

---

## Table of Contents

1. [Profile Schema v2](#1-profile-schema-v2)
2. [Knowledge Management](#2-knowledge-management)
3. [Reusable Skill Session Abstraction](#3-reusable-skill-session-abstraction)
4. [Event-Driven Daemon](#4-event-driven-daemon)
5. [One-Shot Execution Model](#5-one-shot-execution-model)
6. [Team Formations](#6-team-formations)
7. [Local Formation](#7-local-formation)
8. [K8s Formation](#8-k8s-formation)
9. [Formation-Aware Status](#9-formation-aware-status)
10. [Testing](#10-testing)
11. [Data Structures](#11-data-structures)
12. [CLI Changes](#12-cli-changes)
13. [Acceptance Criteria](#13-acceptance-criteria)
14. [Edge Cases and Error Conditions](#14-edge-cases-and-error-conditions)
15. [Non-Functional Requirements](#15-non-functional-requirements)
16. [Out of Scope](#16-out-of-scope)
17. [Implementation Ordering](#17-implementation-ordering)

---

## 1. Profile Schema v2

### Summary

Bump the profile schema from `v1` to `v2`, adding `skills/` and `formations/` directories to the
profile layout. Existing v1 teams MUST receive a clear error when attempting v2-dependent operations.

### Current State

- Schema version lives in `botminter.yml` as `schema_version: v1`.
- Schema layout defined in `.schema/v1.yml` — maps directory names for `knowledge`, `invariants`,
  `projects`, `members` at both team and member levels.
- `profile::check_schema_version()` in `crates/bm/src/profile.rs:200` compares the embedded
  profile's schema version against the team repo's `botminter.yml`. Mismatch → error suggesting
  `bm upgrade`.
- Both `rh-scrum` and `compact` profiles currently use `schema_version: v1`.

### Changes Required

#### 1.1. New schema file: `.schema/v2.yml`

Each profile MUST gain a `.schema/v2.yml` file alongside the existing `.schema/v1.yml`:

```yaml
name: v2
team:
  knowledge: "knowledge"
  invariants: "invariants"
  projects: "projects"
  members: "team"
  skills: "skills"
  formations: "formations"
member:
  knowledge: "knowledge"
  invariants: "invariants"
  projects: "projects"
  skills: "skills"
```

#### 1.2. `botminter.yml` version bump

Both profiles (`rh-scrum`, `compact`) MUST update `schema_version` from `v1` to `v2`.

#### 1.3. Profile directory additions

Each profile MUST gain these directories:

```
profiles/<profile>/
  skills/                        # NEW — profile-level skills (e.g., knowledge-manager)
    knowledge-manager/
      SKILL.md                   # Claude Code skill for knowledge management
  formations/                    # NEW — formation definitions
    local/
      formation.yml              # Local formation config (default)
    k8s/
      formation.yml              # K8s formation config
      ralph.yml                  # Formation manager Ralph config
      PROMPT.md                  # Formation manager prompt
      hats/                      # Hat instructions for the formation manager
        deployer/
        verifier/
        topology-writer/
```

#### 1.4. `extract_profile_to` update

`profile::extract_profile_to()` currently skips `members/` and `.schema/`. The skip predicate
MUST remain unchanged — `skills/` and `formations/` MUST be extracted to the team repo like
`knowledge/` and `invariants/`.

#### 1.5. Schema version gate for v2 features

Commands that require v2 features MUST check the team's schema version and produce a clear error
when the team uses v1:

- `bm knowledge` → requires v2
- `bm knowledge list` → requires v2
- `bm knowledge show` → requires v2
- `bm start --formation <name>` (any non-default formation) → requires v2
- `bm daemon start` → requires v2

The error message MUST be:

```
Error: This feature requires schema v2, but team '<team_name>' uses schema v1.
Run `bm upgrade` to migrate the team, or re-init with a v2 profile.
```

#### 1.6. `ProfileManifest` struct update

The `ProfileManifest` struct in `profile.rs` MUST NOT change for v2 — the `skills` and
`formations` directories are part of the schema layout definition (`.schema/v2.yml`), not the
manifest. The manifest already carries `schema_version: String` which is sufficient.

### Acceptance Criteria

```
Given profiles with schema_version v2 and .schema/v2.yml
When `bm profiles describe <profile>` is run
Then the output shows schema_version: v2
```

```
Given a team initialized from a v2 profile
When `bm teams sync` is run
Then the team repo has skills/ and formations/ directories
```

```
Given a team with schema_version v1 in botminter.yml
When `bm knowledge list` is run
Then bm exits with error mentioning schema v2 requirement
```

```
Given a team with schema_version v1
When `bm start --formation k8s` is run
Then bm exits with error mentioning schema v2 requirement
```

---

## 2. Knowledge Management

### Summary

Add `bm knowledge` commands for managing the knowledge/invariant hierarchy. `bm knowledge list`
and `bm knowledge show` are deterministic (no Claude session). `bm knowledge` (bare) launches an
interactive Claude Code session with a profile-embedded skill.

### Current State

- Knowledge files live at four scopes (per `specs/design-principles.md` Section 7):
  1. Team-level: `knowledge/`, `invariants/`
  2. Project-level: `projects/<project>/knowledge/`, `projects/<project>/invariants/`
  3. Member-level: `team/<member>/knowledge/`, `team/<member>/invariants/`
  4. Member+project: `team/<member>/projects/<project>/knowledge/`
- Files are plain Markdown.
- Changes propagate via `bm teams sync` (pulls `.botminter/`, re-surfaces files).
- No interactive management exists — operators edit files manually.

### Commands

#### 2.1. `bm knowledge list [-t team] [--scope <scope>]`

**Deterministic.** No Claude session.

Lists all knowledge and invariant files in the team repo, grouped by scope. The `--scope` flag
filters to a specific scope: `team`, `project`, `member`, or `member-project`.

**Output format:**

```
Team: my-team (schema v2)

Team scope:
  knowledge/
    commit-convention.md
    pr-standards.md
  invariants/
    code-review-required.md
    test-coverage.md

Project scope (my-project):
  projects/my-project/knowledge/
    api-conventions.md
  projects/my-project/invariants/
    (none)

Member scope (architect-alice):
  team/architect-alice/knowledge/
    design-patterns.md
  team/architect-alice/invariants/
    design-quality.md

Member+Project scope (architect-alice/my-project):
  team/architect-alice/projects/my-project/knowledge/
    (none)
```

**Implementation:**
- Read team repo path from config (`team.path.join("team")`).
- Walk the directory tree following the schema v2 layout paths.
- Group files by scope, list `.md` files under each scope directory.
- Show `(none)` for empty scopes.
- The `--scope` flag filters output to a single scope level.

#### 2.2. `bm knowledge show <path> [-t team]`

**Deterministic.** No Claude session.

Displays the contents of a knowledge or invariant file. The `<path>` is relative to the team
repo root (e.g., `knowledge/commit-convention.md` or
`team/architect-alice/invariants/design-quality.md`).

**Behavior:**
- Resolve the path relative to the team repo.
- MUST validate the file exists and is within a recognized knowledge or invariant directory.
- MUST reject paths outside the team repo or to non-knowledge files.
- Print the file contents to stdout.

**Error cases:**
- File not found → `Error: File not found: knowledge/nonexistent.md`
- Path outside knowledge/invariant dirs → `Error: Path is not within a knowledge or invariant directory`

#### 2.3. `bm knowledge [-t team] [--scope <scope>]`

**Interactive.** Launches a Claude Code session.

Launches an interactive Claude Code session pre-loaded with the `knowledge-manager` skill from
the team's profile. The skill MUST be embedded in the profile under `skills/knowledge-manager/SKILL.md`.

**Behavior:**
1. Resolve team and verify schema v2.
2. Locate the skill file at `{team_repo}/skills/knowledge-manager/SKILL.md`.
3. Launch `claude --skill <skill_path>` (or equivalent Claude Code invocation) in the team repo
   directory.
4. The skill session operates on the team repo directly.
5. Changes are local to the team repo — they propagate to workspaces via `bm teams sync`.

**The skill MUST understand:**
- The four-level knowledge/invariant hierarchy.
- What file formats are allowed (Markdown).
- How to create, edit, move, and delete knowledge/invariant files.
- Which scope a file should live in based on its purpose.
- The naming convention for knowledge/invariant files.

**The `--scope` flag** pre-filters the skill's context to a specific scope level, helping it
focus on the right directory.

### Knowledge Manager Skill

The skill MUST be embedded in each profile at `skills/knowledge-manager/SKILL.md`. The skill
content MUST include:

1. **Hierarchy documentation** — explains the four scopes and when to use each.
2. **File format rules** — Markdown only, naming conventions, frontmatter (if any).
3. **Operations** — create, edit, move, delete, list.
4. **Scoping guidance** — decision tree for which scope a piece of knowledge belongs to.
5. **Invariant rules** — what makes a good invariant (verifiable, actionable, scoped).

The skill MUST NOT contain any operational code — it is a Claude Code prompt that gives Claude
the context to manage knowledge files correctly.

---

## 3. Reusable Skill Session Abstraction

### Summary

The mechanism that launches Claude Code sessions for knowledge management MUST be reusable for
other skill-based commands. It MUST also support launching one-shot headless Ralph sessions.

### Design

#### 3.1. Skill session module: `crates/bm/src/session.rs`

A new module MUST provide two session types:

```rust
/// Launch an interactive Claude Code session with a skill.
pub fn interactive_claude_session(
    working_dir: &Path,
    skill_path: &Path,
    env_vars: &[(String, String)],
) -> Result<()>;

/// Launch a one-shot headless Ralph session.
/// Returns when the Ralph session completes.
pub fn oneshot_ralph_session(
    working_dir: &Path,
    prompt_path: &Path,
    ralph_yml_path: &Path,
    env_vars: &[(String, String)],
) -> Result<ExitStatus>;
```

#### 3.2. Interactive Claude session

**Implementation:**
1. Verify `claude` binary exists in PATH.
2. Spawn `claude` with the skill flag, inheriting stdin/stdout/stderr (interactive).
3. Set the working directory.
4. Pass environment variables (e.g., `GH_TOKEN`).
5. Wait for the process to exit.
6. Return success/failure.

**Claude CLI invocation:**
```bash
claude --skill <skill_path>
```

If the Claude CLI doesn't support `--skill` directly, the skill content MUST be injected via
the appropriate mechanism (e.g., `--prompt` with the skill content prepended, or by placing the
skill in the Claude config directory).

#### 3.3. One-shot Ralph session

**Implementation:**
1. Verify `ralph` binary exists in PATH.
2. Spawn `ralph run -p <prompt_path>` with `persistent: false` in the ralph.yml.
3. Set the working directory.
4. Pass environment variables.
5. Unset `CLAUDECODE` (same as current `launch_ralph` in `start.rs`).
6. Wait for the process to exit and return exit status.
7. Do NOT detach from the terminal — the caller blocks until completion.

**Difference from `start.rs::launch_ralph`:**
- `launch_ralph` detaches (stdin/stdout/stderr null, returns PID).
- `oneshot_ralph_session` blocks until completion (stdin null, stdout/stderr inherited or captured).
- `launch_ralph` is for persistent members.
- `oneshot_ralph_session` is for formation managers and other one-shot operations.

#### 3.4. Future extensibility

The session abstraction MUST be generic enough for future skill-based commands to use. Examples:
- `bm review` could launch a Claude session with a code-review skill.
- `bm plan` could launch a Claude session with a planning skill.
- Formation managers use `oneshot_ralph_session` for deployment.

---

## 4. Event-Driven Daemon

### Summary

`bm daemon start/stop/status` provides a long-running process that receives GitHub events and
starts team members one-shot when there's work. Eliminates idle token burn.

### Commands

#### 4.1. `bm daemon start [-t team] [--mode <webhook|poll>] [--port <port>] [--interval <secs>]`

Starts a background daemon process for the specified team.

**Flags:**
- `--mode webhook` (default) — listens for GitHub webhook events via HTTP.
- `--mode poll` — polls `gh api` at a configurable interval.
- `--port <port>` — HTTP listener port for webhook mode (default: 8484).
- `--interval <secs>` — polling interval for poll mode (default: 60).

**Behavior:**
1. Verify schema v2.
2. Verify no daemon already running for this team (check PID file).
3. Fork a daemon process.
4. Write PID file at `~/.botminter/daemon-<team_name>.pid`.
5. Write daemon config at `~/.botminter/daemon-<team_name>.json`.
6. The daemon process runs until stopped.

**Daemon config file (`daemon-<team_name>.json`):**
```json
{
  "team": "my-team",
  "mode": "webhook",
  "port": 8484,
  "pid": 12345,
  "started_at": "2026-02-21T10:00:00Z"
}
```

#### 4.2. `bm daemon stop [-t team]`

Stops the running daemon for the specified team.

**Behavior:**
1. Read PID from `~/.botminter/daemon-<team_name>.pid`.
2. Send SIGTERM.
3. Wait up to 30 seconds for exit.
4. If still alive, send SIGKILL.
5. Remove PID file and daemon config.

#### 4.3. `bm daemon status [-t team]`

Shows daemon status for the specified team.

**Output when running:**
```
Daemon: running (PID 12345)
Mode: webhook (port 8484)
Team: my-team
Started: 2026-02-21 10:00:00 UTC
```

**Output when not running:**
```
Daemon: not running
```

### Daemon Process Architecture

#### 4.4. Webhook mode

1. Start an HTTP server on the configured port.
2. Accept POST requests at `/webhook`.
3. Validate the GitHub webhook payload signature (using a webhook secret from credentials).
4. Parse the event type from the `X-GitHub-Event` header.
5. For relevant events (issues, issue_comment, pull_request):
   - Start all team members one-shot.
   - Wait for all members to exit.
   - Clean up state.

**Relevant GitHub events:**
- `issues` — issue opened, labeled, unlabeled, assigned
- `issue_comment` — comment created
- `pull_request` — PR opened, review requested, review submitted

#### 4.5. Poll mode

1. Run a polling loop at the configured interval.
2. Each iteration:
   - Call `gh api repos/<owner>/<repo>/events --paginate` (or similar).
   - Track last-seen event ID to avoid reprocessing.
   - If new relevant events are found, start all members one-shot.
   - Wait for all members to exit.
   - Clean up state.

**State file for poll tracking (`~/.botminter/daemon-<team_name>-poll.json`):**
```json
{
  "last_event_id": "12345678",
  "last_poll_at": "2026-02-21T10:00:00Z"
}
```

#### 4.6. One-shot member execution

When the daemon detects relevant events, it MUST:

1. Discover all members (same logic as `start.rs::list_member_dirs`).
2. For each member, find the workspace (same logic as `start.rs::find_workspace`).
3. Launch each member via `ralph run -p PROMPT.md` with `persistent: false` implied
   by the daemon (the ralph.yml in the workspace controls persistence; the daemon expects
   the member to process all available work and exit).
4. Track PIDs in the runtime state.
5. Wait for all members to exit (poll PIDs).
6. Remove member entries from runtime state.
7. Log the run result.

**Key constraint:** Members started by the daemon MUST run one-shot — they scan all available
work, process it, and exit. The daemon handles restart responsibility on the next event.

#### 4.7. Daemon logging

The daemon MUST log to `~/.botminter/logs/daemon-<team_name>.log`:
- Startup/shutdown events
- Event receipt (type, timestamp, relevant details)
- Member launch/exit events
- Errors

Logs MUST rotate or truncate at 10MB.

### Credential Requirements

- Webhook mode: `GH_TOKEN` required (passed to members). `webhook_secret` optional in credentials
  (for payload verification).
- Poll mode: `GH_TOKEN` required (for API calls and member launch).

### Daemon State Machine

```
                         ┌─────────┐
                         │  Idle   │
                         └────┬────┘
                              │ event received
                              ▼
                         ┌─────────┐
                         │Launching│
                         │ members │
                         └────┬────┘
                              │ all launched
                              ▼
                         ┌─────────┐
                         │ Waiting │
                         │for exit │
                         └────┬────┘
                              │ all exited
                              ▼
                         ┌─────────┐
                         │ Cleanup │
                         └────┬────┘
                              │
                              ▼
                         ┌─────────┐
                         │  Idle   │
                         └─────────┘
```

If new events arrive while members are running, the daemon MUST NOT launch duplicate members.
It MUST queue the event and start members again after the current run completes.

---

## 5. One-Shot Execution Model

### Summary

Update `specs/design-principles.md` Section 2 to distinguish poll-based members (`persistent: true`)
from event-triggered members (`persistent: false`).

### Changes to design-principles.md

Add a new subsection after the current Section 2 content:

#### 5.1. Two execution models

| Model | `persistent` | Entry point | Exit condition | Restart responsibility |
|-------|-------------|-------------|---------------|----------------------|
| **Poll-based** | `true` | Board scanner on `board.scan` | Never (LOOP_COMPLETE → `task.resume` → `board.scan`) | Self (Ralph runtime) |
| **Event-triggered** | `false` | Board scanner on `board.scan` | LOOP_COMPLETE after no work found | External (daemon) |

#### 5.2. Behavioral differences

- **Poll-based (existing):** Board scanner runs in a loop. On LOOP_COMPLETE, Ralph injects
  `task.resume` which routes back to `board.scan`. The member stays alive indefinitely.
  `cooldown_delay_seconds` controls the scan interval.

- **Event-triggered (new):** Board scanner runs once (or until no work remains). It MUST scan
  ALL matching issues — not just the first one. When no work is found, it publishes
  LOOP_COMPLETE and Ralph exits. The daemon handles the next restart.

#### 5.3. Ralph.yml configuration

The execution model is controlled by the `event_loop.persistent` field in `ralph.yml`:

```yaml
event_loop:
  persistent: false              # event-triggered mode
  max_iterations: 200            # safety limit per invocation
```

The board scanner MUST process ALL matching work items in priority order before publishing
LOOP_COMPLETE. This is a behavioral change from the poll-based model where the scanner processes
one item per cycle.

#### 5.4. Board scanner changes for one-shot mode

The board scanner hat instructions for event-triggered members MUST include:

1. Self-clear (overwrite scratchpad, delete tasks.jsonl).
2. Sync (`just -f .botminter/Justfile sync`).
3. Scan ALL issues with matching status labels.
4. Process them in priority order, dispatching one at a time (each hat cycle handles one issue).
5. After each work hat completes, re-scan for remaining work.
6. When no matching issues remain, publish LOOP_COMPLETE.

The key invariant: **the board scanner is always the universal entry point, regardless of
execution model.**

---

## 6. Team Formations

### Summary

`bm start` MUST accept a `--formation` flag. A formation defines WHERE members run. `bm` MUST
NOT contain deployment logic — a formation manager (one-shot Ralph session) handles deployment
and produces a topology file.

### Design

#### 6.1. Formation flag

```
bm start [-t team] [--formation <name>]
```

- Default formation: `local` (current behavior + topology file).
- Available formations are defined in the team repo's `formations/` directory.
- Each formation has a `formation.yml` config file.

#### 6.2. Formation config: `formation.yml`

```yaml
name: local
description: "Run members as local processes"
type: local
```

```yaml
name: k8s
description: "Deploy members as pods to a local Kubernetes cluster"
type: k8s
manager:
  ralph_yml: ralph.yml
  prompt: PROMPT.md
  hats_dir: hats/
```

The `type` field determines how `bm` handles the formation:
- `local` — `bm` launches members directly (current behavior), writes a topology file.
- Any other type — `bm` delegates to a formation manager (one-shot Ralph session).

#### 6.3. Formation manager

For non-local formations, `bm start --formation <name>`:

1. Locate formation config at `{team_repo}/formations/<name>/formation.yml`.
2. Verify the formation manager files exist (ralph.yml, PROMPT.md).
3. Prepare a working directory for the formation manager (temp dir or `{team_ws}/.formations/<name>/`).
4. Launch the formation manager via `session::oneshot_ralph_session()`.
5. The formation manager:
   - Reads the team repo to discover members.
   - Deploys members to the target environment.
   - Verifies deployments are healthy.
   - Writes a topology file at `{team_ws}/topology.json`.
6. `bm` reads the topology file to confirm success.

#### 6.4. Topology file: `topology.json`

The topology file is the single source of truth for where members are running. It lives at the
team workspace root: `{workzone}/{team_name}/topology.json`.

**Schema:**

```json
{
  "formation": "local",
  "created_at": "2026-02-21T10:00:00Z",
  "members": {
    "architect-alice": {
      "status": "running",
      "endpoint": {
        "type": "local",
        "pid": 12345,
        "workspace": "/home/user/workzone/my-team/architect-alice/my-project"
      }
    },
    "dev-bob": {
      "status": "running",
      "endpoint": {
        "type": "k8s",
        "namespace": "botminter-my-team",
        "pod": "dev-bob-7d8f9",
        "container": "ralph",
        "context": "kind-botminter"
      }
    }
  }
}
```

**Endpoint types (structured data, NOT shell commands):**

| Type | Fields | Purpose |
|------|--------|---------|
| `local` | `pid`, `workspace` | Local process |
| `k8s` | `namespace`, `pod`, `container`, `context` | Kubernetes pod |

#### 6.5. Topology-based operations

After formation adoption, `bm stop` and `bm status` MUST read the topology file to determine
how to interact with members:

- **Local endpoint:** Use PID-based operations (current behavior).
- **K8s endpoint:** Use `kubectl` commands via the endpoint's structured data.

`bm stop` MUST remove the topology file after all members are torn down.

#### 6.6. Formation resolution order

1. If `--formation` is specified, use that.
2. If no `--formation`, check team repo for `formations/` directory.
3. If `formations/local/formation.yml` exists, default to `local`.
4. If no formations directory exists (v1 team), use legacy behavior (no topology file).

---

## 7. Local Formation

### Summary

The default formation. Current behavior plus a topology file for operational consistency.

### Changes from Current Behavior

The only change to the current `start.rs` is **writing a topology file** after all members
are launched. Everything else remains the same.

#### 7.1. Start flow (local)

1. All current `start.rs` logic remains unchanged (schema check, ralph in PATH, credentials,
   member discovery, workspace finding, PID tracking).
2. After all members are launched, write `{workzone}/{team_name}/topology.json`:
   ```json
   {
     "formation": "local",
     "created_at": "2026-02-21T10:00:00Z",
     "members": {
       "architect-alice": {
         "status": "running",
         "endpoint": {
           "type": "local",
           "pid": 12345,
           "workspace": "/path/to/workspace"
         }
       }
     }
   }
   ```

#### 7.2. Stop flow (local)

1. If topology file exists, read it.
2. Stop members using PID from topology (or fall back to runtime state).
3. Remove topology file after all members stopped.

#### 7.3. Backward compatibility

If no topology file exists (e.g., v1 team, or pre-formation start), `bm stop` and `bm status`
MUST fall back to the runtime state (`~/.botminter/state.json`) — the current behavior.

---

## 8. K8s Formation

### Summary

A formation manager with hat decomposition deploys members as pods to a local Kubernetes
cluster (kind). Idempotent, reconciles stale resources, provisions secrets.

### Formation Manager Design

The k8s formation manager is a one-shot Ralph session with three hats:

#### 8.1. Hat: Deployer

**Trigger:** `k8s.deploy`

**Responsibilities:**
1. Read team repo to discover members and their configurations.
2. Create Kubernetes namespace (`botminter-<team_name>`) if it doesn't exist.
3. For each member:
   - Build a pod spec from the member's workspace configuration.
   - Create a ConfigMap with the member's PROMPT.md, CLAUDE.md, ralph.yml.
   - Create a Secret with GH_TOKEN and optional TELEGRAM_BOT_TOKEN.
   - Apply the pod spec (create or update).
4. Reconcile: delete pods for members that no longer exist in the team repo.
5. Publish `k8s.verify`.

**Idempotency:** If a pod already exists and is healthy, skip redeployment.

**Pod spec template:**
```yaml
apiVersion: v1
kind: Pod
metadata:
  name: <member_dir_name>
  namespace: botminter-<team_name>
  labels:
    app: botminter
    team: <team_name>
    member: <member_dir_name>
spec:
  containers:
    - name: ralph
      image: <ralph-image>    # configurable in formation.yml
      command: ["ralph", "run", "-p", "PROMPT.md"]
      env:
        - name: GH_TOKEN
          valueFrom:
            secretKeyRef:
              name: botminter-credentials
              key: gh-token
      volumeMounts:
        - name: member-config
          mountPath: /workspace
  volumes:
    - name: member-config
      configMap:
        name: <member_dir_name>-config
  restartPolicy: Never          # one-shot mode
```

#### 8.2. Hat: Verifier

**Trigger:** `k8s.verify`

**Responsibilities:**
1. For each expected pod, check status via `kubectl get pod`.
2. Wait up to 120 seconds for all pods to reach `Running` state.
3. If all healthy, publish `k8s.topology`.
4. If any pod fails, report error and publish `LOOP_COMPLETE`.

#### 8.3. Hat: Topology Writer

**Trigger:** `k8s.topology`

**Responsibilities:**
1. Build the topology file with k8s endpoints.
2. Write `topology.json` to the team workspace root.
3. Publish `LOOP_COMPLETE`.

### Kind Cluster Requirements

- The formation manager MUST NOT create the kind cluster — the operator MUST create it beforehand.
- The formation manager MUST verify the cluster is reachable via `kubectl cluster-info`.
- The `kubectl` context MUST be configurable in `formation.yml`:
  ```yaml
  k8s:
    context: kind-botminter     # kubectl context name
    image: ghcr.io/owner/ralph:latest
  ```

### Secret Provisioning

The formation manager MUST create a Kubernetes Secret in the target namespace:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: botminter-credentials
  namespace: botminter-<team_name>
type: Opaque
data:
  gh-token: <base64-encoded>
  telegram-bot-token: <base64-encoded>   # optional
```

Credentials are read from the team's config (`~/.botminter/config.yml`).

### Reconciliation

On each run, the deployer MUST:
1. List existing pods in the namespace with label `app=botminter,team=<team_name>`.
2. Compare with the current member list.
3. Delete pods for removed members.
4. Skip deployment for healthy existing pods.
5. Redeploy pods that are in error state.

---

## 9. Formation-Aware Status

### Summary

`bm status` MUST work across formation types, performing live health checks via the topology
file.

### Changes to `status.rs`

#### 9.1. Topology-based status

When a topology file exists at `{workzone}/{team_name}/topology.json`:

1. Read the topology file.
2. For each member, perform a live health check based on endpoint type:
   - **Local:** Check PID is alive (current behavior via `state::is_alive`).
   - **K8s:** Run `kubectl get pod <pod> -n <namespace> --context <context>` and parse status.
3. Display formation type in header.

**Output format:**
```
Team: my-team (schema v2)
Formation: k8s
Profile: rh-scrum

╭─────────────────┬───────────┬─────────┬──────────────────────┬─────────────────╮
│ Member          │ Role      │ Status  │ Started              │ Endpoint        │
├─────────────────┼───────────┼─────────┼──────────────────────┼─────────────────┤
│ architect-alice │ architect │ running │ 2026-02-21 10:00:00  │ pod/arch-a-7d8  │
│ dev-bob         │ dev       │ running │ 2026-02-21 10:00:02  │ pod/dev-b-3f1   │
╰─────────────────┴───────────┴─────────┴──────────────────────┴─────────────────╯
```

#### 9.2. Unreachable target fallback

When the health check fails (e.g., kubectl times out, kind cluster unreachable):

1. Fall back to cached topology data.
2. Show status as `unknown` with a warning.
3. Print a warning: `Warning: Could not reach <formation> target. Showing cached topology.`

#### 9.3. Daemon status integration

When a daemon is running for the team, `bm status` SHOULD include a daemon section:

```
Daemon: running (PID 12345, webhook mode, port 8484)
```

This is additive — daemon status appears above the member table.

#### 9.4. Backward compatibility

If no topology file exists, fall back to runtime state (`~/.botminter/state.json`) — the
current behavior. The table format remains unchanged for local-only teams without topology.

---

## 10. Testing

### Summary

E2E and integration tests are the TOP PRIORITY. Every feature MUST have corresponding tests.

### Test Strategy

#### 10.1. Unit tests (in source modules)

| Module | Tests to add |
|--------|-------------|
| `profile.rs` | v2 schema parsing, `check_schema_version("v2")`, formation dir extraction |
| `session.rs` | Verify binary existence checks, env var passing (mock subprocess) |
| `state.rs` | Topology file read/write, formation endpoint parsing |
| `workspace.rs` | Skills dir sync, formations dir handling |
| New: `topology.rs` | Topology file serialization, deserialization, endpoint type handling |
| New: `daemon.rs` | Config file read/write, PID file handling, poll state tracking |
| New: `formation.rs` | Formation config parsing, formation resolution |
| New: `knowledge.rs` | Knowledge listing logic, scope filtering, path validation |

#### 10.2. Integration tests (`tests/integration.rs`)

| Scenario | Test |
|----------|------|
| Schema v2 init | `bm init` with v2 profile → team repo has `skills/`, `formations/` dirs |
| Knowledge list | Create knowledge at all 4 scopes → `bm knowledge list` shows all |
| Knowledge show | `bm knowledge show knowledge/foo.md` → outputs file content |
| Knowledge v1 gate | v1 team → `bm knowledge list` fails with schema error |
| Formation flag parsing | `bm start --formation local` parses correctly |
| Formation v1 gate | v1 team → `bm start --formation k8s` fails with schema error |
| Topology file lifecycle | Start local → topology exists → stop → topology removed |
| Daemon config | Write/read daemon config file round-trip |
| Multi-formation status | Status with topology file vs without |

#### 10.3. E2E tests (`tests/e2e/`)

Gated behind `--features e2e`.

| Scenario | Test |
|----------|------|
| Knowledge list E2E | Init v2 team → hire → add knowledge files → `bm knowledge list` |
| Start local with topology | Init → hire → sync → start → verify topology file → stop → verify topology removed |
| Daemon lifecycle | `bm daemon start` → `bm daemon status` → `bm daemon stop` |
| Daemon poll mode | Daemon in poll mode → simulate event → verify member launched |
| Formation-aware status | Start with topology → `bm status` shows formation info |

**Interactive Claude sessions MUST NOT be tested in CI.** Test the session abstraction (binary
existence check, argument construction, env vars) without actually launching Claude.

#### 10.4. Kind tests (`tests/e2e/` with `kind-tests` feature)

Gated behind `--features e2e,kind-tests`.

**Prerequisites:** `kind` and `kubectl` in PATH, Docker running.

| Scenario | Test |
|----------|------|
| K8s formation deploy | Deploy members as pods → verify pods running |
| K8s formation idempotency | Deploy twice → second run skips healthy pods |
| K8s formation teardown | Stop → pods deleted, namespace cleaned |
| K8s formation reconciliation | Remove a member → redeploy → stale pod removed |
| K8s status | Status shows k8s endpoints with live health |
| K8s unreachable fallback | Delete cluster → status shows cached with warning |

**Cluster management in tests:**
- Each test creates a dedicated kind cluster (`bm-test-<random>`).
- Tests clean up the cluster on drop (RAII pattern like TempRepo).
- Use a `KindCluster` test helper struct.

#### 10.5. Webhook tests

Webhook tests MUST use simulated payloads — no actual GitHub webhooks.

| Scenario | Test |
|----------|------|
| Webhook payload parsing | Simulate POST with issues event → daemon triggers |
| Webhook signature validation | Invalid signature → request rejected |
| Webhook irrelevant event | Push event → no members started |

#### 10.6. Feature flags in Cargo.toml

```toml
[features]
e2e = []
kind-tests = ["e2e"]
```

The `kind-tests` feature implies `e2e`.

#### 10.7. Justfile recipes

```
just test               # cargo test -p bm
just test-e2e           # cargo test -p bm --features e2e
just test-kind          # cargo test -p bm --features e2e,kind-tests
```

---

## 11. Data Structures

### 11.1. Topology File (`topology.json`)

```json
{
  "formation": "local|k8s",
  "created_at": "ISO-8601",
  "members": {
    "<member_dir_name>": {
      "status": "running|stopped|error",
      "endpoint": {
        "type": "local",
        "pid": 12345,
        "workspace": "/absolute/path"
      }
    }
  }
}
```

K8s endpoint variant:
```json
{
  "type": "k8s",
  "namespace": "botminter-<team>",
  "pod": "<pod-name>",
  "container": "ralph",
  "context": "<kubectl-context>"
}
```

### 11.2. Daemon Config (`daemon-<team>.json`)

```json
{
  "team": "my-team",
  "mode": "webhook|poll",
  "port": 8484,
  "interval_secs": 60,
  "pid": 12345,
  "started_at": "ISO-8601"
}
```

### 11.3. Poll State (`daemon-<team>-poll.json`)

```json
{
  "last_event_id": "string",
  "last_poll_at": "ISO-8601"
}
```

### 11.4. Formation Config (`formation.yml`)

```yaml
name: "local|k8s|<custom>"
description: "Human-readable description"
type: "local|k8s"
# k8s-specific:
k8s:
  context: "kind-botminter"
  image: "ghcr.io/owner/ralph:latest"
  namespace_prefix: "botminter"
# manager config (non-local only):
manager:
  ralph_yml: "ralph.yml"
  prompt: "PROMPT.md"
  hats_dir: "hats/"
```

### 11.5. Credentials extension

`BotminterConfig` → `Credentials` struct gains an optional field:

```rust
pub struct Credentials {
    pub gh_token: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub webhook_secret: Option<String>,    // NEW — for daemon webhook verification
}
```

### 11.6. Schema v2 layout (`.schema/v2.yml`)

```yaml
name: v2
team:
  knowledge: "knowledge"
  invariants: "invariants"
  projects: "projects"
  members: "team"
  skills: "skills"
  formations: "formations"
member:
  knowledge: "knowledge"
  invariants: "invariants"
  projects: "projects"
  skills: "skills"
```

---

## 12. CLI Changes

### New commands

| Command | Flags | Description |
|---------|-------|-------------|
| `bm knowledge` | `-t team`, `--scope <scope>` | Interactive Claude session for knowledge management |
| `bm knowledge list` | `-t team`, `--scope <scope>` | List knowledge/invariant files by scope |
| `bm knowledge show <path>` | `-t team` | Display knowledge/invariant file contents |
| `bm daemon start` | `-t team`, `--mode <mode>`, `--port <port>`, `--interval <secs>` | Start event-driven daemon |
| `bm daemon stop` | `-t team` | Stop daemon |
| `bm daemon status` | `-t team` | Show daemon status |

### Modified commands

| Command | Change |
|---------|--------|
| `bm start` | Add `--formation <name>` flag (default: `local`) |
| `bm stop` | Read topology file, handle formation-specific teardown, remove topology file |
| `bm status` | Show formation type, daemon status, formation-aware health checks |

### CLI enum additions to `cli.rs`

```rust
// New top-level commands
Knowledge {
    #[command(subcommand)]
    command: Option<KnowledgeCommand>,
    #[arg(short, long)]
    team: Option<String>,
    #[arg(long)]
    scope: Option<String>,
},

Daemon {
    #[command(subcommand)]
    command: DaemonCommand,
},

// New subcommand enums
pub enum KnowledgeCommand {
    List {
        #[arg(short, long)]
        team: Option<String>,
        #[arg(long)]
        scope: Option<String>,
    },
    Show {
        path: String,
        #[arg(short, long)]
        team: Option<String>,
    },
}

pub enum DaemonCommand {
    Start {
        #[arg(short, long)]
        team: Option<String>,
        #[arg(long, default_value = "webhook")]
        mode: String,
        #[arg(long, default_value = "8484")]
        port: u16,
        #[arg(long, default_value = "60")]
        interval: u64,
    },
    Stop {
        #[arg(short, long)]
        team: Option<String>,
    },
    Status {
        #[arg(short, long)]
        team: Option<String>,
    },
}

// Modified Start command
Start {
    #[arg(short, long)]
    team: Option<String>,
    #[arg(long, default_value = "local")]
    formation: String,
},
```

### New source files

| File | Purpose |
|------|---------|
| `crates/bm/src/session.rs` | Skill session abstraction (interactive Claude + one-shot Ralph) |
| `crates/bm/src/topology.rs` | Topology file read/write/types |
| `crates/bm/src/formation.rs` | Formation config parsing and resolution |
| `crates/bm/src/commands/knowledge.rs` | Knowledge list, show, interactive session |
| `crates/bm/src/commands/daemon.rs` | Daemon start, stop, status |

### New dependencies (Cargo.toml)

| Crate | Purpose |
|-------|---------|
| `tokio` (optional, daemon feature) | Async runtime for webhook HTTP server |
| `axum` or `tiny_http` (optional, daemon feature) | HTTP server for webhook mode |
| `signal-hook` | Signal handling for daemon |

**Decision:** The daemon's HTTP server dependency SHOULD be minimal. `tiny_http` (sync, no
async runtime) is preferred over `axum` (requires tokio) to keep the binary lightweight. If
async is needed later, this can be swapped.

Alternatively, the daemon MAY use a feature flag to gate the HTTP dependencies:

```toml
[features]
daemon = ["tiny_http", "signal-hook"]
e2e = []
kind-tests = ["e2e"]
```

---

## 13. Acceptance Criteria

### Schema v2

```
Given profiles with schema_version v2 and .schema/v2.yml
When a new team is initialized with `bm init`
Then the team repo contains skills/ and formations/ directories
And botminter.yml in the team repo shows schema_version: v2
```

```
Given a team with schema_version v1
When `bm knowledge list` is run
Then bm exits with exit code 1
And stderr contains "requires schema v2"
And stderr contains "bm upgrade"
```

```
Given a team with schema_version v1
When `bm start --formation k8s` is run
Then bm exits with exit code 1
And stderr contains "requires schema v2"
```

```
Given a team with schema_version v1
When `bm daemon start` is run
Then bm exits with exit code 1
And stderr contains "requires schema v2"
```

```
Given a team with schema_version v1
When `bm start` is run (no --formation flag)
Then bm starts members normally (backward compatible, no topology file required)
```

### Knowledge Management

```
Given a team with knowledge files at team, project, member, and member+project scopes
When `bm knowledge list` is run
Then all knowledge and invariant files are shown grouped by scope
And no Claude session is launched
And the output lists files under each scope heading
```

```
Given a team with knowledge files at multiple scopes
When `bm knowledge list --scope team` is run
Then only team-level knowledge and invariant files are shown
```

```
Given a team with a knowledge file at knowledge/commit-convention.md
When `bm knowledge show knowledge/commit-convention.md` is run
Then the file contents are printed to stdout
```

```
Given a team with no file at the specified path
When `bm knowledge show knowledge/nonexistent.md` is run
Then bm exits with exit code 1
And stderr contains "File not found"
```

```
Given a path outside knowledge/invariant directories (e.g., "botminter.yml")
When `bm knowledge show botminter.yml` is run
Then bm exits with exit code 1
And stderr contains "not within a knowledge or invariant directory"
```

### Daemon Lifecycle

```
Given no daemon running
When `bm daemon start` is run
Then a daemon process is started
And a PID file is created at ~/.botminter/daemon-<team>.pid
And a config file is created at ~/.botminter/daemon-<team>.json
And stdout shows "Daemon started (PID <pid>)"
```

```
Given a running daemon
When `bm daemon status` is run
Then output shows "Daemon: running (PID <pid>)"
And output shows the mode and team name
```

```
Given a running daemon
When `bm daemon stop` is run
Then the daemon process is terminated
And the PID file is removed
And the config file is removed
And stdout shows "Daemon stopped"
```

```
Given a running daemon
When `bm daemon start` is run again
Then bm exits with exit code 1
And stderr contains "already running"
```

```
Given no daemon running
When `bm daemon stop` is run
Then bm exits with exit code 1
And stderr contains "not running"
```

```
Given no daemon running
When `bm daemon status` is run
Then output shows "Daemon: not running"
And exit code is 0
```

### Event-Triggered Members

```
Given a running daemon in poll mode
When a new relevant GitHub event is detected
Then the daemon starts all team members one-shot
And members process all available work
And members exit after processing
And daemon cleans up member state
```

```
Given a running daemon with members currently running
When a new event arrives
Then the daemon does NOT launch duplicate members
And the event is processed after the current run completes
```

### Local Formation

```
Given synced workspaces
When `bm start --formation local` is run
Then members are launched as local processes
And a topology file is written at {workzone}/{team}/topology.json
And the topology file contains "formation": "local"
And each member has an endpoint with type "local", pid, and workspace path
```

```
Given a running local formation
When `bm stop` is run
Then all members are stopped
And the topology file is removed
```

```
Given a topology file exists
When `bm start` is run (members already running)
Then running members are skipped
And the topology file is updated with new members only
```

### K8s Formation

```
Given a reachable kind cluster
When `bm start --formation k8s` is run
Then a formation manager (one-shot Ralph session) is launched
And members are deployed as pods in namespace botminter-<team>
And pods are verified healthy
And a topology file is written with k8s endpoints
```

```
Given pods already running from a previous deployment
When `bm start --formation k8s` is run again
Then the formation manager detects healthy pods
And skips redeployment for healthy pods
And the topology file is updated
```

```
Given a running k8s formation
When `bm stop` is run
Then pods are deleted
And the namespace is cleaned up (or left for reuse)
And the topology file is removed
```

```
Given a member was removed from the team
When `bm start --formation k8s` is run
Then the stale pod for the removed member is deleted
And only current members are deployed
```

```
Given a kind cluster with deployed pods
When a pod is in error state
Then the formation manager redeploys that pod
And verifies the new pod is healthy
```

### Formation-Aware Status

```
Given a local formation with a topology file
When `bm status` is run
Then the output shows "Formation: local"
And each member shows PID-based health
```

```
Given a k8s formation with a topology file
When `bm status` is run
Then the output shows "Formation: k8s"
And each member shows pod-based health
And the endpoint column shows pod names
```

```
Given a k8s formation and the cluster is unreachable
When `bm status` is run
Then output falls back to cached topology
And a warning is printed: "Could not reach k8s target"
And member status shows "unknown"
```

```
Given a daemon is running
When `bm status` is run
Then daemon status is shown above the member table
```

```
Given no topology file exists (legacy/v1 team)
When `bm status` is run
Then status falls back to runtime state (current behavior)
And no formation line is shown
```

### Testing

```
Given the full test suite
When `cargo test -p bm` is run
Then all unit and integration tests pass
And clippy reports no warnings
```

```
Given E2E prerequisites met
When `cargo test -p bm --features e2e` is run
Then all E2E tests pass (excluding kind tests)
```

```
Given a kind cluster is available and Docker is running
When `cargo test -p bm --features e2e,kind-tests` is run
Then k8s formation deploy, teardown, idempotency, and reconciliation tests pass
```

```
Given no kind cluster available
When `cargo test -p bm --features e2e,kind-tests` is run
Then kind tests are skipped gracefully (not failed)
```

---

## 14. Edge Cases and Error Conditions

### Schema version

| Condition | Expected behavior |
|-----------|-------------------|
| Team botminter.yml missing `schema_version` | Treated as empty string → mismatch error |
| Team botminter.yml has `schema_version: v3` | Mismatch with embedded v2 → error |
| Profile has v2, team has v2 | Match → proceed |
| Profile has v2, team has v1 | Mismatch → error with upgrade message |

### Knowledge management

| Condition | Expected behavior |
|-----------|-------------------|
| Team has no knowledge files at any scope | `bm knowledge list` shows all scopes with `(none)` |
| Path traversal attempt (`../../etc/passwd`) | Reject: path resolves outside team repo |
| Binary file in knowledge dir | Listed but `bm knowledge show` warns about binary content |
| Symlink in knowledge dir | Follow symlink, display target content |
| Knowledge dir doesn't exist | Show `(none)` for that scope, don't error |

### Daemon

| Condition | Expected behavior |
|-----------|-------------------|
| Daemon process crashes | PID file remains, `bm daemon status` detects dead PID, reports "not running (stale PID file)" |
| Port already in use (webhook mode) | Daemon exits with error, PID file cleaned up |
| `gh` CLI not authenticated (poll mode) | Daemon logs error, retries on next interval |
| Multiple teams | Each team has its own daemon instance |
| Daemon started but team deleted | Daemon continues running but member launches fail |

### Formations

| Condition | Expected behavior |
|-----------|-------------------|
| `--formation nonexistent` | Error: "Formation 'nonexistent' not found in team repo" |
| Formation manager Ralph session fails | `bm start` exits with error, no topology file written |
| Topology file is corrupted JSON | Fall back to runtime state with warning |
| Topology file refers to dead PIDs | Status shows `crashed`, stop cleans up |
| Kind cluster not reachable | Formation manager fails, reports error |
| `kubectl` not in PATH | Formation manager fails with clear error |
| Docker not running (kind tests) | Tests skipped, not failed |

### Backward compatibility

| Condition | Expected behavior |
|-----------|-------------------|
| v1 team, `bm start` (no formation flag) | Works exactly as before, no topology file |
| v1 team, `bm stop` | Works exactly as before |
| v1 team, `bm status` | Works exactly as before |
| v2 team, `bm start` (no formation flag) | Defaults to local, writes topology file |
| Mixed v1 and v2 teams | Each team operates independently at its schema version |

---

## 15. Non-Functional Requirements

### Performance

- `bm knowledge list` MUST complete in < 1 second for a team with 100 knowledge files.
- `bm daemon status` MUST complete in < 2 seconds.
- `bm status` with local topology MUST complete in < 2 seconds.
- `bm status` with k8s topology MUST complete in < 10 seconds (kubectl network latency).
- Daemon polling MUST NOT consume more than 1 API call per interval.

### Security

- Topology file MUST be written with 0600 permissions (contains PIDs, paths).
- Daemon PID file MUST be written with 0600 permissions.
- Webhook secret MUST NOT be logged or printed.
- K8s secrets MUST be created via kubectl, not written to disk.
- GH_TOKEN MUST NOT appear in daemon logs.
- Credentials in `config.yml` maintain existing 0600 permissions.

### Reliability

- Daemon MUST handle SIGTERM gracefully — stop in-progress member runs, clean up state.
- Daemon MUST NOT leave orphaned member processes on crash.
- Topology file writes MUST be atomic (write temp + rename).
- Formation manager failures MUST NOT corrupt existing topology.

### Observability

- Daemon logs to `~/.botminter/logs/daemon-<team>.log`.
- Log format: `[ISO-8601] [LEVEL] message`.
- Log rotation at 10MB.
- `bm daemon status` shows uptime, last event time, event count.

---

## 16. Out of Scope

These items are explicitly NOT part of this milestone:

- **`bm upgrade`** — schema migration from v1 to v2 (teams must re-init).
- **Per-member daemon routing** — daemon starts ALL members, not specific ones.
- **Cloud VM formations** — only local and k8s (kind) formations.
- **Production k8s** — kind (local) only, not EKS/GKE/AKS.
- **Interactive Claude session testing in CI** — test the abstraction, not the AI.
- **Daemon clustering/HA** — single daemon per team.
- **Webhook tunnel setup** — operator provides the tunnel (e.g., ngrok, cloudflared).
- **Custom formation types** — only `local` and `k8s` are implemented.
- **Knowledge file versioning** — knowledge changes are tracked by git, no additional versioning.
- **Automatic schema migration** — v1 teams must re-init to v2.

---

## 17. Implementation Ordering

Suggested task ordering (phases are sequential, tasks within a phase are parallel):

### Phase 1: Foundation

1. Profile schema v2 (new schema file, botminter.yml bump, profile directory additions)
2. Schema v2 gate function (reusable check for v2-dependent commands)
3. Topology module (`topology.rs` — types, read/write, serialization)
4. Formation config module (`formation.rs` — parsing, resolution)

### Phase 2: Knowledge Management

5. Knowledge list command (deterministic, no Claude)
6. Knowledge show command (deterministic, no Claude)
7. Session abstraction module (`session.rs`)
8. Knowledge interactive command (Claude session launch)
9. Knowledge manager skill embedded in profiles

### Phase 3: Formations

10. Local formation — topology file write on start, read on stop/status
11. Formation flag on `bm start`
12. Formation-aware stop (read topology, clean up)
13. Formation-aware status (read topology, health checks, fallback)

### Phase 4: Daemon

14. Daemon start/stop/status lifecycle
15. Daemon poll mode
16. Daemon webhook mode
17. Daemon member launch (one-shot execution)

### Phase 5: K8s Formation

18. K8s formation manager (Ralph session with hats)
19. K8s formation integration (formation.yml, manager launch)
20. K8s formation tests (kind cluster)

### Phase 6: Testing & Polish

21. Unit tests for all new modules
22. Integration tests for cross-feature scenarios
23. E2E tests for lifecycle scenarios
24. Kind E2E tests
25. Design principles update (Section 2 — one-shot model)
26. Clippy clean, documentation
