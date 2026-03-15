# Domain Pitfalls

**Domain:** Pluggable communication bridge for agent orchestration (BotMinter v0.07)
**Researched:** 2026-03-08

## Critical Pitfalls

Mistakes that cause rewrites or major issues.

### Pitfall 1: Leaky Abstraction — Bridge Knows Too Much About Ralph's Robot

**What goes wrong:** The bridge abstraction (BotMinter layer) bleeds into Ralph's `RobotService` trait, or vice versa. You end up with the bridge shell script needing to understand Ralph event semantics (`human.interact`, `human.response`), or Ralph's robot trait needing to know about bridge lifecycle (start/stop/health). The two layers collapse into one.

**Why it happens:** Ralph already has a `RobotService` trait in `ralph-proto` with methods like `send_question()`, `wait_for_response()`, and `send_checkin()`. BotMinter's bridge abstraction manages service lifecycle (start/stop/health/configure). These are genuinely different concerns, but the temptation is to make the bridge script also implement message routing — or to have Ralph's robot implementation call bridge lifecycle methods.

**Consequences:** If the bridge leaks into Ralph, the upstream PR becomes unacceptable (too BotMinter-specific). If Ralph leaks into the bridge, you cannot swap communication backends without also changing Ralph's internals. Either way, you lose the pluggability that is the entire point of v0.07.

**Prevention:**
- Draw a hard line: the bridge manages **service lifecycle and connection config** (URL, credentials, room IDs). Ralph's robot manages **message semantics** (questions, responses, check-ins).
- The bridge script outputs connection config (URL, token, room). Ralph's robot implementation consumes that config to send/receive messages. The bridge never sees event payloads.
- Write the bridge spec first (ADR-001) with explicit "what the bridge does NOT do" section.

**Detection:** If a bridge script has the words "human.interact" or "event" in it, the abstraction is leaking. If a `RobotService` implementation calls `docker compose up`, it is leaking the other direction.

**Confidence:** HIGH — directly observable from the existing `RobotService` trait in `ralph-proto/src/robot.rs` and the stated design goal.

---

### Pitfall 2: N Bot Users Requires Admin-Level API Access on Rocket.Chat

**What goes wrong:** The bridge script for Rocket.Chat needs to create N bot user accounts (one per team member) programmatically. Rocket.Chat requires admin credentials to create users with the `bot` role. Each bot user also needs a unique email address. Teams discover this only after building the happy path with a single bot account.

**Why it happens:** Rocket.Chat bot accounts are "regular user accounts with the bot role" — admin-only creation. The REST API endpoint `users.create` requires `admin` or `create-user` permission. There is no self-service bot registration. Additionally, each user requires a unique email, which means either generating fake emails or using Gmail +address tricks.

**Consequences:** The bridge `configure` step becomes a privileged admin operation rather than a simple "connect to service" step. If the bridge script only has a regular user token, it silently fails or errors cryptically. Teams with restricted Rocket.Chat instances cannot use per-agent identity at all.

**Prevention:**
- The bridge `configure` script must explicitly require and validate admin credentials upfront, failing fast with a clear error message.
- Generate deterministic bot emails using a pattern like `{member-name}@botminter.local` — Rocket.Chat does not validate email deliverability.
- Document the admin credential requirement prominently. Consider a fallback mode where all agents share one bot account (degraded but functional).
- Test the full N-user creation flow early (Phase 2 not Phase 4).

**Detection:** If the bridge configure step succeeds with a non-admin token in testing, something is wrong — you are probably not actually creating bot users.

