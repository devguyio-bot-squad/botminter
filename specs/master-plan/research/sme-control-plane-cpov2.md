# Dream-World UX Vision — Control Plane SME Perspective

> Independent vision from the Control Plane SME agent. No cross-pollination with other SME outputs.
> Part of [Shift Week Plan](shift-week-plan.md), input to [Living Codebase Vision](ux-living-codebase-vision.md)

---

## Preamble: Why Everything Before This Has Been Wrong

Every developer tool built in the last decade has made the same mistake: it treats the developer as a typist who needs faster typing. Copilot autocompletes lines. Chat agents answer questions. CI systems report pass/fail. They are all incremental improvements on a workflow designed in 1995: edit, compile, test, commit, review, merge.

HyperShift is not a 1995 codebase. It is a distributed control system that manages other distributed control systems across multiple cloud providers, with version skew, split-brain architecture, and namespace isolation constraints that most developers never encounter in their careers. The cognitive load is not in typing code. It is in holding the entire system model in your head while making a change that touches reconciliation logic in one controller, affects component lifecycle in another, and must not violate purity contracts in a third.

The dream-world system does not help you type faster. It holds the system model FOR you, reasons about it WITH you, and acts on it WHEN you are ready.

I am calling this system **The Forge**, because the metaphor is deliberate: raw material (requirements) goes in, finished artifacts (merged, tested, deployed changes) come out, and the smith (developer) controls the heat, the shape, and the timing, but does not need to mine the ore or hand-crank the bellows.

---

## Part I: The Fundamental Paradigm Shift

### From "Files and Diffs" to "Intent and Contracts"

The current developer workflow is file-centric. You open files, you change lines, you create diffs, you review diffs. This is absurd for control plane work. When someone says "add Prometheus support as a CPOv2 component," the actual intent is:

- A new `controlPlaneComponent` must exist in the HCP reconciliation graph
- It must have an adapt function that is a pure transformation
- It needs deployment, service, service monitor manifests
- It must respect the component's resource request/limit contract
- It must handle the platform-specific variations (some platforms may not need it)
- It must be wired into the HCP controller's component list
- Its RBAC must be scoped to the hosted control plane namespace, not the management cluster
- It must handle version skew: what if the CPO is v4.17 and the hypershift-operator is v4.18?

None of these are "files." They are **contracts**. The Forge operates at the contract level. Files are an implementation detail that the agent handles.

### From "Review My Code" to "Verify My Contracts"

Code review in HyperShift today is a nightmare. A senior engineer must mentally simulate the reconciliation loop, check that adapt functions have no side effects, verify namespace isolation, confirm RBAC scoping, and validate platform-specific behavior, all by reading Go code in a GitHub diff view. This is like reviewing a bridge design by reading individual bolt specifications.

The Forge does not ask you to review code. It asks you to verify contracts. "This component will request 100m CPU and 128Mi memory. It will mount these secrets. It will be reconciled in this order relative to these other components. Its adapt function touches these fields and nothing else. Here is the proof."

### From "CI Passed" to "Contract Satisfaction Evidence"

A green CI badge means almost nothing for control plane changes. It means the specific test that ran on the specific platform with the specific version combination happened to pass. It does not tell you whether the component respects namespace isolation under version skew, whether the adapt function is actually pure, or whether the resource footprint is acceptable.

The Forge produces **evidence bundles**: structured proof that each contract is satisfied, with the specific test runs, static analysis results, and invariant checks that demonstrate satisfaction. A reviewer does not ask "did CI pass?" They ask "show me the purity evidence for the adapt function" and get a formal answer.

---

## Part II: The Developer's Day

### 07:45 - Morning Sync (Mobile)

Ahmed opens his phone. The Forge's mobile surface (a progressive web app, not a native app, because this is not a product, it is a developer's interface to their own work) shows:

```
Assalamu alaikum, Ahmed.

OVERNIGHT:
- PR #4521 (KAS audit logging component): CI passed on AWS, Azure.
  KubeVirt run had a known flake (etcd timeout, tracked in OCPBUGS-41233).
  I re-triggered. Currently running.
- PR #4519 (OAuth server CPOv2 migration): Toni left 2 comments.
  Both are about the adapt function. I've drafted responses with
  contract evidence. Ready for your review.

TODAY'S FORGE:
- Story HOSTEDCP-1234: "Add cluster-policy-controller as CPOv2 component"
  Status: Spec approved. Implementation ready to begin.
  Estimated contracts: 6 (deployment, RBAC, adapt purity, resource
  budget, platform matrix, version skew handling)
  Shall I begin implementation while you review Toni's comments?

- Story HOSTEDCP-1238: "Fix KMS key rotation for AWS"
  Status: Blocked on clarification. The epic says "support rotation"
  but doesn't specify whether we re-encrypt existing secrets or only
  encrypt new ones. I've drafted a clarifying comment on the JIRA.
  Want me to post it?
```

Ahmed taps "Begin HOSTEDCP-1234" and "Post the clarification." He puts his phone down and goes to make coffee. The Forge is now working. Not in a background terminal. Not in a CI job. It is an autonomous entity that understands what "add cluster-policy-controller as CPOv2 component" means at the contract level, and is producing an implementation.

### 08:30 - Deep Work Begins (IDE)

