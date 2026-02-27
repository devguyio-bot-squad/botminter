# Ideal Agentic Workflow UX — Karim's First Week

> The target experience both POCs are measured against.
> Part of [Shift Week Plan](shift-week-plan.md), input to [Ralph vs Claude Code POC](ralph-vs-claude-code-poc.md)
>
> This is written as the onboarding story of a new HyperShift developer. If the final workflow doesn't feel like this, something is wrong.
>
> The system has two components:
> - **RH-O**: The orchestrator (manages the workflow, tracks state, handles handoffs between phases)
> - **RH-A**: The agent (does the actual work — reads code, writes code, runs commands, interacts with the developer)
>
> These are black boxes. Could be any tool or combination of tools. The UX is what matters.

---

## Monday Morning — Day 1

Karim sets his coffee down and opens his laptop. His Red Hat account is active, his VPN works, and he has a fresh Fedora install. He opens a terminal.

```
$ rh-dev init
```

That is the only command he runs to set up his development environment. RH-O takes over. It detects his OS, installs Go, `oc`, `kubectl`, `hypershift` CLI, `kind`, container runtime, and the agent toolchain. It clones `openshift/hypershift` into `~/GolandProjects/github.com/openshift/hypershift`. It takes about twelve minutes. Karim checks his email while it runs.

When it finishes, his terminal shows:

```
Environment ready.
  Go 1.22.4 | oc 4.17 | kind 0.23 | podman 5.2
  Repo: ~/GolandProjects/github.com/openshift/hypershift (branch: main)
  Knowledge base: synced (last updated 2h ago by the team)

You're set up. Type 'rh' to start your first session.
```

That "knowledge base" line is important. Inside the repo, there is a directory called `.devagent/`. Karim runs `ls` on it.

```
$ ls ~/GolandProjects/github.com/openshift/hypershift/.devagent/

agent-config.yaml
knowledge/
  architecture.md
  split-brain.md
  cpov2-patterns.md
  platform-isolation.md
  nodepool-lifecycle.md
  debugging-patterns.md
  kubevirt-specifics.md
  aws-specifics.md
  azure-specifics.md
  ...
constraints/
  api-conventions.md
  adapt-function-rules.md
  generated-code.md
  backward-compat.md
  test-requirements.md
workflows/
  plan.yaml
  design.yaml
  implement.yaml
  review.yaml
  ci-response.yaml
```

This is the team's accumulated institutional knowledge, checked into the repo like any other code. The `knowledge/` directory contains things like "the management cluster never initiates connections to the hosted cluster data plane" and "each control plane namespace must be network-isolated." The `constraints/` directory contains rules the agent must follow — "never modify generated files directly, always modify the API types and run `make api`" and "adapt functions in CPO v2 must be pure transformations with no side effects or external calls." The `workflows/` directory defines the phases RH-O walks through for different task types.

These files are written in plain English. Any developer on the team can read them, edit them, and submit a PR to update them. They are not magic configuration. They are documentation that happens to also be machine-readable.

Karim does not need to read all of this today. That is the point. The agent has read it.

---

## Monday Afternoon — First Task

Karim's tech lead, Sara, pings him on Slack: "I assigned you OCPSTRAT-1751. Good starter story. Ping me if you get stuck."

Karim opens his terminal.

```
$ rh start OCPSTRAT-1751
```

RH-O fetches the JIRA story. It also fetches the parent epic, the parent feature, and the RFE. It reads the acceptance criteria, the linked design doc, the comments. It reads the related stories in the epic to understand what has already been done and what comes next. Then it presents a plan.

What Karim sees on screen is not a wall of text. It is a structured breakdown:

