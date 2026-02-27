You are planning a new milestone using Prompt-Driven Development (PDD).

## Directory Convention

Planning artifacts are organized under `specs/` with categorized nesting:

```
specs/
  master-plan/                          # Project-level vision and research archive
  design-principles.md                  # Living design doc
  prompts/                              # Reusable PDD prompts (this file)
  presets/                              # Ralph preset configs

  milestones/                           # Full PDD initiatives
    completed/                          # Shipped milestones
      <name>/
    <name>/                             # Actively planned milestones (next up)
    future/                             # Recognized ideas, not yet committed
      <name>/

  tasks/                                # Standalone task batches (no full PDD)
    completed/                          # Finished task batches
      <name>/
    <name>/                             # Active task batches
```

**Rules:**
- Milestones use descriptive kebab-case names — NO numbered prefixes (e.g., `bm-cli/`, not `milestone-3-bm-cli/`).
- A **milestone** has full PDD artifacts: requirements.md, research/, design.md, plan.md, summary.md. It may also contain a `tasks/` subdir if the plan is broken into code-tasks.
- A **task batch** has PROMPT.md + `tasks/*.code-task.md`. Use for focused work that doesn't warrant full PDD.
- Completed work moves to `{milestones,tasks}/completed/<name>/` when shipped.
- Actively planned milestones live at `specs/milestones/<name>/` (root level).
- Future ideas live at `specs/milestones/future/<name>/` — promoted to root when committed to.
- `docs/content/roadmap.md` is the canonical status tracker — directory location is for organization, not status tracking.

## Detection

Before doing anything, orient yourself and detect which milestone the user likely wants to plan:
1. Read `docs/content/roadmap.md` for the current status of all milestones.
2. List `specs/milestones/` — directories at the root level (not inside `completed/` or `future/`) are actively planned.
3. List `specs/milestones/future/` — these are recognized ideas, candidates for promotion.
4. List `specs/milestones/completed/` — these are done, skip them.
4. Read `specs/master-plan/summary.md` for cross-milestone context and suggested next steps.
5. Identify which milestones (root or future) don't yet have substantial PDD artifacts (requirements, design, plan).
6. Infer the next milestone to plan based on the roadmap's "Planned" section and master plan's suggested next steps.
7. If the detected milestone is in `future/`, propose promoting it to root level as part of setup.

Present your detection to the user: state which milestone you think is next, why you think so, and ask if that's correct or if they had a different one in mind. Also ask if they have specific concerns, constraints, or topics to focus on.

If a planning directory already exists for the detected milestone and contains substantial artifacts, tell the user — they may want the resume prompt instead.

## Setup

Create the milestone directory at `specs/milestones/<name>/` with:
- `requirements.md` — Q&A record (initially empty header)
- `research/` — directory for research notes

Present the project structure to the user. Do NOT proceed until they confirm.

Then ask the user their preferred starting point:
- Requirements clarification (default)
- Preliminary research on specific topics
- Provide additional context first

Do NOT automatically start any phase without the user choosing.

## Process Rules

- **User-driven flow:** Never proceed to the next step without explicit user confirmation. At each transition, ask the user what they want to do next.
- **Iterative:** The user can move between requirements clarification and research at any time. Always offer this option at phase transitions.
- **Record as you go:** Append questions, answers, and findings to project files in real time — don't batch-write at the end.
- **Mermaid diagrams:** Include diagrams for architectures, data flows, and component relationships in research and design documents.
- **Sources:** Cite references and links in research documents when based on external materials.
- **Planning only:** This process produces planning artifacts. You MUST NOT implement code, run containers, execute scripts, or begin any implementation work.

## Steps

### 1. Requirements Clarification

Guide the user through questions to refine the milestone into a thorough specification.

- Ask ONE question at a time — do not list multiple questions.
- For each question: (1) append question to requirements.md, (2) present to user and wait, (3) append answer to requirements.md, (4) next question.
- Cover edge cases, user experience, technical constraints, and success criteria. Suggest options when the user is unsure.
- Ask the user if requirements clarification is complete before moving on.
- Offer the option to conduct research if questions arise that would benefit from investigation.

### 2. Research

Conduct research on technologies, libraries, existing code, or codebase patterns to inform the design.

- Propose a research plan to the user and incorporate their suggestions.
- Document findings in the milestone's `research/` directory as separate topic files.
- Periodically check in with the user to share findings and confirm direction.
- Summarize key findings before moving on.
- Offer to return to requirements clarification if research uncovers new questions.

### 3. Iteration Checkpoint

Summarize the current state of requirements and research, then ask the user:
- Proceed to design?
- Return to requirements clarification?
- Conduct additional research?

Support iterating between requirements and research as many times as needed.

### 4. Create Detailed Design

Create `design.md` in the milestone directory as a standalone document with:
- Overview
- Detailed Requirements (consolidated from requirements.md)
- Architecture Overview
- Components and Interfaces
- Data Models
- Error Handling
- Acceptance Criteria (Given-When-Then format for machine verification)
- Testing Strategy
- Appendices (Technology Choices, Research Findings, Alternative Approaches)

The design must be standalone — understandable without reading other files. Consolidate all requirements from requirements.md. Include an appendix summarizing research. Review with the user and iterate on feedback. Offer to return to requirements or research if gaps are identified.

### 5. Develop Implementation Plan

Create `plan.md` in the milestone directory — a numbered series of incremental implementation steps.

Guiding principle: Each step builds on previous steps, results in working demoable functionality, and follows TDD practices. No orphaned code — every step ends with integration. Core end-to-end functionality should be available as early as possible.

- Include a checklist at the top tracking each step.
- Format as "Step N:" with: objective, implementation guidance, test requirements, integration notes, and demo description.
- Ensure the plan covers all aspects of the design without duplicating design details.

### 6. Summarize and Present Results

Create `summary.md` in the milestone directory listing all artifacts, a brief overview, and suggested next steps.

### 7. Create PROMPT.md

Ask: "Would you like me to create a PROMPT.md for autonomous implementation of this milestone?"

If yes, create a concise PROMPT.md (under 100 lines) with:
- Objective statement
- Key requirements
- Acceptance criteria (Given-When-Then)
- Reference to the milestone's planning directory

## Troubleshooting

- **Requirements stall:** Suggest switching to a different aspect, provide examples, or pivot to research to unblock decisions.
- **Research limitations:** Document what's missing, suggest alternatives with available information, ask user for additional context.
- **Design complexity:** Break into smaller components, focus on core functionality first, suggest phased implementation, return to requirements to re-prioritize.
