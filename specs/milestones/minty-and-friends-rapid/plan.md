# Implementation Plan — Minty and Friends [RAPID]

> Incremental implementation steps. Each step builds on the previous, results in working demoable functionality, and follows TDD practices.
> Inputs: [design.md](design.md), [requirements.md](requirements.md), [research/](research/).
>
> **Guiding principle:** Core end-to-end functionality available as early as possible. No orphaned code — every step ends with integration. Documentation updates are delivered per sprint, not batched. Alpha policy: no migration paths.

---

## Checklist

- [ ] Step 1: Agent tag filter library
- [ ] Step 2: `CodingAgentDef` data model + `botminter.yml` schema
- [ ] Step 3: Profile restructuring — `agent/` → `coding-agent/`, `CLAUDE.md` → `context.md` with tags
- [ ] Step 4: Extraction pipeline — tag filtering + context file rename
- [ ] Step 5: Workspace parameterization — eliminate hardcoded agent strings
- [ ] Step 6: Sprint 1 documentation + cleanup
- [ ] Step 7: `bm profiles init` — disk extraction command
- [ ] Step 8: Disk-based profile API + auto-prompt pattern
- [ ] Step 9: Sprint 2 documentation
- [ ] Step 10: Workspace repo creation — `bm teams sync --push` (new workspaces)
- [ ] Step 11: Workspace repo sync — `bm teams sync` (existing workspaces) + `bm start` adaptation
- [ ] Step 12: Workspace status commands + Sprint 3 documentation
- [x] Step 13: Board-scanner skill migration + profile directory restructuring
- [ ] Step 14: Ralph prompt shipping — extract to `ralph-prompts/`
- [ ] Step 15: Status-workflow skill extraction + Sprint 4 documentation
- [ ] Step 16: Team Manager role — definition, skeleton, statuses
- [ ] Step 17: `bm chat` — interactive session launcher
- [ ] Step 18: Sprint 5 documentation
- [ ] Step 19: Minty config structure + launch command
- [ ] Step 20: Minty skills + Sprint 6 documentation

---

## Sprint 1: Coding-Agent-Agnostic Cleanup

### Step 1: Agent Tag Filter Library

**Objective:** Build the core line-based filter that processes `+agent:NAME` / `-agent` tags in files. This is the foundational building block for the entire coding-agent-agnostic architecture — every subsequent extraction and workspace operation depends on it.

**Implementation guidance:**

1. Create a new module `agent_tags.rs` in `crates/bm/src/`
2. Implement `filter_agent_tags(content: &str, agent: &str, comment_syntax: CommentSyntax) -> String`:
   - `CommentSyntax` enum: `Html` (`<!-- -->`), `Hash` (`#`)
   - Line-by-line processing: track inclusion state via a simple state machine
   - Content outside any tags → always included
   - Content inside `+agent:NAME` / `-agent` → included only when `NAME` matches `agent`
   - Tag lines themselves are always stripped from output
   - No nesting support — tags are flat open/close pairs
3. Implement `detect_comment_syntax(filename: &str) -> CommentSyntax`:
   - `.md`, `.html` → `Html`
   - `.yml`, `.yaml`, `.sh` → `Hash`
   - Default: `Hash`
4. Implement convenience wrapper `filter_file(content: &str, filename: &str, agent: &str) -> String` that combines detection + filtering

**Test requirements:**
- Common-only content (no tags) passes through unchanged
- Matching agent sections are included, tag lines stripped
- Non-matching agent sections are excluded, tag lines stripped
- Multiple agent blocks in one file (e.g., claude-code and gemini-cli sections interleaved)
- Files with only common content + matching tags produce clean output
- Files with only common content + non-matching tags exclude those sections
- Empty file returns empty string
- YAML files with tagged duplicate keys (e.g., two `backend:` lines) produce valid single-key YAML
- HTML comment syntax for `.md` files: `<!-- +agent:claude-code -->` / `<!-- -agent -->`
- Hash comment syntax for `.yml` files: `# +agent:claude-code` / `# -agent`
- Hash comment syntax for `.sh` files

**Integration notes:** This is a pure library function with no side effects. It doesn't touch the filesystem or profiles yet — just string processing. All subsequent steps use this filter.

**Demo:**
```bash
cargo test -p bm agent_tags
```

---

### Step 2: `CodingAgentDef` Data Model + Schema

**Objective:** Add the `CodingAgentDef` struct and update `ProfileManifest` to include `coding_agents` and `default_coding_agent` fields. Update `TeamEntry` with an optional `coding_agent` override. Add `resolve_coding_agent()` to determine the effective agent for a team.

**Implementation guidance:**

1. Add `CodingAgentDef` struct to `profile.rs` (or a new `coding_agent.rs` module):
   ```rust
   pub struct CodingAgentDef {
       pub name: String,
       pub display_name: String,
       pub context_file: String,   // e.g., "CLAUDE.md"
       pub agent_dir: String,      // e.g., ".claude"
       pub binary: String,         // e.g., "claude"
   }
   ```
2. Update `ProfileManifest` with new fields:
   - `coding_agents: HashMap<String, CodingAgentDef>`
   - `default_coding_agent: String`
3. Update `TeamEntry` with `coding_agent: Option<String>` override field
4. Implement `resolve_coding_agent(team: &TeamEntry, manifest: &ProfileManifest) -> Result<&CodingAgentDef>`:
   - If team has override → look up in manifest's `coding_agents`
   - Otherwise → use manifest's `default_coding_agent`
   - Error if resolved agent not found in manifest's `coding_agents` map
