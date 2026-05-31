# Session Model

BotMinter uses an **ephemeral session** model for team member workspaces. Each `bm start` command creates ephemeral workspaces that are cleaned up when you stop the session, enabling flexible on-demand collaboration without manual workspace management.

## Ephemeral Sessions

An **ephemeral session** is a temporary, isolated workspace created on-demand for a team member. Sessions are:

- **Temporary** — Automatically cleaned up after use
- **Isolated** — Each session has its own workspace directory
- **On-demand** — Created when you start a member, destroyed when you stop

### Creating Ephemeral Sessions

The `bm start` command **creates ephemeral** workspaces automatically:

```bash
# Start a member in an ephemeral session
bm start alice

# Start multiple members
bm start alice bob charlie
```

Each invocation provisions fresh workspaces in `.sessions/<session-id>/` with team and project repositories synchronized.

### Stopping and Cleanup

Sessions can be stopped with optional work preservation:

```bash
# Stop and discard changes (default)
bm stop alice

# Stop and preserve work for later review
bm stop --preserve alice
```

The `--preserve` flag saves uncommitted work and unpushed commits before cleanup, allowing you to inspect or recover changes later.

## Session Lifecycle

Sessions progress through several states:

1. **Created** — Fresh workspace provisioned, repos cloned
2. **Running** — Member's Ralph loop is active
3. **Stopped** — Ralph process terminated, workspace retained temporarily
4. **Finalized** — Work preserved (if requested) or discarded
5. **Cleaned** — Workspace directory removed

### Session Daemon

The **session daemon** is a background process (part of `bm start`) that manages session lifecycle events. It:

- Monitors running Ralph processes
- Triggers session finalization when members stop
- Orchestrates session garbage collection
- Maintains session state and metadata

The daemon runs automatically — no manual management required.

## Session Retention

The **session retention** policy controls how long stopped sessions remain on disk before garbage collection:

- **Default retention**: 24 hours after stop
- **Preserved sessions**: 7 days (set via `--preserve`)
- **Active sessions**: No expiration while running

Retention periods ensure you have time to review preserved work while preventing unbounded disk growth from abandoned sessions.

## Session Garbage Collection

**Session garbage collection** automatically removes expired sessions. The session daemon runs GC periodically (every 6 hours by default) to:

1. Identify stopped sessions past their retention period
2. Verify work has been preserved if requested
3. Remove workspace directories
4. Update session indexes

You can trigger manual GC:

```bash
# Clean up all expired sessions immediately
bm sessions gc

# Show what would be removed (dry-run)
bm sessions gc --dry-run
```

## Session Management Operations

BotMinter provides several operations for managing sessions, including **session inspection**, **session cleanup**, and **session finalization**.

### Session Inspection

The **session inspection** operation lets you view active and recent sessions:

```bash
# Show all sessions with session detail
bm status

# List only active sessions
bm sessions list --active

# Inspect a specific session
bm sessions show <session-id>
```

The **session detail** view shows session ID, member name, state, creation time, and retention deadline.

### Session Cleanup

The **session cleanup** operation removes sessions manually when needed:

```bash
# Clean a specific stopped session
bm sessions clean <session-id>

# Clean all stopped sessions (respects retention)
bm sessions clean --all

# Force cleanup (bypass retention period)
bm sessions clean --force <session-id>
```

### Session Finalization

The **session finalization** operation is the preservation step that runs when you stop with `--preserve`. You can re-trigger session finalization if it failed:

```bash
# Re-run finalization for a stopped session
bm sessions finalize <session-id>

# Show finalization status
bm sessions show <session-id> --include-finalization
```

Finalization bundles uncommitted changes, unpushed commits, and Ralph state into a timestamped archive under the team directory.

## Migration from Permanent Workspaces

Prior to the session model, BotMinter used permanent workspaces provisioned during team setup. If you have existing permanent workspaces:

1. **They continue to work** — Old workspaces are not affected
2. **New starts use sessions** — `bm start` creates ephemeral sessions
3. **Gradual migration** — Convert members to sessions as needed:

```bash
# Archive old workspace (optional)
tar -czf ~/backups/alice-workspace.tar.gz ~/workspaces/my-team/alice

# Remove permanent workspace
rm -rf ~/workspaces/my-team/alice

# Next bm start alice creates an ephemeral session
```

The `bm minty` command is the primary setup workflow. It provisions team infrastructure without creating permanent workspaces.

## See Also

- [CLI Reference](../reference/cli.md) — Full command documentation
- [Workspace Model](workspace-model.md) — Directory layout and structure
- [Daemon Operations](../reference/daemon-operations.md) — Background process management
