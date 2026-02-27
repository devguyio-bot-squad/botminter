# Design Review Round 2 — `bm` CLI (Milestone 3)

> Review of [design.md](design.md) after all Round 1 resolutions were incorporated.
> Three parallel reviewers with focused scopes: Architecture & Data Models, Command Surface & UX, Acceptance Criteria & Testing.
> Reviewer: Claude Code (PDD session, 3-agent parallel review)
> Date: 2026-02-20

---

## Overall Assessment

The design is **comprehensive and well-structured** — 1,276 lines covering 12 commands, 5 data models, a full workzone layout, and embedded profile model. Round 1 gaps (G1–G7, C1–C5) were properly resolved and incorporated. The architecture is sound, the scope is disciplined, and the acceptance criteria cover the primary happy paths.

However, this round found **one critical inconsistency** (workspace path convention), **4 high-priority gaps**, and **18 medium-priority items** across all three review domains. The critical finding has architectural implications and must be resolved before implementation.

Findings are organized by severity, then by domain. Duplicate findings discovered independently by multiple reviewers are merged with cross-references.

---

## 🔴 Critical (1)

### CRIT-01: Workspace layout breaks all `.botminter/` path references in member skeletons

**Domain:** Architecture
**Section(s):** 7 (Workzone Layout), 6.4 (teams sync), existing member skeletons

**Finding:** The new workzone layout (Section 7.1) places the team repo as a **sibling** to member workspaces:

```
~/.botminter/workspaces/hypershift/
    team/                          # team repo
    architect-bob/hypershift/      # member workspace
```

Symlinks in Section 6.4 confirm: `PROMPT.md → ../../team/team/architect-bob/PROMPT.md`.

But every existing member skeleton hardcodes `.botminter/` as the team repo path **inside** the workspace:

- `ralph.yml` skill paths: `.botminter/agent/skills/...`
- Hat instructions: `.botminter/knowledge/`, `.botminter/team/architect/knowledge/`
- `CLAUDE.md` access tables: `.botminter/...` prefix throughout
- Board scanner: `TEAM_REPO=$(cd .botminter && gh repo view ...)`

Under the new layout, none of these paths resolve. The team repo is no longer at `.botminter/` inside the workspace — it's a sibling directory two levels up.

**Impact:** Every member skeleton's runtime content (hats, skills, knowledge paths) would be broken on first launch. This is a **functional blocker**.

**Options:**
- **(a) Preserve `.botminter/` convention:** During `bm teams sync`, clone or symlink the team repo into each project workspace at `.botminter/`. This is minimally disruptive — all existing skeleton content works unchanged.
- **(b) Migrate skeleton content:** Update all path references in every member skeleton (ralph.yml, CLAUDE.md, hat instructions, PROCESS.md) to use the new relative path scheme. This is a significant scope expansion touching many files across all profiles.

**Recommendation:** Option (a) for M3. It preserves backward compatibility and keeps skeleton migration out of scope.

---

## 🟠 High Priority (4)

### HIGH-01: Missing schema version guard ACs for 4+ commands

**Domain:** Acceptance Criteria
**Section(s):** 2.3 (Invariants), 6.4, 6.5, 10.3, 10.4, 10.5, 10.6

**Finding:** Section 2.3 states: "Any command that reads or writes team repo content (`hire`, `teams sync`, `start`, `stop`, `status`) checks the team's `botminter.yml` `schema_version`." Section 10.2 has an excellent AC for this in `bm hire`. But `bm teams sync`, `bm start`, `bm stop`, and `bm status` have no schema mismatch ACs despite the invariant.

Secondary inconsistency: `bm stop` and `bm status` are listed in the invariant but their command specs (Sections 6.6, 6.7) don't mention schema guards. Clarify whether they actually check.

**Recommendation:** Add schema mismatch ACs for at least `bm teams sync` and `bm start` (which have explicit guard language in their specs). Resolve whether `bm stop` and `bm status` also check.

### HIGH-02: No AC for `-t`/`--team` flag resolution

**Domain:** Acceptance Criteria
**Section(s):** 6.1, 10.1–10.9

**Finding:** The `-t`/`--team` flag and default team resolution is the most commonly used cross-cutting behavior — it affects 8 commands. No AC tests either the `-t` override or the "no default team and no flag" error path.

**Recommendation:** Add ACs:
1. Given two teams with one default, `bm members list -t <non-default>` shows the non-default team.
2. Given two teams with no default, `bm members list` (no `-t`) errors with a hint.

