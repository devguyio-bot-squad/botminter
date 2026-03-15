# Use MADR 4.0.0 for Architecture Decision Records

---
status: accepted
date: 2026-03-08
decision-makers: [BotMinter maintainers]
---

## Context and Problem Statement

BotMinter needs a systematic way to document architectural decisions so future contributors understand the rationale behind design choices. As the project grows through milestones, decisions accumulate -- without a record, the "why" behind the codebase becomes tribal knowledge that is easily lost.

How should we record architectural decisions?

## Decision Drivers

* Decisions must be discoverable alongside the code (not in a wiki or external tool)
* Format must be lightweight -- plain markdown, no special tooling required
* Must support a decision lifecycle (proposed, accepted, deprecated, superseded)
* Should encourage capturing alternatives considered, not just the final choice

## Considered Options

* MADR 4.0.0 (Markdown Any Decision Record)
* Nygard-style ADR (original 2011 format)
* Y-statements (one-sentence decision records)
* No formal ADR practice

## Decision Outcome

Chosen option: "MADR 4.0.0", because it provides structured sections (Decision Drivers, Considered Options with Pros/Cons, Consequences, Confirmation) that make decisions self-documenting without requiring external context.

### Consequences

* Good, because MADR 4.0.0 is the most widely adopted ADR template with active maintenance
* Good, because optional sections allow lightweight ADRs for simple decisions while supporting thorough analysis for complex ones
* Good, because YAML front matter enables machine-readable status and date tracking
* Neutral, because the template is slightly more verbose than Nygard-style (mitigated by optional sections)
* Bad, because team members need to learn the format (mitigated by the template file at `adr-template.md`)

### Confirmation

The existence of this ADR (0001) and subsequent ADRs using the MADR format confirms adoption. The ADR index in `README.md` tracks all records.

## Pros and Cons of the Options

### MADR 4.0.0

* Good, because structured sections guide thorough analysis
* Good, because YAML front matter supports tooling integration
* Good, because widely adopted across the industry
* Neutral, because more verbose than minimal formats
* Bad, because template has many optional sections that could feel heavyweight for simple decisions

### Nygard-style ADR

* Good, because extremely simple (Title, Status, Context, Decision, Consequences)
* Good, because the original ADR format with broad recognition
* Bad, because no structured space for alternatives considered
* Bad, because no YAML front matter for machine-readable metadata

### Y-statements

* Good, because ultra-concise (one sentence per decision)
* Bad, because too terse to capture meaningful rationale
* Bad, because no standard template or community tooling

### No formal ADR practice

* Good, because zero overhead
* Bad, because architectural decisions become tribal knowledge
* Bad, because new contributors cannot understand design rationale
* Bad, because decisions get relitigated without record of prior analysis

## More Information

### Conventions

- **Numbering:** Sequential integers, four-digit zero-padded (`0001`, `0002`, ...). Never reuse a number.
- **File naming:** `NNNN-kebab-case-title.md` (e.g., `0001-adr-process.md`)
- **Location:** `.planning/adrs/`
- **Immutability:** Accepted ADRs are not edited. If a decision changes, write a new ADR that supersedes the old one (set the old ADR's status to `superseded by [ADR-NNNN](NNNN-title.md)`).
- **When to write an ADR:** Any architectural decision with long-term impact -- technology choices, structural patterns, interface contracts, process conventions. When in doubt, write one; they are cheap.
- **Status lifecycle:** `proposed` -> `accepted` -> optionally `deprecated` or `superseded by [ADR-NNNN]`
- **Index:** Update `README.md` whenever an ADR is added or its status changes.
- **Template:** Use `adr-template.md` as the starting point for new ADRs.