5. Update `botminter.yml` schema (`.schema/`) to include the new fields
6. Update existing profile `botminter.yml` files to include the `coding_agents` section with `claude-code` as the only entry and `default_coding_agent: claude-code`

**Test requirements:**
- `CodingAgentDef` deserializes correctly from YAML
- `ProfileManifest` parses with new `coding_agents` and `default_coding_agent` fields
- `resolve_coding_agent()` returns profile default when team has no override
- `resolve_coding_agent()` returns team override when present
- `resolve_coding_agent()` errors when override references unknown agent
- Existing integration tests still pass (backwards-compatible schema addition)
- `bm profiles describe` output includes coding agent information

**Integration notes:** This step updates the schema but doesn't yet change extraction or workspace behavior. The existing `include_dir!` embedding path still works — the new fields are simply additive. `bm profiles describe` gains a "Coding Agents" section showing supported agents.

**Demo:**
```bash
cargo run -p bm -- profiles describe scrum
# Shows coding agents section: claude-code (default)
```

---

### Step 3: Profile Restructuring

**Objective:** Restructure all profiles to use coding-agent-agnostic conventions: rename `agent/` → `coding-agent/`, rename `CLAUDE.md` → `context.md`, and insert inline agent tags in files with Claude Code-specific content.

**Implementation guidance:**

This is a content migration within `profiles/`, not runtime code:

1. Rename directories across all profiles and all scopes:
   - Team-level: `agent/` → `coding-agent/`
   - Project-level: `projects/<p>/agent/` → `projects/<p>/coding-agent/`
   - Member-level: `members/<m>/agent/` → `members/<m>/coding-agent/`
2. Rename context files across all profiles:
   - Team-level: `CLAUDE.md` → `context.md`
   - Member-level: `members/<m>/CLAUDE.md` → `members/<m>/context.md`
3. Add inline agent tags to files containing Claude Code-specific content:
   - `context.md` files: wrap Claude Code-specific sections with `<!-- +agent:claude-code -->` / `<!-- -agent -->`
   - `ralph.yml` files: wrap `cli.backend: claude` with `# +agent:claude-code` / `# -agent`
   - Any shell scripts with agent-specific commands
4. Ensure common (agent-agnostic) content remains untagged
5. Verify files with tags produce correct output when filtered for `claude-code`

**Test requirements:**
- No `agent/` directories exist in any profile (all renamed to `coding-agent/`)
- No `CLAUDE.md` files exist in profiles (all renamed to `context.md`)
- All `context.md` files contain valid agent tags (balanced open/close)
- Filtering `context.md` for `claude-code` produces content identical to the original `CLAUDE.md`
- `ralph.yml` files with agent tags produce valid YAML after filtering
- Profile schema validation still passes (update `.schema/` for the renames)
- Existing integration tests still pass (embedding still works since include_dir reads the new structure)

**Integration notes:** This step changes the embedded profile content. The extraction pipeline doesn't use the tag filter yet (that's Step 4), but the content is now ready. Tests should verify that the raw profile structure is correct and that filtering produces expected output.

**Demo:**
```bash
tree profiles/scrum/ -L 2
# Shows coding-agent/ instead of agent/, context.md instead of CLAUDE.md
head -20 profiles/scrum/context.md
# Shows agent tags wrapping Claude Code-specific content
```

---

### Step 4: Extraction Pipeline — Tag Filtering + Context Rename

**Objective:** Update `extract_profile_to()` and `extract_member_to()` to run the agent tag filter during extraction and rename `context.md` to the resolved agent's `context_file` (e.g., `CLAUDE.md`). After this step, `bm init` produces a team repo with cleanly filtered, properly named files.

**Implementation guidance:**

1. Update `extract_profile_to(name, target, coding_agent: &CodingAgentDef)`:
   - Copy all files (skip `members/`, `.schema/`)
   - For `context.md`: run `filter_agent_tags()`, write result as `coding_agent.context_file`
   - For all other `.md`, `.yml`, `.yaml`, `.sh` files: run `filter_agent_tags()` to strip non-matching sections
   - Copy remaining files (images, scripts) verbatim
2. Update `extract_member_to(name, role, target, coding_agent: &CodingAgentDef)`:
   - Same pattern: filter + rename context.md → CLAUDE.md (or equivalent)
   - All `.md`/`.yml` files get filtered
3. Update `bm init` to resolve the coding agent before extraction:
   - Read `default_coding_agent` from the selected profile's manifest
   - Pass `CodingAgentDef` to extraction functions
4. Rename `coding-agent/` → `coding_agent.agent_dir`-based name in output? No — `coding-agent/` stays as-is in the team repo. Only `context.md` gets renamed. The `coding-agent/` directory is a BotMinter convention, not agent-specific.

**Test requirements:**
- `extract_profile_to()` with `claude-code` agent produces `CLAUDE.md` (not `context.md`) at team repo root
- `CLAUDE.md` in team repo contains common + claude-code sections, no tag markers
- `extract_member_to()` produces `CLAUDE.md` in member dir, filtered and clean
- `ralph.yml` in extracted member dir has `cli.backend: claude` with no tag markers
- If a hypothetical `gemini-cli` agent were resolved, `context.md` → `GEMINI.md` (tests can use a mock agent)
- All existing `bm init` integration tests pass with the updated extraction
- `bm profiles describe <profile> --show-tags` shows agent tag summary (per design acceptance criteria)

**Integration notes:** This is the first step where the coding-agent abstraction is functionally active end-to-end. After this step, `bm init` → team repo → files are all mediated by the agent config. However, `workspace.rs` still uses hardcoded strings — that's Step 5.

