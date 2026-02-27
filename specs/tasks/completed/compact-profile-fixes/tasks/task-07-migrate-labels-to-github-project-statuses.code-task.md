---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: Migrate from labels to GitHub Projects v2 statuses

## Description
Replace the label-based status tracking (`status/po:triage`, `status/arch:design`, etc.) with GitHub Projects v2 status fields. Issues will be tracked via a GitHub Project board where status transitions happen through the Projects API instead of label swaps. This affects the `gh` skill, all hat instructions that perform status transitions, the board scanner's querying logic, and the `bm init` bootstrapping flow.

## Background
Currently, botminter tracks issue lifecycle state using labels following the `status/<role>:<phase>` convention. While functional, this has drawbacks:
- Labels are a flat, repo-global namespace — status labels clutter the label list alongside `kind/*` and `parent/*` labels
- No built-in board visualization — GitHub Projects v2 provides native Kanban views based on status fields
- Label transitions require two operations (remove old + add new) instead of one field update

GitHub Projects v2 uses a "Status" single-select field on project items. The `gh` CLI supports this via `gh project item-list`, `gh project item-edit`, and `gh project field-list` subcommands.

### Key research findings

**What works well:**
- `gh project item-list <N> --owner <OWNER> --query "status:Todo"` — filter items by status value
- `gh project item-edit` — update a single field on an item
- `gh project field-list` — discover field IDs and option IDs

**Significant complexity:**
- Status updates require **3 opaque ID lookups** before the actual edit: project node ID (`PVT_...`), status field ID (`PVTSSF_...`), and option ID for the target status value. These are not human-readable and must be resolved via `gh project view`, `gh project field-list`, etc.
- `gh issue list` **cannot** filter by project status — only `gh project item-list` can. The board scanner must switch from `gh issue list --label "status/..."` to `gh project item-list --query "status:..."`.
- Each `gh project item-edit` call updates **one field only**.
- The `project` token scope must be explicitly added: `gh auth refresh -s project`.

**Mitigation strategy:**
- Cache the project ID, field ID, and option ID mappings at the start of each board scan cycle (one-time cost per cycle)
- Create a helper script or convention in the `gh` skill that wraps the multi-step ID lookup into a single logical operation
- Define project status option names to match the existing phase names (e.g., "po:triage", "arch:design") for continuity

## Reference Documentation
**Required:**

Compact profile:
- `profiles/compact/agent/skills/gh/SKILL.md` — current gh skill (labels-based operations)
- `profiles/compact/members/superman/ralph.yml` — board scanner and all hat instructions referencing label transitions
- `profiles/compact/PROCESS.md` — status label convention definition
- `profiles/compact/botminter.yml` — label definitions in manifest

rh-scrum profile:
- `profiles/rh-scrum/agent/skills/gh/SKILL.md` — same gh skill pattern
- `profiles/rh-scrum/members/architect/ralph.yml` — architect board scanner + status transitions (lines 150, 217, 272, 320)
- `profiles/rh-scrum/members/human-assistant/ralph.yml` — human-assistant board scanner + status transitions (lines 107, 116, 127)
- `profiles/rh-scrum/PROCESS.md` — status label convention definition
- `profiles/rh-scrum/botminter.yml` — label definitions in manifest

