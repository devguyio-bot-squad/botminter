# Codebase Concerns

**Analysis Date:** 2026-03-04

## Tech Debt

**Duplicated `find_workspace` with divergent logic (BUG):**
- Issue: `crates/bm/src/commands/daemon.rs` (line 713) uses the **old** `.botminter/` directory-based workspace detection, while `crates/bm/src/commands/start.rs` (line 212) uses the **current** `.botminter.workspace` marker file. The daemon will fail to find workspaces created by the current `bm teams sync`.
- Files: `crates/bm/src/commands/daemon.rs`, `crates/bm/src/commands/start.rs`
- Impact: `bm daemon start` (both webhook and poll modes) will silently skip all members, logging "no workspace found" warnings. The daemon becomes a no-op for any team provisioned with the current workspace model.
- Fix approach: Replace `daemon.rs::find_workspace` with the same `.botminter.workspace` marker logic from `start.rs`. Better yet, extract a shared `find_workspace` function to eliminate duplication entirely.

**Triplicated `list_member_dirs` function:**
- Issue: Three identical implementations of `list_member_dirs` exist across the codebase, each with its own tests.
- Files: `crates/bm/src/commands/start.rs` (line 191), `crates/bm/src/commands/daemon.rs` (line 695), `crates/bm/src/completions.rs` (line 223)
- Impact: Maintenance burden. Changes to member directory conventions must be applied in three places. The daemon version already drifted (see above).
- Fix approach: Extract to a shared module (e.g., `crates/bm/src/workspace.rs` already exists and would be a natural home). Re-export and use from all three call sites.

**Roadmap page is stale:**
- Issue: `docs/content/roadmap.md` lists "Minty and Friends" as "Planned" with sub-items (Minty, profile externalization, workspace repository model, coding-agent-agnostic cleanup) that are already implemented in the current codebase. The CLI already has `bm minty`, `bm profiles init`, workspace repos with `.botminter.workspace`, and agent tag support.
- Files: `docs/content/roadmap.md`
- Impact: Confuses users about project maturity. Features they can use today appear as future work.
- Fix approach: Move "Minty and Friends" to the Completed section. Update the "Planned" and "Future" sections to reflect actual next milestones.

## Known Bugs

**Daemon workspace detection uses obsolete model:**
- Symptoms: `bm daemon start` in poll or webhook mode triggers member launches, but every member gets "no workspace found, skipping" in daemon logs. No actual work happens.
- Files: `crates/bm/src/commands/daemon.rs` (lines 713-738)
- Trigger: Any team provisioned with current `bm teams sync` (which writes `.botminter.workspace` marker, not `.botminter/` directories).
- Workaround: Use `bm start` (persistent mode) instead of `bm daemon start`.

## Security Considerations

**Webhook server binds to 0.0.0.0:**
- Risk: The daemon webhook server binds to `0.0.0.0:{port}` (line 359 of `crates/bm/src/commands/daemon.rs`), accepting connections from any network interface. If the host has a public IP, the webhook endpoint is exposed to the internet.
- Files: `crates/bm/src/commands/daemon.rs`
- Current mitigation: HMAC-SHA256 webhook signature validation is available when `webhook_secret` is configured. Without it, any POST to `/webhook` with valid event headers triggers member launches.
- Recommendations: Default to `127.0.0.1` binding. Add a `--bind` flag for explicit override. Warn if no `webhook_secret` is configured in webhook mode.

**GH token stored in plaintext YAML:**
- Risk: `~/.botminter/config.yml` stores `gh_token`, `telegram_bot_token`, and `webhook_secret` as plaintext YAML values.
- Files: `crates/bm/src/config.rs`, `crates/bm/src/commands/init.rs`
- Current mitigation: File is created with `0600` permissions. `check_permissions()` warns on load if permissions are wrong.
- Recommendations: Consider integration with OS keychain (via `keyring` crate) or at minimum document the security model. The permission check only warns to stderr; it does not block operation.