**Demo:**
```bash
# Create a team, inspect the result
bm init
ls my-team/CLAUDE.md          # Exists (renamed from context.md)
grep "+agent" my-team/CLAUDE.md  # No hits — tags stripped
ls my-team/coding-agent/       # Exists (renamed from agent/)
```

---

### Step 5: Workspace Parameterization

**Objective:** Eliminate all hardcoded `"CLAUDE.md"` and `".claude"` strings in `workspace.rs`. Replace with values read from the resolved `CodingAgentDef`. After this step, the workspace creation and sync logic is fully agent-agnostic.

**Implementation guidance:**

1. Update `workspace.rs` to accept `CodingAgentDef` (or its relevant fields) in all functions:
   - `BM_GITIGNORE_ENTRIES`: use `coding_agent.agent_dir` instead of hardcoded `".claude"`
   - `surface_files()`: use `coding_agent.context_file` instead of `"CLAUDE.md"`
   - `assemble_claude_dir()` → rename to `assemble_agent_dir()`: use `coding_agent.agent_dir`
   - `sync_workspace()`: pass coding agent config through
   - `verify_symlink()`: no changes (agent-agnostic already)
2. Thread `CodingAgentDef` through the call chain:
   - `bm teams sync` → resolve coding agent → pass to workspace functions
   - `bm hire` → resolve coding agent → pass to member extraction
3. Search for any remaining hardcoded `"CLAUDE.md"`, `".claude"`, `"claude"` strings in non-test code and parameterize them
4. Test fixtures and assertions can still reference concrete values — they test with `claude-code` as the resolved agent

**Test requirements:**
- No hardcoded `"CLAUDE.md"` or `".claude"` strings in `workspace.rs` outside of test fixtures
- `bm teams sync` creates workspace with parameterized paths (verify by inspection)
- `.gitignore` in workspace uses the agent dir name from config
- Symlinks in workspace use the agent dir name from config
- Existing integration tests pass (they resolve `claude-code` and get the same concrete values)

**Integration notes:** After this step, Sprint 1 is functionally complete. The entire pipeline — profile → extraction → team repo → workspace — is agent-agnostic. Only Claude Code is implemented as a concrete agent, but the architecture is pluggable.

**Demo:**
```bash
# Verify no hardcoded strings remain
grep -rn '"CLAUDE.md"\|"\.claude"' crates/bm/src/ --include='*.rs' | grep -v test
# Should return nothing (or only test code)
```

---

### Step 6: Sprint 1 Documentation + Cleanup

**Objective:** Update all documentation pages affected by the coding-agent-agnostic cleanup. Remove any hardcoded Claude Code references where the design is now agent-agnostic.

**Implementation guidance:**

Per the design's documentation impact matrix (Sprint 1):

1. `docs/content/concepts/profiles.md` — add coding-agent abstraction concept; explain `coding-agent/` directory rename and inline agent tags
2. `docs/content/reference/configuration.md` — document `coding_agents` and `default_coding_agent` in `botminter.yml`; add `coding_agent` team-level override
3. `docs/content/reference/cli.md` — update `bm init` to mention coding agent selection; add `--show-tags` to `bm profiles describe`
4. `docs/content/getting-started/index.md` — generalize prerequisites: "a supported coding agent" with Claude Code as the current option
5. `docs/content/faq.md` — update "Do I need Claude Code?" answer
6. Update CLAUDE.md at repo root if it references old `agent/` directory convention

**Test requirements:**
- No docs page hardcodes Claude Code where the design is now agent-agnostic (except as "currently the only supported agent")
- New `botminter.yml` fields are documented with examples
- `bm profiles describe` docs mention `--show-tags` flag
- Existing doc links are not broken

**Integration notes:** This is a docs-only step. No code changes.

**Demo:**
```bash
# Spot-check docs
grep -r "agent/" docs/content/ --include='*.md' | head
# Should show "coding-agent/" not bare "agent/"
```

---

## Sprint 2: Profile Externalization

### Step 7: `bm profiles init` — Disk Extraction Command

**Objective:** Implement the `bm profiles init [--force]` command that extracts all embedded profiles to `~/.config/botminter/profiles/`. This is the bridge between the compile-time embedded profiles and the new disk-based model.

**Implementation guidance:**

1. Add `profiles init` subcommand to `cli.rs` with optional `--force` flag
2. Implement `commands/profiles_init.rs`:
   - Determine target: `dirs::config_dir().join("botminter").join("profiles")`
   - If target doesn't exist: create it, extract all embedded profiles
   - If target exists and `--force` not set: list existing profiles, prompt per-profile overwrite/skip
   - If target exists and `--force` set: overwrite all without prompting
3. Extraction logic:
   - Iterate embedded profiles from `include_dir!` static
   - For each profile: write all files/directories to `~/.config/botminter/profiles/<name>/`
   - Preserve directory structure exactly (including `.schema/`)
   - **Do NOT apply agent tag filtering** during this extraction — profiles are stored as-is on disk with their tags intact; filtering happens during `bm init` / `bm hire` (extraction to team repo)
4. Print summary: number of profiles extracted, path

**Test requirements:**
- Fresh install: `bm profiles init` creates `~/.config/botminter/profiles/` with all embedded profiles
- Each extracted profile has `botminter.yml` and expected directory structure
- Re-run without `--force`: prompts for overwrite/skip (test with mock stdin or `--force`)
- Re-run with `--force`: overwrites silently
- Extracted profile content matches embedded content byte-for-byte
- Target directory is created recursively if parents don't exist

**Integration notes:** After this step, profiles exist on disk but nothing reads from disk yet — that's Step 8. The `include_dir!` static is still the primary source for all other commands. This step adds a single new command without breaking anything.