```
---------------------------------------------------------------------
OCPSTRAT-1751: Add RegistryOverrides support to NodePool for KubeVirt
---------------------------------------------------------------------

Context (from RFE -> Feature -> Epic chain):
  RFE:     OCPSTRAT-1200 - Registry mirroring for hosted clusters
  Feature: OCPSTRAT-1400 - Platform-specific registry override propagation
  Epic:    OCPSTRAT-1700 - KubeVirt registry overrides

  Stories already completed in this epic:
    done OCPSTRAT-1748 - API types for RegistryOverrides on KubeVirt NodePool
    done OCPSTRAT-1749 - CPO reconciliation for registry overrides (AWS)
    done OCPSTRAT-1750 - E2e test for AWS registry overrides

  This story:
    OCPSTRAT-1751 - CPO reconciliation for registry overrides (KubeVirt)

  Stories after this:
    OCPSTRAT-1752 - E2e test for KubeVirt registry overrides
    OCPSTRAT-1753 - Documentation update

---------------------------------------------------------------------

Plan:

  Phase 1 - Design
    Identify where KubeVirt NodePool reconciliation handles image
    references. Determine how RegistryOverrides should be threaded from
    the HostedCluster spec through the control plane namespace into the
    KubeVirt worker node configuration.

    Key constraint: The RegistryOverrides data must flow from the
    management cluster side. It must not require the guest cluster to
    pull configuration from the management cluster. (unidirectional
    communication)

    Key constraint: This must not break AWS or Azure NodePool
    reconciliation. Platform-specific logic must remain isolated in
    platform-specific files.

  Phase 2 - Implementation
    Modify the KubeVirt-specific reconciliation in the CPO to read
    RegistryOverrides from the HostedCluster spec and apply them when
    constructing KubeVirt VirtualMachine templates.

    Expected files:
      control-plane-operator/controllers/hostedcontrolplane/
        kubevirt/kubevirt.go
      May also touch:
        api/hypershift/v1beta1/nodepool_types.go (if API needs change)
        hypershift-operator/controllers/nodepool/kubevirt/kubevirt.go

    Generated code: API changes will require 'make api' and 'make
    clients'. This will be handled automatically.

  Phase 3 - Testing
    Unit tests for the new reconciliation logic.
    Verify that existing KubeVirt tests still pass.
    Verify that AWS and Azure tests are unaffected.

    Note: KubeVirt e2e tests take ~75 minutes. Unit tests will run
    first. E2e validation is a separate story (OCPSTRAT-1752).

  Phase 4 - PR and review

---------------------------------------------------------------------

Risks:
  * OCPSTRAT-1748 added the API types but they may not yet be in the
    release branch. Need to verify the types exist on main.
  * The CPO version that ships with 4.17 will need to handle
    HostedClusters created by hypershift-operator 4.16 which won't
    have this field. Backward compatibility: must handle nil
    RegistryOverrides gracefully.

---------------------------------------------------------------------

Ready to proceed? [approve / adjust / questions]
```

Karim reads through this. The plan looks right, but something catches his eye.

```
$ rh respond "The API types from 1748 — are they actually on main? Check
  if RegistryOverrides is defined in nodepool_types.go"
```

RH-A checks the file.

```
Checked api/hypershift/v1beta1/nodepool_types.go on main.

RegistryOverrides is defined in the KubeVirtNodePoolPlatform struct at
line 847:

  // RegistryOverrides defines image registry mirror rules that should be
  // applied to the KubeVirt VirtualMachine instances created for this
  // NodePool.
  // +optional
  RegistryOverrides []RegistryOverride `json:"registryOverrides,omitempty"`

The type exists. Proceeding with the plan as written.
```

Karim is satisfied.

```
$ rh respond "approve"
```

---

## Tuesday Morning — Design and Implementation

Karim opens his laptop, runs `rh status`, and sees where things stand.

```
OCPSTRAT-1751: Add RegistryOverrides support to NodePool for KubeVirt
---------------------------------------------------------------------

  Phase 1 - Design    done (completed yesterday, you approved)
  Phase 2 - Implementation    in progress
  Phase 3 - Testing   pending
  Phase 4 - PR        pending
```

RH-O had moved to Phase 2 automatically after Karim approved the plan. RH-A is working. Karim sees a live activity view:

```
Implementation progress:

  done Read existing KubeVirt reconciliation code
  done Read AWS implementation of RegistryOverrides for reference pattern
  done Identified injection point in kubevirt.go ReconcileNodePool
  >    Writing reconciliation logic for KubeVirt RegistryOverrides
  -    Running 'make api' (if API touched)
  -    Running 'make verify'
  -    Writing unit tests
```

Karim lets it work. He reads some documentation about the KubeVirt provider, reviews a teammate's PR, drinks another coffee. About twenty minutes later, his terminal shows:

