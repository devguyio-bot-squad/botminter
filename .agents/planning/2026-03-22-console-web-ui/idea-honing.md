# Idea Honing: BotMinter Console Web UI

Requirements clarification through interactive Q&A.

---

## Q1: Who is the primary user of this console, and what is the #1 thing they need to see at a glance?

Botminter operators can range from a solo dev running a small team to someone managing multiple teams with many members. What's the most critical information the console should surface immediately on load?

**A1:** The primary user is the BotMinter operator doing **day-two operations** — not just monitoring but actively managing and evolving the team. The MVP scope has three priorities in order:

1. **Process visibility** — See the workflow/value chain of each team. Understand how the process flows (status transitions, role handoffs, issue lifecycle).
2. **Member design visibility** — Inspect each member's hat collection design. Understand what hats a member wears and how they're configured.
3. **Profile/team editing** — Edit the team configuration and profile, with changes reflected on current members and the GitHub Project.

This is an **operational control plane**, not just a monitoring dashboard.

---

## Q2: What does "process visibility" look like concretely?

A profile defines a process (e.g., scrum's PROCESS.md with status labels like `po:triage -> arch:design -> dev:implement -> qe:verify`). When you say "visibility into the value chain," what do you envision?

**A2:** The focus is on the **process design itself**, NOT on live issue data. The dashboard is not duplicating GitHub's issue board — it's visualizing the **profile's process definition**.

Concretely:
- **Pipeline view:** A workflow diagram showing the status transitions as defined in the profile. No live issue counts for MVP — just the process structure.
- **Role view:** For each role, show the workflow from that role's perspective — what statuses it owns, what triggers it, what it hands off to.
- **Role details:** Show the prompts, behavior rules, communication patterns, and verification criteria associated with each role. This lives in the hats (hat collections) and CLAUDE.md content.

The dashboard is a **process design introspection tool** — helping a chief of staff understand "what does my team's methodology look like?" before worrying about "what are they working on right now?"

---

## Q3: What level of hat/member introspection do you need?

Each member has a hat collection (YAML) with hats defining triggers, tools, prompts, and behavior. What should the console show about member design?

**A3:** For MVP, a **full YAML viewer** for the `ralph.yml` file — syntax-highlighted, searchable, collapsible. Show the raw source of truth.

Future iterations will add richer visualizations (summary cards, hat relationship graphs, etc.), but MVP keeps it simple: render the YAML with good UX.

---

## Q4: What does "edit the team/profile" mean concretely for the MVP?

Editing could range from simple config changes to a full visual profile editor. What level of editing is in scope?

**A4:** The key insight is that the **navigation and structure visualization is more important than the editing itself**. The profile is a tree of files and directories (PROCESS.md, roles/, members/, knowledge/, invariants/, hats/). The MVP needs:

1. **A file/directory browser** that presents the profile's building blocks in a navigable, structured way — making the architecture legible.
2. **When you reach a file, edit it** as YAML/Markdown in-browser (Monaco/CodeMirror style).
3. **Save triggers sync** — changes propagate to members and GitHub.

The innovation is in the **browsing and structure visualization**, not in a fancy form-based editor. The file editor is just a YAML/Markdown editor for now.

---

## Q5: How should the web server be delivered and launched?

**A5:** The console is **part of the daemon** — when `bm start` launches the daemon, the console is available at a port. Key details:

- The console reads from the **local team repo on disk** (not embedded profiles). Embedded profiles are only for bootstrapping.
- If the team isn't initialized, the console should tell the user to initialize first.
- Implementation can be a **separate crate** (`bm-console`) that the daemon launches/integrates.
- Always available when the team is running — no separate `bm console` command needed.

---

## Q6: What frontend technology should the console use?

**Research note:** Ralph Orchestrator's dashboard uses React 19 + Vite 7 + Tailwind v4 + shadcn/ui + Zustand + TanStack Query + @xyflow/react + Axum backend. Assets served separately (not embedded).

**A6:** Go with **Svelte 5** — lighter, simpler, better fit for a local dev console.

Final stack decision:
- **Frontend:** Svelte 5 (runes) + TypeScript
- **Build:** Vite 7
- **Styling:** Tailwind v4 + shadcn-svelte
- **Editor:** CodeMirror 6 (YAML/Markdown editing)
- **State:** Built-in Svelte reactivity (`$state`, `$derived`)
- **Backend:** Axum (Rust) JSON API

Rationale: BotMinter's console is a local dev tool with file browsing + YAML editing + process visualization. Svelte's compiled output is smaller, the mental model is simpler, and the codebase is easier to maintain alongside the Rust CLI. No need to align with Ralph's React stack since the UIs serve different purposes.

Notable Svelte adopters: NYT, IKEA, 1Password, Decathlon, Chess.com, Square, NBA, Brave.

---

## Q7: How should the console get its data — what's the API model?

**A7:** Use **RESTful API** over HTTP.

Rationale:
- Simpler, universally understood, debuggable with curl/browser
- URL structure maps naturally to the profile's resource model (members, process, files)
- Action endpoints (POST /api/sync) handle non-CRUD operations
- WebSocket can be added separately later for live features (logs, chat)

MVP endpoints:
- GET /api/team — team info
- GET /api/process — PROCESS.md content
- GET /api/members — list members
- GET /api/members/:name — member detail (ralph.yml, CLAUDE.md)
- GET /api/files/*path — read any profile file
- PUT /api/files/*path — save file edits
- POST /api/sync — trigger bm teams sync

---

## Q8: Should the console present the profile as a raw file tree, or as semantic views (process, roles, members)?

**A8:** **Semantic views** — organize by concept, not by file path. The user thinks in terms of "process", "members", "knowledge", not `team/members/batman/ralph.yml`.

Sidebar navigation:
- **Process** — renders PROCESS.md + status label definitions
- **Members** — lists members; drill into each to see ralph.yml, CLAUDE.md
- **Knowledge** — team + project knowledge files
- **Invariants** — team + project invariants
- **Settings** — team configuration

---

## Q9: What is explicitly OUT of scope for the MVP?

Based on our discussion, the MVP is a process design introspection + editing tool. What about runtime/operational features?

**A9:** **Defer all runtime features.** MVP is purely design-time introspection + file editing.

Explicitly out of scope for MVP:
- Live member status monitoring (running/stopped/crashed)
- Log streaming / live activity feed
- Chat with members
- GitHub issue board / Kanban view
- Member start/stop/restart controls
- Visual workflow builder (drag-and-drop hat graph)
- Authentication / multi-user access
- Profile creation wizard (use `bm init` CLI for that) -- **REVISED: team creation IS in scope, see Q11**

---

## Q10: How should frontend assets be delivered to the browser?

**A10:** **Hybrid: Vite dev server in development, embedded in binary for production.**

- **Development:** Vite dev server (port 5173) with hot reload, proxying API calls to the Rust daemon (port 9100). Edit .svelte files, browser updates instantly.
- **Production:** `npm run build` → `dist/`, then `cargo build` embeds assets via rust-embed into the `bm` binary. Single binary serves everything.

This is the standard pattern for Rust+SPA projects — best DX during development, clean single-binary deployment for production.

---

## Q11: How should multi-team and team creation work in the console?

**A11:** **Teams are like Kubernetes namespaces.** The console is multi-team aware:

1. **Team selector** — A dropdown at the top of the sidebar (or header) that lists all registered teams from `~/.botminter/config.yml`. Switching teams rescopes all sidebar views (Process, Members, Knowledge, Invariants, Settings) to that team.

2. **All views are team-scoped** — The sidebar navigation stays the same, but the data changes based on the selected team. URLs could reflect this: `/teams/my-team/process`, `/teams/my-team/members`, etc.

3. **Team creation view** — A web-based equivalent of `bm init`. The console provides a guided form/wizard for creating a new team (select profile, choose org, create/select repo, configure bridge). This replaces the CLI-only `bm init` flow for operators who prefer the browser.

This means the console reads from `~/.botminter/config.yml` for the team list, not just a single team repo.

---