**Demo:**
```bash
bm profiles init
ls ~/.config/botminter/profiles/
# scrum  scrum-compact  scrum-compact-telegram
cat ~/.config/botminter/profiles/scrum/botminter.yml
```

---

### Step 8: Disk-Based Profile API + Auto-Prompt

**Objective:** Switch all profile access from `include_dir!` reads to filesystem reads. The `include_dir!` static remains in the binary but is only accessed by `bm profiles init`. All other commands read from `~/.config/botminter/profiles/`. Implement the auto-prompt pattern for commands that require profiles.

**Implementation guidance:**

1. Update `profile.rs` — all public functions switch to filesystem reads:
   ```rust
   fn profiles_dir() -> PathBuf {
       dirs::config_dir().unwrap().join("botminter").join("profiles")
   }

   pub fn list_profiles() -> Result<Vec<String>> {
       // Read from ~/.config/botminter/profiles/
       fs::read_dir(profiles_dir())?.filter_map(...)
   }

   pub fn read_manifest(name: &str) -> Result<ProfileManifest> {
       // Read from ~/.config/botminter/profiles/<name>/botminter.yml
       let path = profiles_dir().join(name).join("botminter.yml");
       ...
   }
   ```
2. Keep `include_dir!` static but only expose it to the `profiles init` command:
   - Move the static into a `embedded` submodule or keep it gated behind a function only called by init
3. Implement `ensure_profiles_initialized() -> Result<()>`:
   - Check if `profiles_dir()` exists and is non-empty
   - If not: prompt "Profiles not initialized. Do you want me to initialize them now?"
   - If yes: run extraction inline
   - If no: print help message and bail gracefully
4. Add `ensure_profiles_initialized()` call at the top of commands that require profiles:
   - `bm init`, `bm hire`, `bm teams sync`, `bm profiles list`, `bm profiles describe`, `bm roles list`
5. Update extraction functions (`extract_profile_to`, `extract_member_to`) to read from disk instead of embedded

**Test requirements:**
- `bm profiles list` reads from disk, not embedded (verify by extracting, modifying a profile on disk, and checking output reflects modification)
- `bm profiles describe` reads from disk
- `bm init` on fresh install (no disk profiles) triggers auto-prompt
- Auto-prompt yes → profiles initialized → `bm init` continues
- Auto-prompt no → graceful abort with help message
- `bm hire` with profiles on disk works end-to-end
- Extraction (`bm init`) reads profile content from disk directory
- `include_dir!` is NOT accessed by any command except `bm profiles init`

**Integration notes:** This is a critical transition — the data source for profiles shifts from compile-time to runtime. All integration tests need to ensure profiles are on disk before running (either via a test fixture or by calling the init logic in test setup). Consider a test helper that extracts to a tempdir.

**Demo:**
```bash
# Modify a profile on disk and see the change reflected
bm profiles init
echo "# Custom addition" >> ~/.config/botminter/profiles/scrum/context.md
bm profiles describe scrum
# Output reflects disk content
```

---

### Step 9: Sprint 2 Documentation

**Objective:** Update documentation for profile externalization.

**Implementation guidance:**

Per the design's documentation impact matrix (Sprint 2):

1. `docs/content/reference/cli.md` — document `bm profiles init [--force]`
2. `docs/content/concepts/profiles.md` — rewrite storage model: profiles are on disk at `~/.config/botminter/profiles/`, not embedded; explain extraction and customization
3. `docs/content/getting-started/bootstrap-your-team.md` — add `bm profiles init` as prerequisite (or explain auto-prompt)
4. `docs/content/how-to/generate-team-repo.md` — update `bm init` flow for disk-based profiles and auto-prompt
5. `docs/content/reference/configuration.md` — document `~/.config/botminter/` layout alongside `~/.botminter/`

**Test requirements:**
- All affected doc pages updated
- No references to "embedded profiles" as the active data source (except in `bm profiles init` context)

**Integration notes:** Docs-only step.

**Demo:**
```bash
grep "profiles init" docs/content/reference/cli.md
# Should document the new command
```

---

## Sprint 3: Workspace Repository Model

### Step 10: Workspace Repo Creation — `bm teams sync --push` (New Workspaces)

**Objective:** Replace the current `.botminter/` clone workspace model with dedicated workspace repositories on GitHub. `bm teams sync --push` creates a GitHub repo per member, initializes it with submodules (team repo + project forks), copies context files to the root, and assembles the agent directory.

**Implementation guidance:**

1. Update `workspace.rs` with new workspace creation flow:
   1. Create GitHub repo: `gh repo create <org>/<team>-<member> --private`
   2. Clone locally: `git clone <url> workzone/<team>/<member>/`
   3. Add team repo submodule: `git submodule add <team-repo-url> team`
   4. Checkout member branch in team submodule: `git -C team checkout -b <member>`
   5. For each assigned project: `git submodule add <fork-url> projects/<project>`
   6. Checkout member branch in project submodules
   7. Copy context files from `team/members/<member>/` to workspace root (`CLAUDE.md`, `PROMPT.md`, `ralph.yml`)
   8. Assemble agent dir (e.g., `.claude/agents/`) with symlinks into submodule paths:
      - Team-level: `team/coding-agent/agents/*.md`
      - Project-level: `team/projects/<project>/coding-agent/agents/*.md`
      - Member-level: `team/members/<member>/coding-agent/agents/*.md`
   9. Write `.gitignore` (for `.ralph/`, agent dir)
   10. Write `.botminter.workspace` marker file
   11. Commit and push