```
! ESCALATION — Need your input

I'm implementing the RegistryOverrides reconciliation for KubeVirt and
I've hit a design question.

The AWS implementation (OCPSTRAT-1749) injects registry overrides into
the ignition config, which gets consumed by the machine's container
runtime on boot. This works because AWS nodes boot from an AMI and run
ignition.

KubeVirt nodes are different. They boot from a container disk image inside
a VirtualMachine CR. There are two approaches:

  Option A: Inject overrides into the ignition config embedded in the
  VirtualMachine's cloudInit section. This follows the AWS pattern closely.
  The data flows: HostedCluster spec -> CPO reconciler -> KubeVirt VM
  cloudInit -> guest node container runtime config.

  Option B: Mount a ConfigMap containing the overrides into the
  VirtualMachine as an extra disk, and have the node's MachineConfigDaemon
  pick it up. This avoids modifying cloudInit but adds a dependency on MCD
  being healthy.

The knowledge base notes that "changes to anything running on the data
plane should not trigger a lifecycle action on components running
management side." Option A has the overrides baked into the VM spec at
creation time (management-side only). Option B requires MCD on the data
plane to process the ConfigMap, which means a failure on the data plane
could surface as a management-side reconciliation error if we're watching
for completion.

I recommend Option A. It keeps the data flow unidirectional and doesn't
create a feedback loop from the data plane back to the management
reconciler.

Which approach? [A / B / discuss]
```

Karim types:

```
$ rh respond "A. Good catch on the feedback loop. Go with ignition injection."
```

RH-A resumes. Twenty-five minutes later:

```
Implementation complete.

Files changed:
  M  control-plane-operator/controllers/hostedcontrolplane/kubevirt/kubevirt.go
  A  control-plane-operator/controllers/hostedcontrolplane/kubevirt/registry_overrides.go
  M  control-plane-operator/controllers/hostedcontrolplane/kubevirt/kubevirt_test.go

Summary:
  * Added registryOverridesToIgnitionSnippet() that converts RegistryOverride
    entries into container runtime mirror configuration fragments.
  * Called from ReconcileNodePool when len(nodePool.Spec.Platform.KubeVirt.
    RegistryOverrides) > 0.
  * Nil/empty RegistryOverrides is a no-op (backward compatible with older
    hypershift-operator versions that don't set this field).
  * Unit tests cover: empty overrides, single override, multiple overrides,
    override with multiple mirrors, malformed registry reference.

'make verify' passed.
'make test' passed (unit tests, 42s).

No API types were modified, so 'make api' and 'make clients' were not needed.

Ready for Phase 3 — Testing, or would you like to review the code first?
```

### Karim Reviews the Code

Karim does not need to read every line. He asks:

```
$ rh respond "What test cases did you write?"
```

```
Unit tests in kubevirt_test.go:

  TestRegistryOverridesToIgnitionSnippet
    |- empty overrides -> returns empty string, no error
    |- nil overrides -> returns empty string, no error
    |- single override with one mirror -> correct JSON output
    |- single override with multiple mirrors -> correct JSON output
    |- multiple overrides -> all present in output
    |- override with empty Source -> skipped silently
    |- override with empty Mirrors slice -> included with empty endpoints

  TestReconcileNodePoolWithRegistryOverrides
    |- nodepool with no overrides -> VM spec unchanged (regression)
    |- nodepool with overrides -> ignition snippet present in cloudInit
    |- nodepool with overrides, existing cloudInit data -> merged correctly

All passing.
```

That is enough. Karim can see the test coverage without reading the assertions.

```
$ rh respond "looks good, proceed to PR"
```

---

## Tuesday Afternoon — The PR

RH-O creates the branch, pushes, and opens the PR. What Karim sees:

```
PR created: https://github.com/openshift/hypershift/pull/4821

Title: OCPSTRAT-1751: Add RegistryOverrides reconciliation for KubeVirt NodePool
```

The PR body:

```markdown
## What
Implements registry override propagation for KubeVirt-based NodePools.
Registry mirror configuration from HostedCluster.Spec is converted to
CRI-O mirror config and injected into the KubeVirt VM's ignition at
creation time.

## Why
Part of the registry mirroring feature (OCPSTRAT-1200). AWS support
landed in OCPSTRAT-1749. This adds KubeVirt support using the same
pattern adapted for the KubeVirt boot mechanism.

## How
- Pure transformation function converts RegistryOverride API types to
  ignition-compatible JSON
- Injected during CPO reconciliation (management side only)
- Unidirectional: baked into VM spec at creation, no data plane callback
- Nil-safe: no-op when RegistryOverrides is empty (backward compat with
  older hypershift-operator versions)

## Testing
- Unit tests for the conversion function (7 cases)
- Unit tests for reconciliation integration (3 cases)
- 'make verify' passes
- E2e coverage tracked in OCPSTRAT-1752

## Platform isolation
- Changes are entirely within the kubevirt/ package
- AWS and Azure reconciliation paths are untouched
- Verified: 'make test' for AWS and Azure packages pass

Jira: https://issues.redhat.com/browse/OCPSTRAT-1751
```

