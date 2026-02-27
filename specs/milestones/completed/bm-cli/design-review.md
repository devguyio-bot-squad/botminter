# Design Review — `bm` CLI (Milestone 3)

> Review of [design.md](design.md) against [requirements.md](requirements.md), [UX.md](UX.md), and [research/](research/).
> Reviewer: Claude Code (PDD session)
> Date: 2026-02-20

---

## Overall Assessment

The design is **solid and implementation-ready**. All 22 requirements are covered, architecture choices are well-justified by the research, and acceptance criteria are testable. The scope is appropriately bounded — no feature creep, no daemon, no LLM calls.

Findings are organized as **gaps** (things the design should address before implementation), **concerns** (things worth discussing), and **strengths** (things done well).

---

## Gaps — Things Missing or Underspecified

### G1. Project fork URL — where does `bm start` get it?

Section 6.4 says workspace creation includes "Clone or pull the team's fork" for each project. But the design doesn't specify where the fork URL is stored. The team repo has `projects/{name}/knowledge/` and `projects/{name}/invariants/`, but no metadata file with the repository URL.

Options:
- A `projects/{name}/project.yml` manifest (URL, upstream, branch convention)
- Storing project URLs in `~/.botminter/config.yml` under each team
- Collecting URLs during `bm init` when the user says "add projects"

**Severity:** Functional gap — `bm start` can't clone workspaces without knowing what to clone.

### G2. Adding projects after init — no command exists

`bm init` asks "Want to add projects?" but there's no `bm projects add` in the M3 command tree. If the team starts working on a second repo later, the operator has to manually create `projects/{name}/` and store the fork URL somewhere. Either add a `bm projects add` command or document the manual process.

**Severity:** UX gap.

### G3. `.claude/` directory assembly — unspecified

Section 6.4 mentions "Assemble `.claude/` directory with agents/skills" as a workspace creation step, but doesn't specify what goes in there. The current `just create-workspace` has specific logic for this (agents, skills, settings). The design should at least reference the source directories and assembly rules, since this directly affects whether Ralph and Claude Code work correctly in the workspace.

**Severity:** Functional gap — implementer won't know what to assemble without referencing old Justfile code.

### G4. Label definitions in profiles — not specified

Section 6.2 step 11 says `bm init` bootstraps standard labels on GitHub. But the labels (like `status/po:triage`, `status/arch:design`) are profile-specific — the `rh-scrum` profile has different labels than `compact` would. Where are labels defined within the profile? A `labels.yml` or similar in the profile directory would make this explicit.

**Severity:** Functional gap — `bm init` can't bootstrap labels without knowing what they are.

### G5. No `bm teams set-default` command

The config has `default_team` but no command to change it. The operator would need to manually edit `config.yml`. Since `bm` aims to be the "no manual text editing" experience (per Q2), this is a UX gap.

**Severity:** Minor UX gap.

### G6. No `bm import` / `bm register` for recovery

Section 2.3 states the invariant: "Delete `~/.botminter/` → reconstruct from team repos on disk." But no command operationalizes this. If the config is lost, how does the operator re-register an existing team repo at a known path?

**Severity:** Minor — recovery story stated but not operationalized.

### G7. Missing high-level architecture/component diagram

There are good Mermaid flowcharts for `bm init` and `bm start`, and a useful read/write matrix table. But no diagram showing the overall component relationships: `bm` binary <-> `~/.botminter/` <-> team repo <-> workzone <-> Ralph processes <-> GitHub.

**Severity:** Minor — documentation completeness.

---

## Concerns — Things Worth Discussing

### C1. `bm status -v` depends on Ralph CLI commands that may not exist yet

The verbose status mode calls `ralph hats`, `ralph loops list`, `ralph events`, and `ralph bot status`. Are all of these available in the current Ralph CLI? If not, this is a cross-project dependency. The design should either confirm these exist or flag them as "implemented as available; gracefully degrade if not."

### C2. `bm hire` auto-pushes to GitHub

Section 6.3: "If team repo has GitHub remote: `git push`". Auto-pushing without confirmation could surprise users. For M3 (author-only) this is probably fine, but it's worth being intentional about — is auto-push the desired behavior, or should there be a `--push` flag?

### C3. PID tracking vs. Ralph's own process management

`bm start` launches `ralph run` as a background process and tracks the PID. But `bm stop` runs `ralph loops stop` from the workspace directory. This means Ralph needs to know how to find and stop its own loop from the workspace context (presumably via its own state files). If Ralph's PID and `bm`'s tracked PID are different (e.g., Ralph spawns child processes), the fallback SIGTERM might kill the wrong process or miss children.

Recommendation: verify that the PID `bm` captures from `ralph run &` is the PID that `ralph loops stop` expects, and that SIGTERM on that PID cleans up children.

### C4. Member name = role name (1:1 mapping)

Section 6.8 shows member and role columns with identical values. This implies you can only hire one architect, one dev, etc. Is that the intent? If a team ever needs two developers, the current model doesn't support it (the directory is `team/dev/`, and you can't have two). This is fine for M3 — just worth being explicit that M3 assumes 1:1.

### C5. `serde_yaml` is deprecated

The dependencies list `serde_yaml = "0.9"`. The crate's author has deprecated it. Consider `serde_yml` (its successor) or `yaml-rust2` + `serde`. Not a blocker for M3 but worth noting for forward compatibility.

