# Ralph Orchestrator Robot Internals

**Researched:** 2026-03-08
**Source:** Direct codebase analysis of `/opt/workspace/ralph-orchestrator` (v2.7.0)
**Confidence:** HIGH (all findings from source code)

---

## ralph.yml Robot Config

The robot section is called `RObot` (case-sensitive serde rename) in ralph.yml. Example from the actual codebase's own ralph.yml:

```yaml
RObot:
  enabled: false
  timeout_seconds: 120
```

Full config example with all fields:

```yaml
RObot:
  enabled: true
  timeout_seconds: 300           # Required when enabled (no default)
  checkin_interval_seconds: 120  # Optional: periodic status updates
  telegram:
    bot_token: "123456:ABC-DEF"  # Optional if env var or keychain set
    api_url: "http://localhost:8081"  # Optional: custom API URL for testing
```

### RobotConfig struct (`ralph-core/src/config.rs`)

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RobotConfig {
    #[serde(default)]
    pub enabled: bool,

    pub timeout_seconds: Option<u64>,        // Required when enabled
    pub checkin_interval_seconds: Option<u64>, // Optional periodic check-ins

    #[serde(default)]
    pub telegram: Option<TelegramBotConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramBotConfig {
    pub bot_token: Option<String>,
    pub api_url: Option<String>,
}
```

### Key observations:

- The serde rename `#[serde(rename = "RObot")]` means the YAML key is literally `RObot`, not `robot`.
- When `enabled: false` (the default), validation is skipped entirely.
- When `enabled: true`, `timeout_seconds` is **required** (no default value).
- The `telegram` sub-section is the only backend currently supported. There is no `backend` enum or selector field -- the config hardcodes Telegram.

---

## RobotService Trait (full signatures)

**Location:** `ralph-proto/src/robot.rs`

```rust
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[derive(Debug, Default)]
pub struct CheckinContext {
    pub current_hat: Option<String>,
    pub open_tasks: usize,
    pub closed_tasks: usize,
    pub cumulative_cost: f64,
}

pub trait RobotService: Send + Sync {
    /// Send a question to the human and store it as pending.
    /// Returns platform-specific message ID, or 0 if no recipient configured.
    fn send_question(&self, payload: &str) -> anyhow::Result<i32>;

    /// Poll the events file for a `human.response` event.
    /// Blocks until response or timeout. Returns Ok(Some(response)) or Ok(None).
    fn wait_for_response(&self, events_path: &Path) -> anyhow::Result<Option<String>>;

    /// Send a periodic check-in message.
    /// Returns Ok(0) if skipped, or message ID on success.
    fn send_checkin(
        &self,
        iteration: u32,
        elapsed: Duration,
        context: Option<&CheckinContext>,
    ) -> anyhow::Result<i32>;

    /// Get the configured response timeout in seconds.
    fn timeout_secs(&self) -> u64;

    /// Get a clone of the shutdown flag for cooperative interruption.
    fn shutdown_flag(&self) -> Arc<AtomicBool>;

    /// Stop the service gracefully. Called during loop termination.
    fn stop(self: Box<Self>);
}
```

### Trait design notes:

1. **Synchronous interface.** All methods are `&self` with no `async`. The Telegram implementation uses `tokio::task::block_in_place()` + `Handle::block_on()` to bridge to async internally.
2. **File-based response polling.** `wait_for_response()` takes an `events_path` (a `.jsonl` file) and polls it for new `human.response` events. This is NOT a platform callback -- it's a file watcher.
3. **Object-safe trait.** Used as `Box<dyn RobotService>` throughout.
4. **`stop(self: Box<Self>)`** -- takes ownership, consuming the boxed service.
5. The trait lives in `ralph-proto`, the protocol crate with zero platform dependencies.

---

## TelegramService Implementation

**Location:** `ralph-telegram/src/service.rs`

### Construction

```rust
pub struct TelegramService {
    workspace_root: PathBuf,
    bot_token: String,
    api_url: Option<String>,
    timeout_secs: u64,
    loop_id: String,
    state_manager: StateManager,
    handler: MessageHandler,
    bot: TelegramBot,
    shutdown: Arc<AtomicBool>,
}
```

Created via `TelegramService::new(workspace_root, bot_token, api_url, timeout_secs, loop_id)`.

### RobotService impl

The `impl ralph_proto::RobotService for TelegramService` block at line 761 simply delegates:

```rust
impl ralph_proto::RobotService for TelegramService {
    fn send_question(&self, payload: &str) -> anyhow::Result<i32> {
        Ok(TelegramService::send_question(self, payload)?)
    }
    fn wait_for_response(&self, events_path: &Path) -> anyhow::Result<Option<String>> {
        Ok(TelegramService::wait_for_response(self, events_path)?)
    }
    fn send_checkin(&self, iteration: u32, elapsed: Duration, context: Option<&ralph_proto::CheckinContext>) -> anyhow::Result<i32> {
        // Converts ralph_proto::CheckinContext to local CheckinContext
        let local_context = context.map(|ctx| CheckinContext { ... });
        Ok(TelegramService::send_checkin(self, iteration, elapsed, local_context.as_ref())?)
    }
    fn timeout_secs(&self) -> u64 { self.timeout_secs }
    fn shutdown_flag(&self) -> Arc<AtomicBool> { self.shutdown.clone() }
    fn stop(self: Box<Self>) { TelegramService::stop(*self) }
}
```

### How send_question works:

1. Loads state from `.ralph/telegram-state.json`
2. If `chat_id` is known, sends message via Telegram with retry (3 attempts, exponential backoff)
3. Records the pending question in state (maps `loop_id -> message_id`)
4. Returns message ID (or 0 if no chat_id configured)

### How wait_for_response works:

1. Records the initial file position in the events JSONL file
2. Polls every 250ms for new lines
3. Looks for lines with `"topic": "human.response"` in JSON
4. Checks `shutdown` flag each iteration (cooperative cancellation)
5. On timeout: removes pending question, returns `None`
6. On response: removes pending question, returns `Some(response_text)`

### How the background poller works (start()):

1. Spawns an async task on the host tokio runtime
2. Long-polls Telegram's `getUpdates` API with 10s timeout
3. Routes messages through `MessageHandler` which writes events to the loop's events JSONL
4. Commands (slash-prefixed) are intercepted before reaching the handler
5. Non-command messages become either `human.response` (if a pending question exists for the target loop) or `human.guidance` (proactive)
6. Reacts with emoji (thumbs-up for responses, eyes for guidance)

---

## Bot Commands

### Full command list

| Command | Description | Implementation |
|---------|-------------|---------------|
| `/help` | List available commands | Static text |
| `/status` | Current loop status (PID, elapsed, iterations) | Reads `.ralph/loop.lock` |
| `/tasks` | Open tasks | Reads `.ralph/agent/tasks.jsonl` |
| `/memories` | Recent memories (last 5) | Reads `.ralph/agent/memories.md` |
| `/tail` | Last 20 events | Reads current events JSONL |
| `/model` | Show current backend/model | Reads lock file + config |
| `/models` | Show all configured models | Scans ralph*.yml files |
| `/restart` | Restart the loop | Writes `.ralph/restart-requested` |
| `/stop` | Stop the loop | Writes `.ralph/stop-requested` |

### Command registration

Commands are registered with Telegram's `setMyCommands` API during `poll_updates()` startup via `register_commands()`. This makes them appear in Telegram's command menu.

### Command routing

The command system is shared between the `TelegramService` (in-loop) and the `TelegramDaemon` (idle mode). Both call `crate::commands::handle_command(text, workspace_root)` from the `commands.rs` module.

Key function:
```rust
pub fn handle_command(text: &str, workspace_root: &Path) -> Option<String> {
    let (command, args) = parse_command(text);
    match command {
        "/help" => Some(cmd_help()),
        "/status" => Some(cmd_status(workspace_root)),
        // ... etc
        _ => None,
    }
}
```

Commands are all **synchronous** and **read-only** (except `/restart` and `/stop` which write signal files). They operate on local filesystem state, not Telegram-specific state.

---

## Wiring (create_robot_service / loop runner)

### Where the service is created

**Location:** `ralph-cli/src/loop_runner.rs`, function `create_robot_service()`

```rust
fn create_robot_service(
    config: &RalphConfig,
    context: &LoopContext,
) -> Option<Box<dyn ralph_proto::RobotService>> {
    let workspace_root = context.workspace().to_path_buf();
    let bot_token = config.robot.resolve_bot_token();
    let api_url = config.robot.resolve_api_url();
    let timeout_secs = config.robot.timeout_seconds.unwrap_or(300);
    let loop_id = context.loop_id()
        .map(String::from)
        .unwrap_or_else(|| "main".to_string());

    match TelegramService::new(workspace_root, bot_token, api_url, timeout_secs, loop_id) {
        Ok(service) => {
            if let Err(e) = service.start() {
                warn!(error = %e, "Failed to start robot service");
                return None;
            }
            Some(Box::new(service))
        }
        Err(e) => {
            warn!(error = %e, "Failed to create robot service");
            None
        }
    }
}
```

### How it's wired into the loop

In `run_loop_impl()`:

```rust
let mut event_loop = EventLoop::with_context(config.clone(), ctx.clone());

// Inject robot service (Telegram) for human-in-the-loop communication
if config.robot.enabled && ctx.is_primary()
    && let Some(service) = create_robot_service(&config, &ctx)
{
    event_loop.set_robot_service(service);
}

// Capture shutdown flag for signal handlers
let robot_shutdown = event_loop.robot_shutdown_flag();
```

The signal handlers (SIGINT, SIGTERM, SIGHUP) all set the `robot_shutdown` flag to interrupt any blocking `wait_for_response()`.

### EventLoop's use of robot_service

The EventLoop holds `robot_service: Option<Box<dyn RobotService>>` and uses it at three points:

1. **human.interact event processing** (`process_events()`): When agents emit `human.interact`, the event loop calls `robot_service.send_question()` then `robot_service.wait_for_response()`, blocking the loop.
2. **Periodic check-ins** (`post_iteration()`): Every `checkin_interval_seconds`, calls `robot_service.send_checkin()`.
3. **Loop termination** (`publish_terminate_event()`): Calls `stop_robot_service()` which takes ownership and calls `.stop()`.

---

## Config Resolution

### ralph.yml loading

The config is loaded by `ralph-core`'s config system. The `RalphConfig` struct deserializes from YAML. The `RObot` section uses `#[serde(default, rename = "RObot")]` so:
- Missing `RObot:` section = defaults (disabled)
- Present but `enabled: false` = no validation
- Present with `enabled: true` = full validation required

### Bot token resolution order

`RobotConfig::resolve_bot_token()` checks three sources:

1. **Environment variable:** `RALPH_TELEGRAM_BOT_TOKEN` (highest priority)
2. **Config file:** `RObot.telegram.bot_token` in ralph.yml
3. **OS keychain:** service `"ralph"`, user `"telegram-bot-token"` (via `keyring` crate)

### API URL resolution order

`RobotConfig::resolve_api_url()` checks two sources:

1. **Environment variable:** `RALPH_TELEGRAM_API_URL`
2. **Config file:** `RObot.telegram.api_url`

### Key env vars

| Variable | Purpose |
|----------|---------|
| `RALPH_TELEGRAM_BOT_TOKEN` | Telegram bot token |
| `RALPH_TELEGRAM_API_URL` | Custom Telegram API endpoint (for testing) |

---

## Message Format

### Outgoing messages (Ralph -> Human)

All Telegram messages use **HTML parse mode** (`ParseMode::Html`).

**Question format:**
```
"question" -> TelegramBot::format_question(hat, iteration, loop_id, question)
```
Produces:
```html
[emoji] <b>{hat}</b> (iteration {N}, loop <code>{loop_id}</code>)

{question body converted from markdown to Telegram HTML}
```

**Markdown-to-HTML conversion** (`markdown_to_telegram_html()`):
- `**bold**` -> `<b>bold</b>`
- `` `inline` `` -> `<code>inline</code>`
- Fenced code blocks -> `<pre>...</pre>`
- `# Header` -> `<b>Header</b>`
- `- item` / `* item` -> bullet character
- HTML entities in content are escaped (`<`, `>`, `&`)

**Check-in format:**
```html
Still working -- iteration <b>N</b>, <code>Xm Ys</code> elapsed.
Hat: <code>builder</code>
Tasks: <b>3</b> open, 5 closed
Cost: <code>$0.1234</code>
```

**Greeting/farewell:**
```html
[robot emoji] Ralph bot online -- monitoring loop <code>{loop_id}</code>
[wave emoji] Ralph bot shutting down -- loop <code>{loop_id}</code> complete
```

### Incoming messages (Human -> Ralph)

Written to the events JSONL file as:
```json
{"topic": "human.response", "payload": "the user's text", "ts": "2026-..."}
{"topic": "human.guidance", "payload": "proactive advice", "ts": "2026-..."}
```

The `topic` is determined by whether a pending question exists for the target loop:
- **Pending question exists** -> `human.response`
- **No pending question** -> `human.guidance`

### Target loop routing

Messages are routed to loops using:
1. Reply-to a specific question message -> that loop
2. `@loop-id` prefix -> extracted loop ID
3. Default -> `"main"`

---

## Integration Points for BotMinter

### What BotMinter needs to provide

To support a new communication backend (e.g., Rocket.Chat), BotMinter needs:

1. **A new crate** implementing `ralph_proto::RobotService` (6 methods)
2. **Config extension** to `RobotConfig` or a parallel config section
3. **A factory function** to replace `create_robot_service()` in the loop runner

### The trait is already backend-agnostic

The `RobotService` trait in `ralph-proto` was explicitly designed for pluggability. The doc comment says "communication backends (Telegram, Slack, etc.) implement" it. The trait contains zero Telegram-specific types.

### The config is NOT backend-agnostic

The `RobotConfig` struct hardcodes `telegram: Option<TelegramBotConfig>`. There is no backend selector. Adding a new backend requires either:
- Adding a new field (e.g., `rocketchat: Option<RocketChatConfig>`) -- ugly, doesn't scale
- Replacing with an enum (e.g., `backend: BackendConfig`) -- breaking change
- Moving config resolution outside of Ralph -- BotMinter handles config and passes a `Box<dyn RobotService>` to Ralph

### The factory is NOT backend-agnostic

`create_robot_service()` in `ralph-cli` directly constructs `TelegramService`. It's a private function in `loop_runner.rs`. To support multiple backends, this needs to be made injectable or configurable.

---

## What Needs to Change for Pluggable Backends

### Changes needed IN Ralph Orchestrator

1. **Config: Add a backend selector or make config extensible**
   - Option A: Add `backend: String` field to `RobotConfig` with backend-specific config as a `serde_yaml::Value`
   - Option B: Accept `Box<dyn RobotService>` from outside (BotMinter injects it)
   - Option B is cleaner -- Ralph's core already accepts `set_robot_service()`

2. **Factory: Make `create_robot_service()` pluggable**
   - Currently private in `ralph-cli/src/loop_runner.rs`
   - Could accept a factory function/closure: `Fn(&RalphConfig, &LoopContext) -> Option<Box<dyn RobotService>>`
   - Or expose a public trait/hook that BotMinter can implement

3. **Commands: Extract command handling**
   - The `/status`, `/tasks`, etc. commands read from filesystem -- they're backend-agnostic
   - The `commands::handle_command()` function is already reusable
   - A new backend just needs to call this same function

### Changes needed IN BotMinter

1. **Implement `RobotService`** for Rocket.Chat (or whatever backend)
2. **Provide a factory** that creates the right service based on team config
3. **Handle config** for the new backend in BotMinter's own config layer (team repo config, not ralph.yml)
4. **Adapt message format** -- Ralph sends markdown, convert to target platform format

### Minimal viable approach (no Ralph changes needed)

Since `EventLoop::set_robot_service()` is public and takes any `Box<dyn RobotService>`:

1. BotMinter creates its own `RobotService` implementation
2. BotMinter constructs it with its own config
3. BotMinter injects it into the EventLoop before running

The challenge: BotMinter doesn't control `run_loop_impl()` -- that's in `ralph-cli`. BotMinter currently invokes Ralph as a subprocess via `ralph run` or `claude`. So direct injection isn't possible without either:
- Modifying Ralph to accept a plugin/factory
- BotMinter running Ralph's event loop directly (using `ralph-core` as a library)
- Using the bridge pattern at the subprocess boundary (e.g., BotMinter runs a sidecar that implements the file-based protocol)

### The file-based protocol option (no Ralph changes at all)

Since `wait_for_response()` polls a JSONL file and the handler writes events to the same file format, a new backend could:

1. Disable Ralph's built-in robot (`RObot.enabled: false`)
2. Run a sidecar process that:
   - Watches for `human.interact` events in the JSONL file
   - Sends questions via Rocket.Chat / Slack / etc.
   - Writes `human.response` or `human.guidance` events back to the JSONL file
3. This requires no changes to Ralph at all

**Caveat:** This loses the greeting/farewell/check-in features (those are in the RobotService, not the event bus). But it's the simplest integration path.

### The DaemonAdapter parallel

Ralph also has a `DaemonAdapter` trait (in `ralph-proto/src/daemon.rs`) for persistent bot mode. `TelegramDaemon` implements it. BotMinter would need a parallel daemon adapter for its backend if it wants daemon-mode support.

---

## Summary of Key Files

| File | What It Contains |
|------|------------------|
| `ralph-proto/src/robot.rs` | `RobotService` trait + `CheckinContext` |
| `ralph-core/src/config.rs` | `RobotConfig`, `TelegramBotConfig`, validation |
| `ralph-core/src/event_loop/mod.rs` | EventLoop integration (set_robot_service, process_events, post_iteration) |
| `ralph-telegram/src/service.rs` | `TelegramService` (RobotService impl) |
| `ralph-telegram/src/bot.rs` | `BotApi` trait, `TelegramBot`, message formatting |
| `ralph-telegram/src/commands.rs` | Slash commands (filesystem-based, reusable) |
| `ralph-telegram/src/handler.rs` | `MessageHandler` (incoming message -> JSONL event) |
| `ralph-telegram/src/state.rs` | `TelegramState`, `StateManager`, pending question tracking |
| `ralph-telegram/src/daemon.rs` | `TelegramDaemon` (DaemonAdapter impl) |
| `ralph-cli/src/loop_runner.rs` | `create_robot_service()` factory, wiring into event loop |