**Additional References:**
- [gh project item-list docs](https://cli.github.com/manual/gh_project_item-list)
- [gh project item-edit docs](https://cli.github.com/manual/gh_project_item-edit)
- [gh project field-list docs](https://cli.github.com/manual/gh_project_field-list)
- [GitHub Projects filtering docs](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/filtering-projects)

## Technical Requirements

### 1. `bm init` changes (Rust CLI)
1. During GitHub repo creation, also create a GitHub Project (v2) with a "Status" single-select field
2. Populate the Status field options from the profile's status definitions (e.g., "po:triage", "arch:design", "arch:plan", etc.)
3. Newly created issues must be added to the project automatically (via `gh project item-add`)
4. Remove status label bootstrapping (labels like `status/*` are no longer needed; keep `kind/*` and `parent/*` labels)

### 2. `gh` skill rewrite (Profile)
1. Replace the "Status Transition" operation: instead of `gh issue edit --remove-label / --add-label`, use the `gh project item-edit` flow with cached IDs
2. Add a "Project Setup" section documenting how to resolve and cache IDs:
   - Project node ID: `gh project view <N> --owner <OWNER> --format json | jq -r '.id'`
   - Field ID + option IDs: `gh project field-list <N> --owner <OWNER> --format json`
3. Replace "Board View" operation: switch from `gh issue list --label` to `gh project item-list --query "status:..."` with JSON output
4. Keep "Create Issue" operation but add `gh project item-add` after issue creation
5. Document the `project` token scope requirement

### 3. Board scanner rewrite (Profile)
1. Replace `gh issue list ... --json ... labels` with `gh project item-list --query` for each status value
2. Cache the project ID and field/option mappings once per scan cycle (step 3, after team repo detection)
3. Status transitions use `gh project item-edit` instead of `gh issue edit --remove-label / --add-label`
4. Auto-advance logic updates to use project status transitions

### 4. All hat instructions update (Profile)
1. Every hat that does `gh issue edit --remove-label "status/..." --add-label "status/..."` must switch to the project item-edit flow
2. Hats should reference a cached `$STATUS_FIELD_ID` and `$OPTION_<status>` variables resolved by the board scanner or skill helper
3. Update PROCESS.md to document the new status mechanism

### 5. `botminter.yml` manifest update
1. Status definitions should be expressed as project field options, not labels
2. Keep `kind/*` and `parent/*` as labels (they're not status)

## Dependencies
- Task 3 (fix board scanner sync) — the board scanner instructions will be rewritten anyway
- Task 4 (remove LOOP_COMPLETE) — same files being modified
- Recommended: complete tasks 1-4 first, then do this migration on clean files

## Implementation Approach
1. Update `bm init` to create a GitHub Project with Status field options derived from the profile manifest
2. Rewrite the `gh` skill's status transition and board view operations (both profiles share the same skill structure)
3. Rewrite all board scanners to query via `gh project item-list` and cache ID mappings
4. Update each hat's status transition commands in both profiles
5. Update both profiles' PROCESS.md to reflect the new mechanism
6. Update both profiles' `botminter.yml` manifest format for status definitions
7. Test end-to-end with both compact and rh-scrum profiles

## Acceptance Criteria

1. **`bm init` creates a GitHub Project with Status field**
   - Given a new team initialization with GitHub enabled
   - When `bm init` completes
   - Then a GitHub Project v2 exists with a "Status" field containing all status options from the profile

2. **Board scanner queries via project status**
   - Given the board scanner hat running a scan cycle
   - When checking for work items
   - Then it uses `gh project item-list --query "status:..."` (not `gh issue list --label`)

3. **Status transitions use project item-edit**
   - Given any hat performing a status transition
   - When moving an issue from one status to another
   - Then it uses `gh project item-edit` with the correct field and option IDs

4. **New issues are added to the project**
   - Given a hat creating a new issue (e.g., arch_breakdown creating stories)
   - When the issue is created via `gh issue create`
   - Then it is also added to the project via `gh project item-add`

5. **No status/* labels remain**
   - Given the updated profile
   - When searching for `--label "status/` or `--remove-label "status/` or `--add-label "status/`
   - Then zero matches are found in hat instructions

6. **kind/* and parent/* labels still work**
   - Given the updated profile
   - When creating issues with `kind/epic`, `kind/story`, or `parent/<N>` labels
   - Then labels are still used (these are not migrated to project fields)

7. **gh skill documents ID caching pattern**
   - Given the updated `gh` SKILL.md
   - When reading the Status Transition section
   - Then it documents how to resolve and cache project ID, field ID, and option IDs

8. **Token scope documented**
   - Given the updated `gh` SKILL.md prerequisites
   - When reading requirements
   - Then the `project` token scope requirement is documented

## Metadata
- **Complexity**: High
- **Labels**: feature, profile, compact, gh-skill, breaking-change
- **Required Skills**: GitHub Projects v2 API, gh CLI, YAML, markdown, Rust