Ahmed sits down at his IDE. The Forge is not a panel in the IDE. It is not a chat window. It is a **layer on top of the IDE** that transforms how the IDE presents information.

When Ahmed opens the HCP controller file, he does not see 3000 lines of Go code. He sees:

```
HOSTED CONTROL PLANE RECONCILIATION GRAPH

[etcd] → [kube-apiserver] → [kube-controller-manager]
                           → [kube-scheduler]
                           → [openshift-apiserver]
                           → [openshift-controller-manager]
                           → [oauth-openshift] (CPOv2 migration in PR #4519)
                           → [cluster-policy-controller] ← YOU ARE HERE (HOSTEDCP-1234)

Component: cluster-policy-controller
Status: FORGING (implementation in progress)
Contracts: 2/6 satisfied
  ✓ Deployment manifest generated
  ✓ RBAC scoped to HCP namespace
  ◐ Adapt function (generating, 40% complete)
  ○ Resource budget
  ○ Platform matrix
  ○ Version skew handling
```

This is the **topology view**. It shows the control plane as a graph, not as files. Ahmed can see where his new component fits, what it depends on, what depends on it, and the current state of the Forge's work on it.

He clicks on the adapt function contract. The Forge shows:

```
ADAPT FUNCTION: cluster-policy-controller

Contract: The adapt function must be a pure transformation.
  Input:  *appsv1.Deployment (generic) + CPOv2 workload params
  Output: *appsv1.Deployment (adapted)

  Side effects: NONE PERMITTED
  API calls: NONE PERMITTED
  File I/O: NONE PERMITTED

Current implementation (draft):

  func adaptDeployment(cpContext WorkloadContext,
      deployment *appsv1.Deployment) error {

      // Set replicas based on availability configuration
      util.SetReplicas(cpContext, deployment)

      // Configure container args from HCP spec
      args := []string{
          "--namespace=$(NAMESPACE)",
          "--config=/etc/kubernetes/config/config.yaml",
      }
      util.SetContainerArgs(deployment, containerMain, args)

      // Mount required volumes
      util.SetVolumeMounts(deployment, containerMain, volumeMounts()...)

      return nil
  }

Purity analysis:
  ✓ No network calls detected
  ✓ No file system access detected
  ✓ No global state mutation detected
  ✓ All modifications are on the input deployment object
  ⚠ WARNING: util.SetReplicas reads from cpContext.HCP.Spec -
    this is read-only access to the HCP spec, which is permitted.
    But verify: is there any platform-specific branching needed here?

  The existing pattern (e.g., kube-controller-manager) shows that
  platform-specific replica counts are handled via the availability
  model, not in the adapt function. Conforming to that pattern.
```

Ahmed has not opened a single file. He has not read a single line of raw Go code. He is looking at **contracts and their satisfaction**. The Forge wrote the code. The Forge analyzed the code for purity. The Forge compared the code to existing patterns. Ahmed's job is to verify that the contracts are correct and the implementation matches his intent.

He notices something. The Forge assumed the cluster-policy-controller uses a config file mounted from a ConfigMap. But actually, in the version of OCP they are targeting, this component switched to using flags only. He speaks (yes, speaks, because the Forge supports voice input in the IDE, and for a quick correction this is faster than typing):

"The cluster-policy-controller in 4.18 dropped the config file. It takes all configuration through command-line flags. Check the 4.18 release notes or the upstream repo."

The Forge responds in the topology view:

```
CORRECTION ACKNOWLEDGED

Checking openshift/cluster-policy-controller@release-4.18...
Confirmed: config file removed in commit abc123. All configuration
via flags.

Impact:
  - Adapt function: Remove config volume mount, update args
  - Deployment manifest: Remove ConfigMap volume
  - RBAC: No change (was not reading ConfigMap via API)
  - Purity: No impact (still pure)

Updating... Done.

Contract status: 3/6 satisfied
  ✓ Deployment manifest generated (updated)
  ✓ RBAC scoped to HCP namespace
  ✓ Adapt function (updated, purity re-verified)
  ○ Resource budget
  ○ Platform matrix
  ○ Version skew handling
```

### 09:15 - The Version Skew Contract

This is where the Forge earns its keep. Version skew is the single most subtle source of bugs in HyperShift. The hypershift-operator runs at one version. The control-plane-operator runs at another. They share API types, but the types might have different fields available. A new feature added to the HostedCluster API by the hypershift-operator might not be understood by an older CPO.

The Forge does not treat this as something to "remember." It treats it as a **formal contract**.

```
VERSION SKEW CONTRACT: cluster-policy-controller

The Forge needs to determine:
1. What HCP spec fields does this component's adapt function read?
2. When were those fields introduced?
3. What is the minimum CPO version that will run this code?
4. What happens if a field is not present (zero value)?

Analysis:
  Fields read from HCP spec:
    - .spec.configuration.apiServer.audit (present since 4.14)
    - .spec.platform.type (present since 4.12)
    - .spec.controllerAvailabilityPolicy (present since 4.12)

  Minimum CPO version for this component: 4.18 (new addition)

  Version skew scenarios:
    Scenario 1: HO=4.18, CPO=4.18 → Full feature set. No issues.
    Scenario 2: HO=4.19, CPO=4.18 → HO may set new fields that
      CPO 4.18 does not read. Safe: unread fields are ignored.
    Scenario 3: HO=4.18, CPO=4.19 → CPO may expect fields that
      HO 4.18 does not set. RISK: if CPO 4.19 adds a required
      field, it will get zero value from HO 4.18.

  RECOMMENDATION: All field reads in the adapt function should
  have explicit zero-value handling. Current implementation
  already does this (SetReplicas defaults to 1 if availability
  policy is unset).

  ✓ Version skew contract: SATISFIED
```