The PR body was written by RH-A but reviewed by RH-O against the team's PR template conventions before posting. Karim did not write a single line of the description. He scans it. It is accurate.

---

## Wednesday Morning — CI

CI runs overnight. Wednesday morning, Karim checks.

```
$ rh status
```

```
OCPSTRAT-1751 — PR #4821

CI Results:
  done unit tests (3m)
  done verify / lint / vet (2m)
  done go build (1m)
  FAIL e2e-kubevirt (74m)
      TestNodePoolKubeVirt/TestRegistryMirrorConfig:
        timeout waiting for VirtualMachine to reach Running phase
  done e2e-aws (68m)
  done e2e-azure (71m)

RH-O Analysis:
  The failing test is NOT one of ours — it's a pre-existing e2e test for
  KubeVirt registry config that runs in the same suite. Our changes did
  not modify this test.

  Checked: this test has failed 3 times in the last 7 days on unrelated
  PRs. It appears to be a flaky test related to KubeVirt VM boot timing
  in the CI environment.

  Recommendation: /retest the KubeVirt e2e job. If it fails again on the
  same test, it's a pre-existing issue. Our unit tests and the AWS/Azure
  e2e suites all passed cleanly.

  [retest / investigate / wait]
```

RH-O did not just say "test failed." It checked the test history across other PRs, determined the failure is not related to Karim's changes, and gave a concrete recommendation. This is the kind of thing that would take a new developer thirty minutes of clicking through Prow logs and asking teammates "is this test flaky?"

```
$ rh respond "retest"
```

RH-O triggers the retest. An hour and a half later, Karim gets a notification:

```
PR #4821 — e2e-kubevirt passed on retry. All CI green.
```

### Review Feedback

Sara reviews the PR, leaves one comment about adding a code comment explaining why empty Source entries are skipped silently. RH-O picks it up.

```
Review feedback on PR #4821:
  Sara commented on registry_overrides.go line 31:
    "Can you add a comment explaining why we skip these silently
     instead of returning an error? Future readers will wonder."

  This is a style/documentation request. I can handle it.
  [auto-fix / let me handle it / discuss]
```

```
$ rh respond "auto-fix"
```

RH-A adds the comment, pushes:

```
Pushed fixup commit: "Add code comment explaining silent skip of
malformed RegistryOverride entries"

Comment added at line 31:
  // Skip entries with empty Source. This can happen when a
  // HostedCluster created by an older hypershift-operator version
  // (pre-4.17) serializes a zero-value RegistryOverride. We skip
  // silently rather than erroring because this is a valid state
  // during version skew between the hypershift-operator and CPO.
```

Sara approves. The PR merges.

---

## Wednesday Afternoon — Knowledge Capture

During the work on OCPSTRAT-1751, Karim learned something. When KubeVirt VMs boot with custom ignition snippets, the container disk image caching behavior changes. If the ignition modifies the container runtime config, the cached container disk gets invalidated on the next boot cycle. This is not documented anywhere. He found it by reading the KubeVirt source code while RH-A was implementing.

He captures it:

```
$ rh learn "KubeVirt image caching: When ignition snippets modify CRI-O
  config (like registry mirrors), the KubeVirt DataVolume's container disk
  cache gets invalidated on next VM restart. This means VMs with registry
  overrides will have a slower first restart (~2-3 min extra) compared to
  vanilla VMs. This is a KubeVirt behavior, not ours, but it affects
  NodePool rollout timing estimates. Discovered while working OCPSTRAT-1751."
```

RH-O responds:

```
Captured. I'll add this to the knowledge base.

  File: .devagent/knowledge/kubevirt-specifics.md
  Section: Image Caching Behavior

  Also tagged for: nodepool-lifecycle.md (cross-reference)

  This will be available to all team members on next sync.
  Want me to submit a PR for this knowledge update? [yes / no]
```

```
$ rh respond "yes"
```

A small PR is created that adds this note to the knowledge base. It gets merged by any team member during their normal review cycle. It is now part of the team's institutional memory.

---

## Thursday — The Knowledge Flywheel in Action

Toni, Karim's teammate, starts working on OCPSTRAT-1752, the e2e test for KubeVirt registry overrides. She types:

```
$ rh start OCPSTRAT-1752
```

