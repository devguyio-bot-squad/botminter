# Sprint 4 Requirements: Automated HIL Telegram Tests

> Q&A record for Sprint 4 planning.

## Q1: What does "automated Telegram tests" mean?

### A1:

Use Ralph's `telegram.api_url` / `RALPH_TELEGRAM_API_URL` to redirect all Telegram
API calls to [tg-mock](https://github.com/watzon/tg-mock), a mock Telegram Bot API
server running as a Docker container. Tests inject canned responses via tg-mock's
`/__control/` API. No real Telegram bots, fully unattended.

Note: Ralph's `api_url` feature is untested — Sprint 4 is the first real validation.

## Q2: Which test scenarios?

### A2:

All seven from M2 design.md Section 8:
1. Full lifecycle traversal
2. Design rejection loop
3. Plan rejection loop
4. Concurrent operations
5. Push-conflict resolution
6. Crash-during-lock recovery
7. Knowledge propagation verification

## Q3: Test implementation approach?

### A3:

- **Rust test crate** (`tests/hil/`) — integration tests using `reqwest` to
  interact with tg-mock's control API, `std::process::Command` to run `just`
  recipes and launch agents
- **YAML fixture files** — each test scenario defined as data: response rules
  (match pattern → canned reply, with optional `times` and `context` filters)
  and expected outcomes
- **Just tasks** — `test-server-up` / `test-server-down` for tg-mock lifecycle,
  `test-hil` to run the full suite
- **No bash scripted human** — the response loop lives in the Rust test harness,
  driven by fixture data
