# Exploratory Test Plan: Sync, Bridge & Lima Idempotency

**Date:** 2026-03-20
**Build:** bm 0.2.0-pre-alpha (local debug, post-RemoteRepoOps + Lima --overwrite)
**Environment:** Linux x86_64, podman rootless, limactl 2.1.0, gh authenticated (devguyio)
**Org:** devguyio-bot-squad

## Scope

Four features under test:

1. **Lima boot script idempotency** — `--overwrite` flag on `dnf config-manager addrepo`, full VM boot cycle
2. **Workspace creation idempotency** — `RemoteRepoOps` trait, stale dir cleanup, context file assembly
3. **Bridge provisioning idempotency** — Tuwunel onboard/start lifecycle, recovery from container failures
4. **Full sync (`-a`) idempotency** — all subsystems together, repeated runs, member additions

## Prerequisites

- Port 8008 free (Tuwunel default)
- Keyring unlocked (`secret-tool store/lookup` works)
- `gh` authenticated with delete permission on devguyio-bot-squad
- `podman`, `just`, `limactl`, `curl`, `jq` available
- No existing `exploratory-test` team state

---

## Phase A: Lima VM Boot Script Idempotency

Tests that the generated Lima template's provision scripts survive multiple VM boots.

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| A0 | Set up team for bm runtime create | `bm init --non-interactive` | Team created (required for runtime create) |
| A0.5 | Verify template has `--overwrite` | `bm runtime create --render` + grep | `addrepo --overwrite` present |
| A1 | Create VM with `bm runtime create` | `bm runtime create --non-interactive --name lima-idem-test` | VM created, provisioned, readiness probe passes |
| A2 | Verify tools installed inside VM | `limactl shell lima-idem-test -- which bm ralph claude gh git just` | All tools found |
| A3 | Stop VM | `limactl stop lima-idem-test` | VM stopped cleanly |
| A4 | Start VM again (re-runs provision scripts) | `limactl start lima-idem-test` | VM starts without errors — boot scripts idempotent |
| A5 | Verify tools still present after restart | Same as A2 | All tools still found |
| A6 | Verify gh auth survives restart (if token provided) | `limactl shell -- gh auth status` | Auth intact |
| A7 | Third boot cycle | Stop + start again | Still succeeds — no cumulative drift |
| A8 | Delete VM | `limactl delete --force lima-idem-test` | Cleaned up |
| A9 | Clean up Phase A team artifacts | Delete GitHub repo + project + local state | Clean state for Phase B |

## Phase B: Team Init + Hire

Foundation setup for subsequent phases.

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| B1 | Fresh init with tuwunel bridge | `bm init --non-interactive --profile scrum-compact --team-name exploratory-test --org devguyio-bot-squad --repo exploratory-test-team --bridge tuwunel --github-project-board "Exploratory Test Board"` | Team created, labels/project bootstrapped |
| B2 | Verify GitHub repo exists and is private | `gh repo view devguyio-bot-squad/exploratory-test-team --json name,visibility` | `{"name":"exploratory-test-team","visibility":"PRIVATE"}` |
| B3 | Verify GitHub project board created | `gh project list --owner devguyio-bot-squad` | "Exploratory Test Board" present |
| B4 | Verify labels bootstrapped | `gh label list -R devguyio-bot-squad/exploratory-test-team` | Status labels from scrum-compact profile |
| B5 | Verify team config registered | `cat ~/.botminter/config.yml` | Team entry with github_repo, bridge, project_number |
| B6 | Verify team repo has profile content | `ls team/members/ team/knowledge/ team/PROCESS.md` | Profile skeleton files present |
| B7 | Init again (idempotency) | Same command as B1 | Expected: error (init is intentionally not idempotent — "directory exists") |
| B8 | Hire first member (alice) | `bm hire superman --name alice` | Member dir with PROMPT.md, CLAUDE.md, ralph.yml |
| B9 | Hire second member (bob) | `bm hire superman --name bob` | Second member dir created |
| B10 | Verify member config files | `ls team/members/superman-alice/` | PROMPT.md, CLAUDE.md, ralph.yml, coding-agent/ |
| B11 | Hire duplicate (alice again) | `bm hire superman --name alice` | Expected: error "already exists" (hire not idempotent) |

## Phase C: Bridge Lifecycle (Tuwunel)