Ahmed did not ask for this analysis. The Forge produced it because it understands, at a structural level, that version skew is a contract that must be satisfied for every control plane component. This understanding comes from the **Team Knowledge** layer.

### 10:00 - Toni's Review Comments

Ahmed switches to Toni's comments on the OAuth server PR. The Forge shows:

```
PR #4519: OAuth server CPOv2 migration

TONI'S COMMENTS (2):

1. Line 47 of adapt.go:
   "Why are we reading the OAuth config from the HCP spec here
    instead of from the OAuth CR? This seems like it could diverge."

   FORGE ANALYSIS: Toni is correct that there is a potential divergence.
   However, the CPOv2 pattern requires adapt functions to read from
   the HCP spec, not from secondary resources. The OAuth CR is
   reconciled by a separate controller. Reading from it in the adapt
   function would require an API call, violating purity.

   SUGGESTED RESPONSE:
   "Good catch on the potential divergence. In CPOv2, adapt functions
    must be pure - no API calls. The HCP spec is the single source of
    truth passed to the adapt function. The OAuth CR reconciliation
    should be updated to read FROM the HCP spec as well, keeping them
    in sync. I'll file a follow-up story for that alignment."

   Shall I post this response and create the follow-up JIRA story?

2. Line 112 of adapt.go:
   "This volume mount path doesn't match what we use in the
    non-CPOv2 version. Bug?"

   FORGE ANALYSIS: Comparing with the legacy (non-CPOv2) OAuth
   server deployment:
     Legacy path: /etc/oauth-openshift/config
     CPOv2 path:  /etc/kubernetes/oauth/config

   The CPOv2 path follows the standardized convention established
   in the CPOv2 framework README. The legacy path was
   component-specific. This is an intentional change, not a bug.

   However: if we are doing a rolling migration, both paths need
   to work during the transition period.

   SUGGESTED RESPONSE:
   "Not a bug - the new path follows the CPOv2 convention. But you
    raise a good point about the transition. During rolling updates,
    the old and new pod versions need to coexist. I'll add a symlink
    in the container image. Filing a task."

   Shall I post this response and create the task?
```

Ahmed reviews. The first response is exactly right. The second one, he adjusts: "Skip the symlink. We do a hard cutover for CPOv2 migrations, not rolling. Just explain that in the response." The Forge updates and posts both comments.

### 12:30 - The Resource Budget (After Dhuhr)

Ahmed is back from prayer and lunch. The Forge has been working.

```
PROGRESS: HOSTEDCP-1234

Contracts: 5/6 satisfied
  ✓ Deployment manifest generated
  ✓ RBAC scoped to HCP namespace
  ✓ Adapt function (purity verified)
  ✓ Platform matrix (all platforms, no platform-specific behavior)
  ✓ Version skew handling
  ◐ Resource budget (NEEDS YOUR INPUT)

RESOURCE BUDGET ANALYSIS:

The cluster-policy-controller upstream uses:
  Requests: 100m CPU, 200Mi memory
  Limits:   (none set upstream)

Current HCP resource footprint (sum of all components):
  Total requests: 2.4 CPU, 8.2Gi memory (AWS profile)

Adding this component increases total by:
  +4.2% CPU, +2.4% memory

Comparable components in the control plane:
  kube-controller-manager: 200m CPU, 400Mi memory
  openshift-controller-manager: 100m CPU, 200Mi memory
  cluster-policy-controller is lighter-weight than both.

QUESTION: The upstream does not set limits. The CPOv2 convention
is to set limits = 2x requests for CPU, 1.5x for memory.
Apply the convention?

Options:
  A) Convention: 200m CPU limit, 300Mi memory limit
  B) Match upstream: no limits
  C) Custom: specify your own
```

Ahmed says "A, use the convention." The final contract is satisfied. The Forge prepares the PR.

### 13:00 - PR Creation

```
PR READY: HOSTEDCP-1234

"Add cluster-policy-controller as CPOv2 component"

FILES CHANGED: 8
  new: control-plane-operator/controllers/hostedcontrolplane/v2/
         cluster_policy_controller/component.go
  new: control-plane-operator/controllers/hostedcontrolplane/v2/
         cluster_policy_controller/adapt.go
  new: control-plane-operator/controllers/hostedcontrolplane/v2/
         cluster_policy_controller/adapt_test.go
  mod: control-plane-operator/controllers/hostedcontrolplane/v2/
         hcp.go (component registration)
  new: control-plane-operator/controllers/hostedcontrolplane/v2/
         cluster_policy_controller/testdata/...
  mod: control-plane-operator/main.go (component wiring)

CONTRACT EVIDENCE BUNDLE:
  ✓ Purity: Static analysis report (no side effects detected)
  ✓ RBAC: Namespace-scoped roles only, no cluster-level bindings
  ✓ Resources: 100m/200Mi requests, 200m/300Mi limits
  ✓ Platform: All platforms, no platform-specific code paths
  ✓ Version skew: All HCP spec reads have zero-value defaults
  ✓ Tests: Unit tests for adapt function (12 test cases)
  ✓ Pattern conformance: Matches kube-controller-manager structure

PR DESCRIPTION (draft):
  "Adds cluster-policy-controller as a CPOv2-managed component.
   This replaces the legacy reconciliation in the monolithic HCP
   controller.

   Contract Evidence:
   [expandable sections for each contract]

   JIRA: HOSTEDCP-1234
   Depends: none
   Blocks: HOSTEDCP-1240 (remove legacy reconciliation)"

Ready to create PR? [YES / EDIT / REVIEW CODE FIRST]
```

