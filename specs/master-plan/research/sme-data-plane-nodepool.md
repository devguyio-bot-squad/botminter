# Dream-World UX Vision — Data Plane SME Perspective

> Independent vision from the Data Plane SME agent. No cross-pollination with other SME outputs.
> Part of [Shift Week Plan](shift-week-plan.md), input to [Living Codebase Vision](ux-living-codebase-vision.md)

---

## Preamble

I have spent years reasoning about NodePool lifecycles, ClusterAPI reconciliation loops, the boundary between management cluster and data plane, and the platform-specific sprawl that makes HyperShift both powerful and treacherous. What follows is not a tool proposal. It is a redesign of how a human thinks alongside a machine about distributed infrastructure software.

The core insight is this: **the developer should never be working at the wrong level of abstraction.** Today, a developer working on NodePool scaling needs to simultaneously hold in their head the NodePool API surface, the CAPI MachineDeployment/MachineSet/Machine hierarchy, the platform-specific machine template, the CPO adapt functions, the version skew implications, the e2e test topology, and the CI flake patterns. That is not engineering. That is cognitive torture. The system should carry the abstraction stack and let the developer work at exactly the layer that matters for their current decision.

---

## Part I: The Paradigm Shift

### From "Code Editor + Terminal" to "Situation Room"

The fundamental UX metaphor is not an IDE. It is a **situation room** -- a space where the developer has persistent, ambient awareness of the system they are changing, and where the agent acts as a senior colleague who has already read every file, every JIRA ticket, every CI log, and every previous PR in the relevant area.

The paradigm shift is threefold:

**1. The system models the domain, not just the code.**

Today's agents see files. This system sees *NodePool lifecycle state machines*, *CAPI reconciliation contracts*, *platform abstraction boundaries*, and *version skew compatibility matrices*. When a developer says "I need to add a new field to NodePool that affects machine rollout," the system does not grep for `NodePool` -- it activates a domain model that knows:

- NodePool spec changes trigger the `reconcileNodePoolDesiredState` path
- Machine rollout involves MachineDeployment strategy (RollingUpdate vs OnDelete)
- The field needs to propagate through the CPO adapt functions (which must be pure)
- Platform-specific machine templates may need corresponding changes
- The hosted cluster version determines which fields are available
- e2e tests for rolling upgrades take 45+ minutes and run on specific platforms

**2. The system works in *intentions*, not instructions.**

The developer does not say "edit this file, run this command." The developer states an intention: "NodePool should support configurable kubelet memory reservation per platform." The system translates that intention into a plan that respects the architecture, proposes the API surface change, identifies all downstream touchpoints, and executes each phase with the developer's chosen level of oversight.

**3. The system maintains a living model of the data plane.**

Not documentation. Not diagrams. A queryable, evolving model that the agent and the developer both reference. When the developer asks "what happens if I change the default surge count for Azure machine rollouts?" the system can trace the impact through the model before a single line of code is touched.

---

## Part II: A Day in the Life

### Morning: Picking Up Work

Fatima is a data plane engineer. She joined the team four months ago. She opens her workspace -- a browser-based environment that persists between sessions, though she could also be in her IDE or on her phone during a commute. The distinction between these surfaces matters less than it seems; I will address it later.

The system greets her not with a blank prompt but with **situation awareness**:

```
Active context: OCPBUGS-48291 — NodePool upgrade stalls when
Azure VMSS capacity exceeds regional quota

You were exploring the MachineDeployment rollout controller
yesterday. You had identified that the CAPI contract expects
InfrastructureReady to be set before the Machine controller
proceeds, but the Azure provider sets it asynchronously after
VMSS scaling completes.

Your current hypothesis: the NodePool controller should
propagate Azure capacity errors from the Machine status back
to NodePool conditions, but the CPO adapt function for
AzureMachineTemplate does not currently surface VMSS-level
errors.

Three related PRs exist in the codebase:
  - PR #4892 (merged, 2025-09) added GCP quota error surfacing
  - PR #5103 (open, draft) attempts similar work for PowerVS
  - PR #4401 (merged, 2024-11) established the error
    propagation pattern from Machine → NodePool conditions

Team knowledge note: Cesar documented that Azure VMSS errors
appear in the Machine's provider status, not in conditions.
This differs from AWS where errors surface in the
EC2Instance status condition.
```

Fatima did not ask for this. The system assembled it from her last session, the JIRA ticket, git history, team knowledge entries, and the domain model. She scans it in 20 seconds and knows exactly where she is.

She says: **"Show me the error propagation path from Azure Machine to NodePool."**

The system does not show her code. It shows her the **domain flow**:

```
AzureMachine.Status.ProviderStatus (VMSS-level errors)
    ↓ [Azure CAPI provider controller]
Machine.Status.Conditions (InfrastructureReady=False)
    ↓ [CAPI Machine controller]
MachineDeployment.Status.Conditions
    ↓ [NodePool controller: reconcileMachineDeployment()]
NodePool.Status.Conditions (MachinesHealthy / Progressing)

Gap identified: Step 1→2 loses the specific Azure error
message. The CAPI provider controller sets
InfrastructureReady=False but does not propagate the VMSS
capacity error string into the Machine's status message.
```

