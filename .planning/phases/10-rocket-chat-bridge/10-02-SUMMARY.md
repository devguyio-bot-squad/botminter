---
phase: 10-rocket-chat-bridge
plan: 02
subsystem: bridge
tags: [rocketchat, podman, e2e, bridge-lifecycle, operator-journey]

requires:
  - phase: 10-01
    provides: "RC Podman Pod spike — validated patterns for RC 8.2.0 lifecycle, identity, and room management"
  - phase: 09
    provides: "Bridge abstraction, credential store, bridge-type-aware sync/start/stop"
provides:
  - "Rocket.Chat bridge files in both scrum-compact and scrum profiles"
  - "Bridge-type-aware env var dispatch in bm start/daemon"
  - "RObot.rocketchat config injection during bm teams sync"
  - "Full RC operator journey E2E test (init -> hire -> bridge start -> identity -> room -> sync -> health -> stop)"
  - "RcPodGuard cleanup helper for E2E test panic safety"
affects: [bridge, profiles, e2e-tests]

tech-stack:
  added: [rocketchat-8.2.0, mongodb-7.0, podman-pod]
  patterns: [local-bridge-lifecycle, bridge-type-dispatch, real-dbus-passthrough-in-e2e]

key-files:
  created:
    - profiles/scrum-compact/bridges/rocketchat/bridge.yml
    - profiles/scrum-compact/bridges/rocketchat/schema.json
    - profiles/scrum-compact/bridges/rocketchat/Justfile
    - profiles/scrum/bridges/rocketchat/bridge.yml
    - profiles/scrum/bridges/rocketchat/schema.json
    - profiles/scrum/bridges/rocketchat/Justfile
    - crates/bm/tests/e2e/rocketchat.rs
    - crates/bm/tests/e2e/scenarios/rc_operator_journey.rs
  modified:
    - profiles/scrum-compact/botminter.yml
    - profiles/scrum/botminter.yml
    - crates/bm/src/commands/start.rs
    - crates/bm/src/commands/daemon.rs
    - crates/bm/src/workspace.rs
    - crates/bm/src/commands/teams.rs
    - crates/bm/tests/e2e/scenarios/mod.rs
    - crates/bm/tests/e2e/main.rs

key-decisions:
  - "Profile manifest bridges[] array is the bridge discovery mechanism, not filesystem scanning"
  - "Real D-Bus address saved before keyring isolation and passed to bridge commands that need Podman"
  - "BM_BRIDGE_TOKEN env var used during sync to bypass keyring in isolated E2E environment"

patterns-established:
  - "Local bridge pattern: Justfile manages Podman Pod lifecycle, bm invokes recipes"
  - "E2E D-Bus passthrough: save real D-Bus in setup, apply to commands needing Podman"
  - "Env var credential fallback in E2E tests for reliable credential resolution"

requirements-completed: [RC-01, RC-02, RC-04, RC-05, RC-07]

duration: 18min
completed: 2026-03-10
---

# Phase 10 Plan 02: Ship RC Bridge and E2E Operator Journey Summary

**Rocket.Chat bridge with Podman Pod lifecycle, per-agent bot identity, RObot config injection, and full E2E operator journey passing against real RC 8.2.0 + MongoDB 7.0**

## Performance

- **Duration:** 18 min
- **Started:** 2026-03-10T20:41:39Z
- **Completed:** 2026-03-10T21:00:00Z
- **Tasks:** 3 (2 from previous executor + 1 fix task)
- **Files modified:** 16

## Accomplishments
- RC bridge ships in both profiles with all 8 Justfile recipes (start, stop, health, onboard, rotate, remove, room-create, room-list)
- Bridge-type-aware env var dispatch: `bm start` passes RALPH_ROCKETCHAT_AUTH_TOKEN + RALPH_ROCKETCHAT_SERVER_URL for RC bridges
- RObot.rocketchat config injection during `bm teams sync --bridge` writes bot_user_id, room_id, server_url to ralph.yml
- Full RC E2E operator journey passes: init -> hire -> bridge start (boots RC + MongoDB Podman Pod) -> identity add -> room create -> sync (verifies ralph.yml) -> health check -> bridge stop
- Existing Telegram operator journey continues to pass (54/54 cases)

## Task Commits

Each task was committed atomically:

1. **Task 1: Ship RC bridge files and make Rust code bridge-type-aware** - `6d94297` (feat)
2. **Task 2: Create RcPodGuard and RC operator journey E2E scenario** - `7c1b7bd` (feat)
3. **Task 3: Fix bridge discovery and E2E test failures** - `a11b7b1` (fix)

