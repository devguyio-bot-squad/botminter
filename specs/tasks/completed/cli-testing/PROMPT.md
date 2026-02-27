# CLI Testing — Comprehensive Test Coverage

## Objective

Add thorough test coverage to the `bm` CLI: unit tests for untested helpers, integration tests for cross-command consistency, E2E tests against real GitHub repos and tg-mock, and fix the member discovery path bug (tests first).

## Task Directory

All tasks: `specs/cli-testing/tasks/`

## Execution Order

Tasks are grouped into phases. Within a phase, tasks are independent and can be done in any order. Phases must be sequential.

### Phase 1 — Unit & Integration Tests (no external deps)

| Priority | Task | What |
|----------|------|------|
| 1 | `task-01-regression-member-discovery-bug` | Failing regression test proving the status/start bug |
| 2 | `task-02-command-helper-unit-tests` | Unit tests for `find_workspace`, `format_timestamp`, etc. |
| 3 | `task-03-workspace-edge-case-tests` | Symlink, sync, copy_if_newer edge cases |
| 4 | `task-04-integration-test-expansion` | Cross-command consistency, multi-member, error paths |
| 5 | `task-05-cli-argument-parsing-tests` | Clap aliases, flags, help text |

### Phase 2 — E2E Infrastructure & Tests (requires gh CLI + Docker)

| Priority | Task | What |
|----------|------|------|
| 6 | `task-06-e2e-test-harness` | TempRepo, TgMock RAII helpers, Justfile recipes |
| 7 | `task-07-e2e-init-to-sync-lifecycle` | init→hire→sync verified against real GitHub |
| 8 | `task-08-e2e-start-to-stop-lifecycle` | start→status→stop with tg-mock |

### Phase 3 — Bug Fix (tests go green)

| Priority | Task | What |
|----------|------|------|
| 9 | `task-09-fix-member-discovery-path-bug` | 2-line fix in status.rs and start.rs |

## Constraints

- **Tests before fixes, always.** Task-09 must not be started until task-01 exists and fails.
- **Profile-agnostic.** Never hardcode role names or label values. Use `profile::list_roles()` and `profile::read_manifest().labels` dynamically.
- **Schema awareness.** Structural dirs (`knowledge/`, `invariants/`, `agent/`, `projects/`, `team/`) are schema-level, not profile-specific.
- **Feature gate E2E.** E2E tests behind `--features e2e` so `cargo test` stays fast.

## Verification

After each task, run:

```bash
cargo test -p bm          # All unit + integration tests pass
cargo clippy -p bm -- -D warnings  # No warnings
```

After task-06+, also:

```bash
just e2e                  # E2E tests pass (requires gh auth + Docker)
```

## Done When

- `cargo test -p bm` passes with all new tests (task-01 through task-05, plus task-09 fix)
- `just e2e` passes (tasks 06–08)
- Task-01's regression test passes only after task-09 is applied
- No clippy warnings
