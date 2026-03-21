---
name: profile-design
description: >-
  Designs and troubleshoots BotMinter profiles — team methodology templates
  that define roles, statuses, hats, skills, and process conventions. Use when
  the operator asks to "design a profile", "create a new role", "fork a profile",
  "fix my profile", "troubleshoot profile issues", "design a process workflow",
  "add a hat", "customize a profile", or "validate my profile". Operates on
  profile templates, not live team repos.
metadata:
  author: botminter
  version: 1.0.0
  category: profiles
  tags: [profiles, design, roles, hats, troubleshooting, validation]
---

# Profile Design

Designs, customizes, and troubleshoots BotMinter profiles at the template level. Profiles are methodology templates that `bm init` extracts into team repos. This skill works before any team exists or when a team is broken and needs fixes from outside.

For live team changes on a working team, suggest `bm chat team-manager` instead.

## When to Use This vs Team Manager

| Situation | Use This Skill | Use Team Manager |
|-----------|---------------|-----------------|
| No team exists yet | Yes | No |
| Team is broken / won't start | Yes | No |
| Designing a new profile from scratch | Yes | No |
| Forking an existing profile | Yes | No |
| Tweaking a live team's process | No | Yes (`bm chat team-manager`) |
| Running a retrospective | No | Yes |
| Tuning a member's behavior | No | Yes |

## Profile Location

| Context | Path |
|---------|------|
| Extracted profiles | `~/.config/botminter/profiles/<name>/` |
| Source tree (dev) | `profiles/<name>/` |

## Six Operations

### 1. Browse Profile

Show the full structure of a profile: roles, statuses, hats, skills, process, and knowledge.

**Steps:**

1. Read the manifest: `<profile>/botminter.yml`
2. List roles: `ls <profile>/roles/`
3. For each role, list its coding-agent skeleton: `ls <profile>/roles/<role>/coding-agent/`
4. Read process: `<profile>/PROCESS.md`
5. List knowledge: `ls <profile>/knowledge/`

**Present as:**

````
## Profile: <name>

### Roles
| Role | Hats | Skills |
|------|------|--------|
| superman | 15 | 0 |
| team-manager | 2 | 5 |

### Status Lifecycle
<status list from botminter.yml>

### Process
<summary from PROCESS.md>
````

### 2. Design New Role

Guided conversation to create a role template within a profile.

**Interview the operator:**

1. What does this role do? What is its purpose?
2. Is it single-hat or multi-hat? What hats does it need?
3. For each hat: What triggers it? What does it publish? What are its instructions?
4. Does it need skills? Which ones?
5. Does it need role-specific knowledge files?

**Generate these files in `<profile>/roles/<role-name>/`:**

| File | Purpose |
|------|---------|
| `coding-agent/PROMPT.md` | Work objective and role identity |
| `coding-agent/CLAUDE.md` | Context and conventions |
| `coding-agent/ralph.yml` | Hat collection, skill dirs, preset |
| `coding-agent/.botminter.yml` | Agent identity tags |
| `coding-agent/hats/<hat>.md` | One file per hat with instructions |

**Role skeleton template:**

```yaml
# ralph.yml
preset: feature-development  # or custom
skill_dirs:
  - "coding-agent/skills"
```

```markdown
# PROMPT.md
You are the <role-name> for this team.

## Responsibilities
- <responsibility 1>
- <responsibility 2>
```

After generation, run profile validation (see below).

### 3. Design Process

Create or modify the status lifecycle and PROCESS.md for a profile.

**Interview the operator:**

1. Show current statuses if modifying an existing profile
2. What lifecycle stages does the team need?
3. Which stages need human review gates?
4. Which stages should auto-advance?
5. What role handles each status?

**Generate or update:**

- `botminter.yml` statuses section
- `PROCESS.md` with lifecycle documentation
- Verify each status has a role that handles it

**Status naming convention:** `<role-prefix>:<phase>` (e.g., `dev:implement`, `po:review`)

### 4. Design Hats

Create or modify hat collections for a role.

**For each hat, define:**

| Field | Description |
|-------|-------------|
| `name` | kebab-case identifier |
| `trigger` | When this hat activates (status label, event, or schedule) |
| `instructions` | What the hat does (markdown file in `hats/`) |
| `publishes` | Events this hat emits when done |

