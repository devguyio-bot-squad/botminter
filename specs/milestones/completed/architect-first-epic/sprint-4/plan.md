# Sprint 4 Plan: Automated HIL Telegram Tests

> Fixture-driven Rust integration tests using [tg-mock](https://github.com/watzon/tg-mock).
>
> Design reference: [design.md](design.md)
> Mock server research: [research/mock-telegram-server.md](research/mock-telegram-server.md)

## Checklist

- [ ] Step 1: tg-mock integration + api_url validation
- [ ] Step 2: Rust test crate scaffold + tg-mock client
- [ ] Step 3: Fixture format + response loop + lifecycle test
- [ ] Step 4: Rejection loop tests (design + plan)
- [ ] Step 5: Edge case tests (push-conflict, crash-recovery)

---

## Step 1: tg-mock Integration + api_url Validation

**Objective:** Validate that Ralph's `RALPH_TELEGRAM_API_URL` works with tg-mock.
This is the first real test of the api_url feature.

**Implementation:**

Add Justfile recipes:

```just
test-server-up:
    docker run -d --name botminter-tg-mock -p 8081:8081 \
        ghcr.io/watzon/tg-mock --faker-seed 42
    @# health-check loop
    ...

test-server-down:
    docker rm -f botminter-tg-mock 2>/dev/null || true
```

Validate the round-trip manually:
1. `just test-server-up`
2. Verify control API: `curl http://localhost:8081/__control/requests`
3. Run a minimal Ralph loop with `RALPH_TELEGRAM_API_URL=http://localhost:8081`
   and a simple prompt that emits `human.interact`
4. Verify bot sent a message: `curl "http://localhost:8081/__control/requests?method=sendMessage"`
5. Inject reply: `curl -X POST http://localhost:8081/__control/tokens/test-token/updates -d '...'`
6. Verify Ralph received it (check events.jsonl for `human.response`)
7. If api_url has bugs, fix in Ralph before proceeding

Document findings in `research/api-url-validation.md`.

**Test:** Ralph sends and receives through tg-mock.

---

## Step 2: Rust Test Crate Scaffold + tg-mock Client

**Objective:** Create the test crate with a typed tg-mock client and setup helpers.

**Implementation:**

Create `tests/hil/Cargo.toml`:
```toml
[package]
name = "botminter-hil-tests"
version = "0.1.0"
edition = "2021"

[dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
tokio = { version = "1", features = ["full"] }
regex = "1"
tempfile = "3"
```

Create `tests/hil/src/mock_client.rs` — typed wrapper around tg-mock's `/__control/` API:

```rust
pub struct TgMockClient {
    base_url: String,
    client: reqwest::Client,
}

impl TgMockClient {
    pub fn new(base_url: &str) -> Self { ... }

    /// Read bot's sent messages, filtered by token and method
    pub async fn get_requests(&self, token: &str, method: &str) -> Result<Vec<ApiRequest>>

    /// Inject a user message for a specific bot token
    pub async fn inject_update(&self, token: &str, text: &str, chat_id: i64) -> Result<()>

    /// Clear all recorded requests (between tests)
    pub async fn clear_requests(&self) -> Result<()>

    /// Clear all scenarios
    pub async fn clear_scenarios(&self) -> Result<()>
}
```

Create `tests/hil/src/setup.rs` — environment helpers:

```rust
/// Run `just init` + deploy fixtures + add members
pub fn setup_team_repo(path: &Path) -> Result<()>

/// Create workspace for a member
pub fn create_workspace(team_repo: &Path, member: &str, project_repo: &Path) -> Result<PathBuf>

/// Create a minimal synthetic project repo
pub fn create_synth_project(path: &Path) -> Result<()>

/// Launch Ralph agent in background, return Child process
pub fn start_agent(workspace: &Path, token: &str) -> Result<Child>

/// Kill agent process
pub fn stop_agent(child: &mut Child) -> Result<()>

/// Poll issue file until status matches expected, with timeout
pub async fn wait_for_status(issue_path: &Path, expected: &str, timeout: Duration) -> Result<()>

/// Read YAML frontmatter status from an issue file
pub fn read_issue_status(path: &Path) -> Result<String>

/// Assert no .lock files in directory
pub fn assert_no_locks(github_sim_dir: &Path) -> Result<()>
```

Add Justfile recipe:
```just
test-hil: test-server-up
    cd tests/hil && cargo test -- --test-threads=1
    just test-server-down
```

**Test:** `just test-hil` compiles, tg-mock client connects, setup helpers run
`just init` successfully. No scenario tests yet.

---

## Step 3: Fixture Format + Response Loop + Lifecycle Test

**Objective:** Define the fixture YAML format, build the response loop, and
implement the happy-path lifecycle test.

**Implementation:**

Define fixture format in `tests/hil/src/lib.rs`:

```rust
#[derive(Deserialize)]
pub struct Fixture {
    pub name: String,
    pub description: String,
    pub initial_status: String,
    pub responses: Vec<ResponseRule>,
    pub expectations: Expectations,
}

#[derive(Deserialize)]
pub struct ResponseRule {
    pub r#match: String,          // regex pattern on bot message text
    pub reply: String,            // text to inject as user reply
    pub context: Option<String>,  // narrow match (e.g., "review")
    pub times: Option<u32>,       // fire N times, then fall through (None = unlimited)
}

#[derive(Deserialize)]
pub struct Expectations {
    pub epic_status: String,
    pub epic_state: Option<String>,
    pub design_doc_exists: Option<bool>,
    pub design_doc_contains: Option<Vec<String>>,
    pub story_issues_created: Option<bool>,
    pub no_stale_locks: Option<bool>,
    pub rejection_comments: Option<u32>,
}
```

Build the response loop in `run_scenario()`:
1. Load fixture, set up team repo + workspaces, seed epic at `initial_status`
2. Clear tg-mock state (`clear_requests`)
3. Start both agents
4. Loop (with timeout):
   a. Poll `get_requests(token, "sendMessage")` for new messages (track last seen ID)
   b. For each new message, find first matching `ResponseRule` (regex on text,
      optional context filter). Decrement `times` counter if present.
   c. Inject reply via `inject_update(token, rule.reply, chat_id)`
   d. Check if epic has reached expected status → break
5. Stop agents
6. Assert expectations (status, design doc content, story issues, locks, etc.)

Create `tests/hil/fixtures/approve-all.yml`:
```yaml
name: full-lifecycle
description: Epic traverses triage to done with all gates approved
initial_status: "status/po:triage"

responses:
  - match: "triage"
    reply: "Approved. Accept to backlog and activate."
  - match: "design"
    context: "review"
    reply: "Approved. Design looks good, proceed to planning."
  - match: "plan"
    context: "review"
    reply: "Approved. Story breakdown is acceptable."
  - match: "accept"
    reply: "Approved. Epic is complete."
  - match: ".*"
    reply: "Confirmed, proceed."

expectations:
  epic_status: "status/done"
  epic_state: "closed"
  design_doc_exists: true
  design_doc_contains:
    - "reconciler"
    - "composition"
  story_issues_created: true
  no_stale_locks: true
```

Create `tests/hil/src/tests/lifecycle.rs`:
```rust
#[tokio::test]
async fn test_full_lifecycle() {
    let fixture = load_fixture("fixtures/approve-all.yml");
    let result = run_scenario(&fixture, HA_TOKEN, ARCH_TOKEN).await;
    assert!(result.is_ok(), "Lifecycle test failed: {:?}", result.err());
}
```

**Test:** Full lifecycle: epic triage→done, design doc with knowledge markers,
story issues created, no stale locks.

---

## Step 4: Rejection Loop Tests

**Objective:** Design and plan rejection with feedback, architect revision.

**Implementation:**

Create `tests/hil/fixtures/reject-design-once.yml`:
```yaml
name: design-rejection
description: Reject first design, approve revision
initial_status: "status/po:triage"

responses:
  - match: "design"
    context: "review"
    reply: "Rejected. Missing error handling section."
    times: 1
  - match: "design"
    context: "review"
    reply: "Approved. Error handling addressed."
  - match: ".*"
    reply: "Approved, proceed."

expectations:
  epic_status: "status/done"
  rejection_comments: 1
```

Create `tests/hil/fixtures/reject-plan-once.yml`:
```yaml
name: plan-rejection
description: Reject first plan, approve revision
initial_status: "status/po:triage"

responses:
  - match: "plan|breakdown"
    context: "review"
    reply: "Rejected. Stories too large, split further."
    times: 1
  - match: "plan|breakdown"
    context: "review"
    reply: "Approved. Breakdown is acceptable."
  - match: ".*"
    reply: "Approved, proceed."

expectations:
  epic_status: "status/done"
  rejection_comments: 1
```

Create test files:
```rust
// tests/design_rejection.rs
#[tokio::test]
async fn test_design_rejection_loop() {
    let fixture = load_fixture("fixtures/reject-design-once.yml");
    let result = run_scenario(&fixture, HA_TOKEN, ARCH_TOKEN).await;
    assert!(result.is_ok());
}

// tests/plan_rejection.rs
#[tokio::test]
async fn test_plan_rejection_loop() {
    let fixture = load_fixture("fixtures/reject-plan-once.yml");
    let result = run_scenario(&fixture, HA_TOKEN, ARCH_TOKEN).await;
    assert!(result.is_ok());
}
```

**Test:** Rejection→revision→approval cycle works for both design and plan.

---

## Step 5: Edge Case Tests

**Objective:** Push conflicts and crash recovery.

**Implementation:**

Create `tests/hil/fixtures/two-epics.yml`:
```yaml
name: push-conflict
description: Two epics processed simultaneously
initial_status: "status/po:triage"
extra_epics:
  - initial_status: "status/arch:design"

responses:
  - match: ".*"
    reply: "Approved, proceed."

expectations:
  epic_status: "status/done"
  no_stale_locks: true
```

The `extra_epics` field tells the test harness to seed additional epics beyond
the primary one. The `run_scenario` function handles multi-epic setup.

Create `tests/hil/fixtures/stale-lock.yml`:
```yaml
name: crash-recovery
description: Stale lock from crashed agent, verify cleanup
initial_status: "status/po:triage"
stale_lock:
  issue: 1
  content: "crashed-agent:fake-loop-id 2026-01-01T00:00:00Z"

responses:
  - match: ".*"
    reply: "Approved, proceed."

expectations:
  epic_status: "status/done"
  no_stale_locks: true
```

The `stale_lock` field tells setup to pre-create a lock file before starting agents.

Create test files:
```rust
// tests/push_conflict.rs
#[tokio::test]
async fn test_push_conflict() {
    let fixture = load_fixture("fixtures/two-epics.yml");
    let result = run_scenario(&fixture, HA_TOKEN, ARCH_TOKEN).await;
    assert!(result.is_ok());
}

// tests/crash_recovery.rs
#[tokio::test]
async fn test_crash_recovery() {
    let fixture = load_fixture("fixtures/stale-lock.yml");
    let result = run_scenario(&fixture, HA_TOKEN, ARCH_TOKEN).await;
    assert!(result.is_ok());
}
```

**Test:** Edge cases handled — push conflicts resolved, stale locks cleaned.