### HIGH-03: No integration test for schema version mismatch path

**Domain:** Testing Strategy
**Section(s):** 11.2

**Finding:** The schema version guard is a core invariant protecting 5+ commands, yet no integration test exercises the mismatch path end-to-end.

**Recommendation:** Add: "Schema version mismatch: Create team with v1, modify botminter.yml to v2, verify `bm hire`/`bm start`/`bm teams sync` all refuse with upgrade suggestion."

### HIGH-04: Missing `serde_json` dependency

**Domain:** Architecture
**Section(s):** 3.3 (Dependencies), 5.2 (Runtime State)

**Finding:** `state.json` is defined as JSON with `#[derive(Serialize, Deserialize)]` structs, but the dependency list only includes `serde_yml`. Without `serde_json`, `state.json` cannot be read or written.

**Recommendation:** Add `serde_json = "1"` to the dependency list in Section 3.3.

---

## 🟡 Medium Priority (18)

### Data Model Issues

#### MED-01: `bm roles list` description field has no data source

**Domain:** Architecture + UX (found independently by both reviewers)
**Section(s):** 5.4 (Member Manifest), 6.10 (roles list), 8.2 (Profile Operations)

**Finding:** Section 6.10 shows `bm roles list` with a `Description` column. But:
- The `MemberManifest` struct (Section 5.4) has only `name` and `role` — no `description`.
- Member skeletons in the embedded profile don't contain `botminter.yml` (it's generated by `bm hire`).
- Section 8.2 lists roles by subdirectory names, with no description extraction mechanism.

**Recommendation:** Either add a `description` field to a role manifest, add a `roles:` section to the profile's `botminter.yml`, or drop the description column.

#### MED-02: `.botminter.yml` → `botminter.yml` filename migration unaddressed

**Domain:** Architecture
**Section(s):** 3.1 (Skeleton Collapse), 5.4, 8.3

**Finding:** Existing member skeletons use `.botminter.yml` (dotfile). The design consistently says `botminter.yml` (no dot). The skeleton collapse section does not mention this rename. Existing files also have different fields (`role`, `comment_emoji`) vs. the design's `MemberManifest` (`name`, `role`).

**Recommendation:** Add a migration note to Section 3.1 specifying that `.botminter.yml` is renamed and restructured. Decide what happens to `comment_emoji` (see MED-03).

#### MED-03: `comment_emoji` dropped from data models

**Domain:** Architecture
**Section(s):** 5.4, 5.3

**Finding:** Existing `.botminter.yml` files contain `comment_emoji` (e.g., `"🏗️"` for architect). This emoji is used in hat instructions for the GitHub comment format (`### 🏗️ architect — <ISO-timestamp>`). Neither `MemberManifest` nor `ProfileManifest` includes this field.

**Recommendation:** Add `comment_emoji` to `MemberManifest` or to a role definition in the profile manifest.

#### MED-04: `TeamEntry.path` ambiguity

**Domain:** Architecture
**Section(s):** 5.1, 6.2, 7.1

**Finding:** Config stores `path: ~/.botminter/workspaces/hypershift` — the team *directory*. But commands need the team *repo* at `{path}/team/`. The `TeamEntry` struct doesn't clarify which it points to, and different commands could make different assumptions.

**Recommendation:** Document explicitly. Add a convention note or helper method: `fn team_repo_path(&self) -> PathBuf { self.path.join("team") }`.

### Command & UX Issues

#### MED-05: Credential collection missing from `bm init` wizard diagram

**Domain:** UX
**Section(s):** 6.2

**Finding:** The Mermaid flowchart omits credential prompts (GH token, Telegram bot token). Credentials are blocking for certain steps — GH token needed before `gh repo create`. The text mentions "Collected during wizard" but the flow diagram doesn't show when.

**Recommendation:** Add credential prompt steps to the diagram, logically placed after the GitHub repo prompt.

#### MED-06: No `bm init` interruption/partial-failure behavior

**Domain:** UX
**Section(s):** 6.2

**Finding:** The 14-step execution sequence has no specified behavior for Ctrl-C or mid-execution failures (e.g., `gh repo create` fails at step 11). Since "Fails if team dir already exists," a partial failure followed by retry would hit this error. User would need to manually delete the partial directory.

**Recommendation:** At minimum, document the recovery path in the error message: "Directory exists. If from a failed init, delete it and retry."

#### MED-07: Credential-to-env-var mapping unspecified

**Domain:** UX
**Section(s):** 6.5, 5.1

