---
status: accepted
date: 2026-03-08
decision-makers: [BotMinter maintainers]
---

# Shell Script Bridge with YAML Manifest

## Problem

BotMinter needs pluggable communication bridges so operators can connect agentic teams to different chat platforms (Telegram, Rocket.Chat, Matrix). The mechanism must support both self-hosted ("local") services and external SaaS platforms, without recompiling the `bm` binary for each new bridge.

## Constraints

* No vendor lock-in — adding a new bridge must not require changes to BotMinter core
* Must work in diverse self-hosted environments (operators own their infrastructure)
* Must support two categories: local bridges (full lifecycle) and external bridges (identity-only)
* Bridge authors must be able to implement the contract without knowing Rust or BotMinter internals
* Credential exchange must not corrupt stdout (diagnostic output, shell init scripts)

## Decision

Bridges are directories containing three files: `bridge.yml` (YAML manifest declaring capabilities), `schema.json` (config schema), and a `Justfile` (recipe implementations). BotMinter invokes Justfile recipes by name, exchanges data via `$BRIDGE_CONFIG_DIR/config.json` (file-based, never stdout), and stores credentials through the active formation's credential backend.

Key design points:
- **Bridge types:** `local` bridges manage full lifecycle (start, stop, health) and identity. `external` bridges manage identity only.
- **Provisioning model:** Local bridges auto-provision identities (create bot users + tokens). External bridges accept operator-supplied tokens via `BM_BRIDGE_TOKEN_{USERNAME}` env vars.
- **Per-member identity:** Each hired team member gets their own bot user/token for per-agent traceability.
- **File-based config exchange:** Bridge commands write output to `$BRIDGE_CONFIG_DIR/config.json`, not stdout.
- **Formation-aware credential storage:** BotMinter stores credentials through the formation's backend (keyring for local, K8s Secrets for Kubernetes). Credentials are injected as env vars at `bm start` time, never written to `ralph.yml`.
- **API versioning:** `apiVersion: botminter.dev/v1alpha1` follows Kubernetes convention.

## Rejected Alternatives

### Rust trait plugin system

Rejected because: requires recompilation for every new bridge and forces bridge authors to know Rust.

* Full type safety is nice but the cost (compile-time coupling, Rust knowledge requirement) outweighs the benefit for a plugin system
* Dynamic loading (dylib) would avoid recompilation but adds significant complexity

### gRPC bridge protocol

Rejected because: running a gRPC server per bridge is too heavy for simple integrations.

* Adds protobuf/gRPC dependency to BotMinter
* Good for language-agnostic bridges but overkill when shell scripts suffice

### REST API bridge protocol

Rejected because: running an HTTP server per bridge has the same overhead problem as gRPC with weaker typing.

## Consequences

* Bridge authors only need shell scripting knowledge and the YAML contract
* Less type safety than Rust traits, mitigated by `schema.json` validation and conformance tests
* Shell scripts can have platform-specific behavior — mitigated by testing on Linux (the target platform)
* The bridge spec (`.planning/specs/bridge/`) is the canonical contract reference

## Anti-patterns

* **Do NOT** write bridge output to stdout — use `$BRIDGE_CONFIG_DIR/config.json`. Stdout gets corrupted by shell init scripts, diagnostic output, and `set -x` debugging.
* **Do NOT** store credentials in `ralph.yml` — they must go through the formation's credential backend and be injected as env vars at launch time.
* **Do NOT** hardcode recipe commands in BotMinter — the contract specifies recipe names from `bridge.yml`, not shell commands. Bridge authors choose their implementation.
* **Do NOT** make bridges Ralph-aware — bridges output generic credentials (JSON), BotMinter translates to Ralph config. See ADR-0003.