This is the domain model at work. Fatima is not reading controller code yet. She is understanding the **architecture of the problem** before deciding where to intervene.

### Mid-Morning: Design Decision

Fatima decides the fix belongs in two places: the Azure CAPI provider (to propagate the error detail) and the NodePool controller (to surface it in NodePool conditions). But the Azure CAPI provider is in a separate repository with its own release cycle.

She says: **"What are my options for fixing this within hypershift only, without waiting for an upstream CAPI provider change?"**

The system reasons architecturally:

```
Option A: Add a secondary error extraction in the NodePool
controller that reads AzureMachine objects directly.
  Impact: Violates the CAPI abstraction. The NodePool
  controller currently operates on generic Machine objects.
  Reading AzureMachine directly creates platform-specific
  coupling in the management-side controller.
  Precedent: This pattern does NOT exist in the codebase
  today. The NodePool controller is platform-agnostic at
  the Machine level.

Option B: Add an annotation-based bridge. The platform-
specific machine controller (which already has provider
access) annotates the Machine object with the detailed
error. The NodePool controller reads annotations.
  Impact: Preserves abstraction boundary. Annotations are
  the established pattern for cross-controller communication
  in hypershift (see: machineAnnotations in nodepool.go).
  Precedent: Similar to how platform-specific metadata is
  passed via annotations in PR #4892 (GCP quota errors).

Option C: Use the CPO to extract the error. Add an adapt
function that reads the provider-specific Machine status
and translates it to a generic error condition.
  Impact: Keeps platform logic in CPO where it belongs.
  But adapt functions must be pure — they cannot make API
  calls. The error info must already be present on the
  objects passed to the adapt function.
  Constraint: adapt functions receive the Machine object,
  not the AzureMachine. So this option requires Option B
  as a prerequisite anyway.

Recommendation: Option B. It follows established patterns,
preserves the abstraction boundary, and enables a future
upstream fix without requiring changes to the hypershift-
side code.
```

Fatima did not ask the system to write code. She asked an architectural question. The system answered with full awareness of the CAPI abstraction layer, the CPO purity constraints, the existing annotation patterns, and the specific codebase precedents. This is not retrieval-augmented generation. This is **domain-model-driven architectural reasoning**.

She agrees with Option B and says: **"Plan the implementation."**

### Late Morning: The Plan

The system produces a structured implementation plan. Not a todo list -- a **dependency-aware, phase-gated plan** that maps to the team's SDLC:

```
Implementation Plan: OCPBUGS-48291
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Phase 1: API Surface (requires review before proceeding)
  Story: Add NodePool condition type for platform capacity errors
  Changes:
    - api/hypershift/v1beta1/nodepool_types.go
      Add CapacityError condition type to NodePool conditions
    - Generate: make api (triggers deepcopy, scheme registration)
  Validation:
    - Unit test: condition is registered and serializes correctly
    - Verify no API breaking changes (make verify)

Phase 2: Platform Bridge (can proceed after Phase 1 merges)
  Story: Azure machine controller annotates capacity errors
  Changes:
    - control-plane-operator/controllers/hostedcontrolplane/
      azure/machine_controller.go
      Watch AzureMachine status, annotate Machine with
      capacity error detail when VMSS scaling fails
  Constraints:
    - Must work with both CAPI v1beta1 and v1beta2 Machine API
    - Must handle the case where AzureMachine is not yet created
    - Adapt function pattern: this is a controller, not an adapt
      function, so side effects (annotation writes) are permitted
  Validation:
    - Unit test: annotation is set when VMSS reports capacity error
    - Unit test: annotation is cleared when error resolves
    - Unit test: no annotation when AzureMachine does not exist

Phase 3: NodePool Integration
  Story: NodePool controller surfaces capacity errors from
  Machine annotations
  Changes:
    - hypershift-operator/controllers/nodepool/nodepool.go
      In reconcileMachineDeployment, read capacity error
      annotations from Machine objects and set NodePool
      CapacityError condition
  Constraints:
    - Must be platform-agnostic (read annotation, not
      provider-specific objects)
    - Must aggregate errors across multiple Machines
      (NodePool may manage many Machines)
    - Must clear condition when no Machines report errors
  Validation:
    - Unit test: condition set when Machine has annotation
    - Unit test: condition cleared when annotation removed
    - Unit test: aggregation logic for multiple Machines
    - e2e test: Azure-specific, requires VMSS quota limit
      simulation (ask: is this feasible in CI?)

Phase 4: Documentation and Observability
  Story: Surface capacity errors in oc adm inspect output
  Changes:
    - Ensure new condition appears in must-gather
    - Update troubleshooting runbook (if one exists)

Dependencies: Phase 2 and 3 can be developed in parallel
(using mocked annotations for Phase 3 tests) but must be
merged sequentially.

Estimated CI time: ~3 hours total across all phases
(Azure e2e suite is ~90 minutes per run)
```