2. Update workspace discovery to use `.botminter.workspace` marker instead of `.botminter/` directory
3. Remove old `.botminter/` clone logic (Alpha policy: no backwards compat)
4. Update error handling per design:
   - Repo already exists → actionable error with `gh repo delete` command
   - Submodule failure → actionable error with `gh repo view` command

**Test requirements:**
- Integration test (local, no GitHub): workspace repo structure created with correct layout
- `.botminter.workspace` marker exists in workspace root
- `team/` submodule is present and points to team repo
- `projects/<name>/` submodule present for each assigned project
- `CLAUDE.md`, `PROMPT.md`, `ralph.yml` exist at workspace root (copied from team submodule)
- Agent dir contains symlinks into submodule paths at all three scopes
- `.gitignore` contains agent dir and `.ralph/`
- E2E test (GitHub): `bm teams sync --push` creates GitHub repo with naming convention `<team>-<member>`
- E2E test: submodules are properly initialized on GitHub

**Integration notes:** This is the biggest individual step in the milestone. The workspace model is fundamental — `bm start`, `bm status`, and all member operations depend on it. Old tests that reference `.botminter/` must be updated.

**Demo:**
```bash
bm teams sync --push
tree workzone/my-team/alice/ -L 2
# Shows team/ (submodule), projects/, CLAUDE.md, PROMPT.md, ralph.yml at root
```

---

### Step 11: Workspace Sync + `bm start` Adaptation

**Objective:** Implement the sync flow for existing workspace repos (submodule update, context file re-copy, agent dir re-assembly). Adapt `bm start` to launch Ralph from the workspace repo root.

**Implementation guidance:**

1. Implement existing workspace sync flow in `workspace.rs`:
   1. `git submodule update --remote` to fetch latest changes
   2. Checkout member branch in each submodule (never leave detached HEAD)
   3. Re-copy context files if team submodule versions are newer (compare timestamps or content)
   4. Re-copy `ralph.yml` if newer
   5. Re-assemble agent dir symlinks (idempotent)
   6. Commit changes (if any) and push
2. Add `-v` verbose flag to `bm teams sync` for submodule update status, branch checkout results, errors
3. Update `bm start`:
   - Discover workspaces by scanning `workzone/<team>/` for directories with `.botminter.workspace` marker
   - Launch: `cd workzone/<team>/<member>/ && ralph run -p PROMPT.md --env GH_TOKEN=...`
   - Same PID tracking in `~/.botminter/state.json`
4. Update `bm stop` for new workspace paths

**Test requirements:**
- Sync updates submodules to latest
- Context files re-copied when team submodule has newer versions
- Agent dir symlinks rebuilt correctly (idempotent)
- `bm start` discovers workspaces via `.botminter.workspace` marker
- `bm start` launches Ralph at workspace root with correct env
- `bm stop` stops processes launched from new workspace paths
- Verbose output (`-v`) shows submodule status

**Integration notes:** After this step, the full workspace lifecycle works: create → sync → start → stop. The old workspace model is completely gone.

**Demo:**
```bash
# Modify team repo, then sync workspace
bm teams sync -v
# Shows submodule update status
bm start
bm status
bm stop
```

---

### Step 12: Workspace Status Commands + Sprint 3 Documentation

**Objective:** Update `bm status`, `bm teams show`, and `bm members show` to reflect the workspace repo model. Update all Sprint 3 documentation.

**Implementation guidance:**

1. Update `bm status`:
   - Show workspace repo name and branch for each member
   - Show submodule status (up-to-date vs behind)
2. Update `bm teams show`:
   - Include resolved coding agent and profile source (disk path)
3. Update `bm members show <member>`:
   - Include workspace repo URL, checked-out branch, submodule status, resolved coding agent
4. Documentation updates per design matrix (Sprint 3):
   - `docs/content/concepts/workspace-model.md` — major rewrite: workspace repo + submodules
   - `docs/content/concepts/architecture.md` — update runtime diagram
   - `docs/content/how-to/launch-members.md` — rewrite `bm teams sync` workflow
   - `docs/content/reference/cli.md` — update `bm teams sync`, `bm start`
   - `docs/content/concepts/knowledge-invariants.md` — update paths: `team/` instead of `.botminter/`

**Test requirements:**
- `bm status` shows workspace repo info for each member
- `bm teams show` includes coding agent and profile source
- `bm members show` includes workspace repo details
- No docs reference old `.botminter/` clone pattern
- All docs paths updated from `.botminter/` to `team/` (submodule)

**Integration notes:** This step completes Sprint 3. The workspace model transition is fully visible to the operator through updated commands and documentation.

**Demo:**
```bash
bm status
# Shows workspace repos, branches, submodule status
bm members show alice
# Shows workspace repo URL, coding agent, submodule status
```

---

## Sprint 3.5: Board-Scanner Skill Migration (Completed)

### Step 13: Board-Scanner Skill Migration + Profile Directory Restructuring (Completed)

**Objective:** Replace `board_scanner` hat with `board-scanner` auto-inject skill across all profiles. Restructure profile directories for clarity. Delivered in commit `1f8d406`.

**What was done:**

1. **Board-scanner hat → auto-inject skill** — the hatless coordinator now performs board scanning directly via the skill, eliminating one LLM iteration per scan cycle. Created 4 `SKILL.md` files: team-level for compact profiles (all statuses), member-level for scrum (role-scoped).

2. **Profile directory restructuring** — renamed `members/` → `roles/` in profile skeletons (role templates). Renamed `team/` → `members/` in team repo inner directory (`team/members/<member>/` → `team/members/<member>/`).