### C.1: First Bridge Provisioning

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| C1 | First sync --bridge | `bm teams sync --bridge -v` | Container created, admin registered, alice+bob onboarded, room created |
| C2 | Verify container running | `podman ps --filter name=bm-tuwunel-exploratory-test` | Status "Up" |
| C3 | Verify Matrix server healthy | `curl -sf http://127.0.0.1:8008/_matrix/client/versions` | HTTP 200, version list |
| C4 | Verify bridge state file | `jq '{status, identities: (.identities\|keys), rooms}' bridge-state.json` | status=running, 3 identities (bmadmin, alice, bob), 1 room |
| C5 | Verify passwords persisted | `jq 'keys' tuwunel-passwords.json` | [bmadmin, superman-alice, superman-bob] |
| C6 | Verify keyring credentials stored | `secret-tool lookup service botminter-bridge user superman-alice` | Non-empty token |
| C7 | Verify admin can login to Matrix | `curl` Matrix login API with admin creds | access_token returned |
| C8 | Verify room exists | `curl` Matrix room alias API for exploratory-test-general | room_id returned |

### C.2: Bridge Idempotency (Repeated Sync)

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| C9 | Sync --bridge again (no changes) | `bm teams sync --bridge -v` | AlreadyProvisioned for all, no errors |
| C10 | Verify no duplicate identities | `jq '.identities\|length' bridge-state.json` | Still 3 |
| C11 | Verify no duplicate rooms | `jq '.rooms\|length' bridge-state.json` | Still 1 |
| C12 | Verify keyring credentials unchanged | Lookup alice token, compare to C6 | Same token |
| C13 | Third sync --bridge | `bm teams sync --bridge -v` | Still idempotent |

### C.3: Bridge Recovery — Container Stopped

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| C14 | Stop container externally | `podman stop bm-tuwunel-exploratory-test` | Container stopped |
| C15 | Verify bridge state still says "running" | `jq '.status' bridge-state.json` | "running" (stale state) |
| C16 | Matrix server unreachable | `curl http://127.0.0.1:8008/_matrix/client/versions` | Connection refused |
| C17 | Sync --bridge recovers | `bm teams sync --bridge -v` | Bridge auto-restarts or re-provisions |
| C18 | Verify container running again | `podman ps` | "Up" |
| C19 | Verify Matrix server healthy | `curl` versions endpoint | HTTP 200 |
| C20 | Verify identities intact | `jq '.identities\|length' bridge-state.json` | 3 |

### C.4: Bridge Recovery — Container Removed

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| C21 | Force-remove container | `podman rm -f bm-tuwunel-exploratory-test` | Container gone |
| C22 | Sync --bridge recreates from scratch | `bm teams sync --bridge -v` | New container, admin re-registered, members re-onboarded |
| C23 | Verify container running | `podman ps` | New container "Up" |
| C24 | Verify all identities re-provisioned | `jq '.identities\|keys' bridge-state.json` | All 3 present |
| C25 | Verify room re-created or recovered | `jq '.rooms' bridge-state.json` | Room present |
| C26 | Verify keyring credentials valid | `secret-tool lookup` for alice | Non-empty, can login to Matrix |

### C.5: Bridge Recovery — Volume Removed (Full Reset)

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| C27 | Remove container + volume | `podman rm -f ... && podman volume rm ...` | All state gone |
| C28 | Remove bridge-state.json | `rm bridge-state.json` | — |
| C29 | Sync --bridge from scratch | `bm teams sync --bridge -v` | Complete re-creation works |
| C30 | Verify everything functional | Container up, Matrix healthy, all identities, room exists | Full recovery |

### C.6: Onboard Edge Cases

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| C31 | Pre-register user via Matrix API | `curl` register endpoint for "pre-existing" user | User created outside bm |
| C32 | Hire + sync onboards pre-existing user | `bm hire superman --name pre-existing && bm teams sync --bridge -v` | M_USER_IN_USE handled, password reset via admin API |
| C33 | Verify pre-existing user has valid credentials | `secret-tool lookup` + Matrix login | Login succeeds |

## Phase D: Workspace Sync Idempotency (Local Mode)

Note: workspaces were already created in C1 (sync --bridge also creates workspaces).

### D.1: Verify Initial Workspace State

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| D1 | Context files present (alice) | `ls` alice workspace root | ralph.yml, CLAUDE.md, PROMPT.md, .botminter.workspace |
| D2 | Context files present (bob) | `ls` bob workspace root | Same files |
| D3 | Team submodule present | `ls workspace/team/members/` | Team repo content visible |
| D4 | Agent dir assembled | `ls workspace/.claude/agents/` | Symlinks into team/ submodule |
| D5 | Git repo clean | `git -C workspace status --porcelain` | Empty (clean tree) |
| D6 | Git has initial commit | `git -C workspace log --oneline -1` | "Initial workspace setup" |