Ahmed selects "YES." But notice the option: "REVIEW CODE FIRST." For developers who want it, they can drop into the traditional code view at any time. The Forge does not remove control. It layers abstraction on top.

### 14:00 - The New Developer (Parallel Narrative)

While Ahmed works, a new team member, Fatima, is on her second week. She has been assigned HOSTEDCP-1245: "Add cert-manager-operator as CPOv2 component." She has never written a CPOv2 component before.

She opens the Forge and says: "I need to add cert-manager-operator as a CPOv2 component. I've never done this before."

The Forge does not point her to a README. It does not say "read the CPOv2 framework documentation." It does this:

```
GUIDED FORGE: cert-manager-operator (CPOv2)

I'll walk you through this. You'll understand the pattern by the
end, and you'll have a working component.

CPOv2 components follow a strict contract. Let me show you what
that means by comparing with an existing component you can use
as reference.

REFERENCE COMPONENT: kube-controller-manager
(chosen because it's similar in complexity to cert-manager-operator)

A CPOv2 component has these parts:

1. COMPONENT DEFINITION (component.go)
   This tells the framework "what am I?"
   - Name, deployment name, namespace behavior
   - What Kubernetes resources it manages (Deployment, Service, etc.)
   - Dependencies (what must exist before this component)

2. ADAPT FUNCTION (adapt.go)
   This is the CORE. The adapt function takes a generic deployment
   and transforms it into YOUR component's deployment.

   THE CRITICAL RULE: This function must be PURE.
   - No API calls. No reading from Kubernetes. No file I/O.
   - All information comes in through the function parameters.
   - You ONLY modify the deployment object you receive.

   Think of it as: "Given this blank canvas deployment and these
   configuration parameters, paint the deployment I need."

   Why? Because the framework handles all the Kubernetes interaction.
   Your adapt function is called inside the framework's reconcile
   loop. If you make API calls here, you break caching, you break
   testability, and you introduce side effects that the framework
   cannot reason about.

3. TESTS (adapt_test.go)
   Adapt functions are trivially testable because they're pure.
   You create an input deployment, call the function, and assert
   on the output. No mocks needed. No fake clients.

Want me to generate the skeleton and walk you through each decision?
```

Fatima says yes. The Forge generates the skeleton, but unlike Ahmed's fully autonomous flow, it stops at each contract and explains the decision:

```
DECISION POINT: Namespace behavior

cert-manager-operator runs in the HCP namespace (like most
components). But cert-manager itself manages certificates across
namespaces.

Question: Does cert-manager-operator need to watch resources
outside the HCP namespace?

If YES: We need cluster-scoped RBAC, which is unusual for CPOv2
  components and requires additional review. Let me know and I'll
  flag it for Ahmed or another senior reviewer.

If NO: Standard namespace-scoped RBAC. This is the normal path.

Which is it? (It's okay to say "I don't know" - I'll help you
figure it out.)
```

This is the same system Ahmed uses, but with the autonomy dial turned down. Fatima is learning the contracts, not just producing code. By her fifth component, she will be running at Ahmed's autonomy level.

### 17:00 - CI Debugging

Ahmed's PR has been running CI for 90 minutes. The KubeVirt e2e suite failed.

In the old world, Ahmed would open the CI logs, scroll through 50,000 lines of output, find the failure, try to determine if it is a flake or a real bug, and either re-trigger or fix.

The Forge:

```
CI FAILURE: PR #4523 - KubeVirt e2e

Test: TestCreateCluster/KubeVirt/WithPolicyController
Failure: Timeout waiting for cluster-policy-controller deployment
  to become Available (300s deadline exceeded)

FORGE ANALYSIS:

This is NOT a flake. Comparing with the last 20 KubeVirt CI runs
for the base branch:
  - This test does not exist on base (new test, good)
  - Similar timeout patterns exist for other components: 0/20 runs

Root cause analysis:
  The cluster-policy-controller deployment is failing to pull its
  image. The image reference in the adapt function uses the release
  image lookup, but the KubeVirt CI environment uses a custom
  image override mechanism.

  Looking at how other components handle this...
  Found: kube-controller-manager uses util.GetImageFromReleaseImage()
  which respects the image override annotation.

  Your adapt function uses a hardcoded image reference.

FIX:
  In adapt.go, replace:
    container.Image = "registry.ci.openshift.org/..."
  With:
    util.SetContainerImage(cpContext, deployment, containerMain)

  This uses the CPOv2 image resolution, which respects overrides.

Shall I push the fix and re-trigger CI?
```

