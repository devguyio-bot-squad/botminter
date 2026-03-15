# Phase 10: Rocket.Chat Bridge - Context

**Gathered:** 2026-03-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Ship a complete Rocket.Chat bridge as the first local-type bridge implementation, proving the bridge abstraction works end-to-end with full lifecycle management (start/stop/health), per-agent bot identity, room provisioning, and bidirectional communication via Ralph's existing `ralph-rocketchat` crate. The E2E test replaces Telegram as the primary operator journey test.

</domain>

<decisions>
## Implementation Decisions

### Spike-first approach
- First deliverable is a **standalone spike** (outside bridge structure) that proves Ralph + Rocket.Chat Podman Pod works end-to-end
- Spike must prove: Pod boots RC + MongoDB, bot user created via REST API, Ralph sends a question to RC, human replies, Ralph receives the response (full bidirectional human-in-the-loop)
- **Blocker policy:** If Ralph's `ralph-rocketchat` crate doesn't work against a real Rocket.Chat instance during the spike, the phase stops and we fix Ralph first before continuing
- Once spike passes, port container recipes into the bridge Justfile structure

### User journeys (ordered)
1. **Spike: Ralph + RC Podman Pod standalone** — standalone script boots RC + MongoDB via Podman Pod, creates admin + bot user, runs Ralph with RC backend, proves bidirectional messaging works
2. **Operator selects RC during `bm init`** — Rocket.Chat appears as a bridge option in the init wizard
3. **Operator starts/stops RC server** — `bm bridge start` launches RC + MongoDB via Podman Pod, `bm bridge stop` tears it down, `bm bridge status` shows health
4. **Operator provisions bot identities** — `bm bridge identity add <name>` creates a Rocket.Chat user with bot role via REST API and returns credentials
5. **`bm teams sync --bridge` provisions rooms + identities** — team channel created if missing, per-member bot identities provisioned
6. **`bm start` launches Ralph with correct RC credentials** — `bm start` passes bridge-type-aware env vars (not hardcoded Telegram), Ralph connects to RC
7. **Agents communicate via Rocket.Chat** — bot commands (`/status`, `/tasks`, etc.) work using Ralph's existing command handler
8. **Full E2E test covers the complete journey** — replaces Telegram as primary operator journey test

### E2E testing
- **Real Podman + Rocket.Chat** in E2E tests — spin up actual RC + MongoDB via Podman, exercise real REST API calls
- RC E2E test **replaces Telegram** as the primary operator journey test (init -> hire -> sync -> bridge start -> identity -> rooms -> health)
- **Keep a minimal Telegram E2E test** for the external bridge (identity-only) path — proves both bridge types have coverage
- **Podman is required** — RC E2E tests fail if Podman is unavailable (no skip-if-missing)

### Bridge-type-aware credential injection
- `bm start` currently hardcodes `RALPH_TELEGRAM_BOT_TOKEN` (TODO at `start.rs:113`) — must become bridge-type-aware
- For Rocket.Chat, Ralph needs: `RALPH_ROCKETCHAT_AUTH_TOKEN`, `RALPH_ROCKETCHAT_SERVER_URL` as env vars, plus `RObot.rocketchat.bot_user_id` and `RObot.rocketchat.room_id` in ralph.yml
- `inject_robot_enabled` in `workspace.rs` needs to also inject RC-specific config fields into ralph.yml (not just `enabled` flag)

### Planning constraint: user-journey-driven plans
- **Lesson from Phase 9:** Plans focused on small, fine-grained file-level details that individually looked correct but collectively diverged from requirements. The implementation didn't add up to fulfill the actual user journeys, and gaps had to be filled manually outside GSD.
- **Hard rule for Phase 10:** Every plan must be framed as a user journey or scenario, not as file-level tasks. Each plan should be verifiable by running the journey end-to-end, not by checking that individual files exist.
- **Verification = run the journey:** A plan is done when the operator can execute the journey it describes and get the expected result. Not when the code compiles or tests pass in isolation.
- **No plan without a user scenario:** If a piece of work can't be described as something an operator or agent does, it belongs inside a journey plan — not as its own plan.

### Claude's Discretion
- Podman Pod topology (single pod vs separate containers, port mapping)
- MongoDB replica set initialization strategy
- Rocket.Chat admin bootstrap automation (REST API sequence)
- Data persistence model (ephemeral vs volumes for development use)
- Exact env var mapping between BotMinter credential store and Ralph's RC config fields
- Spike script location and structure

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Ralph `ralph-rocketchat` crate** (`/opt/workspace/ralph-orchestrator/crates/ralph-rocketchat/`): Full REST client, service, daemon, bot commands, message handler, state manager — 129 tests. This is the runtime that agents use to communicate via RC.
- **Ralph `RobotConfig`** (`ralph-core/src/config.rs`): Already supports `rocketchat` backend with `server_url`, `bot_user_id`, `auth_token`, `room_id`, `operator_id` fields. Mutually exclusive with `telegram`.
- **Telegram bridge** (`profiles/scrum-compact/bridges/telegram/`): Complete reference for bridge.yml + schema.json + Justfile structure — RC bridge follows same pattern but adds lifecycle recipes.
- **Bridge spec local bridge example** (`.planning/specs/bridge/bridge-spec.md`): Already shows a rocketchat bridge.yml example with lifecycle + identity + room commands.
- **`bridge.rs`**: Full bridge module with credential store, state management, command invocation — RC bridge uses the same infrastructure.
- **`workspace.rs:inject_robot_enabled()`**: Sets `RObot.enabled` in ralph.yml — needs extension for RC-specific config fields.

### Established Patterns
- Bridge commands invoked via `just --justfile {bridge_dir}/Justfile {recipe} {args}` with `BRIDGE_CONFIG_DIR` env var
- Config exchange via `$BRIDGE_CONFIG_DIR/config.json` — RC recipes must write this file
- `LocalCredentialStore` with keyring backend for per-member credential storage
- Idempotent provisioning: check state before creating (same pattern as GitHub repo provisioning)

### Integration Points
- `commands/start.rs:113-114`: TODO — hardcoded `RALPH_TELEGRAM_BOT_TOKEN` needs bridge-type-aware env var injection
- `commands/start.rs:290`: Where per-member token is passed as env var to Ralph
- `workspace.rs:532`: `inject_robot_enabled()` — needs to also inject `RObot.rocketchat.*` fields
- `commands/teams.rs:260`: Bridge discovery and provisioning during sync
- `profiles/scrum-compact/bridges/`: Where the new `rocketchat/` bridge directory will live
- `profile.rs`: `ProfileManifest.bridges` list needs `rocketchat` added to supported bridges

</code_context>

<specifics>
## Specific Ideas

- The spike is a standalone script that proves the full roundtrip (boot -> bot user -> Ralph sends -> human replies -> Ralph receives) before any BotMinter integration work begins
- If Ralph's existing `ralph-rocketchat` implementation doesn't work against real Rocket.Chat, that's a blocker — fix Ralph first, then resume this phase
- The RC E2E test becomes the **primary** operator journey test, with Telegram demoted to a minimal identity-only test

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 10-rocket-chat-bridge*
*Context gathered: 2026-03-10*