**Multiple `unsafe` blocks for signal handling:**
- Risk: Seven `unsafe` blocks across `daemon.rs`, `stop.rs`, and `state.rs` use `libc::kill()` and `libc::signal()` directly. The signal handler in `daemon.rs` (line 349) uses a bare `extern "C"` function writing to a global `AtomicBool`.
- Files: `crates/bm/src/commands/daemon.rs` (lines 212, 226, 313, 577, 588), `crates/bm/src/commands/stop.rs` (line 136), `crates/bm/src/state.rs` (line 74)
- Current mitigation: Each `unsafe` block is minimal and well-documented. Signal handler only writes an atomic bool.
- Recommendations: Consider using the `signal-hook` crate for safe signal handling. The `nix` crate provides safe wrappers for `kill()`.

## Performance Bottlenecks

**GitHub Events API pagination in poll mode:**
- Problem: `poll_github_events()` calls `gh api repos/{repo}/events --paginate` which fetches ALL pages of events, then filters in memory.
- Files: `crates/bm/src/commands/daemon.rs` (line 813-851)
- Cause: The `--paginate` flag retrieves all available events (up to 300 per the GitHub API). On active repos, this is wasteful when only events newer than `last_event_id` are needed.
- Improvement path: Remove `--paginate` (first page of 30 events is usually sufficient). Add `--jq` filtering to stop early. Or use `If-None-Match` / `If-Modified-Since` headers for conditional requests.

**Sequential member launches:**
- Problem: `bm start` launches members sequentially with a 2-second sleep per member to verify process liveness (line 138 of `start.rs`).
- Files: `crates/bm/src/commands/start.rs`
- Cause: The liveness check blocks the launch loop.
- Improvement path: Launch all members first, then batch-verify liveness after a single delay. For N members, this saves (N-1) * 2 seconds.

## Fragile Areas

**`workspace.rs` — largest file in the codebase (1912 lines):**
- Files: `crates/bm/src/workspace.rs`
- Why fragile: Handles workspace creation, submodule management, context file copying, agent directory assembly, gitignore generation, branch management, and sync operations. Many interacting concerns in a single module.
- Safe modification: Changes to workspace sync logic should be tested with `bm teams sync -v` on a real team. The integration test suite (`crates/bm/tests/integration.rs` at 2517 lines) covers the happy path but may miss edge cases in submodule state.
- Test coverage: Integration tests cover init-to-sync flow but not incremental sync scenarios (e.g., adding a new project to an existing team, or re-syncing after upstream changes).

**`profile.rs` — profile extraction pipeline (1796 lines):**
- Files: `crates/bm/src/profile.rs`
- Why fragile: Agent tag filtering, `context.md` → agent-specific filename renaming, schema version gating, and manifest parsing all live here. A bug in agent tag filtering silently drops content from profile files.
- Safe modification: The embedded profile test suite (starting at line 718) covers structural invariants well. Run `just test` after any change.
- Test coverage: Good unit test coverage for extraction and manifest parsing. Agent tag filtering has tests but edge cases (nested tags, multi-agent files) could be under-tested.

**`init.rs` — interactive wizard (1342 lines):**
- Files: `crates/bm/src/commands/init.rs`
- Why fragile: Complex interactive flow with many branches (new repo vs existing, GitHub auth detection, label bootstrapping, project board sync). Failures mid-wizard can leave partial state.
- Safe modification: The wizard uses `cliclack` for prompts which makes automated testing difficult. E2E tests exist in `crates/bm/tests/e2e/init_to_sync.rs` but require GitHub mock infrastructure.
- Test coverage: E2E tests cover the full init flow with mock GitHub server. Unit-level tests for individual wizard steps are sparse.

## Dependencies at Risk

**Ralph Orchestrator (external runtime dependency):**
- Risk: BotMinter depends on `ralph` binary being in `PATH`. Local checkout at `/opt/workspace/ralph-orchestrator` has a custom commit for Telegram mock URL support. If upstream changes the CLI interface, `bm start`, `bm stop`, `bm chat`, and `bm daemon` all break.
- Impact: All member lifecycle commands fail.
- Migration plan: Pin Ralph version in documentation. Consider vendoring a known-good version or adding version detection at startup.

