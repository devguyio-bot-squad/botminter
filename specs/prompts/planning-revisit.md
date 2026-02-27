You are revisiting the plan for a completed milestone to make targeted changes.

## Directory Convention

Planning artifacts are organized under `specs/` with categorized nesting:

```
specs/
  milestones/                           # Full PDD initiatives
    completed/                          # Shipped milestones
    <name>/                             # Actively planned milestones (next up)
    future/                             # Recognized ideas, not yet committed
      <name>/
  tasks/                                # Standalone task batches
    completed/
    <name>/
```

Milestones use descriptive kebab-case names — no numbered prefixes. `docs/content/roadmap.md` is the canonical status tracker.

## Detection

Before doing anything, orient yourself and detect which milestone the user likely wants to revisit:
1. Read `docs/content/roadmap.md` for milestone status overview.
2. List `specs/milestones/completed/` — these are the candidates for revisiting.
3. Also check `specs/milestones/` root level for active milestones that may have partial artifacts worth revisiting.
4. If the user's prompt or recent conversation provides hints (e.g., mentions a milestone name or topic), use that to narrow down. Otherwise, infer the most recently completed milestone (most recently modified with complete artifacts).

Present your detection to the user: state which milestone you think they want to revisit and why. Ask if that's correct, and what the reason for revisiting is (e.g., "learnings from implementation changed assumptions", "need to add a step", "acceptance criteria need updating").

Then orient further:
- Read all artifacts in the milestone's planning directory (design.md, plan.md, PROMPT.md, etc.).
- Read `specs/master-plan/summary.md` for cross-milestone context.
- If the user mentions learnings from a later milestone, read that milestone's artifacts too.

Present a summary of the existing plan and what you think needs to change based on the user's reason. Propose specific changes and wait for the user's direction before modifying any files.

## Rules

- **User-driven:** Never modify files without explicit user confirmation.
- **Surgical changes:** Prefer targeted edits over rewriting entire documents.
- **Trace impact:** If a change affects downstream milestones, flag it.
- **Planning only:** No implementation, no running code.
