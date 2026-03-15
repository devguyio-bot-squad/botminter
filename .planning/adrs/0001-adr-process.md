---
status: accepted
date: 2026-03-13
decision-makers: operator (ahmed), claude
---

# ADR Format: Spotify-style with Anti-patterns

## Problem

BotMinter needs a systematic way to document architectural decisions. The codebase is primarily developed by LLM coding agents (Claude Code via Ralph Orchestrator), which lose context across sessions and tend to re-propose rejected approaches. How should ADRs be structured so LLM agents comply with past decisions instead of re-deriving them?

## Constraints

* Decisions must live alongside the code in `.planning/adrs/`, not in a wiki
* Plain markdown, no special tooling required
* Must support a decision lifecycle (proposed, accepted, deprecated, superseded)
* LLM agents must be able to quickly determine: what was decided, what was rejected, and what mistakes to avoid
* Format must be self-contained — no "see other document for context"

## Decision

Use a Spotify-inspired ADR format with three additions optimized for LLM consumption:

1. **Constraints section** — hard requirements stated upfront, before the decision. LLMs treat these as invariants.
2. **Rejected Alternatives** — explicitly state what was NOT chosen and WHY. Prevents LLMs from re-proposing rejected approaches.
3. **Anti-patterns** — specific mistakes to avoid. The single most impactful section for LLMs — without it, a future session will propose the exact approach that was tried and failed.

The decision appears early in the document (after Problem and Constraints), not after pages of options analysis. LLMs scanning quickly get the answer immediately.

### Conventions

- **Numbering:** Sequential integers, four-digit zero-padded (`0001`, `0002`, ...). Never reuse a number.
- **File naming:** `NNNN-kebab-case-title.md` (e.g., `0001-adr-process.md`)
- **Location:** `.planning/adrs/`
- **Immutability:** Accepted ADRs are not edited for content. If a decision changes, write a new ADR that supersedes the old one.
- **When to write an ADR:** Any architectural decision with long-term impact — technology choices, structural patterns, interface contracts, process conventions. When in doubt, write one; they are cheap.
- **Status lifecycle:** `proposed` → `accepted` → optionally `deprecated` or `superseded by [ADR-NNNN]`
- **Index:** Update `README.md` whenever an ADR is added or its status changes.
- **Template:** Use `adr-template.md` as the starting point.

## Rejected Alternatives

### MADR 4.0.0

Rejected because: balanced pros/cons per option invites LLMs to re-evaluate instead of comply.

* Good structured sections, but the decision is buried after options analysis
* "Decision Drivers" are weaker than explicit Constraints — LLMs treat drivers as suggestions, constraints as rules
* No anti-patterns section — the most important section for LLM compliance

### Nygard-style ADR

Rejected because: no space for rejected alternatives or anti-patterns.

* Simple (Context → Decision → Consequences) but too terse
* LLMs cannot learn what NOT to do from a Nygard ADR

### Y-statements

Rejected because: one sentence per decision leaves no room for rationale or anti-patterns.

## Consequences

* ADRs are slightly longer than Nygard-style but shorter than MADR (no per-option pros/cons tables)
* LLM agents can grep for "Anti-patterns" or "Rejected" to quickly find guardrails
* Existing ADRs (0002-0004) are ported to the new format

## Anti-patterns

* **Do NOT** use balanced pros/cons tables for each option — LLMs re-evaluate them instead of accepting the decision. State the decision, then list rejections with reasons.
* **Do NOT** omit the Anti-patterns section — it is the primary mechanism for preventing LLMs from repeating known mistakes.
* **Do NOT** reference external documents for critical context — LLMs lose cross-document references across sessions. Each ADR must be self-contained.