## Missing Critical Features

**No `bm daemon` workspace detection fix:**
- Problem: The daemon is documented and has a full test suite but cannot discover workspaces created by the current `bm teams sync`. This is the highest-priority bug.
- Blocks: Event-driven autonomous operation via daemon mode.

**No docs search page / 404 page:**
- Problem: The MkDocs site has no custom 404 page. The `docs/content/index.md` is just a frontmatter template redirect — if the home template fails to load, users see a blank page.
- Files: `docs/content/index.md`, `docs/overrides/`

## Test Coverage Gaps

**Daemon workspace discovery not tested against current workspace model:**
- What's not tested: `daemon.rs::find_workspace` is tested against the old `.botminter/` directory model. No test validates it against the current `.botminter.workspace` marker file model.
- Files: `crates/bm/tests/e2e/daemon_lifecycle.rs`, `crates/bm/src/commands/daemon.rs` (tests at line 1190)
- Risk: The daemon silently fails to launch any members. This is the divergent workspace detection bug documented above.
- Priority: High

**No unit tests for `init.rs` wizard sub-functions:**
- What's not tested: Individual steps of the init wizard (token detection, org listing, repo creation) are not unit tested. Only the full E2E flow is covered.
- Files: `crates/bm/src/commands/init.rs`
- Risk: Regressions in wizard sub-steps (e.g., token validation, label bootstrapping) are only caught by expensive E2E tests.
- Priority: Medium

**Incremental `bm teams sync` scenarios:**
- What's not tested: Adding a new project to an existing team and re-syncing, removing a member and re-syncing, or syncing when upstream submodule references have changed.
- Files: `crates/bm/src/workspace.rs`, `crates/bm/tests/integration.rs`
- Risk: Incremental sync bugs could corrupt workspace state or leave stale submodule references.
- Priority: Medium

## Docs-Specific Concerns

**`getting-started/bootstrap-your-team.md` links to wrong prerequisites page:**
- Issue: Line 13 links to `[Prerequisites](index.md)` but the prerequisites content is at `prerequisites.md`. The `index.md` file is the Getting Started overview page, not prerequisites.
- Files: `docs/content/getting-started/bootstrap-your-team.md`
- Impact: Users clicking "Prerequisites" from the bootstrap guide land on the overview page instead of the prerequisites page.
- Fix: Change link to `[Prerequisites](prerequisites.md)`.

**Roadmap milestone statuses are outdated:**
- Issue: "Minty and Friends" is listed as "Planned" but all its sub-items (Minty assistant, profile externalization, workspace repository model, coding-agent abstraction) are implemented. The `bm` CLI milestone description also doesn't mention features added after initial completion (daemon, knowledge, formations, completions, Minty, chat).
- Files: `docs/content/roadmap.md`
- Impact: Misleading status for users evaluating the project.
- Fix: Move "Minty and Friends" to Completed. Update the `bm CLI` milestone description. Add current planning as the new "Planned" item.

**FAQ references `@bot` prefix convention that may be outdated:**
- Issue: FAQ states "you need to prefix comments with `@bot`" but this convention is profile-specific and may have changed.
- Files: `docs/content/faq.md` (line 52-54)
- Impact: Low — FAQ accurately describes the current state but should be validated against latest profile PROCESS.md.
- Fix: Verify against `profiles/scrum/PROCESS.md` and `profiles/scrum-compact/PROCESS.md`.

**Mermaid JS loaded from unpkg CDN:**
- Issue: `docs/mkdocs.yml` loads `mermaid@11` from `unpkg.com` (line 83). This is an external dependency that could break if unpkg has an outage, or if mermaid@11 publishes a breaking update (the `@11` tag is a major version range, not a pinned version).
- Files: `docs/mkdocs.yml`
- Impact: Diagrams on all docs pages could fail to render.
- Fix: Pin to a specific mermaid version (e.g., `mermaid@11.4.1`) or vendor the JS file locally.

---

*Concerns audit: 2026-03-04*