---

## Strengths — Things Done Well

### S1. Schema version guard pattern

Checking `botminter.yml` schema version before any operation that reads or writes team repo content is elegant. It prevents partial migrations and gives a clean error message pointing to `bm upgrade`. Even though v1 is the only version in M3, the guard infrastructure is in place for when it matters.

### S2. State model clarity

The "git = truth, `~/.botminter/` = cache, `state.json` = ephemeral" hierarchy is clean and well-articulated. Atomic writes for state.json (temp -> rename) come directly from the multiclaude research and are the right call.

### S3. Profile extraction selectivity

Section 8.3 clearly defines what gets copied to the team repo vs. what stays internal to `bm`. `members/` extracted on demand, `.schema/` never copied — this keeps the team repo clean while maintaining the embedded profile as the canonical source.

### S4. Acceptance criteria quality

The Given-When-Then format is concrete and testable. The schema version mismatch test (Section 10.2, third criterion) and the idempotency test (Section 10.3, second criterion) are particularly good — they test important edge cases.

### S5. Migration appendix

Appendix C mapping old Justfile commands to new `bm` commands is a valuable reference for anyone familiar with the current system.

### S6. Scope discipline

Section 2.2 is refreshingly clear: "What M3 Does NOT Deliver." No ambiguity about what's deferred. This prevents scope creep during implementation.

---

## Summary: Recommended Actions

| # | Finding | Severity | Recommendation |
|---|---------|----------|----------------|
| G1 | Project fork URL storage | **Gap** | Add `project.yml` manifest or config entry |
| G2 | Adding projects after init | **Gap** | Add `bm projects add` or document manual workflow |
| G3 | `.claude/` assembly rules | **Gap** | Document what files are assembled and from where |
| G4 | Label definitions in profiles | **Gap** | Specify where/how labels are defined in profiles |
| G5 | No `set-default` command | **Minor** | Add `bm teams set-default <name>` to command tree |
| G6 | No `import`/`register` command | **Minor** | Add for recovery story completeness |
| G7 | Missing architecture diagram | **Minor** | Add a component/deployment Mermaid diagram |
| C1 | Ralph CLI dependency | **Concern** | Verify commands exist; design graceful fallback |
| C2 | Auto-push on hire | **Concern** | Decide intentionally: auto-push vs. `--push` flag |
| C3 | PID alignment with Ralph | **Concern** | Verify PID chain is correct |
| C4 | 1:1 role/member | **Clarification** | State explicitly in design |
| C5 | `serde_yaml` deprecation | **Minor** | Consider `serde_yml` |

Items G1–G4 should be resolved before implementation. The rest are refinements that can be addressed during implementation or deferred.

---

## Resolutions (from design review discussion)

All gaps and concerns from items 1, 4, 9, 10, 11 have been resolved. Resolutions recorded in [requirements.md](requirements.md) Q23–Q28.

| Item | Resolution | Requirements |
|---|---|---|
| **G1** | Fork URLs stored in `botminter.yml` `projects:` section in team repo | Q23 |
| **G2** | New command: `bm projects add <url>` | Q23 |
| **G4** | Labels in `botminter.yml` `labels:` section, per profile | Q24 |
| **C2** | No auto-push. Push via `bm teams sync --push` | Q25 |
| **C3** | `bm stop` = graceful (`ralph loops stop`); `bm stop -f` = force-kill via PID | Q26 |
| **C4** | Members have unique identity. Dir: `{role}-{name}`. Auto 2-digit suffix if no name. | Q27 |
| **New** | `bm profiles list` and `bm profiles describe <profile>` | Q28 |
| **New** | `bm teams sync [--push]` — reconcile workspaces + optional push | Q23 |

### New commands added to M3 scope

- `bm teams sync [--push]`
- `bm projects add <url> [-t team]`
- `bm profiles list`
- `bm profiles describe <profile>`

### Design changes required

1. **Remove workspace creation from `bm start`** (Section 6.4) — start only launches Ralph in existing workspaces
2. **Add `bm teams sync`** — new section with workspace reconciliation logic
3. **Add `bm projects add`** — new section
4. **Add `bm profiles list` and `bm profiles describe`** — new sections
5. **Expand `botminter.yml`** (Section 5.3) — add `labels:` and `projects:` sections
6. **Update `bm hire`** (Section 6.3) — `--name` flag, `{role}-{name}` directory convention, member `botminter.yml`
7. **Update `bm stop`** (Section 6.5) — `--force/-f` flag, separate graceful vs. force modes
8. **Update command tree** (Section 6.1) — see final command tree in Q28
9. **Update read/write matrix** (Section 4) — add new commands
10. **Update acceptance criteria** (Section 10) — add criteria for new commands

### Remaining open items (minor, can be addressed during design update or implementation)

| Item | Status |
|---|---|
| G3 (`.claude/` assembly) | Understood from Justfile review; needs documenting in design |
| G5 (set-default) | Deferred — not in M3 scope |
| G6 (import/register) | Deferred — not in M3 scope |
| G7 (architecture diagram) | Add during design update |
| C1 (Ralph CLI dependency) | Verify during implementation |
| C5 (`serde_yaml` deprecation) | Address during implementation |
