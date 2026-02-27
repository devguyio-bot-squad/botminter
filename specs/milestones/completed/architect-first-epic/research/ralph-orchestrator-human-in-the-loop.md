# Human-in-the-Loop: How Telegram Interaction Works

Ralph's RObot system creates a bidirectional communication bridge between AI agents running inside orchestration loops and a human operator on Telegram. Instead of fire-and-forget autonomy, this gives you a live steering wheel — you can answer agent questions, inject guidance, and monitor progress, all from your phone.

This guide goes deep into how the system actually works, the mental models behind it, and the operational patterns that matter when running production loops.

## The Three Communication Flows

Everything in RObot boils down to three distinct flows, each with different behavior:

| Flow | Direction | Blocking? | When It Happens |
|------|-----------|-----------|-----------------|
| **Question** (`human.interact`) | Agent &rarr; Human &rarr; Agent | Yes | Agent hits a decision point and explicitly asks |
| **Guidance** (`human.guidance`) | Human &rarr; Agent | No | You proactively send a message (not a reply) |
| **Check-in** | Agent &rarr; Human | No | Periodic status update on a timer |

Understanding which flow is which — and especially which ones block — is the key to using RObot effectively.

---

## Flow 1: Agent Asks You a Question

This is the **blocking question/answer flow**. It's the most important interaction pattern to understand because it literally pauses the orchestration loop.

### What happens, step by step

1. During execution, the agent decides it needs human input. It emits a `human.interact` event with its question as the payload.

2. The event loop intercepts this event before publishing it to the bus. It detects `human.interact` and switches into blocking mode.

3. The question is sent to Telegram via the bot, formatted with context — the hat name, iteration number, and loop ID — so you know what's asking and why.

4. **The event loop blocks.** It enters a tight polling loop, checking the events file every 250 milliseconds for a `human.response` line to appear.

5. You see the question on Telegram. You reply.

