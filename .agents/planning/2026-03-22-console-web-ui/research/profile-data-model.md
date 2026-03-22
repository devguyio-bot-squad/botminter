# Profile Data Model Research

## Team Repo Layout (after init + hire)

```
my-team/                              # Team repo root
  botminter.yml                       # Profile manifest (master metadata)
  PROCESS.md                          # Process definition (status lifecycle)
  CLAUDE.md                           # Team-level context

  knowledge/                          # Team-level knowledge
  invariants/                         # Team-level invariants
  agreements/                         # Team agreements (decisions, retros, norms)

  coding-agent/                       # Coding agent config
    settings.json
    agents/.gitkeep
    skills/                           # Team-level skills (gh, status-workflow)

  skills/                             # Additional team-level skills
  bridges/                            # Bridge configurations (telegram, tuwunel, rocketchat)
  formations/                         # Deployment formation configs (local, k8s)
  ralph-prompts/                      # Reference prompts for Ralph
  projects/<project>/                 # Project-specific config
  brain/                              # Brain process config (scrum-compact only)

  members/<role>-<suffix>/            # Hired members
    botminter.yml                     # Member manifest (role, name, emoji)
    PROMPT.md                         # Work objective
    CLAUDE.md                         # Member-specific context
    ralph.yml                         # Ralph config (hats, skills, guardrails)
    knowledge/.gitkeep
    invariants/
    projects/.gitkeep
    hats/<hat_name>/knowledge/        # Hat-specific knowledge
    coding-agent/agents/, skills/     # Member-level agent config
```

## Key Data Objects

### ProfileManifest (botminter.yml)
- name, display_name, description, version, schema_version
- coding_agents: Map<String, CodingAgentDef>
- roles: Vec<RoleDef> (name + description)
- labels: Vec<LabelDef> (name, color, description)
- statuses: Vec<StatusDef> (GitHub Projects v2 status values)
- views: Vec<ViewDef> (board views)
- bridges, projects, operator

### Status Lifecycle
Epic: po:triage -> po:backlog -> arch:design -> lead:design-review -> po:design-review -> arch:plan -> lead:plan-review -> po:plan-review -> arch:breakdown -> lead:breakdown-review -> po:ready -> arch:in-progress -> po:accept -> done

Story: dev:ready -> qe:test-design -> dev:implement -> dev:code-review -> qe:verify -> arch:sign-off -> po:merge -> done

### Profile Variations
- **scrum**: architect (4 hats), human-assistant (0 hats), team-manager (1 hat + 6 skills)
- **scrum-compact**: superman (14 hats), team-manager (1 hat + 6 skills)

### Knowledge Resolution (5 additive levels)
1. Team: knowledge/
2. Project: projects/<project>/knowledge/
3. Member: members/<member>/knowledge/
4. Member+Project: members/<member>/projects/<project>/knowledge/
5. Hat: members/<member>/hats/<hat>/knowledge/
