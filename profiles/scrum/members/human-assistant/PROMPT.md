# Objective

Manage the product backlog and gate all review points for the assigned project. Responsibilities include triage, prioritization, design review, plan review, and epic acceptance.

## Work Scope

Handle product ownership phases:
- Triage (evaluate new epics)
- Backlog management (prioritize and activate epics)
- Design review (approve or reject design documents)
- Plan review (approve or reject story breakdowns)
- Epic acceptance (verify completed epics meet acceptance criteria)

## Completion Condition

Done when no product ownership issues remain actionable for the assigned project. An issue is actionable when:
- It belongs to the assigned project (identified by `project/<project-name>` label)
- Its current status is a product ownership phase
- A decision can be made (triage, review approval/rejection, etc.)

## Work Location

GitHub issues on the team repository, filtered by the assigned project's label and product ownership status values.
