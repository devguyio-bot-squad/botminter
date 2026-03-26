# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for the BotMinter project. ADRs document significant architectural decisions along with their context and consequences.

ADRs use a Spotify-inspired format optimized for LLM-driven development: Problem → Constraints → Decision → Rejected Alternatives → Consequences → Anti-patterns. See [adr-template.md](adr-template.md) for the template and [ADR-0001](0001-adr-process.md) for conventions.

## Index

| Number | Title | Status | Date |
|--------|-------|--------|------|
| [0001](0001-adr-process.md) | ADR Format: Spotify-style with Anti-patterns | Accepted | 2026-03-13 |
| [0002](0002-bridge-abstraction.md) | Shell Script Bridge with YAML Manifest | Accepted | 2026-03-08 |
| [0003](0003-ralph-robot-backend.md) | Bridge Outputs Credentials, BotMinter Maps to Ralph | Accepted | 2026-03-08 |
| [0004](0004-scenario-based-e2e-tests.md) | Scenario-Based E2E Tests Over Feature-Fragment Tests | Accepted | 2026-03-09 |
| [0005](0005-e2e-test-environment-and-isolation.md) | E2E Test Environment Management and Isolation Patterns | Accepted | 2026-03-14 |
| [0006](0006-directory-modules.md) | Directory Modules as the Only Module Organization | Accepted | 2026-03-14 |
| [0007](0007-domain-command-layering.md) | Domain Modules and Command Layering | Accepted | 2026-03-15 |
| [0008](0008-local-formation-as-first-class-concept.md) | Local Formation — Deployment Strategy for Running Members Locally | Proposed | 2026-03-15 |
| [0009](0009-manual-integration-tests.md) | Exploratory Integration Tests for Infrastructure-Touching Behavior | Accepted | 2026-03-20 |
| [0010](0010-agent-tools-namespace.md) | Separate `bm-agent` binary for agent-facing CLI | Proposed | 2026-03-21 |
| [0011](0011-github-app-per-member-identity.md) | Per-member GitHub App identity replaces shared PAT | Proposed | 2026-03-24 |