Ahmed says yes. The Forge pushes a fixup commit and re-triggers. Ahmed has spent 45 seconds on a problem that would have taken 30 minutes.

---

## Part III: The Architecture of Knowledge

### The Three Layers

```
┌─────────────────────────────────────────────────────┐
│                   PERSONAL LAYER                     │
│  Ahmed's preferences, history, autonomy settings,    │
│  velocity patterns, review style                     │
│  (stored in ~/.forge/profile, git-ignored)            │
├─────────────────────────────────────────────────────┤
│                    TEAM LAYER                        │
│  Contracts, patterns, known flakes, component        │
│  reference implementations, review checklists,       │
│  JIRA conventions, CI quirks                         │
│  (stored in .forge/ in the repo, git-tracked)        │
├─────────────────────────────────────────────────────┤
│                   DOMAIN LAYER                       │
│  CPOv2 framework semantics, Kubernetes API           │
│  conventions, OpenShift release process,             │
│  HyperShift architecture invariants                  │
│  (stored in .forge/domain/, git-tracked,             │
│   versioned with the code)                           │
└─────────────────────────────────────────────────────┘
```

### Team Knowledge Compounding

This is the most important architectural decision. When Ahmed discovers that the KubeVirt image override mechanism requires `util.SetContainerImage`, the Forge does not just fix his code. It creates a **team knowledge entry**:

```yaml
# .forge/team/patterns/image-resolution.yaml
pattern: container-image-resolution
learned_from: PR #4523, CI failure analysis
applies_to: all CPOv2 adapt functions
rule: |
  Never hardcode container image references in adapt functions.
  Always use util.SetContainerImage(cpContext, deployment, containerName)
  which respects:
    - Release image lookups
    - CI image overrides
    - Custom image overrides via HCP annotations
severity: error  # Block PR if violated
evidence:
  - type: ci_failure
    pr: 4523
    test: TestCreateCluster/KubeVirt/WithPolicyController
```

When Fatima writes her cert-manager-operator component next week, the Forge will not let her make the same mistake. Not as a review comment. Not as a CI failure. It will not generate the code with a hardcoded image in the first place, because it knows the pattern.

This is how team knowledge compounds. Every bug found, every review comment, every CI failure becomes a pattern that prevents the same class of error for everyone.

### Contract Definitions as Living Documents

The contracts are not static configuration files. They evolve:

```yaml
# .forge/domain/contracts/adapt-function-purity.yaml
contract: adapt-function-purity
version: 3  # Updated when new edge cases are discovered
description: |
  Adapt functions in CPOv2 components must be pure transformations.

invariants:
  - name: no-api-calls
    check: static-analysis
    description: "No Kubernetes client calls"

  - name: no-file-io
    check: static-analysis
    description: "No file system operations"

  - name: no-global-state
    check: static-analysis
    description: "No reading or writing package-level variables"

  - name: deterministic
    check: property-test
    description: "Same inputs always produce same outputs"
    # Added in v2 after a bug where time.Now() was used in an adapt function

  - name: no-platform-branching-on-cloud-credentials
    check: ast-analysis
    description: |
      Adapt functions must not branch on cloud credentials.
      Platform-specific behavior should be in the component definition,
      not in the adapt function.
    # Added in v3 after PR #4102 introduced an AWS-specific code path
    # in an adapt function that broke Azure.

known_exceptions:
  - component: kube-apiserver
    invariant: deterministic
    reason: "KAS adapt function uses a hash of the serving cert for annotation.
             This is deterministic for a given input but appears non-deterministic
             to naive static analysis. Exempted with proof."
```

### The Version Skew Knowledge Graph

This is where the Forge's understanding of HyperShift's architecture becomes genuinely powerful. The Forge maintains a **version skew compatibility graph**:

```
VERSION SKEW GRAPH (auto-generated, validated per release)

HCP Spec Fields:
  .spec.platform.type
    Introduced: 4.12
    Read by: ALL adapt functions
    Zero-value behavior: "" → defaults to None platform (safe)

  .spec.platform.aws.kmsKeyARN
    Introduced: 4.14
    Read by: kube-apiserver adapt, etcd adapt
    Zero-value behavior: "" → no encryption configured (safe)

  .spec.platform.aws.kmsKeyARN.rotationPolicy   ← NEW in 4.18
    Introduced: 4.18
    Read by: kube-apiserver adapt (4.18+)
    Zero-value behavior: "" → no rotation (safe)
    SKEW RISK: If HO=4.18 sets this field and CPO=4.17 processes
    the HCP, CPO 4.17 will ignore the field. Customer expects
    rotation but it won't happen.
    MITIGATION: HO 4.18 should set a condition on the HostedCluster
    if the CPO version is < 4.18 and rotation is requested.
```

This graph is not manually maintained. The Forge builds it by analyzing API type changes across releases, tracking which adapt functions read which fields, and computing the skew implications automatically. When a developer adds a new field to the HCP spec, the Forge immediately computes the skew impact and requires explicit handling.

---

## Part IV: The Interaction Surfaces

### The IDE Layer (Primary Surface for Implementation)

Not a sidebar. Not a chat panel. A **semantic layer** over the IDE.

