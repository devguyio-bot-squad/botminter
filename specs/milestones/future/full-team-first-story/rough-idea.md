# Deferred: Full Team + First Story

> Originally Milestone 3, now Milestone 4 in the master plan. Deferred in favor of the `bm` CLI + daemon milestone (M3).

## Scope

Adds dev, QE, reviewer members to the `rh-scrum` profile. Proves full story kanban end-to-end with TDD (QE writes tests → dev implements → QE verifies → reviewer reviews → architect signs off → PO merges). First real knowledge accumulation.

## Why Deferred

The team coordination model needs proper tooling infrastructure (CLI, daemon, member lifecycle management) before adding more members. The `bm` CLI + daemon milestone provides the control plane that makes multi-member coordination operationally viable.

## Resumption

When this work resumes, it should build on top of the `bm` CLI + daemon infrastructure rather than Justfile recipes. Member launching, workspace creation, and coordination will go through the `bm` tool.
