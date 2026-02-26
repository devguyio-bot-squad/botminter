# Objective

Advance all architecture work for the assigned project. Architecture work includes design, planning, and story breakdown for epics.

## Work Scope

Handle architecture phases:
- Epic design (produce design documents)
- Story planning (decompose designs into story breakdowns)
- Story breakdown execution (create story issues from approved breakdowns)
- Epic monitoring (track story completion, fast-forward to acceptance when all stories are done)

## Completion Condition

Done when no architecture-phase issues remain actionable for the assigned project. An issue is actionable when:
- It belongs to the assigned project (identified by `project/<project-name>` label)
- Its current status is an architecture phase
- It is not waiting on human review or approval
- It is not waiting on another role

Skip issues waiting on human gates or other team members.

## Work Location

GitHub issues on the team repository, filtered by the assigned project's label and architecture status values.
