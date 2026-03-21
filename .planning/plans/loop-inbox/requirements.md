# Requirements: Loop Inbox — Brain-to-Loop Feedback Channel

## Problem Statement

The brain process can observe loop activity (via event files) but cannot send feedback to running loops. This creates a one-way observation channel. When a human redirects work ("stop refactoring, fix CI"), the brain has no mechanism to relay that directive to the coding agent executing inside a loop.

## Stakeholders

- **Brain process**: The consciousness layer that monitors loops, receives human messages via bridge, and manages the board. Needs to steer loops.
- **Coding agent (inside loop)**: The Claude Code instance executing work inside a Ralph loop. Needs to receive and comply with brain directives.
- **Human operator**: Communicates with the brain via bridge chat. Expects their redirections to actually affect running work.

## Functional Requirements

### FR-1: Brain can send messages to a running loop

The brain MUST be able to send a text message targeted at the primary running loop. The message MUST be persisted durably (survives process restarts) until consumed.

**Acceptance criteria:**
- Given a running loop, when the brain sends a message, then the message is persisted and available for the loop's coding agent to read
- Given multiple messages sent before consumption, when the coding agent reads, then all messages are returned in chronological order
- Given no running loop, the message is still persisted (fire-and-forget semantics)
- Given a write failure (disk full, permissions), the system MUST report the error to the caller — the failure MUST NOT crash the brain or affect other operations
- An empty message MUST be rejected with an error

### FR-2: Inbox targets the primary loop

The inbox targets the primary loop in the workspace. Multi-loop targeting (worktree loops) is deferred to a future milestone when the worktree model is implemented.

**Acceptance criteria:**
- When the brain sends a message, it targets the primary loop's inbox
- The system MUST be designed so that multi-loop targeting can be added without breaking changes

### FR-3: Coding agent receives inbox messages during execution

Pending inbox messages MUST be delivered to the coding agent without manual operator intervention. The coding agent's subsequent behavior MUST reflect awareness of the delivered message.

**Acceptance criteria:**
- Given a pending inbox message, the message content is delivered to the coding agent automatically
- Given no pending messages, execution proceeds with zero overhead or user-visible noise
- Given multiple pending messages, all are delivered together

### FR-4: Messages are consumed on delivery (best effort)

Once inbox messages are delivered to the coding agent, they SHOULD be consumed (removed from the inbox) so they are not re-delivered. Consumption is best-effort: there is a small window where messages may be lost if the agent process crashes between consumption and processing.

**Acceptance criteria:**
- Given a message has been delivered, when the coding agent performs subsequent actions, then the same message is NOT delivered again
- **Known limitation:** If the agent process crashes between message consumption and processing, the consumed messages are lost. This is accepted because the delivery mechanism (PostToolUse hooks) has no acknowledgment channel, and the fire-and-forget semantics of the inbox make this an acceptable tradeoff.

### FR-5: Coding agent context includes brain feedback guidance

The coding agent's observable context MUST include guidance that brain feedback takes priority over current work.

**Acceptance criteria:**
- The guidance text is present in the agent's observable context (verifiable by inspection)
- The guidance covers: priority of brain feedback, expected acknowledgment behavior, and conflict resolution (feedback wins)

### FR-6: Human operator can inspect pending messages

The operator (or brain) MUST be able to view pending messages without consuming them.

**Acceptance criteria:**
- Given pending messages, when peek is requested, then messages are displayed but remain in the inbox
- Given no pending messages, peek returns an empty result (no error)

### FR-7: Inbox is workspace-scoped

Each loop's inbox is naturally scoped to its workspace. Messages cannot leak between workspaces or between different team members.

**Acceptance criteria:**
- Given two different workspaces, messages sent to one inbox are not visible in the other
- The inbox location is deterministic from the workspace root

### FR-8: Workspace provisioning enables inbox capability

After workspace provisioning or sync, the inbox feature MUST be functional without additional manual steps by the operator.

**Acceptance criteria:**
- Given a freshly synced workspace, the inbox capability is ready to use
- Given a re-synced workspace, pending inbox messages are preserved
- The setup is idempotent (multiple syncs produce the same result)

### FR-9: Brain context documents inbox capability

The brain's context MUST include instructions for when and how to send inbox messages, and when NOT to use the inbox.

**Acceptance criteria:**
- The brain's context includes guidance on when to use inbox (redirect, context passing) and when NOT to use it (status checks, loop stop, new work)
- The brain's context includes the interface for sending messages

### FR-10: Orphaned messages survive loop lifecycle

Messages that are not consumed before a loop completes MUST remain available to the next loop instance started in the same workspace.

**Acceptance criteria:**
- Given a loop that completes without consuming pending messages, those messages are delivered to the next loop instance started in the same workspace
- Given a loop that crashes, pending messages survive for the next loop instance

## Non-Functional Requirements

### NFR-1: Concurrency safety

Multiple concurrent writers and readers MUST be safe. No lost messages, no corrupted reads. This includes the scenario of multiple brain processes writing to the same inbox simultaneously.

### NFR-2: Minimal overhead on coding agent

The inbox check on the coding agent side MUST add negligible latency when no messages are pending. The common case (empty inbox) should be near-zero cost.

### NFR-3: Graceful degradation

If the inbox mechanism is unavailable (e.g., binary not found, file permissions), the coding agent MUST continue working normally — inbox is advisory, not blocking.

### NFR-4: Message attribution

Each message MUST include the sender identity and a machine-parseable timestamp so the coding agent can understand context and recency.

## Out of Scope

- Two-way communication (agent replying to brain via inbox) — brain already has event observation
- Message acknowledgment protocol — fire-and-forget is sufficient
- Message expiration / TTL — messages persist until consumed
- Inbox for non-coding-agent consumers (e.g., other brain processes)
- Encryption or authentication of inbox messages (workspace-local trust boundary)
- Delivered/consumed message history or audit log (peek of pending messages is sufficient for debugging)
- Profile parity beyond scrum-compact (other profiles can adopt inbox in a follow-up)
- Multi-loop targeting via `--loop` flag (deferred until worktree model is implemented)
- Message size limits (no practical risk for text messages in v1)