- **Topology View**: The reconciliation graph, always visible. Components are nodes. Dependencies are edges. Your current work is highlighted. Contract satisfaction is color-coded.
- **Contract Inspector**: Click any component to see its contracts, their satisfaction status, and the evidence. Drill into any contract to see the code, the tests, and the analysis.
- **Voice Input**: For corrections, decisions, and quick questions. The Forge is always listening (locally, no cloud, voice model runs on your machine). "Change the replicas to 3 for single-availability mode" is faster than finding the line and typing.
- **Ambient Context**: The Forge shows relevant information without being asked. When you are looking at an adapt function, it shows the purity analysis. When you are looking at RBAC, it shows the namespace scope. When you are looking at a test, it shows the last 10 CI results for that test.

### The Mobile Surface (Async Management)

Progressive web app. Three functions:

1. **Status**: What is the Forge doing? What needs my attention?
2. **Decisions**: Queue of decisions the Forge cannot make alone (resource budgets, architectural choices, clarification questions)
3. **Approvals**: PRs ready for final approval, with contract evidence summaries

Ahmed checks this before bed. "The KubeVirt CI run passed. PR #4523 is green on all platforms. All contracts satisfied. Ready for review assignment." He taps "Assign reviewers" and goes to sleep.

### The Team Dashboard (Browser)

A shared view of the team's Forge activity. Not a JIRA board. Not a GitHub project board. A **contract satisfaction dashboard**.

```
CONTROL PLANE COMPONENT STATUS

                    Purity  RBAC  Resources  Skew  Tests  CI
etcd                 ✓       ✓      ✓         ✓     ✓     ✓
kube-apiserver       ✓       ✓      ✓         ⚠     ✓     ✓
kube-controller-mgr  ✓       ✓      ✓         ✓     ✓     ✓
kube-scheduler       ✓       ✓      ✓         ✓     ✓     ✓
openshift-apiserver  ✓       ✓      ✓         ✓     ✓     ✓
oauth-server         ◐       ✓      ✓         ✓     ◐     ◐  ← CPOv2 migration
cluster-policy-ctrl  ✓       ✓      ✓         ✓     ✓     ◐  ← New, CI running
cert-manager-op      ○       ○      ○         ○     ○     ○  ← Fatima, in progress

⚠ kube-apiserver skew: KMS rotation field has unmitigated skew
  risk for HO=4.18/CPO=4.17. Story filed: HOSTEDCP-1250.
```

Every team member sees the same picture. Knowledge compounds visibly.

---

## Part V: The Autonomy Spectrum

The Forge has a single, fundamental configuration concept: **autonomy level per phase**. Not per tool. Not per command. Per phase of work.

```yaml
# ~/.forge/profile/autonomy.yaml (personal, not shared)
phases:
  planning:
    breakdown: assisted      # Forge proposes, I approve
    spec_writing: autonomous  # Forge writes specs, I review async

  implementation:
    code_generation: autonomous  # Forge writes code, I verify contracts
    adapt_functions: supervised  # Forge proposes, I approve before commit
    rbac_changes: supervised     # Always review RBAC
    api_changes: manual          # I write API changes, Forge validates

  testing:
    unit_tests: autonomous      # Forge writes and runs
    e2e_tests: assisted         # Forge proposes, I review
    ci_debugging: autonomous    # Forge fixes and re-triggers

  review:
    comment_response: assisted  # Forge drafts, I approve
    review_assignment: autonomous

  release:
    cherry_picks: supervised    # Forge proposes, I approve
    release_notes: autonomous
```

Fatima's profile has more phases set to `supervised` or `assisted`. As she gains experience and the Forge's team knowledge validates her patterns, she can increase autonomy. The Forge might even suggest it: "You've completed 5 CPOv2 components with zero contract violations. Consider setting adapt_functions to autonomous?"

---

## Part VI: The Planning Pipeline (RFE to Merged PR)

### Level 0: RFE (Requirement for Enhancement)

Human-driven. The Forge is a research assistant here.

```
RFE: "Support customer-managed encryption keys for etcd in
      hosted control planes on Azure"

FORGE RESEARCH:
  - Azure Key Vault integration patterns (3 relevant Azure docs)
  - Existing AWS KMS implementation in HyperShift (reference code)
  - OCP encryption-at-rest architecture (reference design doc)
  - Impact on control plane resource footprint: minimal (config only)
  - Impact on API: New fields in HCP spec under .spec.platform.azure
  - Version skew considerations: Field must be optional, zero-value =
    no encryption (backward compatible)
  - Estimated scope: 1 Feature, 2-3 Epics, 6-10 Stories
```

### Level 1: Feature Breakdown

```
FEATURE: Azure customer-managed encryption for etcd

FORGE PROPOSED BREAKDOWN:

Epic 1: API and Validation
  Story 1.1: Add Azure Key Vault fields to HCP spec
  Story 1.2: Add webhook validation for Key Vault URI format
  Story 1.3: Add e2e test for API validation

Epic 2: Control Plane Implementation
  Story 2.1: Update etcd CPOv2 adapt function for Azure encryption
  Story 2.2: Add Azure Key Vault credential mounting
  Story 2.3: Update KAS adapt function to pass encryption config
  Story 2.4: Version skew handling (HO knows Azure KV, old CPO doesn't)

Epic 3: Lifecycle and Operations
  Story 3.1: Key rotation support
  Story 3.2: Monitoring and alerting for key access failures
  Story 3.3: Documentation

DEPENDENCY GRAPH:
  1.1 → 1.2 → 2.1 → 2.2 → 2.3
                          → 2.4
  (Epic 3 can proceed in parallel after 2.1)

Shall I create these in JIRA? (Estimated: 2 sprints for Epic 1+2,
1 sprint for Epic 3, parallelizable)
```