### D.2: Workspace Sync Idempotency

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| D7 | Sync again (no changes) | `bm teams sync -v` | Workspaces synced, no errors |
| D8 | Context files still present | `ls` | All files intact |
| D9 | Sync third time | `bm teams sync -v` | Still clean |

### D.3: Stale Workspace Recovery

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| D10 | Remove .botminter.workspace marker | `rm` marker from alice workspace | — |
| D11 | Sync recovers stale workspace | `bm teams sync -v` | Workspace dir cleaned + recreated, marker restored |
| D12 | All context files restored | `ls` | ralph.yml, CLAUDE.md, PROMPT.md, marker all present |
| D13 | Team submodule intact | `ls workspace/team/` | Content visible |

### D.4: Missing Context File Recovery

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| D14 | Delete CLAUDE.md from workspace | `rm CLAUDE.md` from bob workspace | — |
| D15 | Sync restores CLAUDE.md | `bm teams sync -v` | CLAUDE.md restored (sync re-assembles context) |
| D16 | Delete ralph.yml from workspace | `rm ralph.yml` | — |
| D17 | Sync restores ralph.yml | `bm teams sync -v` | ralph.yml restored |

### D.5: Junk Directory Cleanup

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| D18 | Create dir at member path with junk, no marker | `mkdir + echo junk` at future member path | — |
| D19 | Hire new member (carol) | `bm hire superman --name carol` | Member dir created in team repo |
| D20 | Sync cleans junk + creates proper workspace | `bm teams sync -v` | Junk gone, proper workspace with all context files |

### D.6: Settings.json & Inbox

**User Journeys:**
1. Fresh workspace setup: sync → settings.json surfaced → hook enabled (D21)
2. Inbox full lifecycle: write → peek → read (consume) → peek (empty) (D22)
3. Graceful degradation: hook in workspace with no messages, hook outside workspace (D23, D23b)
4. Sync resilience: write message → re-sync → message survives (D24)

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| D21 | Settings.json surfaced after sync | `cat <workspace>/.claude/settings.json` | Valid JSON with PostToolUse hook referencing `bm-agent claude hook post-tool-use` |
| D22 | Inbox write/peek/read lifecycle | `bm-agent inbox write "test message"` then `bm-agent inbox peek` then `bm-agent inbox read --format json` then `bm-agent inbox peek` | Message visible after write, JSON output on read, empty after consume |
| D23 | Hook graceful degradation (in workspace) | `bm-agent claude hook post-tool-use` in workspace with no pending messages | Exit 0 with no output |
| D23b | Hook graceful degradation (outside workspace) | `bm-agent claude hook post-tool-use` in non-workspace dir | Exit 0 with no output |
| D24 | Re-sync preserves inbox messages | Write message, `bm teams sync -v`, `bm-agent inbox peek` | Message still present after sync |

## Phase E: Full Sync (`--bridge` flag)

Note: `-a` includes `--repos` which requires GitHub workspace repos per member.
For local-only teams (no per-member GitHub repos), use `--bridge` instead.

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| E1 | Full sync with bridge + workspace | `bm teams sync --bridge -v` | Bridge + workspace all succeed |
| E2 | Full sync again (idempotent) | `bm teams sync --bridge -v` | No errors, everything up to date |
| E3 | Hire fourth member, full sync | Hire dave, `bm teams sync --bridge -v` | New workspace, dave onboarded, others untouched |
| E4 | Verify all 4 members have workspaces | `ls` workzone | 4 workspace dirs, all with context files |
| E5 | Verify all 4 in bridge state | `jq '.identities\|keys' bridge-state.json` | 5 identities (admin + 4 members) |

## Phase F: Error Handling & Edge Cases

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| F1 | Sync --bridge without `just` in PATH | `PATH=/usr/bin bm teams sync --bridge` | BridgeAutoStartSkipped, graceful message |
| F2 | bm status shows bridge info | `bm status -v` | Bridge status, member count, container info |
| F3 | bm members list | `bm members list` | All hired members shown |
| F4 | bm teams show | `bm teams show` | Team details with bridge, members, projects |

