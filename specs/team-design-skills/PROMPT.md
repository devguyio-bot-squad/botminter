# Objective

Implement the team design skills suite — a set of day-2 operational skills for the team-manager role and Minty that enable operators to evolve their teams after initial setup.

## Spec Directory

All task files: `specs/team-design-skills/tasks/`

## Execution Order

1. `task-01-team-agreements-convention.code-task.md` — Foundation: directory structure, file format, knowledge docs in both profiles
2. `task-02-retrospective-skill.code-task.md` — Team-manager skill: guided retro → agreements + action items
3. `task-03-role-management-skill.code-task.md` — Team-manager skill: add/remove/inspect roles
4. `task-04-member-tuning-skill.code-task.md` — Team-manager skill: tune PROMPT, CLAUDE, hats, skills, PROCESS
5. `task-05-process-evolution-skill.code-task.md` — Team-manager skill: workflow and status lifecycle changes
6. `task-06-team-design-hub-skill.code-task.md` — Team-manager skill: entry point routing to skills 2-5
7. `task-07-profile-design-skill.code-task.md` — Minty skill: profile-level design and troubleshooting

Tasks 2-5 are independent and can be parallelized. Task 6 depends on 2-5. Task 7 is independent of 2-6.

## Key Constraints

- **Read `knowledge/claude-code-skill-development-guide.md` FIRST** — all SKILL.md files MUST comply with this guide
- Skills are SKILL.md files — pure markdown, no Rust code needed
- Team-manager skills go in `profiles/<profile>/roles/team-manager/coding-agent/skills/<skill-name>/SKILL.md` for BOTH `scrum` and `scrum-compact` profiles
- Minty skills go in `minty/.claude/skills/<skill-name>/SKILL.md`
- Follow existing SKILL.md format (YAML frontmatter with name, description, metadata)
- **Progressive disclosure**: SKILL.md body must stay under 5,000 words. Move detailed procedures, diagnostic trees, validation rules, and examples to `references/` subdirectory within each skill folder
- **Description field**: Must include both what the skill does AND trigger phrases ("Use when...")
- **No README.md** inside skill folders — all docs go in SKILL.md or references/
- The agreements convention adds directories and knowledge files to both profiles
- After adding agreements/ dirs, verify profile extraction still works (`just unit`)
- All skills must reference the team agreements convention for decision records
- Team-manager skills must be registered in ralph.yml skill dirs

## Acceptance Criteria

- All 7 tasks completed with status: complete in frontmatter
- `just unit` passes (profile extraction includes agreements/)
- SKILL.md files exist at correct paths in both profiles
- Minty profile-design skill exists at `minty/.claude/skills/profile-design/`
- PROCESS.md in both profiles updated to reference agreements convention
- Team-manager ralph.yml in both profiles updated with new skill dirs

## Completion Condition

Done when all 7 tasks pass their acceptance criteria and `just unit` is green.