RH-O presents her plan. Inside the plan, without Toni asking for it, she sees:

```
Relevant team knowledge:
  * KubeVirt image caching: When ignition snippets modify CRI-O config
    (like registry mirrors), container disk cache gets invalidated on
    next VM restart. VMs with registry overrides will have ~2-3 min
    slower first restart. (Source: Karim, OCPSTRAT-1751)

    -> This affects the e2e test timeout. The test should allow extra
       time for VM boot when registry overrides are configured.
       Recommend setting the VM ready timeout to 10 minutes instead of
       the default 5 minutes for this specific test case.
```

Toni did not ask Karim. She did not search Slack. She did not grep through old PRs. The knowledge was there, in context, when she needed it.

---

## Thursday Afternoon — Unattended Mode

Karim has a second story assigned: OCPSTRAT-1760, "Add HyperShift team label to the KubeVirt CPO controller." This is a small, mechanical change. Add a label to a Deployment manifest. Karim has done stories like this in his previous job. He trusts the agent.

```
$ rh start OCPSTRAT-1760 --unattended
```

RH-O acknowledges:

```
Starting OCPSTRAT-1760 in unattended mode.

  Scope: Add team label to KubeVirt CPO controller Deployment
  Estimated complexity: Low

  I'll plan, implement, test, and open the PR. You'll get a
  notification when the PR is ready for your review, or immediately
  if I hit something unexpected.

  Working...
```

Karim closes that terminal and works on something else. Eighteen minutes later, a desktop notification appears:

```
OCPSTRAT-1760 complete — PR #4829 ready for review
```

He clicks through to the PR. It is exactly what he expected. One file changed, a label added to the Deployment spec in the right place, unit test updated to assert the label exists, `make verify` passed. He glances at the diff, approves it himself since it is trivial, and moves on.

The PR diff is four lines. The agent did not over-engineer it. It did not refactor surrounding code. It did not add "improvements" Karim did not ask for. It did exactly what the story said.

---

## Friday — Multi-Story Progress

Karim's feature, the full registry mirroring epic for KubeVirt, has five stories. Two are done. The remaining three have dependencies:

- OCPSTRAT-1752 — E2e test (Toni is doing this)
- OCPSTRAT-1753 — Documentation update (depends on 1752 merging, because docs reference test results)
- OCPSTRAT-1754 — Release note and feature gate graduation (depends on 1753)

Karim runs:

```
$ rh epic OCPSTRAT-1700
```

He sees:

```
Epic: OCPSTRAT-1700 — KubeVirt Registry Overrides

  done OCPSTRAT-1748  API types                        merged (PR #4790)
  done OCPSTRAT-1749  CPO reconciliation (AWS)          merged (PR #4801)
  done OCPSTRAT-1750  E2e test (AWS)                    merged (PR #4805)
  done OCPSTRAT-1751  CPO reconciliation (KubeVirt)     merged (PR #4821)
  >    OCPSTRAT-1752  E2e test (KubeVirt)               in progress (Toni, PR #4835)
  -    OCPSTRAT-1753  Documentation update              ready, blocked on 1752
  -    OCPSTRAT-1754  Feature gate graduation           blocked on 1753

  Parallel work possible: None right now. 1753 and 1754 are serial.

  Toni's PR #4835 has CI running. If it merges today, I can start 1753
  automatically. Want me to queue it? [yes / no / assign to someone]
```

```
$ rh respond "yes, queue 1753 for me, unattended when 1752 merges"
```

```
Queued. When OCPSTRAT-1752 (PR #4835) merges, I'll automatically start
OCPSTRAT-1753 in unattended mode and notify you when the PR is ready.
```

Karim closes his laptop Friday at 5:30pm. For once.

---

## Monday Morning, Week Two

Karim opens his laptop. He types:

```
$ rh status
```

```
Good morning.

Over the weekend:
  * OCPSTRAT-1752 (Toni's e2e test) merged Friday evening.
  * OCPSTRAT-1753 (documentation) was started automatically.
    PR #4841 is open and CI has passed. Ready for your review.
  * OCPSTRAT-1754 remains blocked on 1753.

Your active items:
  * PR #4841 — review and approve (OCPSTRAT-1753)
  * OCPSTRAT-1754 — will start once 1753 merges

Team knowledge updates since Friday:
  * Toni added a note about KubeVirt e2e test timeouts:
    "TestRegistryOverridesKubeVirt uses a 10min VM ready timeout per
    Karim's image caching discovery. Standard KubeVirt tests use 5min."
  * Bryan pushed a fix to debugging-patterns.md: added a section on
    diagnosing NodePool stuck in Provisioning when the guest cluster's
    CAPI provider is crashlooping.

1 item needs your attention. Would you like to review PR #4841?
```