### Level 2: Story Specification

For each story, the Forge produces a specification before writing any code:

```
STORY SPEC: HOSTEDCP-1260
"Update etcd CPOv2 adapt function for Azure encryption"

INPUTS:
  - HCP spec field: .spec.platform.azure.etcdEncryption.keyVaultURI
  - HCP spec field: .spec.platform.azure.etcdEncryption.keyName
  - Secret: azure-etcd-encryption-key (mounted by framework)

CONTRACTS:
  1. Adapt function purity: All encryption config comes from HCP spec
     and mounted secrets. No Azure API calls in the adapt function.
  2. Platform isolation: Encryption config only applied when
     platform.type == Azure. No impact on other platforms.
  3. Version skew: Fields are optional. Zero-value = no encryption.
     Existing clusters upgrading from pre-encryption CPO will
     continue to work without encryption.
  4. Resource impact: None. Configuration-only change.
  5. RBAC impact: None. Secret is mounted by the framework.

ACCEPTANCE CRITERIA:
  - etcd starts with encryption enabled when Key Vault fields are set
  - etcd starts normally when Key Vault fields are not set
  - Adapt function passes purity analysis
  - Unit tests cover: encryption enabled, encryption disabled,
    partial config (URI set but key name missing → error)

REFERENCE IMPLEMENTATION:
  AWS KMS encryption in etcd adapt function (link to code)

ESTIMATED EFFORT: 1 developer-day
```

This specification is the **input to the implementation phase**. The Forge generates code that satisfies the specification. The developer verifies that the specification is correct (this is the human judgment) and that the implementation satisfies it (this is contract verification).

---

## Part VII: Confidence Without Line-by-Line Review

This is the hardest problem. How does a reviewer trust agentic code without reading every line?

### The Evidence Bundle

Every PR created by the Forge includes an evidence bundle. Not in the PR description (that is for humans to skim). In a structured, machine-readable format that the Forge can present in the reviewer's IDE.

```
EVIDENCE BUNDLE: PR #4523

1. CONTRACT SATISFACTION
   All 6 contracts satisfied. (Details expandable per contract.)

2. PATTERN CONFORMANCE
   This component structurally matches:
     - kube-controller-manager (94% similarity)
     - openshift-controller-manager (87% similarity)
   Deviations from pattern:
     - Different volume mount paths (justified: different config format)
     - No platform-specific code path (justified: universal component)

3. STATIC ANALYSIS
   - Purity check: PASS
   - RBAC scope check: PASS
   - Import analysis: No unusual imports (no cloud SDKs, no HTTP clients)
   - Cyclomatic complexity: 4 (low)

4. TEST EVIDENCE
   - 12 unit test cases, all passing
   - Test coverage of adapt function: 100% line, 93% branch
   - Property test: 1000 random inputs, all produce valid deployments

5. CI EVIDENCE
   - AWS e2e: PASS (run #47821)
   - Azure e2e: PASS (run #47822)
   - KubeVirt e2e: PASS (run #47825, after image fix)
   - None platform: PASS (run #47820)

6. HUMAN DECISIONS LOG
   - Resource budget: Ahmed chose convention (200m/300Mi limits)
   - Config format: Ahmed corrected (flags, not config file)
   - Image resolution: Forge auto-corrected after CI failure
```

The reviewer (say, Cesar) opens the PR in his Forge. He sees:

```
PR #4523 by Ahmed (via Forge)

CONFIDENCE SCORE: HIGH
  All contracts satisfied, pattern conformant, all CI green.

REVIEW FOCUS AREAS (Forge recommendation):
  1. Verify the adapt function's argument list matches what the
     cluster-policy-controller actually expects in 4.18.
     (The Forge checked the upstream repo, but a human should
      confirm the flag names.)
  2. Verify the resource budget is appropriate for the expected
     workload. (Convention was applied; may need adjustment based
     on real-world observation post-deployment.)

ESTIMATED REVIEW TIME: 15 minutes (contract review, not line review)
```

Cesar does not read 8 files of Go code. He verifies 2 focus areas and checks the evidence bundle. He approves in 15 minutes instead of 90.

### The Confidence Gradient

Over time, the Forge tracks confidence at the team level:

```
TEAM CONFIDENCE METRICS (last 30 days)

Forge-authored PRs: 23
Post-merge bugs found: 1 (image path in KubeVirt, caught in CI)
Contract violations caught pre-PR: 47
Contract violations escaped to review: 0
Average review time: 22 minutes (down from 74 minutes 3 months ago)

PATTERN: The Forge has not produced an adapt function purity
violation in 45 days. Consider reducing review scrutiny on purity
from "verify" to "spot-check"?
```

---

## Part VIII: What Makes This Feel Like the Future

### 1. The Developer Thinks in Contracts, Not Code

