# Research: Claude Code Coupling Audit

> Codebase audit identifying all hardcoded Claude Code-specific references that need abstraction.

## Summary

~80 coupling points across 4 categories. The coupling is **structural** (filenames, directory names) not behavioral — making a config-driven mapping feasible.

## Coupling Points by Category

### 1. `workspace.rs` — HIGH coupling (35+ references)

**Constants:**
- `BM_GITIGNORE_ENTRIES` (lines 9–18): hardcodes `"CLAUDE.md"`, `".claude/"` in gitignore list

**`surface_files()` (lines 238–254):**
- Creates `CLAUDE.md` symlink: `workspace_root/CLAUDE.md` → `.botminter/team/<member>/CLAUDE.md`

**`assemble_claude_dir()` (lines 177–236):**
- Creates `.claude/agents/` directory
- Symlinks `.md` files from 3 scopes into `.claude/agents/`
- Copies `settings.local.json` into `.claude/`

**`sync_workspace()` (lines 102–175):**
- Re-copies `settings.local.json` to `.claude/`
- Re-assembles `.claude/agents/` symlinks
- Verifies `CLAUDE.md` symlink integrity

**Tests (lines 500–1000+):**
- ~18 assertions checking `CLAUDE.md` existence, `.claude/agents/` structure

### 2. Profile Templates — HIGH coupling (7 files)

Every profile contains a `CLAUDE.md` at two levels:
- `profiles/<name>/CLAUDE.md` — team-level context document
- `profiles/<name>/members/<role>/CLAUDE.md` — member-level instructions

Files affected:
- `profiles/scrum/CLAUDE.md`
- `profiles/scrum/members/architect/CLAUDE.md`
- `profiles/scrum/members/human-assistant/CLAUDE.md`
- `profiles/scrum-compact/CLAUDE.md`
- `profiles/scrum-compact/members/superman/CLAUDE.md`
- `profiles/scrum-compact-telegram/CLAUDE.md`
- `profiles/scrum-compact-telegram/members/superman/CLAUDE.md`

### 3. `session.rs` — MEDIUM coupling (4 references)

- Line 6: `"Launch an interactive Claude Code session"`
- Line 17: `"'claude' not found in PATH. Install Claude Code first."`
- Line 37: `"Failed to launch Claude Code session"`
- Line 40: `"Claude Code session exited with error"`

Hardcodes the `claude` binary name for `bm chat` / role-as-skill invocation.

### 4. Metadata & Documentation — LOW coupling (20+ references)

- `Cargo.toml` line 5: `description = "Lead your own Claude Code agents"`
- `docs/mkdocs.yml` line 2: `site_description: Lead your own Claude Code agents`
- `docs/overrides/home.html`: multiple HTML references
- `cli.rs` line 3: module doc comment

## Abstraction Strategy (from requirements)

Per A2–A5:
- Config-driven mapping: `coding_agent` field in profile + team override
- Profiles contain agent-specific file variants (one set per supported agent)
- Claude Code is the only concrete implementation for now
- The mapping determines: context file name (`CLAUDE.md`), agent directory (`.claude/`), binary name (`claude`), settings file format

## Key Files to Modify

| File | Change |
|------|--------|
| `workspace.rs` | Parameterize filenames/directories based on `coding_agent` config |
| `profile.rs` | Add agent variant resolution to extraction |
| `session.rs` | Look up binary name from config instead of hardcoding `claude` |
| `profiles/*/` | Organize agent-specific files under variant directories |
| `botminter.yml` | Add `coding_agent` field and `supported_agents` list |