This plan is not generic. It knows that `make api` is required after API type changes. It knows adapt functions must be pure. It knows Azure e2e tests run about 90 minutes. It knows the annotation pattern from PR #4892. It knows CAPI has version skew concerns between v1beta1 and v1beta2. This is **domain-aware planning**.

Fatima reviews the plan. She adjusts Phase 2 -- she wants the annotation key to follow a specific naming convention. She approves.

She says: **"Execute Phase 1. Full auto. Notify me when the PR is ready for review."**

### Afternoon: Autonomous Execution

The system works Phase 1 autonomously. This is not "generate code and show me." This is:

1. **Write the API change** in `nodepool_types.go`, following the exact patterns used by existing condition types (the system knows the `const` block format, the condition reason conventions, the godoc style)
2. **Run `make api`** to trigger code generation (deepcopy, scheme, clients)
3. **Run `make verify`** to check for API compatibility issues
4. **Write unit tests** that match the existing test patterns in the package (the system knows whether the team uses table-driven tests, what assertion library they use, how test fixtures are structured)
5. **Run unit tests**, capture results
6. **Create the PR** with a description that follows the team's PR template, links the JIRA story, and includes the test evidence
7. **Monitor CI** -- if a test fails, diagnose whether it is a real failure or a known flake. If flake, rerun. If real, fix and push.

Throughout this, Fatima is working on something else. She gets a notification:

```
Phase 1 complete. PR #5267 created.

CI status: 14/16 checks passed.
  - e2e-aws: passed
  - e2e-azure: running (est. 35 min remaining)
  - unit tests: passed (247 passed, 0 failed)
  - verify: passed
  - lint: passed
  - e2e-kubevirt-serial: failed → known flake (TestNodePool
    ReconcilePauseLabel), retriggered automatically

API diff: +1 condition type (CapacityError), no breaking
changes detected. Fully backward compatible.

Ready for review. 2 suggested reviewers based on code
ownership: @cesar, @enxebre
```

Fatima opens the PR. She does not need to read every line. The system has already validated:
- API compatibility (verified by `make verify`)
- Code generation correctness (verified by `make api` + test)
- Test coverage (unit tests pass, e2e running)
- Pattern conformance (matches existing condition type definitions)
- Known flake handling (automatically retriggered)

She reviews the **design decision** (condition type name, reasons, messages) rather than the **implementation mechanics**. This is the confidence model: the system provides evidence that the mechanics are correct so the human can focus on the intent.

### Late Afternoon: The Interesting Part

Phase 3 is where Fatima wants to be hands-on. This is the NodePool controller change -- the core logic. She switches to **collaborative mode**:

**"Start Phase 3. Pair with me. I want to drive the reconciliation logic."**

The system shifts behavior. Instead of executing autonomously, it becomes a pair programming partner. But not a passive one. An opinionated one.

Fatima starts writing the aggregation logic for Machine capacity errors. She writes a loop that iterates through all Machines and collects error annotations. The system interjects:

```
Consideration: You are iterating all Machines owned by this
NodePool inside the reconcile loop. For large NodePools
(100+ machines), this creates O(n) API calls per reconcile.

The existing pattern in reconcileMachineDeployment uses the
Machine list that is already fetched and cached earlier in
the reconciliation. See line 847 of nodepool.go where
machines are listed for health checking.

Suggest: reuse that existing Machine list rather than
re-listing. This also ensures consistency within a single
reconcile pass — you are seeing the same Machine state as
the health check logic.
```

This is not a linter. This is not a code suggestion. This is a colleague who knows the controller's reconciliation flow deeply enough to point out that there is already a Machine list available, and who understands the performance implications of redundant API calls on the management cluster.

Fatima takes the suggestion. She writes the condition-setting logic. The system reviews it in real-time:

```
The condition message concatenates all Machine error
messages. With 50 machines hitting quota, this string could
be very long.

Existing pattern: NodePool conditions use a summary format
with count. See the MachinesHealthy condition, which reports
"3 of 10 machines are not healthy" rather than listing each
one.

Also: the condition reason should be a CamelCase constant.
You used "capacity_error" — the convention in this codebase
is "CapacityError" (see AutoscalingEnabled, UpdatingConfig,
etc. in nodepool_types.go).
```

By end of day, Fatima has written the core logic herself, but with the system catching pattern violations, performance issues, and API convention mismatches in real time. The system did not write the code. It **guarded the architecture** while she wrote it.

---

## Part III: The Interaction Surfaces

### The Primary Surface: A Persistent Workspace

Not an IDE tab. Not a chat window. A **persistent workspace** that maintains state across sessions, across days, across weeks. When Fatima closes her laptop and reopens it the next morning, the workspace knows where she was, what CI results came in overnight, and what decisions are pending.

The workspace has three zones:

**The Situation Zone** (always visible, compact): Current context, active work item, blockers, CI status, team activity relevant to her work. This is the "situation room" ambient awareness. It updates in real time. It takes about 15% of the screen, always present but not intrusive.

**The Work Zone** (primary focus): This is where code, plans, and architectural reasoning happen. It adapts to the current phase. During planning, it shows the domain model and plan structure. During implementation, it shows code with contextual overlays. During review, it shows diffs with domain annotations.

**The Evidence Zone** (expandable): Test results, CI logs, coverage data, API compatibility reports, performance benchmarks. This zone is collapsed by default but expands when the developer needs to verify a claim the system has made.

### Secondary Surfaces

**Mobile (phone/tablet):** Not a reduced IDE. A **decision surface**. The system surfaces only things that need human judgment: "CI failed on Phase 2. The failure is in a test I have not seen fail before. The error suggests the AzureMachine mock is returning a nil status. Should I (a) fix the mock and repush, (b) wait for you to look at it, or (c) skip this test for now?" Fatima taps (a) while on the train. The system proceeds.

**Async notifications:** Configurable by urgency. CI flake retrigger? Silent log entry. Blocking failure the system cannot resolve? Notification with context. PR approved by reviewer? Notification with "ready to merge?" action.

**Team dashboard:** A shared surface (browser-based) showing the team's active work items, their agentic execution status, and accumulated knowledge. When Cesar fixes a pattern for GCP error propagation, Fatima's system knows about it by the next session. This is the knowledge compounding surface.

---

## Part IV: Domain Model Architecture for the Data Plane

This is where my specific expertise matters most. The system's power for NodePool/CAPI work comes from a **structured domain model** that encodes the things an experienced data plane engineer carries in their head.

### The NodePool Lifecycle Model

The system maintains a formal model of the NodePool lifecycle that goes beyond documentation:

```
NodePool Lifecycle States
━━━━━━━━━━━━━━━━━━━━━━━━

Creating
  ├─ SubStates:
  │   InfrastructureProvisioning
  │   SecurityGroupSetup (platform-specific)
  │   MachineDeploymentCreation
  │   InitialMachineProvisioning
  │   NodeJoining
  │
  ├─ Platform Variations:
  │   AWS:  LaunchTemplate → ASG → EC2 → Node
  │   Azure: VMSS → VM → Node
  │   KubeVirt: VM → VMI → Pod → Node
  │   (each with different timing, error modes,
  │    and quota behaviors)
  │
  └─ Failure Modes:
      QuotaExceeded (platform-specific detection)
      SecurityGroupLimit (AWS-specific)
      SubnetExhaustion (AWS, Azure)
      ImageNotFound (all platforms)
      IgnitionConfigFailure (all platforms)

Scaling
  ├─ SubStates:
  │   DesiredReplicaUpdate
  │   MachineDeploymentRollout
  │   MachineCreation
  │   NodeReadiness
  │
  ├─ Constraints:
  │   MaxSurge / MaxUnavailable (from update strategy)
  │   Autoscaler interaction (NodePool.Spec.AutoScaling)
  │   PDB compliance on hosted cluster side
  │
  └─ Invariants:
      Machines must not exceed cloud quota
      Node count must converge to desired within timeout
      Scaling must be interruptible (new spec takes precedence)

Upgrading
  ├─ SubStates:
  │   ConfigPropagation (tuningConfig, kubelet, etc.)
  │   MachineTemplateUpdate (platform-specific)
  │   RollingReplace (MachineDeployment strategy)
  │   NodeDrain (hosted cluster side)
  │   MachineDelete
  │   NewMachineCreate
  │   NodeJoin
  │
  ├─ Version Skew Concerns:
  │   NodePool.Spec.Release vs HostedCluster.Spec.Release
  │   CPO version vs hypershift-operator version
  │   CAPI provider version vs CAPI core version
  │
  ├─ Platform-Specific Rollout:
  │   AWS:  New LaunchTemplate version → ASG rolling update
  │   Azure: New VMSS model → rolling VM replacement
  │   KubeVirt: New VM template → VM replacement
  │
  └─ Invariants:
      Never drain more nodes than MaxUnavailable
      Respect PDBs on the hosted cluster
      Old Machine deleted only after new Machine is Ready
      Version can only move forward, never backward

Deleting
  ├─ SubStates:
  │   NodeDrain (all nodes)
  │   MachineDelete (all machines)
  │   InfrastructureCleanup (platform-specific)
  │   FinalizerRemoval
  │
  └─ Ordering:
      Nodes drained before Machines deleted
      Machines deleted before infrastructure cleaned up
      Infrastructure cleanup is platform-specific and
      may require retries (cloud API eventual consistency)
```

This model is not documentation for the developer to read. It is an **active artifact** that the system uses to reason about changes. When Fatima modifies the upgrade path, the system checks her changes against the model's invariants. When she adds a new platform, the system identifies which lifecycle substates need platform-specific implementations.

### The CAPI Abstraction Map

