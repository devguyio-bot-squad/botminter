---
status: pending
created: 2026-03-03
started: null
completed: null
---
# Task: Minty Config Structure and Prompt

## Description
Create Minty's embedded configuration structure (`minty/`) with the persona prompt, config file, and empty skills directory. This is shipped with the binary and extracted alongside profiles by `bm profiles init`.

## Background
Minty is BotMinter's interactive assistant persona — a thin shell that provides a friendly interface for operators. Unlike team members, Minty doesn't run as a Ralph instance. It's a coding agent session primed with Minty's persona and skills. The actual capabilities come from skills (Step 20).

## Reference Documentation
**Required:**
- Design: specs/milestones/minty-and-friends-rapid/design.md (see "Minty" section)
- Plan: specs/milestones/minty-and-friends-rapid/plan.md (Step 19)

**Note:** Read the design document before beginning implementation.

## Technical Requirements
1. Create Minty's embedded config (shipped with binary):
   ```
   minty/
     prompt.md           # Minty persona + system instructions
     config.yml          # Minty-specific config
     skills/             # Empty initially — populated in Step 20
   ```
2. Write `prompt.md` — Minty persona:
   - Friendly, approachable assistant for BotMinter operators
   - Aware of BotMinter concepts: profiles, teams, config, CLI, conventions
   - Instructs the agent to use available skills for BotMinter operations
   - Cross-team awareness (reads from `~/.botminter/` when available)
   - Graceful handling of missing runtime data
3. Write `config.yml` — Minty-specific settings (coding agent resolution, skill paths)
4. Update `bm profiles init` to also extract Minty config to `~/.config/botminter/minty/`
5. Embed Minty config alongside profiles in the binary (update `include_dir!` or add separate embed)

## Dependencies
- Steps 1-17 complete (Sprints 1-5 finished)

## Implementation Approach
1. Create `minty/` directory in the source tree (alongside `profiles/`)
2. Write the persona prompt with BotMinter domain knowledge
3. Write config.yml with default settings
4. Create empty skills/ directory with .gitkeep
5. Update the binary embedding to include minty/
6. Update `bm profiles init` extraction to handle minty config
7. Test extraction includes minty/

## Acceptance Criteria

1. **Minty config exists in source tree**
   - Given the botminter source tree
   - When listing the minty/ directory
   - Then `prompt.md`, `config.yml`, and `skills/` exist

2. **prompt.md contains persona**
   - Given `minty/prompt.md`
   - When reading the file
   - Then it contains Minty's persona, BotMinter awareness, and skill usage instructions

3. **bm profiles init extracts minty**
   - Given `bm profiles init` running
   - When extraction completes
   - Then `~/.config/botminter/minty/` exists with prompt.md, config.yml, skills/

4. **Minty extraction separate from profiles**
   - Given `~/.config/botminter/`
   - When listing contents
   - Then `profiles/` and `minty/` are separate directories

5. **config.yml has valid structure**
   - Given `minty/config.yml`
   - When parsed as YAML
   - Then it contains valid configuration settings

## Metadata
- **Complexity**: Medium
- **Labels**: minty, config, sprint-6
- **Required Skills**: Rust, include_dir, persona writing, YAML
