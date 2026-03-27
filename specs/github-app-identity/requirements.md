# Requirements: Per-Member GitHub App Identity

This document records the Q&A from requirements clarification.

---

## Q1: CredentialStore reuse vs. new abstraction

The existing `CredentialStore` trait (`bridge/credential.rs`) is tightly coupled to bridge concerns — it stores a single token per member, records identities in `bridge-state.json`, and uses the keyring service name `botminter.{team}.{bridge}`. GitHub App credentials are structurally different: three values per member (App ID, private key PEM, installation ID), no bridge-state.json involvement, and a different keyring key scheme (`botminter/{team}/{member}/github-app-*` per the ADR).

**Options:**
- (A) Extend the existing `CredentialStore` trait to support multi-field credentials (breaking change to the trait)
- (B) Create a new `GitHubAppCredentialStore` trait/struct purpose-built for App credentials, reusing the underlying keyring helpers (`dss_store`, `dss_retrieve`, `with_keyring_dbus`, etc.)
- (C) Use the existing `CredentialStore` trait as-is, storing each App credential as a separate "member" entry (e.g., `{member}/app-id`, `{member}/private-key`, `{member}/installation-id`)

**Additional context (ADR-0008 tension):**

ADR-0008 (Local Formation) says "GitHub token management — Team credential, not a deployment concern." That was true for the PAT model (one token, team-level). But per-member GitHub App credentials change this:

- **Storage** is platform-specific (Linux keyring vs macOS Keychain) — that IS a formation concern
- The `Formation` trait's `credential_store()` method is bridge-specific (`bridge_name`, `state_path` params)
- `LaunchParams.gh_token: &str` assumes a caller-provided token; with Apps, the member generates its own

This raises a deeper question: should the `Formation` trait gain a `github_credential_store()` method (making GitHub App storage formation-aware), or should GitHub App credential storage bypass the Formation abstraction entirely (using the keyring helpers directly)?

**Revised question:** Given ADR-0008's formation architecture:
- (A) Add a `github_credential_store()` method to the `Formation` trait (consistent with how bridge credentials work)
- (B) Create a standalone `GitHubAppCredentialStore` that uses keyring helpers directly, bypassing the Formation trait (simpler, but GitHub creds become the exception to "formations own credential storage")
- (C) Generalize the existing `credential_store()` to support multiple credential domains (bridge, github-app, etc.) via a parameter

**Answer:** Merge ADR-0008 and ADR-0011 into a single implementation plan. The Formation trait doesn't exist yet — implementing it alongside GitHub App identity avoids a double refactor. The Formation trait will own all credential storage (both bridge and GitHub App), and GitHub App identity becomes the forcing function for the trait's credential abstraction. This means the `credential_store()` method on the Formation trait should be generalized from the start to support multiple credential domains, not just bridge.

---

## Q2: ADR-0008 scope — how much of the Formation trait do we implement?

ADR-0008 describes a full Formation trait with: environment setup, environment status checks, prerequisites, credential storage, member lifecycle (launch/stop/health), and topology writing. It also describes a platform split (Linux vs macOS) with platform-specific modules.

Implementing ALL of ADR-0008 is a large refactor of the entire `formation/` module — converting `start_local_members()`, `stop_local_members()`, `launch_ralph()`, `launch_brain()`, and all the bridge auto-start logic from free functions to trait methods.

**Options:**
- (A) **Full ADR-0008**: Implement the complete Formation trait, platform split, move all existing formation functions behind the trait. GitHub App identity is one of the credential domains.
- (B) **Partial — credential + launch focus**: Implement the Formation trait but only the methods needed for GitHub App identity: `credential_store()` (generalized), `launch_member()`, and `check_prerequisites()`. Leave stop/status/topology as free functions for now.
- (C) **Trait shell + GitHub App vertical**: Create the Formation trait with all method signatures, implement only the `LinuxLocalFormation` with full GitHub App support, leave other methods as pass-throughs to existing free functions (incremental migration).

**Question:** How much of ADR-0008 do you want to land in this milestone? Full formation refactor, or just enough to support GitHub App identity cleanly?

**Answer:** All of it. Full ADR-0008 implementation merged with ADR-0011. Additionally, the existing `bm runtime` (Lima VM) concept needs to be reconciled — currently `bm runtime create/delete` and `bm attach` manage Lima VMs as a separate concept alongside the formation module. The Formation trait should absorb or integrate with the runtime/VM concept so there's one unified abstraction for "where and how members run."

---

## Q3: Runtime/VM relationship to Formation trait

Today, `bm runtime create` provisions a Lima VM (`formation/lima.rs`), and `bm attach` shells into it. The VM is recorded in `config.yml` as a `VmEntry` and optionally linked to a team via `TeamEntry.vm: Option<String>`. But the Lima code is separate from the formation resolution logic — it's a parallel infrastructure concept, not integrated into the Formation trait.

ADR-0008 describes the formation as an "environment abstraction" that answers "how is the environment prepared?" — which is exactly what `bm runtime create` does for VMs.

**Options:**
- (A) **VM as a formation type**: `bm runtime create` becomes `formation.setup()` for a "lima" formation type. The Lima VM is the environment, the Formation trait manages its lifecycle. `bm attach` becomes `bm formation shell` or similar.
- (B) **VM as formation infrastructure**: The Lima VM remains a separate infrastructure layer. A "local-isolated" formation uses a VM but the VM lifecycle (create/delete) stays in `bm runtime`. The Formation trait's `setup()` configures the VM internals (install ralph, configure agent) but doesn't create/destroy the VM itself.
- (C) **VM subsumed entirely**: `bm runtime` commands are removed. VM creation/deletion moves into formation config and `bm teams sync`. No separate VM concept.

**Question:** How should VMs relate to the Formation trait?

