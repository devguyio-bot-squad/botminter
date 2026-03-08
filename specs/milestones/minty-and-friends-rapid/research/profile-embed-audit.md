# Research: Profile Embedding Mechanism Audit

> Analysis of the compile-time profile embedding for externalization to disk-based storage.

## Summary

Profiles are embedded via `include_dir!` macro into a static `Dir<'static>`. Six public functions form the profile API. After externalization, all profile operations read exclusively from disk (`~/.config/botminter/profiles/`). The embedded profiles are used ONLY by `bm profiles init` to seed the disk — no fallback, no two-tier resolution.

## Embedding Mechanism

```rust
// crates/bm/src/profile.rs:8
static PROFILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../profiles");
```

- `include_dir` crate (v0.7) compiles entire `profiles/` tree into binary
- Zero-cost runtime access via `Dir<'static>` — no filesystem I/O needed
- Three profiles embedded: `scrum`, `scrum-compact`, `scrum-compact-telegram`

## Profile API

| Function | Signature | Used By |
|----------|-----------|---------|
| `list_profiles()` | `→ Vec<String>` | `bm init`, `bm profiles list` |
| `read_manifest(name)` | `→ ProfileManifest` | `bm init`, `bm profiles list/describe`, `bm roles list` |
| `list_roles(name)` | `→ Vec<String>` | `bm profiles describe`, `bm hire` |
| `extract_profile_to(name, target)` | `→ Result<()>` | `bm init` (team repo creation) |
| `extract_member_to(name, role, target)` | `→ Result<()>` | `bm hire` (member skeleton) |
| `check_schema_version(name, schema)` | `→ Result<()>` | `bm teams sync` |
| `embedded_profiles()` | `→ &'static Dir` | Internal |

### Extraction Logic

**`extract_profile_to()`**: Copies profile to team repo, skipping `members/` and `.schema/`.
**`extract_member_to()`**: Copies member skeleton to `team/<member-dir>/`, no skip predicate.

Both use `extract_dir_recursive()` which walks the `Dir` tree and writes files to disk.

## Profile Directory Structure

```
profiles/<name>/
  botminter.yml              # Manifest (name, roles, labels, statuses, views)
  .schema/                   # Internal schema defs (NOT extracted to team repo)
  PROCESS.md                 # Team process conventions
  CLAUDE.md                  # Team context document
  agent/agents/              # Team-level agents
  agent/skills/              # Team-level skills (e.g., gh/)
  skills/                    # Additional skills
  formations/                # Deployment configs (local/, k8s/)
  knowledge/                 # Team knowledge docs
  invariants/                # Team invariants
  members/<role>/            # Role templates (NOT extracted to team repo)
    .botminter.yml           # Member manifest (role, comment_emoji)
    PROMPT.md, CLAUDE.md, ralph.yml
    agent/agents/, agent/skills/
    hats/<hat>/knowledge/
    knowledge/, invariants/, projects/
```

## `botminter.yml` Schema (ProfileManifest)

```rust
pub struct ProfileManifest {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub schema_version: String,
    pub roles: Vec<RoleDef>,
    pub labels: Vec<LabelDef>,
    pub statuses: Vec<StatusDef>,
    pub projects: Vec<ProjectDef>,
    pub views: Vec<ViewDef>,
}
```

## Externalization Plan (from requirements)

### New Storage Layout
```
~/.config/botminter/
  profiles/                  # Disk profiles — the ONLY source for all operations
    scrum/
    scrum-compact/
    scrum-compact-telegram/
  minty/                     # Minty assistant config (separate concern)
```

### New Command: `bm profiles init`
- Extracts embedded profiles to `~/.config/botminter/profiles/`
- Warns if profiles already exist, offers overwrite/skip
- `--force` flag for scripted use
- This is the ONLY use of embedded profiles — no fallback

### API Changes

The profile API functions switch entirely to **disk-based reads**. Embedded profiles (`include_dir!`) are only accessed by `bm profiles init` for extraction. There is no fallback — if profiles are not on disk, the command fails.

| Function | Change |
|----------|--------|
| `list_profiles()` | Read from disk directory listing (`~/.config/botminter/profiles/`) |
| `read_manifest(name)` | Read `botminter.yml` from disk |
| `list_roles(name)` | Read `members/` from disk |
| `extract_profile_to()` | Read from disk instead of embedded |
| `extract_member_to()` | Read from disk instead of embedded |
| `check_schema_version()` | Read from disk |
| `embedded_profiles()` | Used ONLY by `bm profiles init` — not called by any other command |

### Auto-Prompt Pattern

Commands requiring profiles detect missing `~/.config/botminter/profiles/` and prompt interactively:
> "Profiles not initialized. Do you want me to initialize them now?"

- If yes → run initialization inline, then continue with the original command
- If no → friendly message: "OK. You can initialize them any time using `bm profiles init`." — then abort gracefully (not an error)

## Testing Considerations

- Existing tests use embedded profiles directly — need to either:
  - Extract to tempdir in test setup, or
  - Keep `embedded_profiles()` accessible for test convenience
- E2E tests should test the full disk-based flow