The system maintains a map of how HyperShift's controllers interact with CAPI resources, and where the abstraction boundaries are:

```
Management Cluster (hypershift-operator)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
NodePool Controller
  ├─ Owns: MachineDeployment, MachineSet, Machine (CAPI)
  ├─ Owns: Platform-specific templates (via CPO)
  ├─ Reads: Machine status (generic, not platform-specific)
  ├─ MUST NOT: Read provider-specific Machine objects directly
  └─ Communicates to data plane via:
     Ignition config (for node bootstrap)
     No direct API access to hosted cluster

CPO (Control Plane Operator)
  ├─ Runs IN: the hosted control plane namespace (mgmt cluster)
  ├─ adapt functions: Pure transformations only
  │   Input: desired state from NodePool spec
  │   Output: platform-specific template specs
  │   MUST NOT: make API calls, have side effects
  ├─ Platform controllers: Can have side effects
  │   Handle provider-specific reconciliation
  │   Bridge between generic CAPI and provider API
  └─ Version: may differ from hypershift-operator version

CAPI Controllers (management cluster)
  ├─ MachineDeployment controller
  ├─ MachineSet controller
  ├─ Machine controller
  └─ Provider-specific controllers (CAPA, CAPZ, etc.)

Data Plane (hosted cluster)
━━━━━━━━━━━━━━━━━━━━━━━━━━
  ├─ Nodes: created by CAPI Machine lifecycle
  ├─ kubelet: configured via ignition
  ├─ NO DIRECT COMMUNICATION back to management cluster
  └─ Observed via: Machine status (management side watches)
```

When a developer changes something in the NodePool controller, the system validates against this map: "You are reading `AzureMachine.Status` directly in the NodePool controller. This crosses the CAPI abstraction boundary. The NodePool controller should only read generic `Machine` objects. Platform-specific information should be bridged via annotations or conditions by the CPO or platform controller."

This is the kind of review comment that a senior engineer would make. The system makes it instantly, every time, without fatigue.

### The Platform Variation Matrix

The system knows which behaviors vary by platform and how:

```
                    AWS         Azure       KubeVirt    OpenStack   PowerVS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Machine Template    AWSMachine  AzureMachine KubeVirt   OpenStack   IBMPower
                    Template    Template     Machine    MachineTempl VSMachine
                                             Template               Template

Scaling Primitive   ASG         VMSS        ReplicaSet  ServerGroup  (direct)

Quota Error Source  EC2 API     ARM API     Kubevirt    Nova API    PowerVS API
                    response    response    scheduling  response    response

Boot Time (typ.)    3-5 min     4-7 min     1-2 min     3-6 min    5-10 min

Image Reference     AMI ID      Image Ref   Container   Image UUID  Image ID
                                            Disk Image

Network Model       VPC/Subnet  VNet/Subnet Pod Network Neutron     VPC/Subnet

e2e Test Suite      e2e-aws     e2e-azure   e2e-kubevirt e2e-osp   e2e-powervs
Test Duration       ~60 min     ~90 min     ~45 min      ~75 min   ~90 min
Flake Rate          Low         Medium      Low          Medium    High
```

When a developer modifies NodePool scaling behavior, the system asks: "This change affects the scaling path. It will behave differently on KubeVirt (where the scaling primitive is a ReplicaSet with ~1 min boot time) versus Azure (VMSS with ~5 min boot time). Have you considered timeout behavior on slower platforms?"

---

## Part V: Knowledge Compounding

### How Team Knowledge Accumulates

This is the part most systems get wrong. They store chat histories or "learnings" as flat text. That scales poorly and becomes noise. The system I am describing has **structured knowledge** that compounds.

**Three knowledge tiers:**

**Tier 1: Pattern Library (team-wide, curated)**

These are verified, reviewed patterns that the entire team has agreed are correct. They are first-class artifacts, version-controlled, peer-reviewed like code.

Examples for the data plane domain:
- "Error Propagation Pattern": How to surface platform-specific errors through the CAPI abstraction to NodePool conditions. Includes the annotation bridge pattern, the condition naming conventions, the aggregation approach.
- "New Platform Checklist": Every touchpoint when adding a new platform to HyperShift. Machine template type, CPO adapt functions, platform controller, e2e test suite, must-gather, docs.
- "CPO Adapt Function Contract": What adapt functions may and may not do. Input/output types. How to test them. Common mistakes.
- "NodePool Condition Conventions": Naming, reasons, messages, when to set vs clear, how to aggregate across Machines.

**Tier 2: Experience Logs (team-wide, automated)**

These are automatically captured from the system's interactions with all team members. Not raw transcripts -- structured observations.

Example: "When modifying MachineDeployment rollout strategy, 3 of 4 developers forgot to update the corresponding e2e test assertion for MaxUnavailable. Adding a reminder to the rollout strategy modification flow."

Example: "Azure e2e test `TestNodePoolScaleUp` has failed 12 times in the last month. 11 were flakes (VMSS API timeout). 1 was a real failure (PR #5089 introduced a race in the scale-up path). Flake pattern: always fails at the 'waiting for Machine Ready' step with a 10-minute timeout. Suggest increasing timeout or adding retry."