The work continued while Karim was away. Not because something ran unsupervised in a scary way, but because Karim explicitly queued it and the trigger condition was met. The documentation PR was mechanical: update the feature docs to reflect that KubeVirt registry overrides are now supported, reference the test results. RH-A did not need to ask any questions.

Karim reviews PR #4841. The docs are accurate. He approves it. It merges.

```
$ rh respond "start 1754"
```

The last story in the epic begins. By Tuesday, the epic is closed. Karim has been on the team for eight days.

---

## What Karim Never Had to Think About

Throughout all of this, there were things he never had to deal with.

**Generated code.** He never ran `make api` or `make clients`. When RH-A modified API types on a different story, it ran the generators automatically and included the generated files in the PR, because the constraints file `.devagent/constraints/generated-code.md` told it to.

**Cross-platform safety.** He never worried about whether his KubeVirt change broke the AWS provider. RH-A ran the AWS unit tests and checked that the AWS e2e suite passed in CI, because `.devagent/constraints/platform-isolation.md` told it to verify cross-platform impact.

**Backward compatibility.** He never had to remember that the CPO might be running a different version than the hypershift-operator. The backward compatibility check was in the plan from the start, because `.devagent/constraints/backward-compat.md` describes the version skew between HO and CPO and says "all new fields must handle nil gracefully."

**Pattern discovery.** He never had to ask "what pattern should I follow?" RH-A read the existing AWS implementation, used it as a reference, and adapted it for KubeVirt. Not because someone told it to, but because the workflow definition in `.devagent/workflows/implement.yaml` says: "for platform-specific features, find the existing implementation on another platform and use it as the pattern reference."

**Architecture rules.** He never had to explain the split-brain architecture to the agent. When RH-A escalated the Option A / Option B question about registry overrides, it already understood that data should flow from management cluster to data plane and not the other way around. That understanding came from `.devagent/knowledge/split-brain.md`, which every team member has contributed to over time.

**cpov2 framework rules.** He never had to tell the agent not to set labels in adapt functions, or not to modify container images directly, or not to duplicate what `setDefaultOptions()` does. The constraints file `.devagent/constraints/adapt-function-rules.md` encodes all of this.

---

## What This Actually Is

The knowledge base is not static. When Bryan pushed that NodePool debugging pattern fix on Friday, it was automatically available to every developer on the team the next time they synced. When Karim captured the KubeVirt caching behavior, it helped Toni the next day. The knowledge compounds. Every story makes the next story easier. Not just for the developer who did the work, but for everyone on the team.

That is the experience. Not an AI that writes code for you. A system that knows what your team knows, follows the rules your team has agreed on, does the mechanical work your team should not have to do, and asks you the questions that actually require your judgment.

Karim wrote zero lines of Go in his first week. He made four engineering decisions that mattered. The code shipped.

---

## What the POC Must Validate

Both POCs (Ralph Orchestrator vs Claude Code native) should deliver this experience. The evaluation questions:

1. **Can it do `rh start OCPSTRAT-XXXX`?** — Fetch context, present a plan, let the developer approve/adjust.
2. **Can it escalate cleanly?** — The Option A / Option B moment. Not a wall of text, not a silent failure. A clear question with context.
3. **Can it run unattended?** — `--unattended` mode for simple stories. Notification when done.
4. **Can it queue work?** — "Start 1753 when 1752 merges." Event-driven, not polling.
5. **Can it share knowledge via git?** — `rh learn` -> commit -> teammate benefits on next pull.
6. **Can it triage CI?** — Not just "test failed" but "this test has failed 3 times this week on unrelated PRs, likely flaky."
7. **Does it stay scoped?** — Four-line PR for a four-line story. No over-engineering.

## Open Questions

1. **Sandboxing**: How does the agent execute safely? The developer should never think about this.
2. **Session persistence**: Karim closed his laptop and came back. How does RH-O know where to resume?
3. **Story hand-off format**: What artifact moves from "plan approved" to "design this"?
4. **Multi-story parallelism**: Two independent stories on separate branches. Is this swarm (no) or just two sequential pipelines?
5. **JIRA integration**: Output-only or does RH-O create/update JIRA issues directly?
