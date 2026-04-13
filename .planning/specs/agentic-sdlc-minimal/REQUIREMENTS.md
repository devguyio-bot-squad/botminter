# Requirements: `agentic-sdlc-minimal` Profile

## Context

Create a new BotMinter profile called `agentic-sdlc-minimal` based on the existing `scrum-compact` profile. This profile will be used to set up fresh teams via `bm init --profile agentic-sdlc-minimal`. The `scrum-compact` profile will be deleted after migration.

The changes incorporate lessons learned from operating the compact profile: the sentinel POC (PR merge gatekeeper), the status convention redesign, and the PR lifecycle gap identified during adversarial review of team repo PR #78.

## Requirements

### R1: New status convention — `<role-slug>:<persona>:<activity>`

The current `<role>:<phase>` format (e.g., `po:triage`) conflates member roles with hat personas. The compact profile has two members (superman and chief-of-staff) but statuses use hat-persona prefixes (po, arch, dev, qe, lead, sre, cw) that don't correspond to actual roles.

The new format is `<role-slug>:<persona>:<activity>`:
- Role slug identifies which member role owns the status
- Persona identifies which hat context is active
- Activity identifies what's being done

Role slugs: `sm` (superman), `cos` (chief-of-staff), `snt` (sentinel).

Exceptions: `done`, `error` remain as-is (no role owner).

### R2: Split `po:` into agent-automated and human-gated

Current `po:*` statuses mix agent-automated work (triage, backlog, ready) with human decision gates (design-review, plan-review, accept). This is confusing — `po:triage` looks like it needs human input but doesn't.

Split:
- Agent-automated PO work: `sm:po:triage`, `sm:po:backlog`, `sm:po:ready` (superman does this autonomously)
- Human decision gates: `human:po:design-review`, `human:po:plan-review`, `human:po:accept` (human operator must respond)

The `human:po:*` format follows the 3-part convention — `human` is the role slug, `po` is the persona context, activity is the gate.

### R3: Sentinel as its own role

Sentinel is a dedicated PR merge gatekeeper role with:
- Role slug: `snt`
- Two hats: `pr_gate` (merge gatekeeper) and `pr_triage` (orphaned PR surfacing)
- Custom board scanner that scans PRs on project forks, not just the project board
- Per-project merge gate configuration via knowledge files at `team/projects/<project>/knowledge/merge-gate.md`

Sentinel replaces the `po:merge` auto-advance. The new flow:
- `sm:arch:sign-off` auto-advances to `snt:gate:merge`
- Sentinel runs project-specific tests (e2e, exploratory, coverage)
- If all pass → merges the PR, advances to `done`
- If any fail → rejects, returns to `sm:dev:implement`

Sentinel also scans for orphaned PRs (open PRs with no board issue) and creates triage issues for the PO.

### R4: PR lifecycle for project code

The current process says "PRs are NOT used for code changes to the project repo." This is wrong — code PRs are needed for review, testing, and merge gating.

Add PR lifecycle:
- Branch naming: `feature/<type>-<issue-number>-<description>`
- PR title: `[#<issue-number>] <description>`
- Draft PRs created during `sm:qe:test-design`, marked ready during `sm:dev:implement`
- Code review via `gh pr review` (approve/request-changes), not issue comments
- Merge gated by sentinel (not human, not auto-advance)
- Per-project merge gate knowledge defines test commands and thresholds

### R5: `project/<project>` label documented

The `project/<project>` label is used everywhere (board scanner filtering, sentinel PR resolution, merge-gate knowledge lookup) but isn't documented in PROCESS.md's Labels section. Add it.

### R6: Issue-type discoverability via mermaid diagrams

Add mermaid workflow diagrams to issue-type templates so the lifecycle is visible when viewing an issue. Each issue type (Epic, Task, Bug) gets a template with a mermaid flowchart showing its status transitions.

### R7: Delete `scrum-compact` profile

After the new profile is complete and verified, delete `profiles/scrum-compact/`.

## Complete Status Map