**Confidence:** HIGH — verified via [Rocket.Chat bots architecture docs](https://developer.rocket.chat/docs/bots-architecture) and [REST API user creation](https://docs.rocket.chat/api/rest-api/methods/users/create).

---

### Pitfall 3: stdout Config Exchange Corrupted by Shell Script Diagnostic Output

**What goes wrong:** The bridge contract specifies that scripts output configuration via stdout (JSON with URL, token, room IDs). But shell scripts also produce diagnostic output — `set -x` traces, `echo` debug messages, Docker pull progress, `docker compose up` startup logs, or even `npm install` output. The calling Rust code (`bm`) parses the combined output and gets garbage instead of config.

**Why it happens:** stdout serves dual purposes: structured data exchange and human-readable diagnostics. In a shell script that calls `docker compose up` then outputs config, all of Docker's startup output goes to stdout too, unless explicitly redirected. Developers test scripts interactively (where mixed output looks fine) but the programmatic consumer chokes on it.

**Consequences:** `bm bridge start` succeeds (exit code 0) but returns unparsable config. Ralph gets no connection details and silently operates without communication. The failure is invisible — agents work but nobody can talk to them.

**Prevention:**
- **Contract rule: all diagnostic output MUST go to stderr.** The bridge spec must state this explicitly. The very first line of every bridge script should be `exec 2>&1` only if explicitly NOT using stdout for data — or better, the contract should say "only the LAST line of stdout is config JSON."
- Alternatively, use a **file-based config exchange** instead: the script writes config to a known path (e.g., `$BRIDGE_CONFIG_DIR/config.json`) and stdout is purely diagnostic. This is more robust than stdout parsing.
- `bm` should validate that the stdout output is valid JSON before using it. If parsing fails, show the raw output in the error message so the user can see what went wrong.
- Test bridge scripts with `set -e` and ensure Docker output is redirected to stderr explicitly: `docker compose up -d 2>&1 1>/dev/null` patterns.

**Detection:** If you can run the bridge `start` script manually and see anything other than a single JSON line on stdout, the contract is violated.

**Confidence:** HIGH — this is a well-known Unix IPC pitfall. See [shell stdout/stderr separation patterns](https://www.howtogeek.com/435903/what-are-stdin-stdout-and-stderr-on-linux/).

---

### Pitfall 4: Upstream PR to Ralph Orchestrator Blocks the Entire Milestone

**What goes wrong:** The robot abstraction upstream contribution to Ralph Orchestrator gets stuck in review, maintainer requests major design changes, or the PR scope creeps. Meanwhile, all BotMinter bridge work depends on Ralph having a pluggable robot backend. The milestone stalls.

**Why it happens:** You do not own Ralph Orchestrator. The maintainer (mikeyobrien) has their own priorities and design opinions. Ralph already has a `RobotService` trait, but making the backend truly pluggable (runtime selection, configuration-driven instantiation) touches core event loop code (`ralph-core/src/event_loop/mod.rs`). This is a significant change for an upstream project.

**Consequences:** If the upstream PR is not merged, BotMinter either ships with a local fork (creating "fork drift" maintenance burden) or the milestone slips indefinitely. The local checkout already has a custom commit for Telegram URL support — adding more divergence compounds the problem.

**Prevention:**
- **Develop against a local fork branch from day one**, but structure the changes as clean, upstreamable patches. Do not wait for the PR to merge before building BotMinter features on top.
- Keep the upstream PR minimal: make `RobotService` instantiation configurable via `ralph.yml` (e.g., `robot.backend: "telegram" | "rocketchat" | "custom"`). Do NOT put BotMinter-specific logic in the PR.
- Start the upstream conversation early (Phase 1) — open an issue describing the change before writing code. Get design buy-in first.
- Tag local-only patches clearly (e.g., `[botminter-local]` commit prefix) so you can identify what needs upstreaming vs. what is downstream-only.
- Accept that BotMinter may ship v0.07 against the fork. Plan the Cargo dependency to point at the fork branch with a comment explaining when to switch back to upstream.

**Detection:** If 2+ weeks pass without upstream review response, escalate the conversation or formally accept the fork strategy.

**Confidence:** HIGH — the project context confirms a local checkout with existing local commits, and [fork drift literature](https://preset.io/blog/stop-forking-around-the-hidden-dangers-of-fork-drift-in-open-source-adoption/) extensively documents this risk.

---

### Pitfall 5: MongoDB Replica Set Requirement Breaks "Local Slack-like" Developer Experience

**What goes wrong:** Rocket.Chat requires MongoDB configured as a replica set — not a standalone instance. Docker Compose novices (the target audience for "local Slack-like experience") start a simple MongoDB container, Rocket.Chat refuses to connect, and the error message is cryptic (`MongoServerSelectionError`). The promise of "easy local setup" evaporates.

**Why it happens:** Rocket.Chat uses MongoDB change streams, which require a replica set. This is a hard requirement, not a preference. The official Docker Compose file handles this, but if the bridge script uses a custom compose setup, it is easy to forget the `--replSet rs0` flag and the `rs.initiate()` call.

**Consequences:** Users spend hours debugging MongoDB connectivity instead of trying BotMinter. First impression of the bridge feature is "broken." Support burden shifts from BotMinter features to Docker/MongoDB troubleshooting.

**Prevention:**
- Ship a complete, tested `docker-compose.yml` as part of the Rocket.Chat bridge. Do NOT ask users to configure MongoDB themselves.
- The compose file must include the MongoDB replica set initialization as a startup script or init container. Use the pattern from Rocket.Chat's official compose file.
- The bridge `health` check must verify Rocket.Chat is actually responding to API calls, not just that the container is running. MongoDB being "up" but not initialized as a replica set means Rocket.Chat is in a crash loop.
- Pin specific, tested versions of both Rocket.Chat and MongoDB images. Do not use `latest` tags.
- Handle the [MongoDB keyfile format issue](https://github.com/RocketChat/Rocket.Chat/issues/35039) proactively — this is the most commonly reported Docker deployment problem as of 2025.

**Detection:** If `bm bridge start rocketchat` takes more than 60 seconds on first run (excluding image pulls), something is wrong with the initialization sequence.

**Confidence:** HIGH — verified via [Rocket.Chat Docker deployment docs](https://docs.rocket.chat/docs/deploy-with-docker-docker-compose) and [multiple community reports](https://forums.rocket.chat/t/docker-mongo-issues/14937).

---

## Moderate Pitfalls

### Pitfall 6: Bridge Scripts Are Not Idempotent

**What goes wrong:** Running `bm bridge start` twice creates duplicate containers, duplicate bot users, or duplicate rooms. Running `bm bridge stop` then `bm bridge start` loses state (rooms, conversation history).

**Prevention:**
- Bridge scripts must be idempotent. `start` checks if the service is already running and returns existing config. `stop` is a no-op if already stopped.
- Use named Docker containers (`botminter-rocketchat-{team}`) so `start` can detect existing instances via `docker inspect`.
- Store bridge state in the team config (`~/.botminter/config.yml` or a `bridge.json` in the team directory) so config survives restarts.

**Confidence:** HIGH — standard infrastructure lifecycle pattern.

---

### Pitfall 7: Rocket.Chat Bots API Deprecated in Favor of Apps-Engine

**What goes wrong:** You build the bot integration using the traditional Rocket.Chat Bots SDK or direct REST API bot patterns, only to discover these are deprecated. Rocket.Chat recommends Apps-Engine for new integrations. But Apps-Engine is designed for apps running *inside* Rocket.Chat, not external bots — which is what BotMinter agents are.

**Prevention:**
- Use the REST API directly for bot user management (`users.create`, `login`, `channels.create`, `chat.sendMessage`). The REST API is NOT deprecated — only the "Bots SDK" integration pattern is.
- Do NOT depend on the `Rocket.Chat.js.SDK` for new development. It is tied to the deprecated bots integration path.
- Apps-Engine is irrelevant for BotMinter's use case (external agents posting messages). Stick with REST API + websocket for real-time message reception.
- Document this decision explicitly in ADR format so future contributors understand why Apps-Engine was not used.

**Confidence:** MEDIUM — based on [Rocket.Chat developer docs](https://developer.rocket.chat/docs/bots-architecture) deprecation notice, but REST API stability is not explicitly guaranteed for bot use cases.

---

### Pitfall 8: Per-Agent Identity Creates O(N) Websocket Connections

**What goes wrong:** Each agent has its own bot user on Rocket.Chat. To receive messages directed at a specific agent, each bot user needs its own authenticated websocket connection (or polling loop). With 10 team members, that is 10 persistent connections. Rocket.Chat's default connection limits or resource consumption become a problem on the "local development" target hardware.

**Prevention:**
- Use a single "dispatcher" connection that monitors all relevant rooms, then routes incoming messages to the correct agent based on @-mentions or DM target. This is one connection regardless of team size.
- The dispatcher pattern matches BotMinter's existing architecture: a single `bm` process managing multiple agents. The bridge runs one connection, Ralph instances receive their messages via the events file.
- If direct per-agent connections are needed later, make it a configuration option, not the default.

**Confidence:** MEDIUM — depends on Rocket.Chat websocket behavior and team sizes. Teams of 3-5 agents (typical) are unlikely to hit limits, but the architecture should not bake in O(N) connections.

---

### Pitfall 9: Bridge Auto-Start from `bm start` Creates Circular Dependencies

**What goes wrong:** `bm start` optionally auto-starts the bridge, which starts Docker containers, which takes 30-60 seconds. Meanwhile, Ralph instances are launching and trying to connect to the bridge that is not ready yet. Agents fail to register their bot identity, fall back to no-communication mode, and the operator thinks the bridge is broken.

**Prevention:**
- `bm start` must start the bridge FIRST, wait for the health check to pass, THEN launch Ralph instances. Sequential, not parallel.
- The bridge `start` command must block until the service is healthy (not just until Docker containers are "running"). Use a readiness probe — poll `GET /api/v1/info` for Rocket.Chat until it returns 200.
- Set a reasonable timeout (120 seconds for first start with image pulls, 30 seconds for subsequent starts). Fail clearly if the bridge does not become healthy.
- Ralph instances should have retry logic for initial bot registration — do not fail permanently on first connection attempt.

**Confidence:** HIGH — this is a standard service dependency ordering problem.

---

### Pitfall 10: Shell Script Bridge Contract Is Untestable in CI

**What goes wrong:** Bridge scripts require Docker to run. CI environments (GitHub Actions) support Docker but with restrictions (no nested Docker, limited resources). Integration tests that start Rocket.Chat + MongoDB in CI are slow (2+ minutes), flaky (port conflicts, OOM), and expensive.

**Prevention:**
- Separate the bridge contract into testable layers: (1) script argument parsing and stdout format (testable without Docker), (2) Docker lifecycle (integration test, CI-optional), (3) Rocket.Chat API interaction (mockable with a simple HTTP server).
- For CI, test the contract with a mock bridge script that outputs valid config JSON and exits. Test the real Rocket.Chat bridge only in a dedicated integration test suite with `#[ignore]` or a feature flag.
- Add a `bm bridge test` command that validates a bridge script implements the contract correctly (calls start/stop/health, checks output format) without requiring the actual service.

**Confidence:** HIGH — BotMinter already uses `libtest-mimic` and GithubSuite for E2E tests, showing awareness of CI test cost.

---

### Pitfall 11: ADRs Introduced But Not Integrated Into Development Flow

**What goes wrong:** ADRs are created for the bridge spec but never referenced during implementation. Developers build what they remember from discussions, not what the ADR says. ADRs become stale documentation artifacts rather than living decision records.

**Prevention:**
- ADRs live in `specs/adrs/` (not a separate docs site). Number them sequentially: `001-bridge-abstraction.md`.
- Reference ADRs in code comments where the decision is implemented: `// See ADR-001: bridge scripts output config via stdout`.
- ADRs are immutable once accepted. New decisions supersede old ones with a new ADR that references the superseded one.
- Keep ADRs short (1-2 pages). The format should be: Context, Decision, Consequences. Not a design doc.

**Confidence:** MEDIUM — this is an organizational practice pitfall, not a technical one. Whether it manifests depends on team discipline.

---

## Minor Pitfalls

### Pitfall 12: Docker Compose File Conflicts With User's Existing Setup

**What goes wrong:** The shipped `docker-compose.yml` uses port 3000 (Rocket.Chat default) or 27017 (MongoDB default), which conflict with the user's existing services.

**Prevention:** Use non-standard ports by default (e.g., 13000 for Rocket.Chat, 37017 for MongoDB). Make ports configurable via environment variables in the compose file. The bridge `configure` output includes the actual port used.

**Confidence:** HIGH — trivially preventable.

---

### Pitfall 13: Bridge Script Assumes Bash-Specific Features

**What goes wrong:** Bridge scripts use bashisms (`[[ ]]`, `set -o pipefail`, arrays) but the shebang says `#!/bin/sh`. On systems where `/bin/sh` is dash (Debian/Ubuntu), scripts fail with cryptic syntax errors.

**Prevention:** Either use `#!/usr/bin/env bash` consistently, or write strictly POSIX-compliant shell. Since bridge scripts will likely need JSON parsing (via `jq`) and Docker interaction, bash is the pragmatic choice — just declare it explicitly.

**Confidence:** HIGH — standard shell portability issue.

---

### Pitfall 14: Rocket.Chat Version Pinning Becomes Stale

**What goes wrong:** The bridge ships with Rocket.Chat 7.x Docker image, which works today. Six months later, users pull the bridge and MongoDB 8.x compatibility issues or API changes break the setup.

**Prevention:** Pin exact versions in `docker-compose.yml` (e.g., `rocket.chat:7.5.0`, not `rocket.chat:7` or `rocket.chat:latest`). Document the tested version range. Include a version check in the bridge health script.

**Confidence:** MEDIUM — slow-moving risk but real. Rocket.Chat has [MongoDB version compatibility requirements](https://forums.rocket.chat/t/the-ultimate-guide-upgrading-rocketchat-deployed-in-docker-and-upgrading-mongodb/13886) that change between major releases.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| ADRs + Bridge Spec (Phase 1) | ADR scope creep — trying to spec everything before building anything | Time-box to 2-3 days. ADR covers the contract; implementation discovers edge cases. |
| Ralph Robot Abstraction (Phase 1-2) | Upstream PR scope creep — adding features Ralph does not need | Keep PR to: config-driven robot backend selection + trait already exists. No new features. |
| Bridge Shell Contract (Phase 2) | stdout/stderr mixing (Pitfall 3) | Adopt file-based config exchange or "last line is JSON" rule from the start. |
| Rocket.Chat Bridge (Phase 3) | MongoDB replica set init (Pitfall 5) | Ship complete docker-compose.yml with init script. Test on clean Docker install. |
| Per-Agent Identity (Phase 3-4) | N bot users need admin API (Pitfall 2) | Validate admin creds in configure step. Provide fallback single-bot mode. |
| Telegram Migration (Phase 4) | Breaking existing `scrum-compact-telegram` profile | Keep the existing env var (`RALPH_TELEGRAM_BOT_TOKEN`) working. Bridge wraps existing behavior. |
| `bm bridge start` Integration (Phase 4-5) | Service ordering with `bm start` (Pitfall 9) | Bridge starts first, blocks until healthy, then agents launch. |
| CI Testing (Throughout) | Docker-dependent tests slow CI (Pitfall 10) | Mock bridge for unit tests. Real bridge only in dedicated integration suite. |

## Sources

- [Rocket.Chat Bots Architecture](https://developer.rocket.chat/docs/bots-architecture) — bot role requirements, deprecation notice
- [Rocket.Chat Docker Deployment](https://docs.rocket.chat/docs/deploy-with-docker-docker-compose) — MongoDB replica set requirement, compose setup
- [Rocket.Chat MongoDB Keyfile Issue #35039](https://github.com/RocketChat/Rocket.Chat/issues/35039) — most common Docker deployment blocker
- [Fork Drift in Open Source](https://preset.io/blog/stop-forking-around-the-hidden-dangers-of-fork-drift-in-open-source-adoption/) — upstream contribution risks
- [Salesforce No-Fork Rules](https://engineering.salesforce.com/no-forking-way-dc5fa842649b/) — upstream contribution best practices
- Ralph Orchestrator source: `ralph-proto/src/robot.rs` (RobotService trait), `ralph-core/src/event_loop/mod.rs` (robot service injection), `ralph-telegram/` (Telegram implementation) — verified locally
- BotMinter source: `crates/bm/src/commands/start.rs` (current Telegram token handling) — verified locally
