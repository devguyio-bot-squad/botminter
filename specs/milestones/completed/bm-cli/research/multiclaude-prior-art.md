# Research: Multiclaude Prior Art

> Research for M3 — architecture audit of [multiclaude](https://github.com/dlorenc/multiclaude). Patterns to adopt, patterns to skip, and key architectural differences.

---

## 1. Architecture Overview

Multiclaude is a Go-based client-daemon system for managing multiple Claude Code agents:

```
CLI (thin client) → Unix socket → Daemon (long-running)
                                    ├── tmux session (one per repo)
                                    │     ├── window: supervisor agent
                                    │     ├── window: worker-1 agent
                                    │     └── window: worker-N agent
                                    ├── git worktrees (one per worker)
                                    └── state.json (atomic writes)
```

- **Language:** Go (minimal deps: `fatih/color`, `google/uuid`)
- **IPC:** Unix domain socket with JSON request/response
- **Process container:** tmux (agents are Claude Code processes in tmux windows)
- **Isolation:** Git worktrees (one branch per worker)
- **State:** Single JSON file with atomic write (temp file → rename)
- **Singleton:** PID file at `~/.multiclaude/daemon.pid`

---

## 2. Patterns to Adopt

### A. PID file for daemon singleton
`PIDFile.CheckAndClaim()` — check if PID alive (signal 0), clean stale files, write new PID. Standard Unix technique, directly applicable to `bm`.

### B. Atomic state persistence
Write state to temp file, then rename. Prevents corruption if process crashes mid-write. Use for `~/.botminter/` state.

### C. Session continuity via `--resume`
Multiclaude stores Claude Code session IDs in state and uses `--resume` to restore conversation after crashes. Essential for Ralph instances — store session/loop IDs so `bm start` can resume rather than cold-start.

### D. Health check loop
Periodic check (every 2 minutes) that detects crashed agents. Even if auto-restart is "not P0" for M3, the architecture should support it. At minimum, `bm status` needs to know if a member's process is alive, which means tracking PIDs and checking them.

### E. Separation: definition vs. runtime state
Multiclaude defines agent behavior in config files and tracks runtime state (PID, session ID, tmux window) separately in `state.json`. Maps directly to botminter: member definitions in team repo (`team/<member>/`), runtime state in `~/.botminter/`.

---

## 3. Patterns to NOT Adopt

### A. tmux as process container
**Why not:** `bm start` runs `ralph run`, which already manages its own Claude Code process. tmux would be an unnecessary layer between `bm` and Ralph. Botminter's observability comes from `bm status` (querying Ralph CLI) and the GitHub issue board, not from watching terminal output.

**Instead:** Launch `ralph run` as a background process. Track PIDs directly.

### B. Supervisor agent (central orchestrator)
**Why not:** Botminter's core architecture is decentralized coordination via GitHub issues with status labels. Each member independently pulls work. Adding a supervisor contradicts this.

### C. Git worktrees for isolation
**Why not:** Botminter has its own workspace model with knowledge layering, config surfacing, and project-scoped checkouts. Git worktrees are a lower-level mechanism that doesn't map to this richer model.

### D. Filesystem inter-agent messaging
**Why not:** Botminter members communicate through GitHub issues and PR comments, as defined by the profile's process. A parallel filesystem messaging system would split the coordination fabric.

### E. Agent nudging (periodic pokes)
**Why not:** Ralph has its own orchestration loop with its own timing. External nudging would interfere with Ralph's cadence. The right mechanism is creating work on the issue board.

---

## 4. Key Architectural Differences

| Dimension | Multiclaude | Botminter (`bm`) |
|---|---|---|
| **Model** | Swarm (interchangeable workers) | Team (specialized roles) |
| **Coordination** | Central supervisor agent | Decentralized via GitHub issue labels |
| **Agent runtime** | Raw Claude Code sessions | Ralph orchestrator instances |
| **Isolation** | Git worktrees | Workzone with knowledge layering |
| **Observability** | tmux attach | `bm status` + GitHub board |
| **Communication** | Filesystem JSON messages | GitHub issues/PRs/comments |
| **State source of truth** | `~/.multiclaude/state.json` | Team repo in git |
| **Target user** | Solo dev managing agent swarm | Team operator managing collaborative team |

### The Fundamental Difference

Multiclaude optimizes for **throughput** (many parallel attempts at tasks, CI as quality gate). Botminter optimizes for **structured collaboration** (right member, right work, right order, process-defined handoffs).

The infrastructure patterns (daemon, socket, PID, atomic state) are solid and reusable. The coordination model is fundamentally different and should not be adopted.

---

## Sources

- [multiclaude GitHub](https://github.com/dlorenc/multiclaude)
- multiclaude README, design docs, and source code
