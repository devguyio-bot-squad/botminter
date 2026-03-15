# Meta-Spec: Specification Discipline

This document defines the conventions for writing specifications in BotMinter.

## When to Write a Spec

Write a spec when defining:

- A new external contract (e.g., a plugin interface consumed by third-party implementors)
- A file format that external tools or bridge authors must produce or consume
- An interface boundary where implementations can vary independently

Do NOT write a spec for internal implementation details, internal APIs consumed only by BotMinter code, or design rationale (use an ADR for that).

## Relationship to ADRs

ADRs document the "why" -- the rationale behind a design decision, the alternatives considered, and the consequences accepted. Specs document the "what" -- the precise shape of a contract that implementations must satisfy.

An ADR may reference the spec it motivates. A spec may reference the ADR that explains its design rationale. They are complementary, not overlapping.

## Format Conventions

### Required Sections

Every spec MUST include:

1. **Title and version** -- clear name and `apiVersion` or spec version identifier
2. **Conformance** -- RFC 2119 boilerplate (see below)
3. **Overview** -- 2-3 sentence summary of what the spec defines
4. **Non-goals** -- explicitly state what the spec does NOT cover
5. **Specification body** -- the normative contract definition
6. **Examples** -- inline snippets illustrating key concepts

### Status

Each spec has a status:

- **Draft** -- under active development, subject to breaking changes
- **Accepted** -- stable, implementations may rely on it
- **Deprecated** -- superseded, implementations should migrate

### Versioning

Specs use `apiVersion` strings following the pattern `botminter.dev/v1alpha1`, `botminter.dev/v1beta1`, `botminter.dev/v1`. Alpha versions may introduce breaking changes. Beta versions aim for stability. Stable versions guarantee backwards compatibility.

## RFC 2119 Usage

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in spec documents are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

### Rules

- Conformance keywords MUST be UPPERCASE when used as defined in RFC 2119
- When used in their ordinary English sense (not as conformance terms), they SHOULD be lowercase
- Every spec MUST include the standard RFC 2119 boilerplate paragraph in a "Conformance" section near the top
- Use MUST sparingly -- only for requirements that are essential for interoperability

## Conformance Levels

- **MUST** -- absolute requirement. An implementation that violates a MUST requirement is non-conformant.
- **SHOULD** -- recommended. Valid reasons to deviate may exist, but the implications must be understood.
- **MAY** -- truly optional. Implementors decide based on their needs.

## Directory Structure

Each spec lives in its own subdirectory under `.planning/specs/`:

```
.planning/specs/
  README.md          # Spec index (this updates when specs are added)
  meta-spec.md       # This document
  <spec-name>/
    <spec-name>.md   # Primary spec document
    examples/        # Reference implementations and example files
```

The primary spec document is the normative source. Files in `examples/` are illustrative and MAY be used as conformance test fixtures, but the spec text takes precedence if there is a conflict.
