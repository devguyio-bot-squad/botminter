# Milestone 3: `bm` CLI

## Objective

Replace the Justfile-based tooling with a Rust CLI binary (`bm`) that serves as the single operator interface for managing agentic teams — creating teams, hiring members, provisioning workspaces, and managing Ralph process lifecycle.

## Prerequisites

- M2 complete (architect member skeleton built, two-member coordination validated)
- `profiles/rh-scrum/` exists with `botminter.yml`, `.schema/v1.yml`, and self-contained profile content (Step 1 of plan.md)

## Key References

- Design: `specs/milestone-3-bm-cli/design.md` (standalone, 12 sections)
- Plan: `specs/milestone-3-bm-cli/plan.md` (7 incremental steps)
- Requirements: `specs/milestone-3-bm-cli/requirements.md` (28 Q&A pairs)
- Research: `specs/milestone-3-bm-cli/research/` (4 topic files)

## Requirements

1. **Profile restructuring** — collapse `skeletons/team-repo/` + `skeletons/profiles/` into `profiles/` at repo root. Each profile MUST be self-contained with `botminter.yml` and `.schema/v1.yml`. Per design.md Section 3.1.

2. **Cargo workspace** — Rust workspace at repo root with `crates/bm/` binary crate. Dependencies per design.md Section 3.3. Profiles MUST be embedded via `include_dir`. Per design.md Section 3.2.

3. **Profile commands** — `bm profiles list` and `bm profiles describe <profile>` MUST read from embedded profiles and display metadata, roles, and labels. Per design.md Sections 6.11, 6.12.

4. **Config layer** — `~/.botminter/config.yml` with teams, credentials, defaults. File MUST have `0600` permissions. Per design.md Section 5.1.

5. **`bm init`** — interactive wizard using `cliclack`. MUST extract profile content, create team repo with git init, optionally create GitHub repo + bootstrap labels, register in config. Per design.md Section 6.2.

6. **`bm hire`** — extract member skeleton from embedded profile into `team/{role}-{name}/`. MUST perform schema version guard. Auto-generate 2-digit suffix when `--name` omitted. MUST NOT auto-push. Per design.md Section 6.3.

7. **`bm projects add`** — append project to `botminter.yml`, create project dirs. MUST NOT auto-push. Per design.md Section 6.13.

8. **`bm teams sync`** — provision and reconcile workspaces. Each workspace MUST contain target project (fork @ member branch) + BM (`.botminter/` clone + surfaced files). `.claude/agents/` MUST be assembled from three scopes. Per design.md Sections 6.4, 6.4.1.

9. **`bm start` / `bm stop`** — launch Ralph via `ralph run -p PROMPT.md` in each workspace, track PIDs in `state.json` with atomic writes. Stop MUST support graceful (`ralph loops stop`) and force (`--force`, SIGTERM) modes. Per design.md Sections 6.5, 6.6.

10. **`bm status`** — dashboard with per-member alive/dead status via PID liveness check. Verbose mode (`-v`) MUST query Ralph CLI commands, skipping unavailable ones gracefully. Per design.md Section 6.7.

11. **List commands** — `bm teams list`, `bm members list`, `bm roles list` MUST display tables reading from config, team repo, and embedded profile respectively. All commands accepting `-t`/`--team` MUST resolve to default team when flag omitted. Per design.md Sections 6.8–6.10.

12. **Skeleton cleanup** — remove `skeletons/` directory. Update root CLAUDE.md to reflect new architecture. Per plan.md Step 7.

## Acceptance Criteria

```
Given no existing team
When the operator completes `bm init`
Then a team repo exists with botminter.yml, PROCESS.md, CLAUDE.md, knowledge/, invariants/, agent/
And the team is registered in ~/.botminter/config.yml with 0600 permissions

Given a team with profile rh-scrum
When the operator runs `bm hire architect --name bob`
Then team/architect-bob/ exists with skeleton content and botminter.yml containing name and role
And a local git commit is created with no push

Given a team with hired members and projects in botminter.yml
When the operator runs `bm teams sync`
Then workspaces contain target project clone + .botminter/ clone + surfaced symlinks + .claude/agents/

Given synced workspaces
When the operator runs `bm start`
Then Ralph launches in each workspace with GH_TOKEN set
And state.json contains PIDs verified alive via kill(pid, 0)

Given running members
When the operator runs `bm stop`
Then `ralph loops stop` executes per member and state.json is cleaned

Given a team with schema_version mismatch between team repo and embedded profile
When the operator runs `bm hire`, `bm start`, or `bm teams sync`
Then the command refuses with an error suggesting `bm upgrade`

Given the bm binary with embedded profiles
When the operator runs `bm profiles list`
Then all embedded profiles display with name, version, schema, and description
```