**Answer:** (A) VM as a formation type. `bm runtime create` becomes `formation.setup()` for a "lima" formation type. The Lima VM is the environment, the Formation trait manages its full lifecycle. `bm runtime` commands become formation-aware (or are replaced by formation commands). `bm attach` becomes a formation-level operation (e.g., `bm formation shell` or stays as `bm attach` but delegates to the formation).

---

## Q4: Headless / SSH-only environments and the manifest flow

The GitHub App Manifest flow requires opening a browser — the operator clicks "Create GitHub App" on github.com. This works for desktop operators but breaks for:
- SSH-only servers (no browser)
- CI/CD pipelines
- Lima VMs (where `bm hire` might run inside the VM)

The initial document mentions `bm hire --non-interactive` as an open question with two options: (a) accept pre-generated App credentials via flags/env vars, (b) use the GitHub API with a PAT to create the App programmatically.

**But with the formation merger**, there's a new angle: if a Lima VM is a formation type, does `bm hire` run on the operator's machine (which has a browser) or inside the VM (which doesn't)? This determines whether the manifest flow is even the right default.

**Options:**
- (A) **Manifest flow is always operator-side**: `bm hire` always runs on the operator's machine (where they have a browser). The formation's `setup()` then provisions the credentials into the target environment (VM, container, etc.)
- (B) **Dual mode**: Interactive `bm hire` uses the manifest flow (browser). `bm hire --non-interactive` accepts pre-generated credentials via `--app-id`, `--private-key-file`, `--installation-id` flags.
- (C) **Manifest flow + URL fallback**: For headless environments, print the manifest URL and ask the operator to open it manually, then paste the callback code back into the terminal (like `gh auth login` does with device flow).

**Question:** How should `bm hire` handle environments without a browser?

**Answer:** All modes support all credential sources. The resolution order is:

1. **Pre-generated credentials** (flags/env vars) — if `--app-id`, `--private-key-file`, `--installation-id` are provided, use them directly. Works in both interactive and `--non-interactive` mode.
2. **Browser manifest flow** — if a browser is available and no pre-generated credentials, open the browser for one-click App creation.
3. **URL fallback (headless)** — if no browser is detected (or browser open fails), print the manifest URL and prompt the operator to open it manually and paste back the callback code (like `gh auth login` device flow). Works in interactive mode.

In `--non-interactive` mode without pre-generated credentials, the command fails with clear instructions on how to provide them.

---

## Q5: GitHub App ownership — user-level vs org-level

GitHub Apps can be owned by a user account or by an organization. The initial document doesn't address this explicitly. The manifest flow URL differs:
- User-owned: `https://github.com/settings/apps/new`
- Org-owned: `https://github.com/organizations/{org}/settings/apps/new`

User-owned Apps have simpler setup but are tied to one person's account. Org-owned Apps are visible to all org admins, survive employee turnover, and can be managed by the org.

Since `bm init` already selects an org (or personal account), we have the context to choose.

**Options:**
- (A) **Always user-owned**: Simpler. The operator who runs `bm hire` owns all the Apps. If they leave, Apps need to be transferred or recreated.
- (B) **Always org-owned**: Apps live under the org. Requires the operator to be an org admin. More durable.
- (C) **Follow the team repo owner**: If the team repo is under an org, create org-owned Apps. If it's under a personal account, create user-owned Apps.

**Question:** Who should own the GitHub Apps — the user or the org?

**Answer:** (C) Follow the team repo owner. If the team repo is under an org, create org-owned Apps (`/organizations/{org}/settings/apps/new`). If it's under a personal account, create user-owned Apps (`/settings/apps/new`). Detection via `gh api /repos/{owner}/{repo}` to check `owner.type`.

Important nuance: even if detection resolves to org-owned, the operator might not have org owner permissions. In that case, fall back gracefully to user-owned with a warning explaining the situation (user-owned Apps on org repos work fine — the operator just needs repo admin access to install them).

---

## Q6: App installation scope — single repo or org-wide?

When installing a GitHub App after creation, there are two options:
- **Single repo installation**: The App is installed only on the team repo. Tightest scope.
- **Org-wide installation**: The App is installed on all repos in the org (or selected repos). Needed if members work on project repos that are separate from the team repo.

Currently, members interact with the team repo (issues, PRs, board scans) AND project repos (cloned via `bm projects add`). If the App is only installed on the team repo, members can't push to project repos using their bot identity.

**Options:**
- (A) **Team repo only**: Install on team repo only. Project repo access uses the operator's auth (current behavior via PAT). Simplest, tightest scope.
- (B) **Team repo + project repos**: Install on team repo and each project repo. `bm projects add` would need to add an installation for each member's App on the project repo.
- (C) **Org-wide**: Install on all org repos. Broadest scope, simplest to manage, but grants access to repos the team doesn't use.

**Question:** What scope should the App installation cover?

**Answer:** (B) Team repo + project repos. The App is installed on the team repo during `bm hire`, and on each project repo during `bm projects add`. This means `bm projects add` gains a new responsibility: installing each hired member's App on the project repo. The installation ID may differ per repo — need to track installation IDs per (member, repo) pair, not just per member.

This has a data model implication: the ADR's keyring scheme (`botminter/{team}/{member}/github-installation-id`) assumes one installation ID per member. With multi-repo installations, we need either:
- One installation per App that covers multiple repos (GitHub supports this — an installation can cover selected repos)
- Multiple installation IDs per member (one per repo)

GitHub App installations are per-org or per-account, not per-repo. You select which repos the installation has access to. So a single installation ID per member works — `bm projects add` just adds the project repo to the existing installation's repo selection list.

---

## Q7: `bm fire` and App cleanup

The initial document lists App cleanup on `bm fire` as a future concern. But since we're doing the full formation refactor, should we handle it now?

When a member is removed (`bm fire`):
- The App's installation can be removed via `DELETE /app/installations/{id}`
- The App itself can be deleted via `DELETE /apps/{app_slug}` (requires App credentials)
- Keyring entries need to be cleaned up

**Options:**
- (A) **Full cleanup**: `bm fire` removes the installation, deletes the App, and cleans up keyring entries.
- (B) **Installation removal only**: `bm fire` removes the installation and cleans keyring entries, but leaves the App on GitHub (operator can delete manually). Simpler — no need to authenticate as the App to delete itself.
- (C) **Defer**: `bm fire` only cleans keyring entries. App and installation remain on GitHub. Print instructions for manual cleanup.

**Question:** How thorough should `bm fire` cleanup be?

**Answer:** (A) Build `bm fire` now with full cleanup, but with a `--keep-app` flag that removes the installation and keyring entries but leaves the GitHub App on GitHub for potential reuse. Symmetrically, `bm hire` gets a `--reuse-app` option (or similar) to adopt an existing GitHub App instead of creating a new one.

Important context: both `bm hire` and `bm projects add` can also happen during `bm init` (the init wizard already hires members and adds projects as part of the setup flow). So the GitHub App creation (manifest flow or pre-generated credentials) must work when called from `bm init`'s wizard context, not just as standalone commands.

This means:
- `bm init` wizard → hires members → triggers App creation per member
- `bm init` wizard → adds projects → triggers App installation on project repos for all hired members
- `bm hire` standalone → creates App + installs on team repo + installs on existing project repos
- `bm projects add` standalone → installs all hired members' Apps on the new project repo
- `bm fire` → removes installations, cleans keyring; `--keep-app` skips App deletion
- `bm hire --reuse-app` → adopts existing App credentials instead of creating new

---

## Q8: Token refresh architecture — where does the refresh loop run?

The ADR says the member process (Ralph or Brain) signs a JWT and refreshes the installation token at the 50-minute mark via a background task. But there's a design question about where this loop lives.

**Current member launch flow:**
1. `bm start` reads credentials, generates an installation token
2. Passes the token to the member process as `GH_TOKEN` env var
3. Member process (Ralph/Brain) runs `gh` CLI commands using that env var

The problem: once the process is launched, `GH_TOKEN` is baked into its environment. Refreshing requires either:
- The member process itself having access to the private key and running a refresh loop
- An external sidecar/wrapper that updates `GH_TOKEN` and signals the process
- A token-providing mechanism (file, socket) that `gh` can read from

**Options:**
- (A) **Member-internal refresh**: Pass the private key (or keyring access) to the member process. It runs its own JWT→token refresh loop. Simplest, but the member process needs crypto capabilities.
- (B) **Wrapper process**: `bm start` launches a wrapper that manages the token and re-exports `GH_TOKEN` before delegating to Ralph/Brain. The wrapper handles refresh.
- (C) **Token file**: Write the token to a file. Set `GH_TOKEN` to read from the file (gh supports `GH_TOKEN` from a file via a helper). Refresh loop updates the file.

**Question:** Where should the token refresh loop run?

**Answer:** The daemon becomes a mandatory process in the new architecture. Today the daemon (`bm daemon-run`) is optional — it runs an axum server for webhooks/polling and launches members. In the new model, the daemon becomes the central supervisor process that:

1. Manages member process lifecycle (launch, health check, restart)
2. Holds keyring access and manages token refresh for all members
3. Delivers fresh tokens to member processes (mechanism TBD — file, env update, socket)
4. Runs the webhook/poll event loop (existing functionality)
5. Serves the web console (existing functionality)

`bm start` becomes "start the daemon" rather than "launch member processes directly." The daemon owns the member processes as children.

This resolves the token refresh question cleanly: the daemon has keyring access, runs the JWT→token refresh loop per member, and pushes updated tokens to the member processes. No crypto needed in Ralph/Brain.

**Open sub-question:** How does the daemon deliver refreshed tokens to running member processes? Options:
- Token file per member (daemon writes, `gh` reads via credential helper or `GH_TOKEN=$(cat file)` wrapper)
- Unix socket per member (daemon serves tokens on request)
- Shared memory / env var update (not possible across processes)

This will be resolved during design.

---

## Q9: `bm start` vs daemon relationship

Today `bm start` spawns member processes directly (no daemon required). The daemon is an optional separate mode (`bm daemon start`). With the daemon becoming mandatory:

**Options:**
- (A) **`bm start` launches the daemon**: `bm start` becomes `bm daemon start` under the hood. The daemon then launches members. `bm start <member>` tells the daemon to launch a specific member.
- (B) **Merge `bm start` and `bm daemon start`**: Remove the separate daemon concept. `bm start` always starts the daemon, which manages members. `bm stop` stops the daemon (and all members).
- (C) **Daemon is infrastructure, `bm start` is intent**: `bm start` ensures the daemon is running (starts it if needed), then tells the daemon to launch the requested members. Daemon can outlive individual member starts/stops.

**Question:** How should `bm start` relate to the daemon?

**Answer:** (C) Daemon is infrastructure, `bm start` is intent. The daemon is a long-lived supervisor process. `bm start` ensures the daemon is running (starts it if needed), then tells it to launch the requested members. `bm stop` stops members but leaves the daemon running. `bm stop --all` or `bm daemon stop` stops the daemon too.

This means:
- Daemon persists across member start/stop cycles
- Web console and webhook listener are always available while daemon runs
- Token refresh loops run inside the daemon, not inside members
- `bm status` always has a daemon to query
- Webhooks can trigger member launches even when no members are currently running

The daemon becomes the Formation trait's runtime — `formation.launch_member()` is actually "tell the daemon to launch a member." Direct process spawning (`bm start` → fork/exec Ralph) is replaced by daemon-mediated process management.

---

## Q10: macOS support scope

ADR-0008 describes a platform split with `linux/` and `macos/` sub-modules. The macOS side needs a `KeychainCredentialStore` (Security framework), `sysadminctl`/`dscl` for accounts, `brew` for packages, and `launchd` for services.

Given this is Alpha and the current codebase only supports Linux (gnome-keyring, D-Bus, podman, Lima):

**Options:**
- (A) **Linux only for now**: Implement `LinuxLocalFormation` fully. `MacosLocalFormation` returns "not yet supported" errors. Leave the trait + module structure ready for macOS.
- (B) **Both platforms**: Implement both `LinuxLocalFormation` and `MacosLocalFormation` in this milestone.

**Question:** Should macOS formation support be in scope for this milestone?

**Answer:** (A) Linux only. Implement `LinuxLocalFormation` fully. `MacosLocalFormation` returns clear "not yet supported" errors. The trait and module structure (`formation/local/linux/`, `formation/local/macos/`) are created so macOS can be added later without restructuring.

---

## Q10b: bm-agent binary and Brain/Ralph loop model changes

Two additional impacts from the daemon-as-supervisor model:

**bm-agent binary:**
Currently `bm-agent` provides inbox messaging (brain↔loop) and a Claude Code hook. With the daemon managing tokens, `bm-agent` may need a way for the member process to request a fresh token from the daemon (e.g., `bm-agent token refresh` or `bm-agent token get`). Alternatively, if the daemon pushes tokens via file, `bm-agent` doesn't need to change.

**Brain/Ralph loop relationship:**
Currently Brain is a special member type that runs an ACP session (Claude Code via multiplexer). Brain can spawn Ralph loops via background Bash commands. In the new model where the daemon is the single supervisor, Brain should NOT spawn Ralph loops directly — it should ask the daemon to manage them. This affects:
- Brain's system prompt (instructions about spawning loops)
- The `bm-agent inbox` mechanism (how Brain communicates with the daemon)
- The event watcher (loop events coming from daemon-managed processes, not Brain-spawned ones)

**Answer:** Both need updates:
1. `bm-agent` gains a token endpoint or the daemon delivers tokens via a file-based mechanism that doesn't require `bm-agent` changes
2. Brain's system prompt and instructions are updated so that Brain asks the daemon to manage Ralph loops rather than spawning them directly. Brain becomes a chat-focused agent that delegates work orchestration to the daemon. The daemon manages process lifecycle for both Brain and Ralph loop processes.

---

## Q11: CLI command surface changes

This milestone touches a lot of CLI commands. Let me confirm the expected command surface changes:

**Modified commands:**
- `bm init` — no token prompt, validates `gh auth` session, hires trigger App creation
- `bm hire` — creates GitHub App (manifest flow or pre-generated), installs on team repo + project repos, stores credentials in keyring. New flags: `--reuse-app`, `--app-id`, `--private-key-file`, `--installation-id`
- `bm start` — ensures daemon is running, tells daemon to launch members (no direct process spawning)
- `bm stop` — tells daemon to stop members, daemon stays running. `--all` stops daemon too
- `bm status` — queries daemon for member status, token health, etc.
- `bm projects add` — installs all hired members' Apps on the new project repo
- `bm teams sync` — unchanged? Or does it gain formation-awareness?

**New commands:**
- `bm fire <member>` — removes member, cleans up App/installation/keyring. `--keep-app` flag.
- `bm daemon start/stop/status` — explicit daemon lifecycle control (or do these stay hidden?)

**Removed/replaced commands:**
- `bm runtime create/delete` — replaced by formation setup (`bm formation setup`? or stays as `bm runtime`?)
- `bm attach` — becomes formation-aware (delegates to formation for shell access)

**Updated command surface with Q10b impacts:**

**Modified commands:**
- `bm init` — no token prompt, validates `gh auth` session, hires trigger App creation
- `bm hire` — creates GitHub App (manifest/pre-generated/URL-fallback), installs on team repo + project repos, stores credentials in keyring. New flags: `--reuse-app`, `--app-id`, `--private-key-file`, `--installation-id`
- `bm start` — ensures daemon is running, tells daemon to launch members
- `bm stop` — tells daemon to stop members, daemon persists. `--all` stops daemon.
- `bm status` — queries daemon for member status, token health
- `bm projects add` — installs all hired members' Apps on the new project repo
- `bm teams sync` — gains formation-awareness

**New commands:**
- `bm fire <member>` — removes member, cleans up App/installation/keyring. `--keep-app` flag.
- `bm daemon start/stop/status` — explicit daemon lifecycle control
- `bm-agent` updates — token endpoint or daemon communication for loop management

**Removed/replaced:**
- `bm runtime create/delete` → formation setup
- `bm attach` → formation-aware

**Brain model change:**
- Brain no longer spawns Ralph loops directly
- Brain communicates with daemon to manage loops
- Brain system prompt updated accordingly

**Question:** Does this updated command surface look right? Any commands I'm missing or mischaracterizing?

**Answer:** Yes, confirmed. The command surface is correct as described.

---

## Q12: E2E and exploratory test strategy for GitHub App auth

The current E2E tests use `TESTS_GH_TOKEN` (a shared PAT) for all GitHub operations. With per-member Apps, the test strategy needs to change. The initial document lists this as an open question.

**Key constraints:**
- E2E tests run in CI and locally — need to work in both
- The manifest flow requires a browser (or URL fallback) — hard to automate in CI
- Creating real GitHub Apps per test run is slow and requires cleanup
- Exploratory tests run on `bm-test-user@localhost` — a separate user account

**Options:**
- (A) **Pre-provisioned test App**: Create one GitHub App manually (once), store its credentials in CI secrets. E2E tests use `--reuse-app` with these credentials. Doesn't test the manifest flow itself but tests everything else.
- (B) **Real App creation per test run**: E2E tests create real Apps via the manifest flow (using the URL fallback + automated code exchange). Slow, requires cleanup, but tests the full flow.
- (C) **Hybrid**: One E2E test creates a real App (tests the manifest flow). All other tests use a pre-provisioned App (tests the runtime flow). Cleanup deletes the per-run App.
- (D) **Mock the GitHub API**: Use a mock server for App creation/token exchange in E2E tests. Fast, no cleanup, but doesn't test real GitHub integration.

**Question:** What test strategy for GitHub App auth?

**Answer:** (C) Hybrid. One dedicated E2E test exercises the full manifest flow (App creation via URL fallback + automated code exchange, installation, token generation, cleanup). All other E2E tests use a pre-provisioned test App via `--reuse-app` with credentials from CI secrets / env vars. The manifest flow test cleans up the App it creates. Unit tests mock the HTTP layer for JWT/token exchange logic.

---

## Q13: Migration path — what happens to existing teams?

Alpha policy says "no migration paths, no backward compatibility." But practically:
- Existing `config.yml` files have `credentials.gh_token` — will they cause deserialization errors?
- Existing team repos have no GitHub App credentials in the keyring
- The daemon doesn't exist yet — `bm start` currently spawns processes directly

**Options:**
- (A) **Hard break**: Existing teams stop working. `bm init` must be re-run. `credentials.gh_token` in old configs causes a warning but is ignored (serde `default` + `skip_serializing`).
- (B) **Graceful deprecation**: Old configs load fine (`gh_token` is ignored with a warning). Commands that need App credentials fail with "run `bm hire` to set up GitHub App for this member." No silent fallback to PAT.

**Question:** How should existing teams transition?

**Answer:** (A) Hard break. Existing teams stop working — operators must re-run `bm init`. `credentials.gh_token` in old config files is silently ignored on deserialization (serde `default` + `skip_serializing`) so the config file still loads, but no PAT-based operations are supported. All commands requiring GitHub access fail with clear guidance pointing to `bm init`.

---

## Q14: Token delivery mechanism — GH_CONFIG_DIR + hosts.yml

**Decision:** Use `gh`'s native `GH_CONFIG_DIR` + `hosts.yml` file for token delivery.

**How it works:**
1. Each member workspace gets its own gh config directory: `{workspace}/.config/gh/`
2. The daemon writes `hosts.yml` with the current installation token as `oauth_token`
3. Member process is launched with `GH_CONFIG_DIR={workspace}/.config/gh/`
4. `gh` reads `hosts.yml` on every invocation — always gets the latest token
5. Daemon refreshes the token at 50 min → overwrites `hosts.yml` → next `gh` call uses fresh token
6. For `git` operations: `gh auth setup-git` configures `gh` as the git credential helper in the member's `GH_CONFIG_DIR` — so git also reads fresh tokens via `gh`

**Why this works:**
- No custom `bm-agent` wrapper needed for `gh` — just `GH_CONFIG_DIR`
- No custom git credential helper — `gh auth setup-git` already provides one
- Token refresh is a file write — no IPC, no sockets, no process signaling
- `gh auth status` works out of the box for debugging
- Each member has isolated gh config — no cross-contamination

**File format (`hosts.yml`):**
```yaml
github.com:
  oauth_token: ghs_xxxxxxxxxxxx  # installation access token
  git_protocol: https
  user: {team}-{member}[bot]
```

**Answer:** Locked in. `GH_CONFIG_DIR` per member workspace, daemon writes `hosts.yml`, `gh auth setup-git` for git credential helper. No `GH_TOKEN` env var, no custom credential helpers.

---

## Q15: Formation trait — `LaunchParams` redesign

ADR-0008 defines `LaunchParams` with `gh_token: &str`. With per-member App auth and daemon-managed tokens, this field is obsolete — the daemon writes `hosts.yml` before launching. But `LaunchParams` still needs to convey the member's workspace, bridge credentials, and `GH_CONFIG_DIR` path.

**Question:** What should `LaunchParams` look like in the new model?

**Proposed answer (REVISED per formation-as-top-level-abstraction):** Remove `gh_token` from `LaunchParams`. The formation manages the full member lifecycle — including ensuring a daemon is running and credentials are delivered. New `LaunchParams`:

```rust
pub struct LaunchParams<'a> {
    pub workspace: &'a Path,
    pub gh_config_dir: &'a Path,          // replaces gh_token
    pub member_token: Option<&'a str>,     // bridge credential
    pub bridge_type: Option<&'a str>,
    pub service_url: Option<&'a str>,
    pub room_id: Option<&'a str>,          // added (was resolved in start_members)
    pub user_id: Option<&'a str>,          // added
    pub operator_user_id: Option<&'a str>, // added
    pub team_repo: Option<&'a Path>,       // needed for brain
    pub system_prompt: Option<&'a Path>,   // needed for brain
    pub is_brain: bool,                    // dispatch ralph vs brain
}
```

The formation's `launch_member()` sets `GH_CONFIG_DIR` as an env var on the child process, dispatches to ralph or brain launch, and handles bridge token env var mapping. All the logic currently in `launch_ralph()` and `launch_brain()` moves into the formation's implementation.

---

## Q16: Formation trait — credential delivery as a formation concern

ADR-0008 lists "credential delivery — how credentials reach members at runtime" as a formation responsibility. Today this is `GH_TOKEN` env var injection in `launch_ralph()`/`launch_brain()`. In the new model it's `GH_CONFIG_DIR` + `hosts.yml`.

But the daemon manages the refresh loop, not the formation. So who owns credential delivery?

**Question:** Who writes the initial `hosts.yml` — the formation or the daemon?

**Proposed answer:** The daemon writes `hosts.yml` — both initial and refresh. The daemon is the token lifecycle owner. The sequence is:

1. Daemon reads App credentials from in-memory cache
2. Daemon generates installation token (JWT → exchange)
3. Daemon writes `{workspace}/.config/gh/hosts.yml`
4. Daemon calls `formation.launch_member(LaunchParams { gh_config_dir: ... })`
5. Formation sets `GH_CONFIG_DIR` env var on child process, launches it

The formation's credential delivery role is limited to "set `GH_CONFIG_DIR` on the child process." The formation doesn't know or care about tokens, JWTs, or hosts.yml contents. This keeps the formation focused on "how to run a process in this environment" and keeps token lifecycle in the daemon.

---

## Q17: Formation trait — `setup()` scope

ADR-0008 says `setup()` "installs ralph, configures coding agent, sets up keyring." Currently there is NO `setup()` — ralph installation and keyring setup are manual prerequisites. With Lima becoming a formation type, `setup()` becomes more relevant.

**Question:** What does `setup()` actually do for the local formation vs Lima formation?

**Proposed answer:** (A) for local, (C) for Lima.

- **Local formation `setup()`:** Verification only — check ralph in PATH, check keyring accessible, check `gh auth` session, check coding agent installed. Reports what's missing with actionable fix instructions. Does NOT install software (operator installs manually). This is essentially a structured `check_environment()` that also runs `gh auth setup-git` per workspace.

- **Lima formation `setup()`:** Full lifecycle — create VM via `limactl`, install ralph + coding agent inside VM, configure networking, set up keyring inside VM. This absorbs the existing `bm runtime create` logic from `formation/lima.rs`. `setup()` is idempotent — running it again verifies the VM exists and is configured correctly.

The rationale: local operators manage their own machine — we verify but don't touch. Lima VMs are disposable infrastructure — we own the full lifecycle.

---

## Q18: Formation trait — bridge auto-start relationship

Today bridge auto-start happens inside `start_local_members()` — before launching members. ADR-0008 says "Bridge lifecycle is NOT a formation concern." But the daemon now owns member launch, and bridge auto-start is part of the launch sequence.

**Question:** Where does bridge auto-start live in the new architecture?

**Proposed answer:** (A) Daemon-level. The daemon handles bridge auto-start as part of its pre-launch sequence, alongside token generation. The sequence becomes:

1. `bm start` → ensures daemon running → tells daemon to launch members
2. Daemon: bridge auto-start (if configured and not already running)
3. Daemon: generate tokens, write `hosts.yml` per member
4. Daemon: call `formation.launch_member()` per member

The formation trait has no bridge awareness. Bridge auto-start is a team-level orchestration concern, not a deployment concern. The daemon is the right place — it already knows about bridge config (from the team repo) and already handles pre-launch orchestration.

---

## Q19: Formation trait — `bm chat` and `bm attach`

ADR-0008 says "`bm chat` always execs into a local workspace regardless of formation." But with Lima as a formation type, `bm chat` inside a Lima VM means SSH-ing into the VM first.

**Question:** Should the Formation trait have a `shell()` or `exec_in()` method for interactive access?

**Proposed answer:** Yes. Add two methods to the Formation trait:

```rust
/// Execute a command in the formation's environment.
/// Local: exec directly. Lima: SSH into VM then exec.
fn exec_in(&self, workspace: &Path, cmd: &[&str]) -> Result<()>;

/// Open an interactive shell in the formation's environment.
/// Local: no-op (already in the right env). Lima: SSH into VM.
fn shell(&self) -> Result<()>;
```

- `bm chat <member>` → resolves workspace → `formation.exec_in(workspace, &["claude", "--resume"])` (or equivalent)
- `bm attach` → `formation.shell()`
- Local formation: `exec_in()` just runs the command directly. `shell()` is a no-op or opens a subshell in the workzone.
- Lima formation: `exec_in()` wraps in `limactl shell <vm> -- <cmd>`. `shell()` runs `limactl shell <vm>`.

This keeps `bm chat` and `bm attach` formation-agnostic while supporting boundary crossing transparently.

---

## Q20: Formation — command refactor scope

ADR-0008 says commands are formation-agnostic: resolve → create `Box<dyn Formation>` → call trait methods.

**Question:** Which commands need refactoring to go through the Formation trait, and which stay as-is?

**Proposed answer:**

**Through Formation trait (via daemon):**
- `bm start` → daemon → `formation.launch_member()` (currently: `start_local_members()`)
- `bm stop` → daemon → `formation.stop_member()` (currently: `stop_local_members()`)
- `bm status` → daemon → `formation.is_member_alive()` (currently: PID check in `state.rs`)

**Through Formation trait (direct):**
- `bm chat` → `formation.exec_in()` (currently: direct exec)
- `bm attach` → `formation.shell()` (currently: `limactl shell`)
- `bm credentials export/import` → `formation.credential_store()` (new commands)

**Formation-aware but NOT through trait methods:**
- `bm teams sync` → needs to know `GH_CONFIG_DIR` path convention for workspace setup, but provisioning itself is formation-independent
- `bm runtime create/delete` → replaced by `formation.setup()` / formation teardown

**Stay as-is (no formation involvement):**
- `bm init` — creates team repo, registers team (uses operator's `gh auth`)
- `bm hire` — creates member dir + App credentials (keyring is accessed via `formation.credential_store()` but hire logic is formation-independent)
- `bm fire` — removes member + App cleanup (same — uses `formation.credential_store()` for keyring access)
- `bm projects add/list/show` — project management
- `bm members list/show` — reads from team repo
- `bm roles list` — reads from profile
- `bm profiles list/describe` — reads embedded profiles
- `bm bridge *` — bridge lifecycle (not formation concern per ADR-0008)
- `bm teams list/show` — reads from config

Note: `bm hire` and `bm fire` use `formation.credential_store()` to access the keyring, so they need a formation instance — but the formation doesn't own the hire/fire logic, it just provides the credential backend.

---

## Q21: Formation as top-level runtime abstraction — daemon is internal

**Key architectural clarification:** The formation is the top-level abstraction for running members. The daemon is an implementation detail — always present, but managed by the formation, not by the operator.

**How it works:**
- `bm start` → calls formation → formation ensures members are running (however that works for this formation type)
- `bm stop` → calls formation → formation stops members
- `bm status` → calls formation → formation reports member status

**Per formation type:**

| Formation | What `start` does | Where daemon runs | Daemon scope |
|-----------|-------------------|-------------------|--------------|
| Local | Launch one local daemon process | Operator's machine | All members |
| Lima | SSH into VM, launch daemon inside VM | Inside the Lima VM | All members in that VM |
| K8s (future) | Deploy pods via kubectl | Inside each pod | One member per pod |

The daemon is always "local to the members it manages." The formation decides where and how to deploy it.

**Implications:**
- `bm daemon start/stop/status` become internal/hidden — operators never manage daemons directly
- `bm start/stop/status` are formation commands, not daemon commands
- The Formation trait gains `start_members()`, `stop_members()`, `status()` as the top-level entry points
- The daemon binary remains the same — it's the formation that decides where to run it
- Token refresh, webhook handling, web console all run inside the daemon regardless of formation type

**Formation trait (revised):**
```rust
pub trait Formation {
    fn name(&self) -> &str;

    // Environment
    fn setup(&self, params: &SetupParams) -> Result<()>;
    fn check_environment(&self) -> Result<EnvironmentStatus>;
    fn check_prerequisites(&self) -> Result<()>;

    // Credentials
    fn credential_store(&self, domain: CredentialDomain) -> Result<Box<dyn CredentialStore>>;
    fn deliver_token(&self, member: &str, workspace: &Path, token: &str) -> Result<()>;

    // Member lifecycle (top-level — formation manages daemon internally)
    fn start_members(&self, params: &StartParams) -> Result<StartResult>;
    fn stop_members(&self, params: &StopParams) -> Result<StopResult>;
    fn member_status(&self) -> Result<Vec<MemberStatus>>;

    // Interactive access
    fn exec_in(&self, workspace: &Path, cmd: &[&str]) -> Result<()>;
    fn shell(&self) -> Result<()>;

    // Topology
    fn write_topology(&self, workzone: &Path, team_name: &str, members: &[(String, MemberHandle)]) -> Result<()>;
}
```

Note: `launch_member()`, `stop_member()`, `is_member_alive()` from the original ADR-0008 become internal to the formation — the daemon calls them. The trait's public surface is `start_members()` / `stop_members()` / `member_status()` which handle the full orchestration (daemon management + member lifecycle).

**Answer:** Confirmed. The formation is the runtime abstraction managed by the formation. The daemon is an implementation detail managed by the formation. Operators never interact with daemons directly.

**Refinement (Q21b):** The highest-level abstraction is the **Team**, not the formation. `bm start [member]` resolves the team → team holds its formation → formation manages daemons/members.

```
Team → Formation → Daemon → Members
```

Every runtime CLI command goes through the team, which delegates to its formation:
```rust
let team = resolve_team(flag)?;
let formation = team.formation()?;
formation.start_members(&params)?;
```

No `bm formation` or `bm daemon` commands. Just `bm start/stop/status` operating on the team. The team config already has formation info from the team repo.

---

## Q22: Unified architecture — Team, Runtime, Formation, Daemon

The existing ADR-0008 only covers Formation + local formation details. It doesn't define how Team, Runtime, Formation, and Daemon relate to each other. These need to be crystal clear in a single unified design.

**Concept hierarchy:**

| Concept | What it is | Who sees it |
|---------|-----------|-------------|
| **Team** | The user-facing entity. Has members, projects, a repo. The API boundary. | Operator |
| **Formation** | The deployment strategy. Manages everything below. Internal to the team. | Nobody (implementation detail) |

**What the formation manages (its capabilities):**

| Capability | What it does | Local formation | Lima formation | K8s formation (future) |
|-----------|-------------|----------------|---------------|----------------------|
| **Environment** | Where members run | Operator's machine (verify only) | Lima VM (create/manage) | K8s cluster (connect/configure) |
| **Credentials** | Where secrets are stored | System keyring | Keyring inside VM | K8s Secrets |
| **Credential delivery** | How members access tokens | `hosts.yml` via `GH_CONFIG_DIR` | Same (inside VM) | Mounted secret volume |
| **Member lifecycle** | Launch, stop, health, token refresh | Local processes (via daemon) | Processes inside VM (via daemon) | Pods (daemon per pod) |

The daemon is an implementation detail of member lifecycle — it's how the formation supervises members, refreshes tokens, and handles webhooks. It is not a separate capability or concept. The formation decides when and where to run daemons as part of managing members.

**Operator-facing commands (Team level):**
```
bm start [member] [-t team]      # team.start()
bm stop [member] [-t team]       # team.stop()
bm status [-t team]              # team.status()
bm hire <role> [-t team]         # team.hire()
bm fire <member> [-t team]       # team.fire()
bm chat <member> [-t team]       # team.chat()
bm attach [-t team]              # team.attach() → environment shell
```

**Environment setup (one-time):**
```
bm env create [-t team]          # team.setup_env() → formation.setup()
bm env delete [-t team]          # team.teardown_env()
```

**Everything below Team is internal:**
- Team holds a Formation (resolved from config)
- Formation manages Daemon(s) (started/stopped as needed)
- Daemon manages Member processes (launch, health, token refresh)
- Operator never interacts with Formation or Daemon directly

**The design doc should replace ADR-0008 with a unified architecture** covering Team → Environment → Formation → Daemon → Members as one cohesive design. ADR-0011 (GitHub App identity) is one aspect of this architecture (credentials and token lifecycle).

**Answer:** Confirmed. "Runtime" is renamed to "Environment." The design doc becomes the unified architecture document covering all four concepts. ADR-0008 is superseded by this design. The formation, daemon, and credential lifecycle are implementation details of the Team abstraction.

---

## Adversarial Review Findings

Two adversarial review agents reviewed both ADRs against the codebase. The findings below are organized by severity with decisions recorded.

### Critical Issue R1: Sync vs async Formation trait

`start_members()` is sync but the daemon it manages is async (tokio with axum, webhook loops, token refresh). Cramming daemon start + JWT generation + token delivery + member spawn into a sync call would either block indefinitely or force nested runtimes.

**Options:** (A) async_trait on Formation, (B) Keep trait sync — daemon is a separate process spawned by the formation, CLI communicates via HTTP, (C) Hybrid — some methods async.

**Answer:** (B). The daemon is already spawned as a separate process today. The formation's `start_members()` spawns the daemon process, waits for readiness via health check, then sends HTTP requests to tell it to launch members. No async needed on the CLI side.

---

### Critical Issue R2: CredentialStore trait undefined for multi-field credentials

The existing `CredentialStore` stores one string per member. GitHub App credentials need four fields (App ID, Client ID, private key, installation ID). The trait can't handle this.

**Options:** (A) Key-value store — `store(key, value)` / `retrieve(key)`, callers compose keys like `{member}/github-app-id`, (B) Typed credential structs with separate methods, (C) Serialize to JSON as single string.

**Answer:** (A). The trait becomes a simple key-value secret store. Each credential domain composes its own keys. Formation-agnostic — works for keyring, K8s Secrets, any backend.

---

### Critical Issue R3: `deliver_token()` too narrow

Takes just `token: &str` but writing `hosts.yml` needs bot username, git protocol, and initial setup requires configuring git credential helper. These are different concerns — setup is one-time, refresh is every 50 minutes.

**Options:** (A) Richer struct parameter, (B) Formation derives extra fields internally, (C) Split into `setup_token_delivery()` (one-time) and `refresh_token()` (frequent).

**Answer:** (C). Setup and refresh are separate concerns. `setup_token_delivery()` creates dirs, writes config, configures git credential helper. `refresh_token()` just updates `hosts.yml` atomically.

---

### Critical Issue R4: Installation tokens can't call `/user`

`validate_token()` calls `gh api user` to verify tokens. Installation tokens return 403 on `/user`. Risk of accidentally validating with member context.

**Options:** (A) Split into `validate_operator_token()` and `verify_installation_token()`, (B) Single function with mode, (C) Don't validate installation tokens — trust the JWT exchange flow.

**Answer:** (C). The token exchange endpoint either succeeds or fails. If it returned a token, it works. Only validate operator tokens. Ensure no code path calls `validate_token()` with member context.

---

### Critical Issue R5: `organization_projects` doesn't work on personal accounts

`organization_projects: admin` is an org-level permission. Personal accounts don't support it. Members on personal-account teams can't access Projects v2.

**Options:** (A) Document as limitation, (B) Detect and error, (C) Block personal accounts.

**Answer:** (C). Require an org for `bm init`. Personal accounts are not supported. The team repo must be in a GitHub organization.

---

### Critical Issue R6: `gh auth setup-git` writes to global `.gitconfig`

`gh auth setup-git` modifies `~/.gitconfig` — affects all git operations globally. Multiple members overwrite each other. Git hooks/subcommands spawned without `GH_CONFIG_DIR` use wrong credentials.

**Options:** (A) `GIT_CONFIG_GLOBAL` per workspace, (B) Skip `gh auth setup-git`, write credential helper directly into workspace `.git/config`, (C) Run setup with `GIT_CONFIG` scoping.

**Answer:** (B). During `setup_token_delivery()`, write the credential helper config directly into the workspace repo's `.git/config`. No global side effects. `gh auth git-credential` respects `GH_CONFIG_DIR` already set on the member process.

---

### Important Gap R7: CLI↔Daemon IPC mechanism unspecified

The daemon owns member lifecycle and token refresh. The CLI needs to tell the daemon "start member X" without spawning processes directly. The IPC mechanism is absent from the design.

**Answer:** Daemon exposes a RESTful HTTP API on localhost (already has axum) with an OpenAPI schema. CLI sends requests. Daemon writes PID + port to a state file. `bm start` checks if daemon is running via PID/health check, starts if needed, then sends HTTP requests. OpenAPI schema provides a documented contract for the daemon API.

---

### Important Gap R8: `state.json` race condition

CLI and daemon both read/write `state.json` without file locking.

**Answer:** Daemon owns `state.json`. All mutations go through daemon's HTTP API. CLI reads only. No concurrent writes.

---

### Important Gap R9: `bm credentials export` contradicts "no keys on disk"

Exports private keys in plaintext YAML while the ADR says keys must be in credential store, not on disk.

**Answer:** Intentional escape hatch for machine portability. File written with 0600 permissions. CLI prints explicit security warning. Documented as the operator's responsibility to store securely. Not a contradiction — it's a controlled export, not persistent storage.

---

### Important Gap R10: Manifest flow callback timeout

If the operator walks away after opening the browser, the local server hangs indefinitely with a dangling code.

**Answer:** 5-minute timeout on the local axum server. After timeout, clean up server, print clear error with instructions to retry.

---

### Important Gap R11: Bridge lifecycle contradiction

`StartParams` has `no_bridge: bool` implying formation handles bridge decisions, but "What formation does NOT own" excludes bridges.

**Answer:** Remove `no_bridge` from `StartParams`. Bridge auto-start is daemon-level orchestration — the daemon checks bridge config and starts bridge before launching members. The formation trait has zero bridge awareness. The `--no-bridge` CLI flag is handled by the command layer before calling `team.start()`.