6. The background polling task picks up your reply, classifies it as a `human.response` (because there's a pending question for that loop), and writes a JSON event to the loop's events file.

7. The blocking poll detects the new line, extracts the payload, and unblocks. Your response is published to the event bus as a `human.response` event.

8. The next hat iteration receives your response in its prompt context and acts on it.

### Timeout behavior

If you don't reply within `timeout_seconds`, the loop **continues without a response**. The agent won't crash or hang forever. It'll proceed with whatever information it has. The pending question is cleaned up either way.

If the message fails to send (network issue, invalid token), it retries with exponential backoff — 1 second, then 2, then 4 — up to 3 attempts total. If all retries fail, the event loop treats it the same as a timeout: log the failure, continue without blocking.

### Why it blocks

You might wonder why the loop doesn't just keep going and check for a response later. The design is intentional: if the agent asked a question, the answer likely changes the entire direction of the next iteration. Running a full iteration with stale context and then discarding it wastes tokens and time. Blocking is the cheaper option.

---

## Flow 2: You Send Proactive Guidance

This is the **non-blocking steering mechanism**. You don't need to wait for the agent to ask — you can inject guidance at any time.

### What happens, step by step

1. You send a message to the bot that is **not a reply** to a pending question. (If there's no pending question at all, every message is guidance.)

2. The bot classifies this as `human.guidance` and writes it to the loop's events file.

3. The bot reacts with an eyes emoji and sends a short acknowledgment: "Guidance received — will apply next iteration."

4. At the start of the next iteration, during prompt construction, the event loop collects all `human.guidance` events, deduplicates them, and squashes them into a numbered list.

5. This list is injected into the agent's prompt under a `## ROBOT GUIDANCE` header. The agent sees it as authoritative direction from the human operator.

6. Guidance is also persisted to the scratchpad, so it survives process restarts.

### When to use guidance vs. questions

Use guidance when you're watching the loop and want to steer it without waiting for it to ask. Common scenarios:

- You notice the agent going down the wrong path → send correction
- You want to add a constraint the original prompt didn't mention → inject it
- You want to reprioritize → tell it what to focus on next

The key difference from a question: guidance is **fire-and-forget from the agent's perspective**. The agent doesn't know guidance is coming and doesn't wait for it. It just finds it in its prompt on the next iteration.

---

## Flow 3: Periodic Check-ins

If you configure `checkin_interval_seconds`, the event loop sends you periodic status messages. These are purely informational — the agent doesn't block, doesn't expect a reply, and won't even know the check-in was sent.

A check-in message includes:

- **Iteration number** and elapsed wall-clock time
- **Current hat** (e.g., executor, reviewer, planner)
- **Task progress** — open and closed counts
- **Cumulative cost** in USD

This is useful for long-running loops where you want a heartbeat without watching the TUI.

---

## Parallel Loop Routing

When you run multiple loops in parallel via worktrees, there's a single Telegram bot handling all of them. Only the **primary loop** (the one holding `.ralph/loop.lock`) runs the Telegram service. Worktree loops don't start their own bots — they share the primary loop's bot for both sending and receiving.

This means you need a way to route messages to the right loop. The system uses a three-tier priority:

### Priority 1: Reply-to routing

If you reply directly to a bot message (a question from a specific loop), the reply is automatically routed to the loop that sent that message. This is the most natural and reliable routing method.

Under the hood, when a question is sent, the Telegram message ID is stored in state as a `PendingQuestion` keyed by loop ID. When a reply comes in, the bot looks up which loop owns that message ID.

### Priority 2: @-prefix routing

Start your message with `@loop-id` to explicitly route it:

- `@feature-auth check the edge cases` → routes to the `feature-auth` worktree loop
- `@ralph-20260130-a3f2 skip the flaky test` → routes to that specific loop

The loop ID is everything between `@` and the first whitespace.

### Priority 3: Default to primary

Any message without reply-to context and without an `@-prefix` goes to the primary (main) loop.

### Where messages land

Each loop has its own events file:

- **Primary loop**: `.ralph/events-<timestamp>.jsonl` (resolved via `.ralph/current-events` marker)
- **Worktree loops**: `.worktrees/<loop-id>/.ralph/events-<timestamp>.jsonl`

The bot resolves the active events file by reading a `current-events` marker file. If the marker doesn't exist, it falls back to the default `events.jsonl`. This marker system ensures events land in the correct timestamped file even after event rotation.

---

## Chat ID Auto-Detection

You don't need to configure your Telegram chat ID anywhere. The bot auto-detects it from the **first message you send**. Here's how:

1. You start a loop with RObot enabled
2. The bot starts polling but has no `chat_id` in state
3. You send any message to the bot (even just "hi")
4. The message handler sees `chat_id: None` in state, records your `chat_id`, and persists it
5. From this point forward, all outgoing messages (questions, check-ins, greetings, farewells) are sent to your chat

The `chat_id` is persisted in `.ralph/telegram-state.json` and survives process restarts. You only need to do this once per project.

---

## Bot Commands

The bot registers slash commands with Telegram so they appear in the autocomplete menu:

| Command | What It Does |
|---------|-------------|
| `/status` | Current loop status — PID, elapsed time, iteration count, prompt preview |
| `/tasks` | Lists all open tasks from `.ralph/agent/tasks.jsonl` |
| `/memories` | Shows the 5 most recent memories |
| `/tail` | Last 20 events from the active events file |
| `/stop` | Requests the loop to stop (writes a signal file; loop terminates at the next iteration boundary) |
| `/restart` | Requests a loop restart (same signal-file mechanism) |
| `/help` | Lists available commands |

Commands are handled inline during the polling loop — they don't write events to the JSONL file and don't trigger any orchestration behavior.

### Signal files for /stop and /restart

`/stop` and `/restart` work by writing marker files (`.ralph/stop-requested`, `.ralph/restart-requested`) that the event loop checks at iteration boundaries. This is a clean, non-disruptive coordination mechanism — the loop finishes its current iteration before acting on the signal.

---

## Lifecycle

### Startup

1. Bot token is resolved (env var → config file → error)
2. State is loaded from `.ralph/telegram-state.json` (or initialized empty)
3. A background async task is spawned for long-polling Telegram updates (10-second poll timeout)
4. Bot commands are registered with Telegram's API
5. If `chat_id` is known, a greeting message is sent

### Running

- The background task continuously polls `getUpdates` and routes incoming messages through the message handler
- The event loop calls `send_question()` and `wait_for_response()` synchronously when `human.interact` events are detected
- Check-ins are sent at the configured interval
- State is persisted after every incoming message

### Shutdown

1. A farewell message is sent to the chat
2. The shutdown flag (`AtomicBool`) is set to `true`
3. The background polling task exits its loop
4. Any active `wait_for_response()` call returns immediately with `None`

The shutdown flag is shared with signal handlers (Ctrl+C, SIGTERM, SIGHUP), so interrupting Ralph will cleanly exit any blocking wait without riding out the full timeout.

---

## State File Reference

All bot state lives in `.ralph/telegram-state.json`:

| Field | Type | Purpose |
|-------|------|---------|
| `chat_id` | `i64` or `null` | Auto-detected Telegram chat ID |
| `last_seen` | ISO 8601 timestamp | When the last incoming message was processed |
| `last_update_id` | `i32` or `null` | Last Telegram update ID; polling resumes from here after restart |
| `pending_questions` | Map of loop ID &rarr; question | Tracks outstanding questions for reply routing |

Each entry in `pending_questions` contains:

| Field | Type | Purpose |
|-------|------|---------|
| `asked_at` | ISO 8601 timestamp | When the question was sent |
| `message_id` | `i32` | Telegram message ID used to match reply-to routing |

The state file is written atomically (write to temp file, then rename) to prevent corruption from crashes.

---

## Retry and Error Behavior

All outgoing messages — questions, check-ins, greetings, farewells, documents, photos — use the same retry strategy:

| Attempt | Delay Before Retry |
|---------|-------------------|
| 1st | (immediate) |
| 2nd | 1 second |
| 3rd | 2 seconds |

If all 3 attempts fail:

- For **questions**: the failure is logged to diagnostics and the loop continues as if the human timed out. No crash, no hang.
- For **check-ins**: silently skipped. Check-ins are best-effort.
- For **greetings/farewells**: logged as a warning. Non-critical.

The exponential backoff formula is `base_delay * 2^(attempt - 1)` with a base of 1 second. There is no jitter.

---

## Configuration Reference

```yaml
RObot:
  enabled: true                    # Required. Activates the Telegram integration.
  timeout_seconds: 300             # Required. How long to block on human.interact.
  checkin_interval_seconds: 120    # Optional. Periodic status updates to your chat.
  telegram:
    bot_token: "your-token"        # Optional if RALPH_TELEGRAM_BOT_TOKEN env var is set.
```

### Token resolution order

1. `RALPH_TELEGRAM_BOT_TOKEN` environment variable (highest priority)
2. `RObot.telegram.bot_token` in the config file
3. OS keychain entry (`ralph` / `telegram-bot-token`) — best-effort fallback

### Tuning for long-running loops

For loops that may run for hours:

```yaml
RObot:
  enabled: true
  timeout_seconds: 43200            # 12 hours — plenty of time to respond
  checkin_interval_seconds: 900     # Check in every 15 minutes
```

For fast, interactive sessions where you're actively watching:

```yaml
RObot:
  enabled: true
  timeout_seconds: 60               # Short timeout — keep things moving
  checkin_interval_seconds: 30      # Frequent check-ins
```

---

## Operational Patterns

### Pattern: Steering a multi-loop session

When running parallel loops, keep your phone handy and use reply-to routing for precision:

1. Start the primary loop and one or two worktree loops
2. Watch for questions on Telegram — each will show the loop ID and hat
3. Reply directly to question messages (reply-to routing is automatic)
4. For proactive guidance to a specific loop, use `@loop-id your message`
5. For guidance to the primary loop, just send a plain message

### Pattern: Emergency stop from your phone

Send `/stop` to gracefully terminate the primary loop. The loop will finish its current iteration and then exit cleanly. For worktree loops, you'll need to target them with a stop signal from the terminal.

### Pattern: Checking progress remotely

Configure `checkin_interval_seconds` and let the bot update you. Combined with `/status` and `/tasks`, you can monitor a running loop entirely from Telegram without SSH access.

### Pattern: Working with no prior chat ID

If you're starting fresh (no `.ralph/telegram-state.json` yet), the bot can't send outgoing messages until you message it first. The startup greeting will be skipped, and any early `human.interact` questions will be logged but not delivered. Just send the bot a message at any point to establish the connection — the chat ID is immediately persisted and subsequent messages will work.

---

## Troubleshooting

### Bot doesn't respond to messages

- Verify your bot token: `curl https://api.telegram.org/bot<TOKEN>/getMe`
- Ensure `RObot.enabled: true` is set in your config
- Check that the loop is actually running (the polling task only runs while the loop is active)

### Questions are sent but replies aren't received

- Make sure you're **replying** to the question message (use Telegram's reply feature, not just sending a new message)
- If not replying, the message is classified as `human.guidance`, not `human.response`
- Check `.ralph/telegram-state.json` to see if a `pending_question` exists for your loop

### Messages go to the wrong loop

- Use reply-to (most reliable) or `@loop-id` prefix for explicit routing
- Unrouted messages always default to the primary loop
- Check `ralph loops` to see the exact loop IDs

### Timeout fires before you can respond

- Increase `timeout_seconds` in your config
- The timeout is measured from when the question is sent, not from when you see it
- Network delays on the Telegram side don't extend the timeout

### "No chat ID configured" warnings in logs

- This is normal on first run — the bot learns your chat ID from your first message
- Send any message to the bot to establish the connection
- The warning will not appear on subsequent runs

---

## See Also

- [Telegram Setup Guide](../guide/telegram.md) — Initial setup and configuration
- [Parallel Loops](parallel-loops.md) — Running multiple loops with worktrees
- [Event System Design](event-system.md) — How events flow through the system
- [Diagnostics](diagnostics.md) — Debugging with full visibility