## Files Created/Modified
- `profiles/scrum-compact/bridges/rocketchat/bridge.yml` - RC bridge manifest (type: local, lifecycle + identity + room)
- `profiles/scrum-compact/bridges/rocketchat/schema.json` - RC config schema with operator_id
- `profiles/scrum-compact/bridges/rocketchat/Justfile` - All 8 bridge recipes ported from spike.sh
- `profiles/scrum/bridges/rocketchat/*` - Same files for scrum profile
- `profiles/scrum-compact/botminter.yml` - Added rocketchat to bridges[] array
- `profiles/scrum/botminter.yml` - Added rocketchat to bridges[] array
- `crates/bm/src/commands/start.rs` - Bridge-type-aware launch_ralph with RC env vars
- `crates/bm/src/commands/daemon.rs` - Bridge-type-aware launch_ralph_oneshot
- `crates/bm/src/workspace.rs` - inject_robot_config with RObot.rocketchat fields
- `crates/bm/src/commands/teams.rs` - Pass bridge config to inject_robot_config
- `crates/bm/tests/e2e/rocketchat.rs` - RcPodGuard cleanup helper
- `crates/bm/tests/e2e/scenarios/rc_operator_journey.rs` - Full RC E2E scenario (9 cases)
- `crates/bm/tests/e2e/scenarios/mod.rs` - Registered RC scenario
- `crates/bm/tests/e2e/main.rs` - Added rocketchat module

## Decisions Made
- Bridge discovery uses the `bridges[]` array in botminter.yml, not filesystem scanning. The previous executor created the bridge files but forgot to register them in the manifest.
- E2E tests save the real DBUS_SESSION_BUS_ADDRESS before KeyringGuard isolation and pass it to bridge commands that invoke Podman (which needs systemd D-Bus for cgroup management).
- BM_BRIDGE_TOKEN env var set during sync step to bypass keyring access issues in the isolated test environment.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Missing rocketchat entry in profile manifest bridges[] array**
- **Found during:** Task 3 (E2E verification)
- **Issue:** `bm init --bridge rocketchat` failed with "Bridge 'rocketchat' not found in profile" because the bridge was not registered in botminter.yml despite files existing at profiles/*/bridges/rocketchat/
- **Fix:** Added rocketchat BridgeDef to both scrum-compact and scrum botminter.yml
- **Files modified:** profiles/scrum-compact/botminter.yml, profiles/scrum/botminter.yml
- **Verification:** `just e2e` -- init step passes, all 9 RC journey cases pass
- **Committed in:** a11b7b1

**2. [Rule 1 - Bug] E2E D-Bus isolation conflict with Podman cgroup management**
- **Found during:** Task 3 (E2E verification)
- **Issue:** KeyringGuard replaces DBUS_SESSION_BUS_ADDRESS with an isolated daemon, but Podman needs the real systemd D-Bus to create pod cgroups. `podman pod create` failed with "unable to create pod cgroup: Process org.freedesktop.systemd1 exited with status 1"
- **Fix:** Save real D-Bus address during setup (before isolation), apply to bridge commands via apply_real_dbus_env helper
- **Files modified:** crates/bm/tests/e2e/scenarios/rc_operator_journey.rs
- **Verification:** `just e2e` -- bridge start succeeds, pod creates and boots RC + MongoDB
- **Committed in:** a11b7b1

**3. [Rule 1 - Bug] Credential resolution fails during sync with real D-Bus**
- **Found during:** Task 3 (E2E verification)
- **Issue:** RObot.enabled was false after sync because credential stored in isolated keyring was not accessible when sync ran with real D-Bus address
- **Fix:** Set BM_BRIDGE_TOKEN_SUPERMAN_BOT_ALICE env var on sync command for reliable credential resolution via env var path
- **Files modified:** crates/bm/tests/e2e/scenarios/rc_operator_journey.rs
- **Verification:** `just e2e` -- sync step passes, RObot.enabled = true, RObot.rocketchat fields present
- **Committed in:** a11b7b1

---

**Total deviations:** 3 auto-fixed (3 bugs)
**Impact on plan:** All fixes were necessary for the E2E test to pass. The root cause (missing manifest entry) was a gap from the previous executor. The D-Bus and credential fixes are test infrastructure issues specific to the E2E isolation model.

## Issues Encountered
- Pre-existing flaky test: `daemon_start_poll_existing` in the operator journey failed intermittently on one run but passed on subsequent runs. Not related to RC changes. Not fixed (pre-existing).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- RC bridge is complete and proven via E2E
- All 5 requirements (RC-01, RC-02, RC-04, RC-05, RC-07) exercised in E2E
- Phase 10 is the final phase in the v0.07 milestone
- Ready for milestone release

---
*Phase: 10-rocket-chat-bridge*
*Completed: 2026-03-10*
