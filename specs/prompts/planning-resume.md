You are resuming the planning of a milestone that was started using Prompt-Driven Development (PDD).

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

Before doing anything, ask the user:
1. Which milestone? (e.g., by name or topic)
2. Any specific context about where they left off, or should you figure it out from the files?

Then orient yourself:
- List `specs/milestones/` (root level, not `completed/` or `future/`) to find active milestones.
- Read all files in the matching milestone directory to understand current state.
- Read `specs/master-plan/summary.md` for cross-milestone context.
- Read `docs/content/roadmap.md` for current project status.
- Determine which PDD phase was last completed based on what artifacts exist:
  - Only `requirements.md` with Q&A entries → mid-requirements or ready for research
  - `research/` has topic files → mid-research or ready for design
  - `design.md` exists → mid-design review or ready for plan
  - `plan.md` exists → mid-plan review or ready for summary
  - `summary.md` exists → nearly done, may need PROMPT.md

Present your assessment of the current state to the user and ask where they want to pick up.

## Process Rules

- **User-driven flow:** Never proceed to the next step without explicit user confirmation.
- **Iterative:** The user can move between different stages at any time.
- **Mermaid diagrams:** Include diagrams for architectures, data flows, and component relationships.

## PDD Steps Reference

1. Requirements clarification — one question at a time, append to requirements.md
2. Research — document in research/ as separate topic files
3. Iteration checkpoint — summarize state, user decides next phase
4. Design — write standalone design.md
5. Implementation plan — write plan.md with incremental steps
6. Summary — write summary.md
7. PROMPT.md — optional, for autonomous implementation
