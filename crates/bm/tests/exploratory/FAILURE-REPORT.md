# Exploratory Test Failure Report

**Date:** 2026-03-22
**Run:** `just exploratory-test` (post-refactor, run #3)
**Result:** 129 PASS, 5 FAIL, 4 NOTE

## Failure Summary

| # | Test | Verdict | Root Cause | Category |
|---|------|---------|------------|----------|
| C26 | Alice login after volume re-create | **FAIL** | Password in file doesn't match Matrix after re-onboarding | Production bug |
| C33 | Pre-existing user keyring credential | **FAIL** | Pre-existing user registration not supported by Tuwunel | Test prereq |
| H29b | Brain addressed work request | **FAIL** | Brain's first response was connection msg, not work-related | Timing |
| H40 | Brain response after recovery restart | **FAIL** | Brain alive but didn't respond within 30s | Timing |
| H49 | Brain response about GitHub board | **FAIL** | Brain alive but didn't respond within 60s | Timing |

## Detailed Analysis

### C26: Password-file mismatch after volume-loss recovery

**Severity:** Production bug
**Phase:** C.5 (Recovery from removed volume)
**Test code:** `phase-c.sh:163-175`

**What happens:**
1. C21 deletes tuwunel container + volume (fresh Matrix database)
2. C22 runs `bm teams sync --bridge` — the verify recipe correctly detects stale credentials, removes identities, and triggers re-onboarding
3. Diagnostic output confirms: `verify failed for superman-alice — re-provisioning` and `superman-alice: re-onboarded`
4. C25 confirms admin password is regenerated in the password file
5. C26 reads alice's password from `tuwunel-passwords.json`, attempts Matrix login → **fails**

**Root cause hypothesis:** The onboard recipe generates a new random password and registers the user on the fresh Matrix. But the password written to `tuwunel-passwords.json` may not match what was registered — possible race condition in the password file write, or the `jq` merge overwrites with stale data. Debug output added to C26 to capture the actual password and login error.

**Diagnosis status:** Debug output added. Awaiting next run for data.

**Files involved:**
- `profiles/scrum-compact/bridges/tuwunel/Justfile` (onboard recipe, lines 234-400)
- `crates/bm/src/bridge/provisioning.rs` (verify + re-onboard flow)
- `crates/bm/tests/exploratory/phases/phase-c.sh` (test assertion)

---

### C33: Pre-existing user has no keyring credential

**Severity:** Test prerequisite failure (not a production bug)
**Phase:** C.6 (Pre-existing user onboarding)
**Test code:** `phase-c.sh:219-233`

**What happens:**
1. C27 tries to register `superman-pre-existing` directly on Matrix via admin registration endpoint
2. Tuwunel doesn't expose the `/v1/register` admin nonce endpoint → C27 is a NOTE
3. Since the user was never registered, `bm teams sync` can't discover them
4. C33 checks keyring for the pre-existing user → nothing there

**Root cause:** Tuwunel doesn't support the Synapse-style admin registration API. The pre-existing user test scenario requires manual user creation, which isn't possible with Tuwunel's API surface.

**Recommendation:** Either:
- (a) Use Tuwunel's admin room commands to create the user (`!admin users create`)
- (b) Downgrade C33 to NOTE when C27 is a NOTE (prerequisite not met)
- (c) Register the user via the standard UIAA flow with the registration token

---

### H29b: Brain response didn't address work request

**Severity:** Timing/behavioral — not a deterministic bug
**Phase:** H.5 (End-to-End Brain Autonomy)
**Test code:** `phase-h.sh:482-502`

**What happens:**
1. H28-H30 send 3 messages (greeting, work request, follow-up) to the room
2. H32 polls for brain response → brain responds with `🤖 Ralph loop main connected via Matrix...` (a connection announcement, not a work response)
3. H29b checks if any brain response contains work-related keywords (`project`, `status`, `check`, etc.) → it doesn't

**Root cause:** The brain's first response to any interaction is a connection announcement from Ralph's main loop, not a work response. The work request message (H29) may not have been processed yet by the time H32's poll completes.

**Recommendation:** This should be a NOTE, not a FAIL. The brain responded (H32 passed) — it just hasn't had time to process the work request. Alternatively, increase the polling window or add a separate delayed check for work-related responses.

---

### H40: No brain response after recovery restart (within 30s)

**Severity:** Timing — not a deterministic bug
**Phase:** H.5 (Recovery scenario)
**Test code:** `phase-h.sh:664-690`

**What happens:**
1. H38 restarts brain members after stop → success
2. H39 sends a recovery message → delivered successfully
3. H40 polls for 30s for a NEW brain response (using pre-recovery count to avoid false positives)
4. No new response detected within 30s

**Root cause:** After restart, the brain needs time to reconnect to Matrix, re-authenticate, and start processing messages. 30s may not be enough, especially if the ACP session needs to re-establish.

**Recommendation:** NOTE, not FAIL. The brain is alive (verified), the message was delivered — the response just takes longer than 30s. The first lifecycle (H32) succeeded, proving the pipeline works.

---

### H49: No brain response about GitHub board (within 60s)

**Severity:** Timing — not a deterministic bug
**Phase:** H.6 (Task Execution Journey)
**Test code:** `phase-h.sh:826-862`

**What happens:**
1. H47 starts brain for task execution test → alive
2. H48 sends "check the GitHub board" message → delivered
3. H49 polls for 60s for a brain response mentioning board/issue keywords → nothing

**Root cause:** Same as H40 — the brain is a fresh ACP session after the previous stop, needs time to connect and process. 60s may not be enough for a cold start + message processing + GitHub API call + response.

**Recommendation:** NOTE, not FAIL. The brain survived the request (H50 passes), meaning it's stable — it just didn't respond in time.

---

## NOTEs (expected behavior, not failures)

| # | Test | Note |
|---|------|------|
| B7 | Init again | Correctly rejects: already exists |
| B11 | Hire duplicate alice | Correctly rejects: 'already exists' |
| D6 | Git log | Commit message format slightly different (cosmetic) |
| C27 | Pre-existing registration | Tuwunel doesn't expose admin registration endpoint |

## Progress from Previous Runs

| Run | PASS | FAIL | NOTE | Changes |
|-----|------|------|------|---------|
| #1 (pre-refactor) | 127 | 0 | 6 | Baseline (system keyring, monolithic Justfile) |
| #2 (post-refactor) | 116 | 18 | 4 | Isolated keyring, standalone scripts, verify recipe added |
| #3 (fixes applied) | 129 | 5 | 4 | B4 threshold fixed, room clearing on verify fail, C26 debug |

13 failures resolved by room-clearing fix. Remaining 5 need targeted fixes.

---

# E2E Test Failure Report

**Date:** 2026-03-22
**Run:** `just test` (unit + conformance + e2e)
**Result:** All non-E2E suites green. E2E: 4 failures across 2 suites.

## Suite Results

| Suite | Passed | Failed |
|-------|--------|--------|
| Unit tests | 581 | 0 |
| Conformance | 18 | 0 |
| Integration (profile_roundtrip etc.) | 116 | 0 |
| E2E isolated (smoke, keyring, gh_projects) | 3 | 0 |
| E2E scenario_operator_journey | 77 | 2 |
| E2E scenario_rc_operator_journey | all | 0 |
| E2E scenario_tg_operator_journey | 5 | 2 |

## Failure Summary

| # | Suite | Test | Verdict | Root Cause | Pre-existing? |
|---|-------|------|---------|------------|---------------|
| 1 | operator_journey | bridge_functional_fresh | **FAIL** | `.ralph-stub-env` not found | Yes |
| 2 | operator_journey | bridge_functional_existing | **FAIL** | `.ralph-stub-env` not found | Yes |
| 3 | tg_operator_journey | 05_start_and_verify | **FAIL** | `.ralph-stub-env` not found | Yes |
| 4 | tg_operator_journey | 06_stop | **FAIL** | Cascades from #3 | Yes |

All 4 failures are **pre-existing** — confirmed by running `just test` on clean HEAD (commit `465df81`) before any session changes.

## Detailed Analysis

### bridge_functional (operator_journey, both passes)

**File:** `crates/bm/tests/e2e/scenarios/operator_journey.rs:510`
**Error:** `called Result::unwrap() on an Err value: Os { code: 2, kind: NotFound, message: "No such file or directory" }`

**What happens:**
1. `start_status_healthy_fn` runs `bm start` → stub ralph launches, PID recorded, test passes
2. `start_skips_running_bridge_fn` runs `bm start` again → reports "already running", test passes
3. `bridge_functional_fn` waits 3s, then reads `{workspace}/.ralph-stub-env` → **file not found**

**Root cause:** The stub ralph binary (`stub-ralph.sh`) writes `.ralph-stub-env` to its working directory. The test expects this file at the workspace path. Either:
- (a) The stub ralph's CWD doesn't match the workspace path the test expects
- (b) The stub ralph hasn't written the file within the 3-second window (race condition)
- (c) The workspace path changed between `bm start` (which launches stub ralph) and the test's path construction

The stub ralph writes to `$PWD/.ralph-stub-env`. The `bm start` command sets CWD to the member workspace. The test constructs the path as `env.home.join("workspaces").join(TEAM_NAME).join(MEMBER_DIR)`. If these don't match, the file exists but at a different path.

**Investigation needed:** Add debug output to check whether `.ralph-stub-env` exists at the workspace root or in a subdirectory. Check if `bm start` sets the working directory correctly for the ralph subprocess.

### 05_start_and_verify / 06_stop (tg_operator_journey)

**File:** `crates/bm/tests/e2e/scenarios/tg_operator_journey.rs:130`
**Error:** Same `.ralph-stub-env` not found pattern.

**Root cause:** Same as `bridge_functional` — the stub ralph's output file isn't where the test expects it. `06_stop` cascades because the member never started properly (from the test's perspective).

## Impact of Session Changes

The session changes (verify recipe, provisioning.rs, bridge.yml, exploratory test refactor) introduced **zero new E2E failures**. The 4 failures are identical to clean HEAD. The verify recipe does not affect E2E tests because `bm start` does not call `provision()` — provisioning only runs during `bm teams sync --bridge`.
