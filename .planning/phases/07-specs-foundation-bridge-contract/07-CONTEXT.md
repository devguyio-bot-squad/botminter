# Phase 7: Specs Foundation & Bridge Contract - Context

**Gathered:** 2026-03-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Formally specify the bridge plugin contract so any developer can read the spec and build a conformant bridge implementation. Establish ADR practice with MADR 4.0.0. Clean up legacy `specs/` directory. No Rust runtime code — this phase produces specification documents, ADRs, and conformance tests only.

</domain>

<decisions>
## Implementation Decisions

### ADR Practice
- MADR 4.0.0 format in `.planning/adrs/`
- Sequential integer numbering: `0001-title.md`, `0002-title.md`
- Include MADR template file for future ADRs
- **First ADR (`0001`) is the meta-ADR**: documents the ADR process itself — format, conventions, numbering, lifecycle, when to write one, how to supersede. This is the single document anyone needs to read before writing an ADR.
- **`README.md` in `.planning/adrs/`** serves as the ADR index — lists all ADRs with number, title, status, and date. Updated whenever an ADR is added or superseded.
- Two additional ADRs required: bridge abstraction design decisions, and Ralph robot backend decisions
- ADRs are immutable records — superseded by new ADRs, not edited after acceptance

### Specs Practice
- **`README.md` in `.planning/specs/`** serves as the spec index — lists all specs with name, status, and summary. Updated whenever a spec is added.
- **Meta-spec document** in `.planning/specs/` defines the spec discipline itself — format conventions, when to write a spec, RFC 2119 usage, conformance levels, directory structure per spec, and how specs relate to ADRs. The single document anyone needs to read before writing a spec.

### Bridge Spec Structure
- Spec document lives in `.planning/specs/bridge/`
- RFC 2119 conformance language (MUST, SHOULD, MAY)
- Single primary spec document covering: `bridge.yml` format, `schema.json` contract, lifecycle operations, identity operations, file-based config exchange
- Complete example `bridge.yml` and `schema.json` ship alongside the spec as reference
- Spec explicitly distinguishes local bridges (full lifecycle: start, stop, health) from external bridges (identity-only: onboard, rotate, remove)

### Bridge Contract Shape
- `bridge.yml` declares all integration points — lifecycle commands, identity commands, config schema reference, bridge type
- Commands are Justfile recipes — no hardcoded command names in the contract
- `schema.json` validates bridge-specific configuration values
- Config exchange uses file-based output: `$BRIDGE_CONFIG_DIR/config.json`, not stdout
- Identity commands: onboard, rotate-credentials, remove (per-user bot lifecycle)
- Lifecycle commands: start, stop, health (local bridges only)

### Conformance Test Suite
- Rust conformance tests in `crates/bm/tests/` that parse `bridge.yml` and `schema.json` and validate structure against the spec
- A stub/no-op bridge implementation ships with the spec as the reference fixture for conformance testing
- Minimal scope: structure validation, command presence, schema validation — not runtime behavior (that's Phase 8)

### Legacy specs/ Cleanup
- Remove `specs/master-plan/`, `specs/milestones/`, `specs/prompts/`, `specs/tasks/` from tree
- Preserved in git history — no migration needed
- `specs/design-principles.md` and `specs/presets/` also removed (design principles captured in PROJECT.md, presets are Ralph-specific)

### Spec Document Conventions
- Every spec MUST include a non-goals section — explicitly state what the spec does NOT cover
- Inline snippets in the spec text for illustration, plus complete working examples as separate files in the spec directory (e.g., `.planning/specs/bridge/examples/`)
- Conformance tests validate structure only — `bridge.yml` and `schema.json` parse correctly and contain all required fields. No command execution at this phase.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/bm/tests/integration.rs`: Existing test patterns for parsing YAML/JSON structures — conformance tests follow the same style
- `profiles/scrum-compact/botminter.yml`: Example of YAML manifest with schema — similar pattern to `bridge.yml`

### Established Patterns
- Profile manifests use YAML (`botminter.yml`) — bridge manifest follows same convention
- Schema validation exists conceptually (profile schema in `.schema/`) — `schema.json` extends this pattern
- Justfile recipes used in profiles (`formations/`) — bridge commands follow the same recipe pattern

### Integration Points
- `.planning/` directory already established as the home for project planning artifacts — ADRs and specs nest naturally here
- `specs/` directory exists but will be emptied — contents are historical PDD artifacts from pre-GSD workflow

</code_context>

<specifics>
## Specific Ideas

- Knative-style specs as mentioned in PROJECT.md — the bridge spec should feel like a Knative resource spec (declarative, versioned, with clear conformance levels)
- The spec should be readable enough that a third-party developer could implement a bridge without reading BotMinter source code

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 07-specs-foundation-bridge-contract*
*Context gathered: 2026-03-08*