3. **Cleanup** — removed `github-mutations-hat-only` invariant, added role-scoped failure events to hats, fixed 149 stale `.botminter/` path references.

**Tasks:** `tasks/step13/`

**Impact on later steps:**
- Step 15: Status-workflow skill extraction scope reduced — board scanning logic already extracted
- Step 16: Team Manager skeleton uses `roles/` directory and auto-inject skill pattern

---

## Sprint 4: Skills Extraction & Ralph Prompt Shipping

### Step 14: Ralph Prompt Shipping

**Objective:** Extract Ralph Orchestrator's hardcoded system prompts into `ralph-prompts/` within each profile. These are reference copies — Ralph still uses its compiled-in versions during orchestration. The profile copies enable Sprint 5's `bm chat` to reconstruct similar context without Ralph at runtime.

**Implementation guidance:**

This step requires reading from the Ralph Orchestrator codebase. The exact source files and content are documented in `research/ralph-injected-prompts.md` and the design's Sprint 4 extraction table.

1. Create `ralph-prompts/` directory in each profile (`profiles/scrum/ralph-prompts/`, etc.)
2. Extract and write the following files (content sourced from Ralph's Rust codebase):

   | Source | Destination | Purpose |
   |--------|-------------|---------|
   | `hatless_ralph.rs` guardrails | `ralph-prompts/guardrails.md` | Guardrails wrapper |
   | `hatless_ralph.rs` orientation | `ralph-prompts/orientation.md` | Role identity framing |
   | `instructions.rs` hat template | `ralph-prompts/hat-template.md` | Custom hat instruction wrapper |
   | `hatless_ralph.rs` workflows | `ralph-prompts/reference/workflows.md` | Workflow variants |
   | `hatless_ralph.rs` event writing | `ralph-prompts/reference/event-writing.md` | Event mechanics |
   | `hatless_ralph.rs` completion | `ralph-prompts/reference/completion.md` | Completion mechanics |
   | `data/ralph-tools.md` | `ralph-prompts/reference/ralph-tools.md` | Task/memory CLI |
   | `data/robot-interaction-skill.md` | `ralph-prompts/reference/robot-interaction.md` | HIL interaction |

3. These are static reference files — no code changes to the `bm` CLI in this step
4. Ensure the content accurately represents what Ralph injects (compare against Ralph's source)

**Test requirements:**
- Each profile contains `ralph-prompts/` with all expected files
- `ralph-prompts/guardrails.md`, `ralph-prompts/orientation.md`, `ralph-prompts/hat-template.md` exist
- `ralph-prompts/reference/` subdirectory exists with workflow, event-writing, completion, ralph-tools files
- Content matches Ralph Orchestrator's compiled-in prompts (manual verification or snapshot test)
- `bm profiles init` extracts `ralph-prompts/` to disk correctly

**Integration notes:** This is a content-only step within profiles. No CLI behavior changes. The extracted prompts become inputs for `bm chat` in Step 17.

**Demo:**
```bash
tree profiles/scrum/ralph-prompts/
# Shows guardrails.md, orientation.md, hat-template.md, reference/
```

---

### Step 15: Status-Workflow Skill Extraction + Sprint 4 Documentation

**Objective:** Extract the status *mutation* helpers (duplicated across hat instructions) into a shared `coding-agent/skills/status-workflow/` skill. Update Sprint 4 documentation.

> **Scope note (Step 13 impact):** Board scanning queries were already extracted into the `board-scanner` auto-inject skill in Step 13. This step focuses on the remaining duplication: status field update mutations and label operations that hats still inline.

**Implementation guidance:**

1. Identify the status *mutation* logic still duplicated across hats in each profile's `ralph.yml`:
   - Status field update mutations (`gh project item-edit`)
   - Label operations for status transitions
   - *(Board scanning queries are already in the `board-scanner` skill — do not duplicate)*
2. Create `coding-agent/skills/status-workflow/` in each profile:
   - `SKILL.md` — YAML frontmatter + markdown following the existing skill pattern
   - `references/` — GraphQL mutation templates
3. Update hat instructions in `ralph.yml` to reference the shared skill instead of inlining mutation logic
4. Documentation updates per design matrix (Sprint 4):
   - `docs/content/reference/configuration.md` — document skill format and scoping
   - `docs/content/concepts/architecture.md` — add skills extraction concept

**Test requirements:**
- `coding-agent/skills/status-workflow/SKILL.md` exists in each profile
- Skill covers status mutations and label operations (not board scanning — that's in `board-scanner`)
- Hat instructions in `ralph.yml` reference the shared skill (no more inline duplication)
- `bm profiles init` extracts skills to disk correctly
- Existing functionality is preserved (skills are read by Ralph via `skills.dirs` config)

**Integration notes:** This step completes Sprint 4. Combined with the `board-scanner` skill from Step 13, hats are now focused on domain logic — all board scanning and status workflow mechanics live in shared skills.

**Demo:**
```bash
cat profiles/scrum/coding-agent/skills/status-workflow/SKILL.md
# Shows skill definition with status mutation logic
```

---

## Sprint 5: Team Manager Role

### Step 16: Team Manager — Role Definition + Skeleton

**Objective:** Add the team-manager role to the scrum profile. Create the member skeleton with minimal statuses, hat instructions, and context file. The Team Manager operates on the team repo (its default project) with a simple 3-status workflow.

**Implementation guidance:**

1. Update `botminter.yml` in scrum profile:
   - Add `team-manager` to `roles:` list with description
   - Add `mgr:todo`, `mgr:in-progress`, `mgr:done` to `statuses:`
   - Add `role/team-manager` label (color: `"5319E7"`)
2. Create `profiles/scrum/roles/team-manager/`:
   ```
   .botminter.yml          # role: team-manager, emoji: 📋
   context.md              # Team manager context (→ CLAUDE.md)
   ralph.yml               # Persistent loop, executor hat
   coding-agent/
     agents/
     skills/
       board-scanner/      # Auto-inject skill scoped to mgr:* statuses
         SKILL.md
   hats/
     executor/
       knowledge/
   knowledge/
   invariants/
   ```
3. Write `coding-agent/skills/board-scanner/SKILL.md` — auto-inject skill targeting `mgr:todo` status, `role/team-manager` label (follow pattern from existing roles)
4. Write `ralph.yml` — persistent loop with executor hat, `skills.dirs` pointing to `team/coding-agent/skills`
5. Write `context.md` — team manager role context (with agent tags for Claude Code-specific sections)
6. Write executor hat instructions: pick up tasks from board, execute in `team/` submodule, transition through `mgr:` statuses
7. Write `.botminter.yml`: `role: team-manager`, `comment_emoji: "📋"`

**Test requirements:**
- `bm roles list` shows `team-manager` role after profile refresh
- `bm hire team-manager` creates member skeleton with all expected files
- Hired member's `ralph.yml` has persistent loop with executor hat
- Role's `coding-agent/skills/board-scanner/SKILL.md` scans for `mgr:todo` status
- Hired member's `.botminter.yml` has correct role and emoji
- E2E test: `mgr:todo`, `mgr:in-progress`, `mgr:done` status labels bootstrapped on GitHub during `bm init`
- E2E test: `role/team-manager` label bootstrapped on GitHub during `bm init`

**Integration notes:** After this step, the Team Manager can be hired and launched via `bm start` — it runs as a regular Ralph instance scanning for `mgr:` status issues. The `bm chat` capability (interactive session) comes in Step 17.

**Demo:**
```bash
bm hire team-manager --name bob
bm members show bob
# Shows role: team-manager, statuses: mgr:todo/in-progress/done
```

---

### Step 17: `bm chat` — Interactive Session Launcher

**Objective:** Implement `bm chat <member> [-t team] [--hat <hat>] [--render-system-prompt]`. This command builds a meta-prompt from Ralph's shipped prompts, guardrails, hat instructions, and PROMPT.md, then launches the coding agent with `--append-system-prompt-file`.

**Implementation guidance:**

1. Add `chat` subcommand to `cli.rs`:
   - Required: `<member>` positional argument
   - Optional: `-t`/`--team`, `--hat <hat>`, `--render-system-prompt`
2. Implement `commands/chat.rs`:
   1. Resolve workspace path for the member
   2. Resolve coding agent from team config
   3. Read `ralph.yml` from workspace root (guardrails, hat definitions)
   4. Read Ralph prompts from disk profile's `ralph-prompts/`
   5. Read `PROMPT.md` from workspace root
   6. Build meta-prompt following the design's template:
      ```
      # Interactive Session — [Role Name]
      You are [member name], a [role] on the [team name] team.
      ...
      ## Your Capabilities
      [Hat instructions — active hat or all hats]
      ## Guardrails
      [From ralph.yml]
      ## Role Context
      [PROMPT.md content]
      ## Reference: Operation Mode
      [Path to ralph-prompts/reference/]
      ```
   7. If `--hat` specified: include only that hat's instructions (hat-specific mode)
   8. If no `--hat`: include all hats' instructions (hatless mode)
   9. If `--render-system-prompt`: print meta-prompt to stdout and exit
   10. Otherwise: write meta-prompt to temp file, launch coding agent:
       ```
       Command::new(&coding_agent.binary)
           .current_dir(&ws_path)
           .arg("--append-system-prompt-file")
           .arg(&prompt_file)
           .exec();
       ```
3. Implement `build_meta_prompt()` as a testable function (string in, string out)

**Test requirements:**
- `bm chat <member>` resolves correct workspace and coding agent
- Meta-prompt contains: role identity, guardrails, PROMPT.md content, interactive mode framing
- Hatless mode: meta-prompt includes all hats' instructions
- Hat-specific mode (`--hat executor`): meta-prompt includes only executor hat instructions
- `--render-system-prompt`: prints to stdout, does not launch agent
- `--render-system-prompt --hat executor`: prints hat-specific prompt
- Meta-prompt does NOT include event-writing, workflow, or completion mechanics as active directives (they're reference-only)
- `bm chat nonexistent-member` produces helpful error
- Integration test: `build_meta_prompt()` produces well-structured markdown

**Integration notes:** This step delivers the role-as-skill pattern. Any hired member (not just Team Manager) can be interacted with via `bm chat`. The command works because Sprint 4 shipped the Ralph prompts to the profile, and Step 16 created the Team Manager skeleton.

**Demo:**
```bash
bm chat bob --render-system-prompt
# Prints the meta-prompt to stdout — inspect it

bm chat bob --hat executor
# Launches interactive session as the executor hat

bm chat bob
# Launches interactive session in hatless mode
```

---

### Step 18: Sprint 5 Documentation

**Objective:** Update documentation for the Team Manager role and `bm chat` command.

**Implementation guidance:**

Per the design's documentation impact matrix (Sprint 5):

1. `docs/content/reference/member-roles.md` — add team-manager role description
2. `docs/content/reference/process.md` — add `mgr:` statuses and `role/team-manager` label
3. `docs/content/reference/cli.md` — document `bm chat <member> [-t team] [--hat <hat>] [--render-system-prompt]`
4. `docs/content/concepts/coordination-model.md` — add team-manager to coordination model; mention role-as-skill pattern

**Test requirements:**
- `bm chat` documented in CLI reference
- Team Manager role documented in member roles
- `mgr:` statuses documented in process reference

**Integration notes:** Docs-only step.

**Demo:**
```bash
grep "bm chat" docs/content/reference/cli.md
# Shows command documentation
```

---

## Sprint 6: Minty — BotMinter Interactive Assistant

### Step 19: Minty Config Structure + Launch Command

**Objective:** Create the Minty configuration structure at `~/.config/botminter/minty/` and implement the `bm minty [-t team]` launch command. Minty is a thin persona shell — the actual capabilities come from skills (Step 20).

**Implementation guidance:**

1. Create Minty's embedded config (shipped with the binary, extracted alongside profiles):
   ```
   minty/
     prompt.md           # Minty persona + system instructions
     config.yml          # Minty-specific config
     skills/             # Empty initially — populated in Step 20
   ```
2. Write `prompt.md` — Minty persona:
   - Friendly, approachable assistant for BotMinter operators
   - Aware of BotMinter concepts: profiles, teams, config, CLI, conventions
   - Instructs the agent to use available skills for BotMinter operations
   - Cross-team awareness (reads from `~/.botminter/` when available)
   - Graceful handling of missing runtime data
3. Add `minty` subcommand to `cli.rs`:
   - Optional: `-t`/`--team`
4. Implement `commands/minty.rs`:
   1. Ensure Minty is initialized at `~/.config/botminter/minty/`
   2. Resolve coding agent (from team config if `-t` specified, or from first available profile)
   3. Launch coding agent in current working directory with `--append-system-prompt-file` pointing to Minty's prompt
   4. If `~/.botminter/` doesn't exist: log note about profiles-only mode
5. Update `bm profiles init` to also extract Minty config to `~/.config/botminter/minty/`

**Test requirements:**
- `bm profiles init` extracts `minty/` alongside profiles
- `bm minty` launches coding agent with Minty's system prompt
- `bm minty` works when `~/.botminter/` doesn't exist (profiles-only mode with note)
- `bm minty -t my-team` resolves coding agent from team config
- Minty's prompt.md contains persona, BotMinter awareness, and skill usage instructions

**Integration notes:** After this step, `bm minty` launches a coding agent session but without BotMinter-specific skills yet — the agent has the persona and general Claude Code capabilities. Skills come in Step 20.

**Demo:**
```bash
bm profiles init
bm minty
# Launches interactive session with Minty persona
```

---

### Step 20: Minty Skills + Sprint 6 Documentation

**Objective:** Implement Minty's composable skills and update Sprint 6 documentation. Skills provide all BotMinter-specific capabilities — Minty's persona shell just orchestrates them.

**Implementation guidance:**

1. Create skills in `minty/skills/` (embedded, extracted by `bm profiles init`):
   - **`team-overview/SKILL.md`** — reads `~/.botminter/config.yml`, lists teams, members, status. Shows workspace repo URLs, member roles, running state.
   - **`profile-browser/SKILL.md`** — reads `~/.config/botminter/profiles/`, lists and describes available profiles, roles, coding agents, statuses.
   - **`hire-guide/SKILL.md`** — interactive guide for `bm hire` decisions. Shows available roles, explains implications, suggests names.
   - **`workspace-doctor/SKILL.md`** — diagnoses common workspace issues: stale submodules, broken symlinks, missing files, outdated context. Runs checks and reports findings.
2. Each skill follows the SKILL.md pattern (YAML frontmatter + markdown + optional scripts)
3. Documentation updates per design matrix (Sprint 6):
   - `docs/content/reference/cli.md` — document `bm minty [-t team]`
   - `docs/content/concepts/profiles.md` — mention Minty config alongside profiles under `~/.config/botminter/`
   - `docs/content/faq.md` — add Minty FAQ entry

**Test requirements:**
- All four skills exist with valid SKILL.md format
- `bm profiles init` extracts skills to `~/.config/botminter/minty/skills/`
- Each skill is discoverable by the coding agent from Minty's skills directory
- `bm minty` documented in CLI reference
- Minty FAQ entry explains what Minty is and how it differs from team members

**Integration notes:** This is the final step of the milestone. After this, the full Minty and Friends feature set is delivered: coding-agent-agnostic architecture, disk-based profiles, workspace repos, skills extraction, Team Manager with `bm chat`, and Minty assistant.

**Demo:**
```bash
bm minty
# "Hi! I'm Minty. I can help you manage your BotMinter teams."
# "What would you like to do? I can show you your teams, browse profiles, help with hiring, or diagnose workspace issues."
```

---

## Cross-Cutting Concerns

### Schema Version Bump

Schema version bumps from `"1.0"` to `"2.0"` as part of Step 2 (when `coding_agents` section is added to `botminter.yml`). The `check_schema_version()` guard catches stale teams — operators re-create from scratch per Alpha policy.

### Test Infrastructure

Steps that change the profile API (Step 8) require test infrastructure updates:
- Integration test setup should extract profiles to a tempdir via the init logic
- Tests should not depend on `~/.config/botminter/` existing on the developer's machine
- E2E tests (`--features e2e`) must be updated for workspace repo model

### ralph.yml Path Updates

~~Step 10 (workspace repo creation) changes all path references in ralph.yml and hat instructions from `.botminter/` to `team/`.~~ **Done in Step 13** — 149 stale `.botminter/` path references fixed as part of the profile directory restructuring.

### `docs/content/roadmap.md`

Update roadmap to mark "Minty and Friends" as **In Progress** when implementation begins (Step 1), and **Complete** when the final step ships.