### Epic lifecycle
| Old | New |
|-----|-----|
| `po:triage` | `sm:po:triage` |
| `po:backlog` | `sm:po:backlog` |
| `arch:design` | `sm:arch:design` |
| `lead:design-review` | `sm:lead:design-review` |
| `po:design-review` | `human:po:design-review` |
| `arch:plan` | `sm:arch:plan` |
| `lead:plan-review` | `sm:lead:plan-review` |
| `po:plan-review` | `human:po:plan-review` |
| `arch:breakdown` | `sm:arch:breakdown` |
| `lead:breakdown-review` | `sm:lead:breakdown-review` |
| `po:ready` | `sm:po:ready` |
| `arch:in-progress` | `sm:arch:in-progress` |
| `po:accept` | `human:po:accept` |
| `done` | `done` |

### Story lifecycle
| Old | New |
|-----|-----|
| `qe:test-design` | `sm:qe:test-design` |
| `dev:implement` | `sm:dev:implement` |
| `dev:code-review` | `sm:dev:code-review` |
| `qe:verify` | `sm:qe:verify` |
| `arch:sign-off` | `sm:arch:sign-off` (auto-advance) |
| `po:merge` | `snt:gate:merge` (sentinel handles) |
| `done` | `done` |

### Bug lifecycle
| Old | New |
|-----|-----|
| `bug:investigate` | `sm:bug:investigate` |
| `arch:review` | `sm:arch:review` |
| `arch:refine` | `sm:arch:refine` |
| `po:plan-review` | `human:po:plan-review` (reused) |
| `bug:breakdown` | `sm:bug:breakdown` |
| `bug:in-progress` | `sm:bug:in-progress` |
| `qe:verify` | `sm:qe:verify` (reused) |
| `done` | `done` |

### Specialist
| Old | New |
|-----|-----|
| `sre:infra-setup` | `sm:sre:infra-setup` |
| `cw:write` | `sm:cw:write` |
| `cw:review` | `sm:cw:review` |

### Chief of staff
| Old | New |
|-----|-----|
| `cos:todo` | `cos:exec:todo` |
| `cos:in-progress` | `cos:exec:in-progress` |
| `cos:done` | `cos:exec:done` |

### Sentinel (new)
| Status | Description |
|--------|-------------|
| `snt:gate:merge` | Sentinel running merge gates on PR |
| (no board status for triage — sentinel creates issues at `sm:po:triage`) | |

### Common
| Status | Notes |
|--------|-------|
| `done` | Unchanged |
| `error` | Unchanged |

## Files Affected

### Rewrite (major changes)
- `botminter.yml` — profile name, description, roles (add sentinel), all statuses renamed, views updated, labels
- `PROCESS.md` — all status references, PR lifecycle section, human gates, auto-advance rules
- `context.md` — 3-member model, status references
- `roles/superman/ralph.yml` — all status references in 18 hat instructions
- `roles/superman/context.md` — status references, hat table
- `roles/chief-of-staff/ralph.yml` — status references in executor hat
- `roles/chief-of-staff/context.md` — status references
- `coding-agent/skills/board-scanner/SKILL.md` — dispatch tables, auto-advance rules
- `coding-agent/skills/github-project/references/status-lifecycle.md` — status references
- `workflows/*.dot` — all status nodes and edges
- `invariants/code-review-required.md` — PR-based review requirement
- `knowledge/communication-protocols.md` — status references
- `knowledge/pr-standards.md` — project repo PR standards

### New files
- `roles/sentinel/` — complete role directory (`.botminter.yml`, `PROMPT.md`, `ralph.yml`, `context.md`, hats, skills, board scanner override)

### Unchanged (copy as-is)
- `brain/` (system-prompt.md, envelope.md)
- `bridges/` (tuwunel, telegram, rocketchat)
- `formations/` (k8s, local)
- `ralph-prompts/` reference docs
- `agreements/` structure
- `skills/knowledge-manager/`
- `coding-agent/settings.json`
- `coding-agent/skills/github-project/scripts/` (don't hardcode statuses)
- `coding-agent/skills/status-workflow/` (generic)
- `.schema/v1.yml`
- `knowledge/commit-convention.md`
- `knowledge/team-agreements.md`
- `knowledge/github-projects-graphql.md`

### Delete
- `profiles/scrum-compact/` (entire directory)
