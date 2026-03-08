# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v0.06 — Minty and Friends

**Shipped:** 2026-03-08
**Phases:** 6 | **Plans:** 19

### What Was Built
- Coding-agent-agnostic architecture with inline agent tags, CodingAgentDef config model, and filtered extraction pipeline
- Disk-based profile externalization with auto-prompt initialization pattern
- Workspace repository model with GitHub-hosted repos and git submodules
- Composable skills system (board-scanner, status-workflow, gh) with two-level scoping
- `bm chat` interactive sessions with context-aware meta-prompt assembly
- `bm minty` interactive assistant with thin persona shell and 4 composable skills

### What Worked
- **GSD-driven gap closure**: UAT testing after each phase caught real issues (e.g., `.current_dir()` bug in Minty, missing skills table in chat meta-prompt), and gap closure plans resolved them quickly
- **Closure DI pattern for test isolation**: Replaced `env::set_var` with injectable closures in session.rs — eliminated race conditions and made tests deterministic
- **GithubSuite abstraction**: Reduced E2E API rate limit consumption from 19 TempRepo creations to ~11 by sharing GitHub repos across related test cases
- **Independent code audits**: Running audits against requirements caught completion mismatches early (initial migration said 13/30, audits found 29/30 were actually done)

### What Was Inefficient
- **Initial requirement status was wrong**: GSD migration initially marked only 13/30 requirements as complete, but independent audits found 29/30 were already done — wasted cycles on "implementing" already-complete features
- **Phase 1 plan proliferation**: 10 plans in Phase 1 (mostly gap closure for test infrastructure) — some could have been batched into fewer, larger plans
- **Phases 2-4 had no GSD plans on disk**: These phases were implemented before GSD was initialized, so they have summary files but no formal PLAN.md artifacts — inconsistent paper trail

### Patterns Established
- **Two-layer profile model**: Embedded profiles for bootstrap reliability, disk profiles for runtime customization
- **libtest-mimic custom harness**: E2E tests use mandatory CLI args (--gh-token, --gh-org) instead of fragile env vars
- **Skills as shared building blocks**: Same delivery mechanism for team-level and BotMinter-level (Minty) interactions
- **Thin persona shell pattern**: Minty delegates all real work to skills, keeping the persona layer minimal

### Key Lessons
1. **Audit requirements before planning work** — the gap between "what we think is done" and "what's actually done" was 16 requirements. Always verify with code before creating implementation plans.
2. **Test infrastructure is a first-class deliverable** — 7 of 10 Phase 1 plans were test infrastructure (E2E harness, path isolation, binary check injection). Budget for this upfront rather than discovering it as gaps.
3. **UAT after each phase pays for itself** — catching the `.current_dir()` bug and skills table omission during UAT was cheaper than finding them in production.

### Cost Observations
- Model mix: primarily Opus for implementation, Sonnet for UAT/verification
- Timeline: 5 days (2026-03-04 to 2026-03-08)
- Notable: 399 files changed, 34k insertions across the milestone

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Key Change |
|-----------|--------|-------|------------|
| v0.06 | 6 | 19 | First milestone using GSD workflow; UAT gap closure pattern established |

### Cumulative Quality

| Milestone | Tests | LOC (Rust) | Key Metric |
|-----------|-------|------------|------------|
| v0.06 | 471 | 21,267 | 327 unit + 49 cli_parsing + 95 integration |

### Top Lessons (Verified Across Milestones)

1. Audit existing code against requirements before planning — prevents wasted work on already-complete features
2. Test infrastructure deserves its own plans — don't treat it as a side effect of feature work
