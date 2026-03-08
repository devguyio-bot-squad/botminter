# Invariant File Format

Invariants are constraints that coding agents MUST satisfy. They live as `.md` files in `invariants/` directories at team, project, and member scopes.

## Principles

**Small and focused.** Each invariant captures one constraint. Prefer multiple small invariants at the right scope level over one large invariant that covers many concerns. Large invariants pollute context — agents read these in prompts alongside other context, and every token competes.

**Concrete over abstract.** Show what compliance and violation look like through examples. Agents comply better with rules they can pattern-match against, not just abstract statements.

**Honest scope.** State what the invariant applies to AND what it does not apply to. This prevents false positives and wasted agent effort.

## Format

Every invariant file follows this structure:

````markdown
# <Title>

<One-line summary of the constraint.>

## Rule

<The constraint. Uses MUST / MUST NOT / SHOULD language.
 Precise enough that two people reading it would independently agree
 on whether a given artifact complies or not.>

## Applies To

<What artifacts, roles, or activities this covers.
 Also state when it does NOT apply — explicit exclusions.>

## Examples

**Compliant:**
<What following this rule looks like. Use code snippets, file excerpts,
 or prose — whichever is most natural for the invariant.>

**Violating:**
<What breaking this rule looks like. Same form as compliant.>

## Rationale

<Why this invariant exists. Keep it brief — one paragraph.
 Include the origin story if it helps (e.g., a bug that shipped).>
````

## Section Guide

| Section | Required | Purpose |
|---------|----------|---------|
| Title + summary | Yes | Quick identification in file listings |
| Rule | Yes | The normative constraint |
| Applies To | Yes | Scope and exclusions |
| Examples | Yes | Compliant and violating cases |
| Rationale | Yes | Why it matters |

All five sections are required. If you cannot write concrete examples, the rule is likely too vague — sharpen it until you can.

## Sizing

A good invariant fits on one screen (~40-60 lines). If it grows beyond that, split it into multiple invariants at the appropriate scope level.