**Finding:** Config field `telegram_bot_token` maps to env var `RALPH_TELEGRAM_BOT_TOKEN`. The naming convention (uppercase + `RALPH_` prefix for Telegram but not for GH) is not documented. Also unspecified: what happens when an `Option<String>` credential is `None` at start time?

**Recommendation:** Add a credential mapping table. Specify behavior for missing credentials.

#### MED-08: `bm stop` graceful mode lacks timeout/feedback specification

**Domain:** UX
**Section(s):** 6.6

**Finding:** Graceful stop says "Remove from state.json once process exits" but doesn't specify: how `bm` detects process exit (polling interval?), what the user sees during the wait, what happens if `ralph loops stop` itself fails, or whether `bm stop` waits for all members sequentially.

**Recommendation:** Specify polling behavior, per-member feedback ("Stopping architect-bob... done"), and `ralph loops stop` failure handling (suggest `--force`).

#### MED-09: Error categories table incomplete

**Domain:** UX
**Section(s):** 9.2

**Finding:** Missing error categories for: no default team + no `-t` flag, missing credentials at start time, `ralph loops stop` failure, permission denied on workzone directory.

**Recommendation:** Add these categories to Section 9.2.

### Acceptance Criteria Gaps

#### MED-10: No AC for stale PID handling in `bm start`

**Domain:** AC
**Section(s):** 6.5, 10.4

**Finding:** `bm start` specifies: "If PID is stale → clean up state, re-launch." No AC tests this distinct behavior path.

**Recommendation:** Add AC: Given member with dead PID in state.json → stale entry cleaned → re-launched → new PID recorded.

#### MED-11: No AC for `bm status` crashed state

**Domain:** AC
**Section(s):** 6.7, 10.6

**Finding:** Three status states defined (running, crashed, stopped) but only running and stopped are tested. The "crashed" state's visibility and lifecycle is also unclear — does the user ever see it, or is it immediately cleaned to "stopped"?

**Recommendation:** Decide crashed state visibility. Add AC for stale PID detection in status output.

#### MED-12: No AC for `bm status -v` verbose mode

**Domain:** AC
**Section(s):** 6.7, 10.6

**Finding:** Verbose mode runs 4 Ralph commands with graceful degradation. No AC covers this.

**Recommendation:** Add AC verifying Ralph runtime details and graceful skip of unavailable commands.

#### MED-13: No AC for credential storage or config permissions

**Domain:** AC
**Section(s):** 5.1, 6.2, 10.1

**Finding:** Credential storage in config.yml and 0600 file permissions are specified behaviors with no AC coverage.

**Recommendation:** Add postconditions to `bm init` AC: credentials stored, config.yml has 0600 permissions.

#### MED-14: No AC for duplicate project or unknown role errors

**Domain:** AC
**Section(s):** 6.3, 6.13, 9.2, 10.2, 10.9

**Finding:** Both error paths are specified in command flows and error categories but have no ACs.

**Recommendation:** Add error-path ACs for both: unknown role → error listing available roles; duplicate project → error.

#### MED-15: No AC for multi-project or no-project workspace creation

**Domain:** AC
**Section(s):** 6.4, 10.3

**Finding:** `bm teams sync` iterates "for each member × project" and has a separate code path for no-projects teams. Only single-project happy path is tested.

**Recommendation:** Add ACs for both: two-project team → two workspace subdirs per member; no-project team → simple workspace without project subdirectory.

#### MED-16: No AC for prerequisite tool checks

**Domain:** AC
**Section(s):** 9.1, 10.x

**Finding:** `which` checks for git/gh/ralph with specific error messages are specified but untested by any AC.

**Recommendation:** Add AC: git not in PATH → `bm init` errors with install message.

#### MED-17: No unit tests for `commands/` modules

**Domain:** Testing
**Section(s):** 11.1

**Finding:** 10 command modules in `commands/` (init, hire, start, stop, status, teams, members, roles, profiles, projects) are not listed in the unit test table. Business logic like URL-to-project-name derivation, auto-suffix generation, and PID liveness checking lives here.

**Recommendation:** Either add `commands/*` to the unit test table with specific targets, or note they are thin wrappers tested via integration tests only.

#### MED-18: `bm stop --force` AC precondition implies fallback model

**Domain:** AC (Inconsistency)
**Section(s):** 6.6, 10.5

**Finding:** AC says "Given architect-bob running **but not responding to `ralph loops stop`**" — implying a retry/fallback. But the design says force is a standalone operator decision, not a response to graceful failure.

**Recommendation:** Simplify Given clause to: "Given architect-bob running (PID in state.json)."

