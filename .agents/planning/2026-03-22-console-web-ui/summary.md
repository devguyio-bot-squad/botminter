# Project Summary: BotMinter Console Web UI

## What

A browser-accessible web console for day-two operations of BotMinter agentic teams. It's a **design-time introspection and editing tool** -- not a runtime monitoring dashboard. Think "Kubernetes Dashboard for agentic team methodology."

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| UI Type | Web-based dashboard (not TUI) | Rich visualization and editing capabilities |
| Frontend | Svelte 5 + SvelteKit + TypeScript | Lighter than React, simpler mental model, compiled output is small |
| Backend | Axum (Rust) REST API | Standard Rust web framework, already in ecosystem |
| Styling | Tailwind v4 + shadcn-svelte | Dark theme, dev console aesthetic |
| Editor | CodeMirror 6 | Framework-agnostic, lighter than Monaco, excellent YAML/Markdown |
| API style | REST | Simple, debuggable, maps to file/resource model |
| Assets | Hybrid: Vite dev / rust-embed prod | Best DX + clean production |
| Multi-team | Teams as namespaces | Team selector in sidebar, all views scoped |
| Delivery | Part of daemon (`bm daemon start`) | Always available when daemon is running |
| Process viz | Graphviz DOT files | State machine definition, rendered to SVG via viz.js WASM |

## MVP Scope

1. **Process visibility** -- Graphviz-rendered workflow state machines, role responsibilities, human gates
2. **Member design visibility** -- YAML viewer for ralph.yml, hat summaries, CLAUDE.md/PROMPT.md
3. **Profile/team editing** -- Semantic file browser, in-browser CodeMirror editor, save (auto git commit) & sync
4. **Multi-team** -- Team selector (teams as namespaces), all views team-scoped
5. **NOT in scope:** Runtime monitoring, log streaming, chat, GitHub issue board, start/stop controls, team creation wizard (use `bm init` CLI)

## Artifacts Created

| File | Purpose |
|------|---------|
| `rough-idea.md` | Initial concept |
| `idea-honing.md` | 11 Q&A requirements clarification |
| `research/profile-data-model.md` | Team repo structure and data model |
| `research/tech-stack.md` | Technology choices and comparisons |
| `design/detailed-design.md` | Full design: architecture, API, components, data models |
| `design/mockups/team-page.html` | HTML/Tailwind mockup of team overview page |
| `design/mockups/process-page.html` | HTML/Tailwind mockup of process pipeline page |
| `implementation/plan.md` | 11-step implementation plan with checklist |
| `fixture-gen/fixtures/` | Real team repo artifacts for backend tests (110 files, 488K) |
| `fixture-gen/Justfile` | Reproducible fixture generator (run on bm-dashboard-test-user) |

## Implementation Plan (11 Steps)

1. **Daemon rewrite** (tiny_http -> Axum) -- refactor the event loop to async
2. **Console API skeleton** + teams endpoint
3. **Frontend scaffold** (Svelte + SvelteKit + Tailwind) + `just dev` workflow
4. **Team overview** API + page
5. **Create workflow DOT files** for scrum and scrum-compact profiles
6. **Process** API + Graphviz pipeline visualization (viz.js WASM)
7. **Members list + detail** API + YAML viewer (CodeMirror 6)
8. **File read/write** API + editor + git auto-commit on save
9. **Knowledge, Invariants, Settings** pages + sync action
10. **Asset embedding** + production build pipeline (rust-embed)
11. **Daemon integration** + smoke tests

Each step results in working, demoable functionality that builds incrementally.

## Adversarial Review: Issues Addressed

Two review rounds conducted. All blockers resolved.

| Issue | Resolution |
|-------|-----------|
| Daemon uses tiny_http (BLOCKER) | Step 1 rewrites daemon to Axum |
| `bm start` != daemon (BLOCKER) | Console lives in the daemon (`bm daemon start`), not `bm start` |
| Team creation not designed (BLOCKER) | Descoped -- operators use `bm init` CLI |
| Steps not incremental (BLOCKER) | Added `just dev` recipe, fixed demo commands |
| DOT files don't exist (BLOCKER) | Step 5 creates workflow DOT files as a dedicated step |
| Pipeline grouping algorithm (WARNING) | DOT files define state machines, rendered via Graphviz WASM |
| File edits don't git commit (WARNING) | Auto git add + commit on PUT |
| No health endpoint (WARNING) | Added `GET /health` |
| Sync is blocking (WARNING) | Uses `spawn_blocking` with 60s timeout |
| Bind address conflict (WARNING) | Daemon binds `0.0.0.0` (webhooks need it), `--bind` flag for restriction |
| Empty states (WARNING) | Process page gracefully degrades when `workflows/` missing |

## Next Steps

1. Review the detailed design at `design/detailed-design.md`
2. Review the implementation plan at `implementation/plan.md`
3. Begin implementation following the checklist
4. To start implementation with Ralph, run:
   - `ralph run --config presets/pdd-to-code-assist.yml --prompt "<task>"`
   - `ralph run -c ralph.yml -H builtin:pdd-to-code-assist -p "<task>"`

## Remaining Open Questions

- Console port: reuse daemon's existing `--port` flag (default 8484) or separate port?
- API versioning: add `/api/v1/` prefix now or wait until API stabilizes?
- File size limits: should the file API reject files > N MB to protect CodeMirror?