These observations are surfaced to developers when relevant. They do not need to be explicitly queried.

**Tier 3: Individual Memory (per developer, private)**

Fatima's preferences, her working patterns, her areas of expertise and areas where she needs more context. The system knows Fatima is experienced with AWS and KubeVirt but less familiar with Azure and PowerVS. It adjusts its level of explanation accordingly. When Fatima works on Azure, it provides more context. When she works on KubeVirt, it is more concise.

### The Knowledge Feedback Loop

When Fatima solves a problem, the system proposes a knowledge contribution:

```
You resolved OCPBUGS-48291 by using the annotation bridge
pattern for Azure capacity errors. This pattern is
documented in the Pattern Library for GCP (PR #4892) but
not for Azure.

Propose: Update the "Error Propagation Pattern" to include
Azure-specific notes:
  - Azure errors surface in AzureMachine.Status.ProviderStatus
    (unlike AWS where they appear in EC2Instance conditions)
  - VMSS capacity errors use error code
    "OperationNotAllowed" with message containing "quota"
  - The annotation key follows:
    hypershift.openshift.io/capacity-error

Create knowledge entry? [yes / edit first / no]
```

Fatima taps "yes." Now, when Karim works on a similar issue for OpenStack next month, the system already knows the general pattern and the platform-specific variations for GCP, AWS, and Azure. It can reason about what the OpenStack variation likely looks like.

This is not a wiki. This is not a search index. This is **compounding architectural knowledge** that makes every subsequent developer interaction faster and more accurate.

---

## Part VI: The Confidence Model

### Why Line-by-Line Review Fails for Agentic Code

The current code review model assumes a human wrote the code and another human checks their work. For agentic code, this model is broken. If the agent writes 500 lines across 8 files for a NodePool feature, asking a human to review every line defeats the purpose.

The system provides **evidence-based confidence** instead:

**Level 1: Structural Confidence**
- "All changes conform to identified patterns" (pattern library match)
- "No abstraction boundary violations detected" (domain model check)
- "API changes are backward compatible" (make verify)
- "Code generation is consistent" (make api + diff)

**Level 2: Behavioral Confidence**
- "Unit tests cover all new code paths" (coverage report)
- "Unit tests pass: 312/312" (test results)
- "e2e tests pass on target platform" (CI results)
- "No regression in adjacent test suites" (CI comparison)

**Level 3: Architectural Confidence**
- "Changes were validated against the NodePool lifecycle model"
- "Platform variation matrix was consulted -- no cross-platform impact"
- "Version skew analysis: no compatibility concerns with CPO v4.16"
- "Similar changes in PR #4892 have been stable for 5 months"

**Level 4: Human Confidence**
- "Developer paired on the core reconciliation logic (Phase 3)"
- "Developer reviewed and approved the architectural approach"
- "Reviewer @cesar approved the API surface change"

The PR description does not say "this code was generated by an agent." It says: "Confidence: L1-structural (all patterns matched), L2-behavioral (100% test coverage, CI green), L3-architectural (lifecycle model validated, no platform impact), L4-human (developer paired on core logic, reviewer approved API)."

A reviewer can then make an informed decision: "L1-L3 are green, and Fatima paired on the core logic. I will review the API surface and the architectural approach, but I trust the implementation mechanics."

### Confidence for Data Plane Changes Specifically

Data plane changes are the most user-visible changes in HyperShift. A bug in NodePool scaling means customers cannot get compute capacity. A bug in upgrade rollout means customers are stuck on old versions. The confidence bar is higher.

For data plane changes, the system adds domain-specific confidence signals:

- **Lifecycle invariant check**: "The upgrade path maintains the invariant that old Machines are only deleted after new Machines are Ready. Verified by unit test TestUpgradeRolloutInvariant."
- **Platform blast radius**: "This change touches the generic NodePool controller only. No platform-specific files modified. Verified by file path analysis and import graph."
- **Scale consideration**: "The new reconciliation logic is O(n) in the number of Machines. For the maximum supported NodePool size (500 machines), the estimated reconcile time increase is <100ms. Verified by benchmark TestReconcilePerformance."
- **Version skew safety**: "The new NodePool condition CapacityError is additive. Older CPO versions will ignore it. Newer CPO versions do not depend on it. Verified by API compatibility analysis."

---

## Part VII: The New Developer Experience

### Day 1: Karim Joins the Data Plane Team

Karim is an experienced Kubernetes developer but new to HyperShift. Today's onboarding: read docs for a week, maybe a month before the first meaningful PR. That is waste.

In the dream system, Karim's Day 1 looks different.

He opens the workspace. The system knows he is new (team config). It starts with orientation, not a task:

```
Welcome to the HyperShift data plane team. I will be your
technical partner.

Before we start on any code, let me show you the domain
model for the area you will be working in.

The core concept: HyperShift separates the Kubernetes
control plane from the data plane. The control plane runs
on a management cluster as pods. The data plane (worker
nodes) runs on a separate infrastructure. Your work will
primarily be on the data plane side — specifically, how
NodePools manage the lifecycle of worker nodes.

Here is the key architectural constraint you need to
internalize: the management cluster can reach the data plane,
but the data plane CANNOT reach back to the management
cluster. All communication is unidirectional.

This means:
  - NodePool controllers run on the management cluster
  - They create and manage Machines (via CAPI) on the
    management cluster
  - Machines provision actual VMs/instances on cloud
    providers
  - Those VMs bootstrap themselves using ignition configs
    and join the hosted cluster as Nodes
  - The NodePool controller observes Machine status to
    understand Node health — it never talks to the hosted
    cluster's API server directly

Want me to walk through a specific lifecycle (create, scale,
upgrade, or delete) in detail, or would you prefer to start
with a small task and learn by doing?
```

Karim says: "Give me a small task."

The system finds a good starter task -- one that is real (not a toy exercise), scoped (not a multi-week project), and educational (touches key abstractions):

```
Starter task: OCPBUGS-49012 — NodePool status does not
report the number of Machines in "Deleting" phase

This is a good starter because:
  - It touches the NodePool controller (the core of your
    domain)
  - It requires reading Machine status (CAPI interaction)
  - It is a status-only change (no risky behavioral changes)
  - It follows an established pattern (similar to how we
    report Ready/NotReady Machine counts)
  - It has clear validation criteria (unit test + e2e)

Estimated time: 2-4 hours with guidance.

I will walk you through the codebase as we go. I will
explain WHY things are structured the way they are, not
just WHERE the code lives.

Ready?
```

Karim is productive on Day 1. Not "read docs" productive. "Shipping code with understanding" productive. The system teaches the architecture through the work, not before it.

By Day 5, Karim has merged his first PR and has a mental model of NodePool lifecycle, CAPI abstraction, and the platform boundary -- not from reading docs, but from having an expert explain each concept at the moment he needed it.

By Month 2, Karim is working on NodePool upgrade logic independently. The system has calibrated its assistance level. It no longer explains what a MachineDeployment is. It does flag when Karim's changes might violate an invariant he has not encountered yet.

---

## Part VIII: RH-O and RH-A -- Orchestrator and Agent Mapping

### How This Maps to the Two-Component Architecture

**RH-O (Orchestrator)** is responsible for:
- Maintaining the domain model (NodePool lifecycle, CAPI abstraction map, platform matrix)
- Managing the SDLC pipeline (JIRA hierarchy → plan phases → execution phases)
- Tracking state across sessions (what Fatima was working on, what CI status is, what knowledge has been contributed)
- Configuring the human-in-the-loop level per phase, per developer, per task
- Maintaining the three-tier knowledge system
- Computing confidence scores
- Managing handoffs between autonomous and collaborative modes

**RH-A (Agent)** is responsible for:
- Reading and writing code within the boundaries set by RH-O
- Running commands (make, tests, CI interactions)
- Interacting with the developer in the moment (pair programming, questions, suggestions)
- Producing evidence (test results, verification output, coverage data)
- Proposing knowledge contributions based on completed work
- Executing within the constraints of the domain model (flagging violations rather than ignoring them)

The key design principle: **RH-O holds the model, RH-A acts within it.** RH-O never writes code. RH-A never decides the workflow. They are complementary.

### Configuration Surface

The system is configured at three levels:

**Team level** (git-portable, shared):
```yaml
# .rh-agentic/team.yaml
domain_models:
  - nodepool-lifecycle.yaml
  - capi-abstraction-map.yaml
  - platform-matrix.yaml

pattern_library:
  path: .rh-agentic/patterns/
  review_required: true

personas:
  data-plane-engineer:
    domain_models: [nodepool-lifecycle, capi-abstraction-map]
    default_confidence_level: L3
    new_developer_guidance: true

  api-engineer:
    domain_models: [nodepool-lifecycle]
    focus: api-surface-only
    default_confidence_level: L2

sdlc:
  hierarchy: [rfe, feature, epic, story]
  phases: [design, api, implementation, test, review]
  ci:
    platforms: [aws, azure, kubevirt]
    flake_detection: true
    auto_retrigger: true

code_generation:
  triggers:
    api_change: "make api"
    verify: "make verify"
  validation:
    post_generation: "git diff --stat"
```

**Role level** (per developer):
```yaml
# .rh-agentic/roles/fatima.yaml
experience_level: senior
platforms_familiar: [aws, kubevirt]
platforms_learning: [azure, powervs]
default_mode:
  planning: collaborative
  api_changes: collaborative
  implementation: autonomous  # for established patterns
  core_logic: collaborative   # for new reconciliation logic
  tests: autonomous
  ci_debugging: autonomous
notification_preferences:
  blocking_failure: immediate
  flake_retrigger: silent
  pr_approved: notification
```