---

## 🟢 Low Priority (10)

| ID | Domain | Finding |
|---|---|---|
| LOW-01 | Architecture | Schema YAML field names differ between UX.md (`teamKnowledge`) and design.md (`knowledge`) |
| LOW-02 | Architecture | `ScopeLayout.members` as `Option<String>` — team-scope-only semantics unclear |
| LOW-03 | Architecture | `bm start` flowchart omits schema version guard step (text mentions it) |
| LOW-04 | Architecture | `serde_yml = "0.0.12"` is very early pre-release; consider alternatives |
| LOW-05 | Architecture | `{member_dir}` path references ambiguous in `bm teams sync` prose |
| LOW-06 | UX | Double `team/team/` nesting confusing in execution steps prose |
| LOW-07 | UX | UX.md stale on `bm members list` output format (design is correct) |
| LOW-08 | UX | `--push` failure during `bm teams sync` — no error branch in diagram |
| LOW-09 | UX | 2-second alive check after `bm start` is best-effort, not health guarantee |
| LOW-10 | UX | `bm projects add` name derivation edge cases (duplicate names, trailing slash) |

Additional low-priority testing items:

| ID | Domain | Finding |
|---|---|---|
| LOW-11 | AC | No AC for first team auto-set as default |
| LOW-12 | AC | No AC for `bm stop` with no running members |
| LOW-13 | AC | No AC for `bm profiles describe` with unknown profile |
| LOW-14 | AC | `bm up` alias not explicitly tested |
| LOW-15 | AC | Manual validation missing `bm init` abort path test |
| LOW-16 | AC | Multi-team integration test references deferred "default switching" |
| LOW-17 | Architecture | Profile extraction logic understates `botminter.yml` special handling |
| LOW-18 | AC | No-projects case missing from `workspace.rs` unit test description |

---

## ✅ Strengths (9)

| # | Finding |
|---|---------|
| S1 | **Schema version guard pattern** — elegant, consistently applied across commands, with clean error messages pointing to `bm upgrade` |
| S2 | **Atomic state writes** — temp-file-then-rename for `state.json`, adopted from multiclaude research |
| S3 | **Ephemeral state model** — "git = truth, config = convenience, state = ephemeral" hierarchy is clean |
| S4 | **Profile extraction selectivity** — clear table of what gets copied vs. what stays internal |
| S5 | **`botminter.yml` dual-purpose design** — profile identity + team configuration in one file, with `#[serde(default)]` for the optional `projects` field |
| S6 | **Provisioning/operations separation** — no command crosses the boundary; each is predictable |
| S7 | **Read/write matrix** — excellent implementation reference showing data flow across all commands |
| S8 | **Comprehensive `bm init` AC** — 9 postconditions with conditional branches covering wizard optionality |
| S9 | **Scope discipline** — Section 2.2 clearly lists what M3 does NOT deliver |

---

## Recommended Resolution Order

1. **CRIT-01** — Resolve workspace path convention (architectural decision needed)
2. **HIGH-01 through HIGH-04** — Fix before implementation begins
3. **MED-01 through MED-04** — Data model clarifications (inform implementation)
4. **MED-05 through MED-09** — UX specifications (can resolve during implementation)
5. **MED-10 through MED-18** — AC and testing gaps (address when writing tests)
6. **LOW-**** — Address during implementation or defer

---

## Resolutions

### CRIT-01 — Resolved

**Decision:** The design was wrong. `.botminter/` inside each workspace is a git clone of the team repo — same convention as the current model. Nothing about the old `.botminter/` convention changes.

**Changes applied to design.md:**
- Section 7.1: Added `.botminter/` clone to workspace layout diagram; updated symlink paths from `../../team/team/` to `.botminter/team/{member_dir}/`
- Section 7.2: Added new principle: "`.botminter/` = team repo clone inside each workspace"
- Section 6.4: Workspace creation now clones team repo into `.botminter/`; all surface paths updated to `.botminter/` convention; `.gitignore` hides `.botminter/`
- Section 6.4 sync: Pull targets `.botminter/`; re-copy and symlink paths reference `.botminter/team/{member_dir}/`
- Section 6.4.1: All `.claude/` assembly symlinks and copies now reference `.botminter/` paths
- Section 10.3: ACs updated to verify `.botminter/` clone exists and all symlinks/copies reference it

### MED-02, MED-03 — Resolved

**Source:** requirements.md Q27 — "The member's name and role are recorded in the member's own `botminter.yml` inside the member directory (where comment format, emoji, and other member-specific config also live)."

