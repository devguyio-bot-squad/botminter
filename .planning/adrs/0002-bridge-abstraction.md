# Shell Script Bridge with YAML Manifest

---
status: accepted
date: 2026-03-08
decision-makers: [BotMinter maintainers]
---

## Context and Problem Statement

BotMinter needs pluggable communication bridges so operators can connect their agentic teams to different chat platforms (Telegram, Rocket.Chat, future services). The bridge mechanism must support both self-hosted ("local") services and external SaaS platforms, without requiring recompilation of the `bm` binary for each new bridge.

How should BotMinter define the bridge plugin contract?

## Decision Drivers

* Operators self-host their infrastructure -- bridges must work in diverse environments
* No vendor lock-in -- adding a new bridge should not require changes to BotMinter core
* Existing Justfile recipe pattern is already established in BotMinter profiles
* Bridge authors need a clear, self-contained contract they can implement independently
* Must support two bridge categories: local (full lifecycle management) and external (identity-only)

## Considered Options

* Shell script bridge with YAML manifest (`bridge.yml` + `schema.json` + Justfile recipes)
* Rust trait plugin system (compiled bridge plugins)
* gRPC bridge protocol (network-based bridge communication)
* REST API bridge protocol (HTTP-based bridge communication)

## Decision Outcome

Chosen option: "Shell script bridge with YAML manifest", because it requires no recompilation, leverages the existing Justfile recipe pattern, and provides a clear file-based contract that bridge authors can implement without knowing Rust or BotMinter internals.

### Consequences

* Good, because no recompilation needed when adding new bridges
* Good, because leverages the existing Justfile recipe pattern from BotMinter profiles
* Good, because bridge authors only need to know shell scripting and the YAML contract
* Good, because file-based config exchange (`$BRIDGE_CONFIG_DIR/config.json`) avoids stdout corruption issues
* Neutral, because less type safety than a Rust trait system (mitigated by `schema.json` validation and conformance tests)
* Bad, because shell scripts are harder to unit test than Rust code (mitigated by conformance tests validating contract structure)

### Confirmation

The bridge spec document (`.planning/specs/bridge/`) and conformance tests validate the contract. A stub/no-op bridge implementation serves as the reference fixture.

## Pros and Cons of the Options

### Shell script bridge with YAML manifest

* Good, because zero compilation -- bridges are directories with YAML, JSON, and a Justfile
* Good, because Justfile recipes are already used in BotMinter profiles (`formations/`)
* Good, because bridge authors do not need Rust knowledge
* Good, because `bridge.yml` is declarative and inspectable
* Neutral, because `schema.json` provides config validation but not runtime type checking
* Bad, because shell scripts can have platform-specific behavior

### Rust trait plugin system

* Good, because full type safety at compile time
* Good, because IDE support for implementing the trait
* Bad, because requires recompilation for every new bridge
* Bad, because bridge authors must know Rust
* Bad, because dynamic loading (dylib) adds significant complexity

### gRPC bridge protocol

* Good, because language-agnostic bridge implementations
* Good, because strong typing via protobuf
* Bad, because requires running a gRPC server for each bridge -- heavy for simple integrations
* Bad, because adds protobuf/gRPC dependency to BotMinter

### REST API bridge protocol

* Good, because widely understood protocol
* Good, because language-agnostic
* Bad, because requires running an HTTP server for each bridge
* Bad, because loose typing without additional schema enforcement

## More Information

### Key Design Decisions

- **Bridge types:** `local` bridges manage full lifecycle (start, stop, health) and identity. `external` bridges manage identity only (the service runs elsewhere).
- **File-based config exchange:** Bridge commands write output to `$BRIDGE_CONFIG_DIR/config.json`, not stdout. This avoids stdout corruption from diagnostic output, logging, or shell initialization scripts.
- **Identity commands:** `onboard` (create bot user), `rotate-credentials` (refresh tokens), `remove` (delete bot user). These are per-agent operations.
- **Lifecycle commands:** `start`, `stop`, `health` (local bridges only). These manage the chat service instance.
- **API versioning:** `apiVersion: botminter.dev/v1alpha1` follows Kubernetes/Knative convention for future evolution.
- **Commands are Justfile recipe names:** The contract specifies recipe names, not hardcoded shell commands. This decouples the contract from implementation details.

### Related

- ADR-0003: Ralph robot backend decisions (how bridge credentials map to Ralph's robot abstraction)
- Bridge spec: `.planning/specs/bridge/` (the full contract specification)
