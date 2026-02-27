---
status: pending
created: 2026-02-21
started: null
completed: null
---
# Task: E2E Test Harness

## Description
Create the infrastructure for full end-to-end tests that exercise `bm` against real external services: GitHub (via `gh` CLI) and Telegram (via `tg-mock` Docker container). This task builds the harness — subsequent tasks (07, 08) write the actual E2E test scenarios.

## Background
The existing integration tests use temp dirs and mock nothing external. Full E2E tests need:
- A real (temporary) GitHub repo to verify label creation, issue management, and PR workflows
- A `tg-mock` Docker container to verify Telegram bot integration for member communication
- Feature-gated compilation so CI can skip E2E when credentials/Docker aren't available

### tg-mock Reference
From `specs/milestone-2-architect-first-epic/sprint-4/research/mock-telegram-server.md`:
- **Image:** `ghcr.io/watzon/tg-mock:latest`
- **Port:** 8081
- **Bot API:** `http://localhost:8081/bot<TOKEN>/<METHOD>`
- **Control API:** `http://localhost:8081/__control/updates` (inject messages), `/__control/requests` (inspect bot output)
- **Flags:** `--faker-seed` for deterministic responses, `--verbose` for debugging

## Reference Documentation
**Required:**
- `specs/milestone-2-architect-first-epic/sprint-4/research/mock-telegram-server.md` — full tg-mock API reference
- `specs/milestone-2-architect-first-epic/sprint-4/research/mock-telegram-server.md` — tg-mock control API and Docker setup
- `crates/bm/Cargo.toml` — current dependencies
- `Justfile` — current recipes

## Technical Requirements

### Feature gate
1. Add `e2e` feature to `crates/bm/Cargo.toml` (empty feature, used only for `#[cfg(feature = "e2e")]`)
2. Create `crates/bm/tests/e2e/` directory with `mod.rs` gated on the feature
3. E2E tests run via `cargo test -p bm --features e2e` (never in default `cargo test`)

### GitHub test helpers (`e2e/github.rs`)
4. `create_temp_repo(prefix: &str) -> TempRepo` — creates a private GitHub repo via `gh repo create`, returns handle with repo name
5. `TempRepo` implements `Drop` to delete the repo via `gh repo delete --yes`
6. `list_labels(repo: &str) -> Vec<String>` — runs `gh label list -R <repo>` and parses output
7. `list_issues(repo: &str) -> Vec<String>` — runs `gh issue list -R <repo>` and parses output
8. Prerequisite check: skip tests if `gh auth status` fails

### Telegram mock helpers (`e2e/telegram.rs`)
9. `TgMock::start() -> TgMock` — pulls and runs `ghcr.io/watzon/tg-mock:latest` on a random port, waits for health check
10. `TgMock::api_url() -> String` — returns `http://localhost:<port>`
11. `TgMock::inject_message(token, text, chat_id)` — POSTs to `/__control/updates`
12. `TgMock::get_requests(token, method) -> Vec<Value>` — GETs `/__control/requests`
13. `TgMock` implements `Drop` to stop and remove the container
14. Prerequisite check: skip tests if `docker` CLI not available

### Shared test helpers (`e2e/helpers.rs`)
15. `bm_cmd() -> Command` — creates `Command::new(env!("CARGO_BIN_EXE_bm"))` with clean env
16. `wait_for_port(port, timeout)` — polls TCP connect until ready
17. `assert_cmd_success(cmd) -> String` — runs command, asserts exit 0, returns stdout
18. `assert_cmd_fails(cmd) -> String` — runs command, asserts non-zero exit, returns stderr

### Justfile recipes
19. Add `just e2e` recipe: `cargo test -p bm --features e2e -- --test-threads=1`
20. Add `just e2e-verbose` recipe: same with `--nocapture`

## Dependencies
- `gh` CLI authenticated (for GitHub tests)
- Docker daemon running (for tg-mock tests)
- `reqwest` (add to dev-deps, blocking feature) for tg-mock control API calls
- `serde_json` (already a dependency) for parsing control API responses

## Implementation Approach
1. Add feature and dev-dependencies to `Cargo.toml`
2. Create `crates/bm/tests/e2e/mod.rs` with the feature gate
3. Implement `TempRepo` with RAII cleanup (Drop deletes GitHub repo)
4. Implement `TgMock` with Docker lifecycle management
5. Implement shared helpers
6. Add Justfile recipes
7. Write a single smoke test (`e2e_harness_smoke`) that creates a temp repo, spins up tg-mock, and tears both down — proving the harness works

## Acceptance Criteria

1. **Feature gate works**
   - Given default `cargo test -p bm`
   - When tests run
   - Then E2E tests are not compiled or executed

2. **GitHub temp repo lifecycle**
   - Given valid `gh` auth
   - When `TempRepo::new("bm-test")` is created
   - Then a private repo exists on GitHub, and when dropped, it's deleted

3. **tg-mock lifecycle**
   - Given Docker available
   - When `TgMock::start()` is called
   - Then container is running, health check passes, and when dropped, container is removed

4. **Control API works**
   - Given a running tg-mock
   - When a message is injected via `inject_message()` and requests are queried via `get_requests()`
   - Then the mock correctly tracks API interactions

5. **Justfile recipes**
   - Given the project
   - When `just e2e` is run
   - Then E2E tests execute with `--test-threads=1` and the `e2e` feature enabled

## Metadata
- **Complexity**: High
- **Labels**: test, e2e, infrastructure, docker, github
- **Required Skills**: Rust, Docker, GitHub CLI, HTTP APIs, RAII patterns