## Phase G: Cleanup

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| G1 | Stop + remove bridge container | `podman stop/rm bm-tuwunel-exploratory-test` | Container cleaned |
| G2 | Remove bridge volume | `podman volume rm bm-tuwunel-exploratory-test-data` | Volume removed |
| G3 | Delete GitHub repo | `gh repo delete devguyio-bot-squad/exploratory-test-team --yes` | Repo deleted |
| G4 | Delete GitHub project | `gh project delete <number>` | Board deleted |
| G5 | Remove local state | `rm -rf ~/.botminter ~/.config/botminter` | Clean state |
| G6 | Verify clean | No leftover containers, repos, or config | Everything gone |
| G7 | Delete Lima test VM (if created) | `limactl delete --force lima-idem-test` | VM removed |

## Phase H: Brain Lifecycle Validation (Chat-First Member)

End-to-end tests for the brain-mode feature. Tests simulate real user journeys:
`bm start` → chat via Matrix → `bm stop`. No internal commands or file injection.

**Prerequisites:** Phases B-E must run first (team init + hire + bridge + workspace sync).

### H.1: Template Rendering & Content

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| H1 | Brain prompt exists after sync | Check `brain-prompt.md` in alice workspace | File exists, non-empty |
| H2 | No unrendered template vars | `grep '{{' brain-prompt.md` | Zero matches — all 5 vars rendered |
| H3 | Contains rendered member name | `grep -q 'alice' brain-prompt.md` | Member name present in rendered output |
| H4 | Contains rendered team name | `grep -q 'exploratory-test' brain-prompt.md` | Team name present |
| H5 | Contains rendered GitHub org | `grep -q 'devguyio-bot-squad' brain-prompt.md` | Org name present |
| H6 | Contains rendered GitHub repo | `grep -q 'exploratory-test-team' brain-prompt.md` | Repo name present |
| H7 | Contains expected sections | `grep` for Identity, Board Awareness, Work Loop, Human Interaction, Dual-Channel | All major sections present |

### H.2: Per-Member Differentiation

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| H8 | Bob also has brain-prompt.md | Check file exists in bob workspace | File present |
| H9 | Alice and bob differ | `diff` alice vs bob brain-prompt.md | Files differ (different member names) |
| H10 | Bob contains bob's name | `grep -q 'bob' bob/brain-prompt.md` | bob's name present, not alice |

### H.3: Brain Mode Detection

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| H11 | bm start detects brain mode | Run `bm start`, check output for brain references | Output mentions "brain" for members with brain-prompt.md |
| H12 | State file has brain_mode=true | After start, inspect `state.json` for `brain_mode` | `brain_mode: true` present |
| H13 | Remove brain-prompt.md disables brain | Remove file, run `bm start`, check state | No `brain_mode: true` or falls back to ralph |
| H14 | Restore brain-prompt.md and stop | Restore file via re-sync, stop all | Clean state after stop |

### H.4: Sync Edge Cases

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| H15 | Modified brain-prompt.md restored | Overwrite with junk, re-sync | Original content restored from template |
| H16 | Deleted brain-prompt.md restored | Delete file, re-sync | File recreated from template |
| H17 | Content idempotent across syncs | Sync twice, diff results | Identical content after both syncs |
| H18 | Verbose sync shows BrainPromptSurfaced | Run `bm teams sync -v` | Output contains brain prompt surfacing message |

### H.5: End-to-End Brain Autonomy Validation

The core autonomy validation: tests the complete user journey of starting brain-mode
members, chatting with them via the tuwunel Matrix bridge, and stopping them cleanly.
Messages are sent via the Matrix API (simulating a real human user) while brain members
are running. The test polls for brain responses to prove autonomous behavior.

**Integrated journeys covered** (per `exploratory-test-user-journey.md` and `exploratory-test-single-journey-smell.md`):

1. **Interactive session** (H25-H37): bm start → greeting → work request → follow-up (multi-turn) →
   malformed input (error handling) → cross-member messaging (alice sends, bob sees) → brain survives
   all interaction → bm stop. This single-session journey proves the brain handles diverse interaction
   patterns within one lifecycle, reflecting how a real user interacts during a work session.
   - H26 validates the brain process IS `bm brain-run` / `claude-code-acp-rs` (not just any PID)
   - H32 validates the brain response content is meaningful (operational indicators checked)
   - H29b validates the brain response addressed the work request (not just generic chat)
2. **Recovery** (H38-H41): bm stop → bm start → send message → poll for NEW brain response → bm stop.
   Independent lifecycle that proves the system recovers and the brain responds after restart.
   Uses pre-recovery baseline count to prevent false positives from stale responses.
