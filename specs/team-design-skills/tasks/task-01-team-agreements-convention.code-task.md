---
status: pending
created: 2026-03-21
started: null
completed: null
---
# Task: Team Agreements Convention

## Description
Define a shared convention for tracking team decisions, process changes, and agreements. This is the foundational artifact that all other team design skills read and write. Agreements are like lightweight ADRs but scoped to team-level decisions: role changes, process tweaks, retrospective outcomes, and team norms.

## Background
Today, process evolution in botminter is informal — either a PR on the team repo or a direct edit. There is no structured record of *why* changes were made, who participated, or what alternatives were considered. The knowledge-manager skill handles knowledge and invariant files, but there is no convention for decision records.

Agreements bridge the gap between retros (what happened) and changes (what we do about it). They provide traceability: "we changed the review process because retro #3 showed reviews were blocking stories for 48+ hours."

Both the team-manager (inside the team) and Minty (outside, for profile-level design) will read/write agreements, so the format must be profile-agnostic.

## Reference Documentation
**Required:**
- Existing knowledge-manager skill: `profiles/scrum/skills/knowledge-manager/SKILL.md`
- Existing PROCESS.md (scrum-compact): `profiles/scrum-compact/PROCESS.md` (see "Process Evolution" section)
- Existing PROCESS.md (scrum): `profiles/scrum/PROCESS.md`

## Technical Requirements

1. **Directory structure**: `agreements/` at the team repo root, with subdirectories:
   - `agreements/decisions/` — formal team decisions (role changes, process changes, tool adoption)
   - `agreements/retros/` — retrospective summaries (output of the retrospective skill)
   - `agreements/norms/` — living team norms/working agreements (e.g., "we prefer small PRs", "all designs need diagrams")

2. **File format**: Markdown with YAML frontmatter. Sequential numbering: `NNNN-<kebab-case-title>.md`
   ```yaml
   ---
   id: 1
   type: decision | retro | norm
   status: proposed | accepted | superseded
   date: 2026-03-21
   participants: [operator, team-manager]
   supersedes: null  # id of previous agreement if replacing one
   refs: []          # related issue numbers, retro ids, etc.
   ---
   # Title

   ## Context
   Why this decision was needed.

   ## Decision
   What was decided.

   ## Alternatives Considered
   What else was considered and why it was rejected.

   ## Consequences
   Expected outcomes, tradeoffs, follow-up actions.
   ```

3. **Norm format** (simpler, for living agreements):
   ```yaml
   ---
   id: 1
   type: norm
   status: active | retired
   date: 2026-03-21
   refs: []
   ---
   # Norm Title

   **Agreement:** One-line statement of the norm.

   **Rationale:** Why this norm exists.

   **Adopted:** Date and context (e.g., "retro #2, 2026-03-15").
   ```

4. **Lifecycle**:
   - Decisions: `proposed` -> `accepted` (or rejected/withdrawn) -> optionally `superseded` by a later decision
   - Norms: `active` -> `retired` when no longer relevant
   - Retros: Always `accepted` (they're records of what happened, not proposals)

5. **Integration points**:
   - The retrospective skill writes to `agreements/retros/`
   - The process evolution skill writes decisions to `agreements/decisions/` before modifying PROCESS.md
   - The role management skill writes decisions to `agreements/decisions/` before adding/removing roles
   - The member tuning skill can reference agreements as justification for changes

6. **Profile integration**: Add `agreements/` to the profile directory structure. Both `scrum` and `scrum-compact` profiles should include an empty `agreements/` tree with a README explaining the convention.

7. **Convention documentation**: Create a `knowledge/team-agreements.md` knowledge file in each profile that documents the convention, so all team members understand the format.

## Dependencies
- None (this is the foundational convention)

## Implementation Approach

1. Create the `agreements/` directory structure in both profile templates (`profiles/scrum/` and `profiles/scrum-compact/`)
2. Write `knowledge/team-agreements.md` documenting the convention for each profile
3. Add a seed README in `agreements/` explaining the directory purpose
4. Update PROCESS.md in both profiles to reference the agreements convention under "Process Evolution"
5. Ensure `profile.rs` extraction includes the `agreements/` directory

## Acceptance Criteria

1. **Convention is documented**
   - Given a freshly extracted team repo
   - When I look at `knowledge/team-agreements.md`
   - Then I find the full convention documented (format, lifecycle, directory structure)

2. **Directory structure exists in profiles**
   - Given the `scrum` and `scrum-compact` profiles
   - When I inspect `agreements/`
   - Then I find `decisions/`, `retros/`, `norms/` subdirectories with `.gitkeep` files and a README

3. **PROCESS.md references agreements**
   - Given the updated PROCESS.md
   - When I read the "Process Evolution" section
   - Then it references the team agreements convention and explains when to create a decision record

4. **Profile extraction includes agreements**
   - Given `bm init` extracts a profile
   - When the team repo is created
   - Then the `agreements/` directory and knowledge file are present

5. **Unit tests**
   - Given existing profile extraction tests
   - When run
   - Then the `agreements/` directory is verified as part of the extracted profile structure

## Metadata
- **Complexity**: Low
- **Labels**: convention, team-management, profiles
- **Required Skills**: Markdown, profile structure, profile extraction (Rust)