**Decision:** `comment_emoji` is NOT dropped. The member skeleton's `.botminter.yml` is a template with role-specific defaults (including `comment_emoji`). During `bm hire`, it is read, augmented with `name`, and written as `botminter.yml` (renamed from dotfile).

**Changes applied to design.md:**
- Section 5.4: Added `comment_emoji` field to YAML example, `MemberManifest` struct, and description. Documented the `.botminter.yml` → `botminter.yml` rename during hire.
- Section 6.3 step 8: Updated to read template `.botminter.yml` from skeleton, augment with `name`, rename to `botminter.yml`.

### HIGH-01 through HIGH-04 — Resolved

**HIGH-01 (missing schema guard ACs):** Added schema version mismatch ACs to `bm teams sync` (10.3), `bm start` (10.4). Decided `bm stop` and `bm status` do NOT perform schema checks (they don't read/write team repo content structurally — stop sends signals, status reads state.json + PIDs).

**HIGH-02 (no `-t` flag AC):** Added Section 10.7.1 with two ACs: `-t` override and "no default, no flag" error.

**HIGH-03 (no schema mismatch integration test):** Added to Section 11.2: "Schema version mismatch" test covering hire/start/sync.

**HIGH-04 (missing `serde_json`):** Added `serde_json = "1"` to Section 3.3 dependency list.

### MED-01 — Resolved

**Decision:** Role descriptions live in `botminter.yml` under a `roles:` section in the profile manifest. Added `RoleDef` struct and `roles: Vec<RoleDef>` to `ProfileManifest`. Updated `bm roles list` (Section 6.10) to reference `botminter.yml` roles section.

### MED-04 — Resolved

**Decision:** `TeamEntry.path` points to the **team directory** (e.g. `~/.botminter/workspaces/hypershift/`), NOT the team repo. Team repo is at `{path}/team/`. Clarified with inline comments in the Rust struct.

### MED-05 — Resolved

Added credential prompt step to `bm init` Mermaid flowchart (between GitHub repo and hire members).

### MED-06 — Resolved

Added "Interruption/partial failure" paragraph to Section 6.2: no automatic cleanup, error message tells user to delete directory and retry.

### MED-07 — Resolved

Added credential-to-env-var mapping table to Section 6.5. Specified: `gh_token` → `GH_TOKEN` (required, error if missing), `telegram_bot_token` → `RALPH_TELEGRAM_BOT_TOKEN` (optional, launched without if absent).

### MED-08 — Resolved

Added "Graceful stop behavior" paragraph to Section 6.6: polls `kill(pid, 0)` every second, per-member feedback, reports `ralph loops stop` failures with `--force` suggestion.

### MED-09 — Resolved

Added 4 error categories to Section 9.2: "No team specified", "Missing credentials", "Profile not found", "Graceful stop failure", "Tool not found".

### MED-10 — Resolved

Added stale PID AC to Section 10.4: dead PID → cleaned → re-launched → new PID recorded.

### MED-11 — Resolved

Added crashed state AC to Section 10.6: dead PID in state.json → shown as "crashed" → stale entry removed.

### MED-12 — Resolved

Added verbose mode AC to Section 10.6: `bm status -v` shows Ralph runtime details, skips unavailable commands.

### MED-13 — Resolved

Added credential storage and 0600 permissions as postconditions to `bm init` AC (Section 10.1). Added default_team auto-set postcondition.

### MED-14 — Resolved

Added unknown role AC to Section 10.2 and duplicate project AC to Section 10.9.

### MED-15 — Resolved

Added multi-project AC (two projects → two workspaces per member) and no-project AC (workspace without project subdirectory) to Section 10.3.

### MED-16 — Resolved

Added Section 10.8.1 "Prerequisite Tool Checks" with ACs for missing `git` and missing `ralph`.

### MED-17 — Resolved

Added `commands/hire.rs`, `commands/projects.rs`, `commands/start.rs`, `commands/status.rs` to Section 11.1 unit test table with specific test targets.

### MED-18 — Resolved

Fixed `bm stop --force` AC precondition: changed from "not responding to ralph loops stop" to "running (PID in state.json)" — force is a standalone operator decision, not a fallback.

### Additional

Added `-t` flag override and schema mismatch integration tests to Section 11.2. Added missing prerequisite tool integration test. Fixed multi-team integration test description (removed "default switching" reference to deferred feature).

### LOW-* — Deferred

All 18 low-priority items deferred to implementation. No design changes required.