**Hat file template:**

````markdown
# <Hat Name>

## Trigger
Activates when: <trigger condition>

## Instructions
<what to do>

## Publishes
- `<event.name>` — <when/why>
````

**Board-scanner pattern:** If the role uses a board-scanner hat, ensure its dispatch table covers all statuses assigned to this role. Every status must route to a hat.

### 5. Fork Profile

Create a new profile by copying and customizing an existing one.

**Steps:**

1. Copy `profiles/<source>/` to `profiles/<new-name>/`
2. Update `botminter.yml`: name, display_name, description
3. Guide the operator through customizations:
   - Add/remove roles
   - Modify statuses
   - Adjust process
4. Run profile validation on the new profile

**Include in forked profiles:**

- `agreements/` directory with `decisions/`, `retros/`, `norms/` subdirectories
- `knowledge/team-agreements.md` convention documentation
- All existing knowledge files from the source profile

### 6. Troubleshoot Profile

Diagnose why a profile is not working correctly.

**Diagnostic checklist:**

1. **Manifest validity** — Does `botminter.yml` parse correctly? Are all required fields present?
2. **Status coverage** — Does every status have at least one role whose hats handle it?
3. **Hat trigger validity** — Do all hat triggers reference statuses or events that exist?
4. **Board-scanner completeness** — Does the board-scanner dispatch table cover all statuses for this role?
5. **Skeleton completeness** — Does each role have the required files?
   - `coding-agent/PROMPT.md`
   - `coding-agent/CLAUDE.md`
   - `coding-agent/ralph.yml`
   - `coding-agent/.botminter.yml`
6. **Process consistency** — Does PROCESS.md match the statuses in botminter.yml?
7. **Skill references** — Do skill dirs in ralph.yml point to directories that exist?
8. **Knowledge paths** — Do knowledge file references resolve?

**Common issues and fixes:**

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| Member won't start | Missing ralph.yml or PROMPT.md | Create the missing skeleton file |
| Hat never activates | Trigger doesn't match any status | Update trigger to match a status in botminter.yml |
| Status stuck | No hat handles this status | Add a hat with this status as trigger |
| Board scanner misses work | Dispatch table incomplete | Add missing status to board-scanner dispatch |
| Broken after fork | botminter.yml name not updated | Update name/display_name in manifest |

**Inspection commands:**

```bash
# Check manifest
cat <profile>/botminter.yml

# List all statuses
grep "name:" <profile>/botminter.yml | grep -v "^name:"

# Check role skeleton
ls <profile>/roles/<role>/coding-agent/

# Check hat triggers
grep -r "trigger" <profile>/roles/<role>/coding-agent/hats/

# Validate ralph.yml
cat <profile>/roles/<role>/coding-agent/ralph.yml
```

## Profile Validation Checklist

Run after any design change. Report each check as PASS or FAIL.

| # | Check | How |
|---|-------|-----|
| 1 | botminter.yml parses | Read and validate YAML structure |
| 2 | All statuses have handlers | Cross-reference statuses with role hat triggers |
| 3 | All hat triggers are valid | Each trigger references a real status or event |
| 4 | Board-scanner dispatch complete | Dispatch table covers all role statuses |
| 5 | Skeleton files present | Each role has PROMPT.md, CLAUDE.md, ralph.yml, .botminter.yml |
| 6 | PROCESS.md consistent | Statuses mentioned in PROCESS.md exist in botminter.yml |
| 7 | Skill dirs exist | Directories in ralph.yml skill_dirs exist on disk |
| 8 | No orphan statuses | Every status is reachable in the lifecycle graph |
| 9 | No dead-end statuses | Every non-terminal status has a successor |
| 10 | Agreements dir present | `agreements/` with `decisions/`, `retros/`, `norms/` exists |

## CLI Quick Reference

| Task | Command |
|------|---------|
| List profiles | `bm profiles list` |
| Describe a profile | `bm profiles describe <name>` |
| Initialize profiles | `bm profiles init` |
| Chat with team manager | `bm chat team-manager` |
| Show team status | `bm status -t <team>` |