3. **Task execution** (H46-H51): Create GitHub issue → bm start → ask brain to check board →
   poll for brain response mentioning the issue/board → verify brain survived → bm stop → close issue.
   This proves the brain does meaningful work — not just responds to chat, but engages with the
   project board, which is the core value proposition of brain-mode members.

Journey categories spanned: interaction variations (single question, multi-turn, cross-member),
lifecycle variations (start/stop, restart), error handling (malformed input), recovery (restart + response),
task execution (board scanning, issue discovery).
Each journey crosses all subsystems: CLI (start/stop), bridge (Matrix), brain (response polling), GitHub (board).

**Prerequisites:** Phases B-E must run first (team init + hire + bridge + workspace sync).

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| H19 | Bridge is running | `curl` Matrix versions endpoint | HTTP 200 (bridge auto-recovers if down) |
| H20 | ACP binary available | `which claude-code-acp-rs` | Binary found in PATH |
| H21 | Admin Matrix login | `curl` login API with admin creds | Access token returned |
| H22 | Alice Matrix login | `curl` login API with alice creds | Access token returned |
| H23 | Room resolution | `curl` room alias API | Room ID returned for team general room |
| H24 | Clean state before lifecycle | `bm stop --force`, rm state.json | Clean slate |
| H25 | Start brain members | `bm start` | Brain mode detected in output |
| H26 | Brain process verified | Check PID from state.json + validate process command is brain-run/acp | Process running AND command contains brain-run or claude-code-acp-rs |
| H27 | Status shows brain label | `bm status` | "brain" label shown during lifecycle |
| H28 | Send greeting while brain running | `curl` PUT room/send as admin | Message delivered to room with brain alive |
| H29 | Send work request while brain running | `curl` PUT room/send as admin | Message delivered to room |
| H30 | Send follow-up question | `curl` PUT room/send as admin | Multi-turn conversation simulated |
| H31 | Edge case: malformed message | Send empty-body + unicode garbage via Matrix while brain running | Brain process survives (no crash) |
| H32 | Poll for brain response (autonomy proof) | Poll room for messages from brain identity (30s), validate content is meaningful | Brain responds with operational content (or NOTE if pipeline not wired) |
| H29b | Work request response check | Check brain responses for work-request-related content (project, status, tools) | Brain response addresses the work request (or NOTE) |
| H33 | User messages visible in history | `curl` GET room/messages | All user messages visible |
| H34 | Cross-member while brain alive | Alice sends message while brain running, bob verifies | Bob sees alice's message with brain process active |
| H35 | Brain survived all interaction | Check brain PID still alive after normal + malformed + cross-member messages | Process stable during chat |
| H36 | Stop brain member | `bm stop` | Clean exit |
| H37 | Processes terminated | Check all PIDs dead | No leftover processes |
| H38 | Recovery: restart brain | `bm start` after stop | Brain restarts successfully |
| H39 | Recovery: send message | Send message via Matrix after brain restart | Message delivered (recovery proof) |
| H40 | Recovery: poll for NEW response | Poll room for brain response after restart (30s), verify count increased vs pre-recovery baseline | NEW brain response detected after recovery (or NOTE) |
| H41 | Recovery: stop cycle | `bm stop` again | Lifecycle idempotent |
| H42 | Status inquiry after lifecycle | Send message to room post-lifecycle | Message sent successfully |
| H43 | Message persistence (incl. recovery + cross-member) | Poll room history for all messages | All messages persist including recovery and cross-member |
| H44 | Multi-member visibility | Login as bob, poll room | Bob sees all messages |

### H.6: Task Execution Journey

The most important journey — validates the brain does meaningful work, not just chat.
Creates a real GitHub issue, starts the brain, asks it to check the board, and validates
the brain acknowledges the issue/board in its response.

| # | Scenario | Method | Expected |
|---|----------|--------|----------|
| H46 | Create GitHub issue for brain | `gh issue create` on team repo | Issue created successfully |
| H47 | Start brain for task execution | `bm start` + verify brain PID alive | Brain running (or NOTE if ACP auth fails) |
| H48 | Ask brain to check board | Send Matrix message requesting board check mentioning the issue | Message delivered |
| H49 | Poll for board-aware response | Poll room for brain response mentioning board/issue/dependency (60s) | Brain acknowledges board/issue (or NOTE) |
| H50 | Brain survived task execution | Check brain PID still alive | Process stable after task request |
| H51 | Task journey cleanup | `bm stop`, close GitHub issue | Clean state |
| H52 | Final cleanup | Stop all, rm state | All artifacts removed |
