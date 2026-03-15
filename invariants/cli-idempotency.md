# CLI Command Idempotency

All state-mutating CLI commands MUST be idempotent — running the same command
with the same arguments multiple times MUST produce the same end state without errors.

## Rule

Every `bm` command that creates, provisions, or modifies external state MUST detect
existing state and handle it gracefully:

- If the resource already exists in the desired state: skip with an info message
- If the resource exists but differs: update to match the desired state
- If the resource does not exist: create it

Commands MUST NOT fail with creation errors when the target resource already exists.
Commands MUST NOT duplicate resources on repeated invocations.

## Applies To

All state-mutating `bm` CLI commands, including but not limited to:

- `bm init` — team creation, repo setup, label/project bootstrap
- `bm teams sync --repos` — workspace repo creation on GitHub
- `bm teams sync --bridge` — bridge identity provisioning
- `bm bridge identity add` — bridge user creation
- `bm bridge room create` — channel/room creation
- `bm hire` — member registration

**Does NOT apply to:**

- Read-only commands (`bm status`, `bm profiles list`, `bm members list`, etc.)
- Destructive commands that are intentionally non-idempotent (`bm stop --force`)
- Test helpers and fixtures

## Examples

**Compliant:**
```rust
// Check if repo exists before creating
let view_output = Command::new("gh")
    .args(["repo", "view", &repo_name, "--json", "name"])
    .output()?;
if view_output.status.success() {
    eprintln!("Repo '{}' already exists, skipping creation.", repo_name);
} else {
    Command::new("gh").args(["repo", "create", &repo_name, "--private"]).output()?;
}
```

**Violating:**
```rust
// Unconditionally creates — fails if repo exists
Command::new("gh")
    .args(["repo", "create", &repo_name, "--private"])
    .output()?;
```

## Rationale

Operators re-run `bm teams sync` after hiring new members, adding projects, or
configuring bridges. Each invocation must safely handle resources created by previous
runs. Phase 9 UAT uncovered this gap: `bm teams sync --repos` failed with a 502
when the workspace repo already existed on GitHub, because `create_workspace_repo`
called `gh repo create` unconditionally. Idempotency is a baseline expectation for
any GitOps-style CLI tool.
