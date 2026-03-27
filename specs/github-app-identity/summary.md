# Summary: Team Runtime Architecture + Per-Member GitHub App Identity

## Artifacts

```
specs/github-app-identity/
  rough-idea.md                    # Original concept from github-app-identity.md
  requirements.md                  # 22 Q&A decisions + 11 adversarial review findings
  research/
    manifest-flow.md               # GitHub App Manifest flow API details
    jwt-and-app-lifecycle.md       # JWT signing, jsonwebtoken crate, App deletion API
    token-delivery.md              # GH_CONFIG_DIR + hosts.yml mechanism
  design.md                        # Unified design document
  plan.md                          # Sprint index with summaries
  sprint-1/
    plan.md                        # Detailed steps for Formation + CredentialStore + Team
    PROMPT.md                      # Autonomous implementation prompt
  sprint-2/
    plan.md                        # Detailed steps for Daemon + CLI + Brain
    PROMPT.md                      # Autonomous implementation prompt
  sprint-3/
    plan.md                        # Detailed steps for GitHub App + fire + export
    PROMPT.md                      # Autonomous implementation prompt
  summary.md                      # This file

.planning/adrs/
  0008-team-runtime-architecture.md   # Rewritten ADR (was: local-formation-as-first-class-concept)
  0011-github-app-per-member-identity.md  # Updated ADR with review fixes
```

## Overview

This milestone merges two architectural changes:

1. **Team as API boundary** — operators interact with teams and members. Formation (deployment strategy) and daemon (process supervisor) are internal implementation details.

2. **Per-member GitHub App identity** — each member gets its own GitHub App with bot identity. Installation tokens refreshed automatically by the daemon.

## Key Decisions

- Team is the only operator-facing abstraction
- Formation manages: environment, credentials (key-value), credential delivery, member lifecycle
- Daemon is an implementation detail of member lifecycle
- Token delivery via `GH_CONFIG_DIR` + `hosts.yml` (not `GH_TOKEN` env var)
- Git credential helper per-workspace `.git/config` (not global `~/.gitconfig`)
- Org required (personal accounts blocked)
- Daemon communicates with CLI via RESTful HTTP API with OpenAPI schema
- Daemon owns `state.json` — CLI reads only
- Formation trait is sync — daemon is a separate process
- CredentialStore is key-value — callers compose domain-specific keys

## Sprint Plan

| Sprint | Focus | Auth Model |
|--------|-------|-----------|
| 1 | Formation trait + CredentialStore + Team API boundary | `gh_token` (unchanged) |
| 2 | Daemon supervisor + CLI through Team + `bm env` + Brain model | `gh_token` (unchanged) |
| 3 | GitHub App identity + `bm fire` + credentials export/import | Swap to GitHub App |

## Codebase Changes Since Planning (Rebased)

These changes landed on `main` after initial planning and have been incorporated:

- **Skill rename:** `gh` → `github-project` (directory, frontmatter, all profile refs). Sprint 3 Step 10 updated.
- **Native issue types:** `kind/*` labels → GitHub native issue types (Epic, Task, Bug). `parent/<N>` labels → native sub-issues. No impact on sprint plans (profile-level, orthogonal to auth).
- **`{{member_dir}}` placeholders:** `render_member_placeholders()` added to `hire_member()` in `profile/member.rs`. Sprint 1 Step 5 and Sprint 3 Step 2 note sequencing — placeholder rendering is profile-level and completes before App credential storage.
- **GraphQL scripts:** `create-issue.sh`, `subtask-ops.sh` rewritten to use `gh api graphql`. Validates `hosts.yml` token delivery — `gh api graphql` reads from `GH_CONFIG_DIR` natively.

## Next Steps

Each sprint's `PROMPT.md` is ready for autonomous implementation:
- `ralph run --config presets/pdd-to-code-assist.yml` for full pipeline
- `ralph run --config presets/spec-driven.yml` for simpler flow
- Or use `bm chat` / `bm minty` to implement interactively
