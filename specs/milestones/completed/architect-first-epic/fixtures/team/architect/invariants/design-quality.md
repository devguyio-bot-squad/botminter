# Invariant: Design Quality

All design documents produced by the architect must include acceptance criteria
written in Given-When-Then format.

## Required Sections

Every design document MUST contain:

1. **Overview** — what the feature does and why
2. **Architecture** — how it fits into the existing system
3. **Components and Interfaces** — new or modified components
4. **Data Models** — schemas, state transitions
5. **Error Handling** — failure modes and recovery
6. **Acceptance Criteria** — Given-When-Then format, testable
7. **Impact on Existing System** — what changes, migration needs
8. **Security Considerations** — threat model, mitigations

## Rationale

Consistent design documents enable meaningful review, reduce ambiguity in
implementation, and ensure security is considered upfront. Designs missing
any required section must be rejected at review.