The paradigm shift is that the developer's primary artifact is not code. It is a set of contracts that the code must satisfy. The Forge produces the code. The developer produces the judgment about what contracts are needed and whether they are sufficient.

This is not "AI writes code for me." It is "I am the architect, the Forge is the builder." Architects do not lay bricks. But they absolutely must understand bricks, mortar, load-bearing walls, and building codes. The developer still needs deep technical knowledge, but they apply it at a higher level of abstraction.

### 2. Knowledge Compounds Automatically

Every bug, every review comment, every CI failure becomes a pattern that prevents future occurrences. Not through documentation that nobody reads, but through the Forge's active enforcement. The team gets better without trying to get better.

### 3. The Autonomy Dial Replaces the On/Off Switch

There is no "use AI" or "don't use AI." There is a continuous spectrum from "I type every character" to "wake me when it's merged." Different developers, different phases, different risk levels, different positions on the dial. And it moves over time as trust builds.

### 4. New Developers Become Productive Immediately

Not productive at senior level. But productive at "making correct, contract-satisfying contributions that senior developers can review efficiently" level. The Forge is the best onboarding tool imaginable because it embodies the team's accumulated knowledge and enforces it gently.

### 5. Review Becomes Verification, Not Archaeology

Reviewers stop asking "what does this code do?" and start asking "are the contracts right?" The evidence bundle makes this possible. Review time drops by 3-5x. Review quality increases because reviewers focus on judgment calls, not typos.

---

## Part IX: What This Does NOT Do

Intellectual honesty demands acknowledging the boundaries.

1. **It does not replace architectural judgment.** The Forge cannot decide whether Azure Key Vault integration should use managed identities or service principals. It can research both, present tradeoffs, and implement whichever you choose. But the choice is yours.

2. **It does not handle novel architecture.** When someone needs to design a fundamentally new pattern (like CPOv2 itself was, once), the Forge is a research assistant, not an architect. It shines at applying known patterns, not inventing new ones.

3. **It does not eliminate the need for deep understanding.** Ahmed can set adapt functions to `autonomous` because he deeply understands what purity means and can verify contracts in seconds. A developer who does not understand purity should not set that phase to `autonomous`. The Forge enables leverage of existing expertise, it does not create expertise from nothing.

4. **It does not make flaky CI not flaky.** It makes flaky CI less painful by recognizing known flakes, auto-retriggering, and tracking flake rates. But the underlying infrastructure problems remain.

5. **It does not solve organizational dysfunction.** If the RFE is poorly defined, the Forge will produce a well-structured breakdown of a poorly defined requirement. Garbage in, structured garbage out.

---

## Part X: Implementation Notes (Not Code, but Architecture)

### RH-O (The Orchestrator) Responsibilities

- Maintains the pipeline state: which phase is current, what decisions are pending, what is blocked
- Manages the autonomy configuration: knows when to proceed and when to ask
- Tracks contract satisfaction across all active stories
- Manages the evidence bundle lifecycle
- Interfaces with external systems: JIRA, GitHub, CI systems
- Maintains the version skew graph and team knowledge repository

### RH-A (The Agent) Responsibilities

- Reads and writes code
- Runs static analysis (purity checks, RBAC scope analysis, import analysis)
- Executes tests locally
- Interacts with the developer through the IDE layer, mobile surface, and voice
- Generates contract evidence
- Produces CI failure analysis
- Generates and responds to review comments

### The Contract Between RH-O and RH-A

(And yes, the irony of defining a contract between the orchestrator and the agent in a document about contract-driven development is intentional.)

```
RH-O → RH-A:
  "Implement HOSTEDCP-1234.
   Spec: [link]
   Contracts: [list]
   Autonomy: [per-phase settings]
   Team knowledge: [relevant patterns]
   Reference: [similar component]"

RH-A → RH-O:
  "Implementation complete.
   Contract satisfaction: [per-contract evidence]
   Decisions made: [list with rationale]
   Decisions needed: [list with options]
   Files changed: [list]
   Tests: [results]"
```

### Storage and Portability

Everything is git. The team knowledge, the contract definitions, the domain model, the pattern library, all lives in `.forge/` in the repository. When someone clones the repo, they get the Forge's knowledge. When someone forks the repo for a different project, they get a starting point for their own Forge. The personal layer (autonomy settings, history, preferences) lives in `~/.forge/` and is never shared.

---

## Closing: The Smith and the Forge

The Forge does not replace the smith. It amplifies the smith. A master smith using a modern forge produces better work, faster, than a master smith with a hammer and an open fire. But a novice with a modern forge still needs the master to teach them where to strike.

The dream-world UX for HyperShift development is one where the developer's time is spent on judgment, not mechanics. Where the team's knowledge compounds automatically. Where new developers become effective immediately. Where reviewers verify contracts instead of reading diffs. Where CI failures are diagnosed in seconds, not hours. Where version skew bugs are prevented at the schema level, not discovered in production.

This is not science fiction. Every individual capability described here exists in some form today. The innovation is in the integration: the contract-driven model, the autonomy spectrum, the knowledge compounding, and the semantic layer over the IDE that makes the developer's mental model the primary interface, not files and diffs.

The control plane is the heart of HyperShift. The Forge should be the heart of the team that builds it.