**Session level** (per task, ephemeral):
Adjusted in real time: "Execute Phase 1 full auto" or "Pair with me on this."

---

## Part IX: What Makes This Specifically Right for HyperShift's Data Plane

Let me be direct about why generic agentic development tools will fail for this domain and why this vision is necessary.

**1. The split-brain architecture is not just a deployment detail.**

It fundamentally shapes how you reason about changes. A developer who does not viscerally understand that the NodePool controller cannot talk to the hosted cluster's API server will write code that works in unit tests and fails in production. The domain model encodes this constraint so the system can catch violations before they reach CI.

**2. Platform-specific code is the primary source of bugs.**

Most NodePool bugs come from platform-specific behavior that was not anticipated during design. The platform variation matrix is not documentation -- it is a reasoning tool. When the system plans an implementation, it consults the matrix and identifies where platform divergence creates risk.

**3. CAPI abstraction violations are subtle and expensive.**

Reading a provider-specific Machine object in the NodePool controller works locally, passes unit tests, and breaks the architecture. These violations are invisible to code-level tools. The abstraction map makes them visible.

**4. Version skew is a real constraint, not an edge case.**

The CPO and hypershift-operator can be different versions. The CAPI core and provider controllers can be different versions. API changes that seem backward-compatible can break when the CPO is one version behind. The domain model tracks compatibility requirements.

**5. CI is slow and expensive.**

A full e2e suite takes 60-90 minutes per platform. Running all platforms takes most of a day. The system's confidence model should reduce unnecessary CI runs (when structural and behavioral confidence is already high) and diagnose CI failures quickly (distinguishing flakes from real failures, using the experience log of known flake patterns).

**6. The team is distributed and knowledge is siloed.**

An engineer who fixes an Azure-specific issue learns something that an engineer working on OpenStack support needs. Today, that knowledge transfers through PRs (if someone reads them), docs (if someone writes them), or meetings (if they happen to overlap). The knowledge compounding system makes this transfer automatic and contextual.

---

## Part X: What This is NOT

- **This is not AI-generated documentation.** The domain model is a working artifact that the system uses to reason, not a static document for humans to read.

- **This is not a chatbot for coding questions.** The system does not answer questions about Kubernetes. It carries the specific, deep context of HyperShift's data plane architecture and uses it to guide development.

- **This is not a code generation tool with a nice UI.** The code generation is one capability among many. The planning, the domain reasoning, the confidence model, the knowledge compounding -- these are the differentiators. Code generation is table stakes.

- **This is not an attempt to replace engineers.** The system amplifies engineers. Fatima is a better data plane engineer with this system because she spends her cognitive energy on architecture and design decisions, not on remembering which file needs a deepcopy annotation or which CI test is a known flake.

- **This is not a generic platform.** It is deeply, specifically built for this domain. A generic agentic tool will generate plausible-looking NodePool code that violates CAPI abstractions, ignores version skew, and fails on platforms the developer did not test. This system will not, because it has the domain model.

---

## Part XI: The Hard Problems I Have Not Solved

Intellectual honesty requires flagging these:

**1. Domain model maintenance.** Who updates the NodePool lifecycle model when the architecture changes? If the model drifts from reality, the system gives bad advice. This needs to be treated like code -- version-controlled, tested, reviewed. But the testing and validation mechanisms for domain models are an unsolved UX problem.

**2. Confidence calibration.** How do you know the confidence model is accurate? If it says "L3 confidence" but the code has a subtle bug, trust in the system erodes. Calibration requires feedback loops: track how often each confidence level correlates with post-merge issues. This data takes months to accumulate.

**3. Mode switching friction.** Moving between "full auto" and "pair with me" should be seamless. In practice, the context transfer between modes is hard. When Fatima says "pair with me on Phase 3" after the system autonomously completed Phases 1 and 2, the system needs to convey what it did and why, without a 20-minute briefing.

**4. Knowledge curation at scale.** Tier 2 knowledge (experience logs) will grow rapidly. Without curation, it becomes noise. Who reviews and promotes experience logs to Tier 1 patterns? Who retires outdated patterns? This is a governance problem masquerading as a technical one.

**5. Multi-agent coordination.** When Fatima and Cesar are both working on related NodePool changes, the system needs to detect potential conflicts early. This requires coordination between their separate agent sessions, which raises questions about privacy, context sharing, and merge conflict prediction.

---

## Closing

The vision I have described is ambitious but grounded in specific, real problems I see daily in data plane development. The current state is this: experienced engineers spend 40% of their time on mechanics (code generation, CI babysitting, pattern recollection, context rebuilding) and 60% on actual engineering (design, architecture, tradeoff analysis). The system I have described inverts this ratio. Maybe beyond it.

The key insight is not "AI writes code." It is "AI carries the domain model so the engineer can think at the right level of abstraction." For the data plane, the right level of abstraction is NodePool lifecycle semantics, CAPI contracts, and platform variation matrices -- not file paths and function signatures.

Build the domain model first. Everything else follows from it.
