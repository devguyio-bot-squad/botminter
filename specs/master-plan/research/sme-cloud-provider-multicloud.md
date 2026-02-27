# Dream-World UX Vision — Cloud Provider SME Perspective

> Independent vision from the Cloud Provider SME agent. No cross-pollination with other SME outputs.
> Part of [Shift Week Plan](shift-week-plan.md), input to [Living Codebase Vision](ux-living-codebase-vision.md)

---

## Part 1: The Fundamental Problem No One Is Naming

Before I design anything, let me name the elephant in the room.

HyperShift's multi-cloud complexity is not primarily a code problem. It is a **mental model problem**. When a developer touches the AWS cloud provider controller, they are not just writing Go code. They are holding in their head a model of:

- How AWS VPCs, subnets, security groups, and IAM roles interact
- How that maps to ClusterAPI's Machine, MachineSet, MachineDeployment abstractions
- How HyperShift's split-brain architecture means the control plane sees one thing and the data plane sees another
- How the NodePool controller reconciles desired state through CAPI providers
- How etcd encryption interacts with AWS KMS vs Azure Key Vault vs IBM HPCS
- What failure modes look like (eventual consistency in AWS, throttling in Azure, quota limits everywhere)

No engineer holds all of this for all platforms. The team's real asset is the **distributed cognition** across 15-20 engineers, each deeply expert in one or two platforms. Today, that knowledge lives in people's heads, in PR review comments that scroll off screen, in Slack threads that evaporate.

The dream system does not just help write code. It makes the team's distributed cognition **durable, queryable, and composable**.

---

## Part 2: The Paradigm Shift

### Kill the Terminal Loop

The current developer experience is:

```
think → type command → read output → think → type command → read output
```

This is a 1970s teletype interaction model wearing a 2025 costume. The dream system operates on a different paradigm:

**Intent-driven continuous collaboration with ambient awareness.**

The developer expresses intent at whatever level of abstraction they are operating at. The system maintains a live, evolving understanding of what is happening and surfaces the right information at the right time without being asked.

This is not "chat with a bot." This is closer to having a principal engineer sitting next to you who:
- Has read every PR in the last two years
- Knows every cloud provider's API intimately
- Can spin up infrastructure to validate assumptions
- Remembers what broke last time someone touched this code path
- Knows which team member is the expert on the thing you are struggling with

### The Three Layers

```
┌──────────────────────────────────────────────────────────┐
│                    INTENT LAYER                          │
│  Developer expresses what they want to achieve           │
│  "Make NodePool controller handle Azure spot VMs"        │
│  Natural language, JIRA reference, or sketch             │
└────────────────────────┬─────────────────────────────────┘
                         │
┌────────────────────────▼─────────────────────────────────┐
│                 UNDERSTANDING LAYER                       │
│  System builds a multi-dimensional model of the change    │
│  - Code impact graph (what files, what abstractions)      │
│  - Platform impact matrix (which clouds affected)         │
│  - API contract analysis (what breaks, what extends)      │
│  - Historical pattern matching (how was this done before) │
│  - Risk surface mapping (what could go wrong)             │
└────────────────────────┬─────────────────────────────────┘
                         │
┌────────────────────────▼─────────────────────────────────┐
│                  EXECUTION LAYER                          │
│  System acts with appropriate autonomy level              │
│  - Generates specs, code, tests                           │
│  - Provisions cloud infrastructure for validation         │
│  - Runs targeted verification                             │
│  - Manages PR lifecycle                                   │
│  - Debugs CI failures                                     │
└──────────────────────────────────────────────────────────┘
```

---

## Part 3: The Developer's Day

### Morning: The Briefing

Fatima opens her laptop. She does not open a terminal. She does not open Slack. She opens the **workspace view** -- a persistent, living document that the system maintains.

What she sees is not a dashboard. It is a **briefing**:

```
────────────────────────────────────────────────────────
WORKSPACE: HOSTEDCLUSTER-2847 - Azure Disk Encryption with Customer-Managed Keys

STATUS: Implementation Phase — Story 3 of 5

OVERNIGHT:
  ✓ CI run completed on Story 2 PR — all platforms green
  ✓ PR approved by Cesar, merged to main
  ! Azure throttling issue detected in e2e — added backoff
    (similar to AWS fix from HOSTEDCLUSTER-2103, applied same pattern)

TODAY'S STORY:
  "As a cluster admin, I can specify a DiskEncryptionSet ID
   in the NodePool spec, and worker nodes use CMK-encrypted OS disks"

WHAT THE SYSTEM ALREADY KNOWS:
  • Azure DiskEncryptionSet requires the cluster's managed identity
    to have Reader role on the DES resource
  • The existing AzureMachineTemplate in CAPI already supports
    OSDisk.ManagedDisk.DiskEncryptionSet — we need to thread it
    through HyperShift's NodePool → CAPI plumbing
  • Similar feature exists for AWS (KMS key in NodePool spec) —
    pattern reference: control-plane-operator/controllers/
    hostedcontrolplane/kas/aws.go
  • CPOv2 adapt function for this exists partially — needs extension
  • 3 files likely touched, 1 new file likely needed
  • Risk: Azure identity permissions are a common failure mode —
    the system recommends an explicit validation webhook

SUGGESTED PLAN:
  1. Extend NodePool API (api/hypershift/v1beta1/nodepool_types.go)
  2. Thread through to AzureMachineTemplate in the NodePool controller
  3. Add CPOv2 adapt function for DES configuration
  4. Add validation webhook for DES + identity permissions
  5. Write unit tests + extend Azure e2e suite

Ready to start, or want to adjust the plan?
────────────────────────────────────────────────────────
```

Notice what happened. The system did not wait to be asked. It:

1. Tracked overnight CI and reported results with context, not just "green/red"
2. Connected a fix it made to a historical precedent (knowledge compounding)
3. Pre-analyzed the next story using its understanding of both HyperShift's architecture AND Azure's cloud primitives
4. Found the analogous AWS implementation and offered it as a pattern reference
5. Identified a risk based on its knowledge of Azure identity failure modes
6. Proposed a plan at the right granularity -- not too detailed, not too vague

Fatima reads this in two minutes. She says: "Good, but skip the webhook for now. We will handle validation in a follow-up story. Start with the API change."

The system acknowledges, adjusts the plan, and begins.

### Mid-Morning: The Platform Impact Matrix

As the system works on the API change, it does something critical. It generates a **Platform Impact Matrix** -- not after the code is written, but as part of the design.

```
────────────────────────────────────────────────────────
PLATFORM IMPACT MATRIX — NodePool.Spec.Platform.Azure.DiskEncryptionSetID

                AWS     Azure   KubeVirt  OpenStack  PowerVS
                ─────   ─────   ────────  ─────────  ───────
API Change      none    NEW     none      none       none
CAPI Template   none    MODIFY  none      none       none
CPO Adapt Fn    none    NEW     none      none       none
Webhook         none    SKIP*   none      none       none
Unit Tests      none    NEW     none      none       none
E2E Tests       none    EXTEND  none      none       none

CROSS-PLATFORM CONCERNS:
  • NodePool reconciler's platform switch statement needs new case
    (nodepool_controller.go:847) — Azure-only, no cross-platform risk
  • The generic "encryption at rest" documentation should mention
    Azure CMK alongside existing AWS KMS docs
  • Feature gate: none needed (Azure platform already gated)

ANALOGOUS FEATURES BY PLATFORM:
  AWS:   NodePool.Spec.Platform.AWS.RootVolume.KMSKeyARN ✓ (shipped)
  Azure: NodePool.Spec.Platform.Azure.DiskEncryptionSetID (this work)
  IBM:   No equivalent yet (tracked: HOSTEDCLUSTER-3012)
  KV:    N/A (uses host encryption)
  OSP:   Barbican integration planned (no JIRA yet)

* Webhook skipped per developer decision — follow-up story
────────────────────────────────────────────────────────
```

This matrix is the system thinking like a cloud provider architect. It is not just checking "does this compile on all platforms." It is reasoning about:

- **API symmetry**: Does Azure's approach to this feature align with how AWS already does it? Should it? Where should patterns converge and where should they diverge?
- **Blast radius**: Which platform-specific code paths are touched? Which shared code paths?
- **Knowledge gaps**: IBM does not have an equivalent -- should a JIRA exist? (It already does, the system found it.)
- **Architectural coherence**: Is this feature being threaded through the right abstraction layers?

### Midday: Collaborative Implementation

The system has generated the API change and the CAPI template modification. Before writing the CPO adapt function, it pauses. This is a CPOv2 pattern, and the system knows the rules are strict.

```
────────────────────────────────────────────────────────
ATTENTION NEEDED — CPOv2 Adapt Function Design Decision

The adapt function for Azure DES configuration needs to set
the DiskEncryptionSet on the AzureMachineTemplate. Two approaches:

OPTION A: Inline in existing azure machine template adapt
  + Simpler, fewer files
  - The existing adapt function is already 180 lines
  - Mixes concerns (VM config + encryption config)

OPTION B: Separate adapt function composed via the reconciler
  + Clean separation of concerns
  + Matches how AWS KMS encryption is handled (separate adapt)
  + Easier to test in isolation
  - One more file, slightly more boilerplate

The system recommends OPTION B based on:
  1. Precedent: AWS KMS uses a separate adapt (kas/aws_kms.go)
  2. CPOv2 principle: adapt functions should be small, testable,
     single-purpose transformations
  3. Team convention: Cesar's review comment on PR #4521 explicitly
     asked for this separation pattern

Proceed with Option B, or discuss?
────────────────────────────────────────────────────────
```

This is where the system earns trust. It is not making an arbitrary decision. It cites:
- Precedent from the same codebase
- Architectural principles from CPOv2
- A specific team member's review comment that established convention

Fatima says "B, go ahead." The system writes the adapt function, the unit tests, and generates a test fixture that uses a mock Azure API response.

### Afternoon: The Cloud Reality Check

Here is where most agentic systems fall apart with cloud provider code. You cannot test Azure DiskEncryptionSet integration locally. The Azure API is remote, authentication is complex, and the interaction involves multiple Azure services (Compute, KeyVault, Identity).

The dream system handles this in layers:

**Layer 1: Contract Testing (immediate, local)**

The system generates tests against the Azure SDK's interfaces, not implementations. It creates mock responses that match Azure's actual API behavior, including edge cases the system has learned from past CI failures:

```
────────────────────────────────────────────────────────
CONTRACT TESTS GENERATED:

  TestDiskEncryptionSetAdapt_ValidDESID
  TestDiskEncryptionSetAdapt_EmptyDESID_NoOp
  TestDiskEncryptionSetAdapt_InvalidDESFormat
  TestDiskEncryptionSetAdapt_DESInDifferentSubscription
  TestDiskEncryptionSetAdapt_DESInDifferentRegion

  Edge cases from team knowledge:
  TestDiskEncryptionSetAdapt_DESWithExpiredKey
    (learned from OCPBUGS-18234 — Azure returns 409 when the
     key version in DES has been disabled in KeyVault)

All passing locally. ✓
────────────────────────────────────────────────────────
```

That last test case -- the expired key scenario -- is knowledge compounding in action. Someone on the team hit that bug six months ago. The system learned from the fix and now proactively tests for it on every encryption-related change.

**Layer 2: Simulated Cloud Validation (minutes, sandboxed)**

The system maintains lightweight cloud API simulators -- not full emulators, but enough to validate the interaction patterns. For Azure, this means:

- ARM template validation (does the generated resource definition parse?)
- RBAC chain validation (does the identity have the right permissions?)
- Resource dependency graph (is DES created before VM that references it?)

```
────────────────────────────────────────────────────────
SIMULATED AZURE VALIDATION:

  ARM Template:     ✓ Valid
  RBAC Chain:       ✓ ManagedIdentity → Reader → DiskEncryptionSet
  Resource Graph:   ✓ DES referenced correctly in VM definition
  API Versioning:   ✓ Using 2023-07-01 (latest stable for Compute)

  Note: Simulation cannot verify actual KeyVault key access.
  This will be validated in e2e. Estimated risk: LOW
  (pattern matches existing working configurations)
────────────────────────────────────────────────────────
```

**Layer 3: Targeted Cloud Validation (30-60 min, real infrastructure)**

For cloud-touching changes, the system can provision targeted, minimal infrastructure to validate the specific interaction. Not a full cluster -- just enough to test the contract.

```
────────────────────────────────────────────────────────
TARGETED AZURE VALIDATION (optional — estimated cost: $2.40)

  Would provision:
  - 1 DiskEncryptionSet in test resource group
  - 1 Managed Disk with CMK encryption
  - Validate encryption status via Azure API
  - Tear down immediately

  This validates the specific Azure API interaction without
  deploying a full cluster. Full e2e will run in CI.

  Run targeted validation? [skip / run / run-and-wait]
────────────────────────────────────────────────────────
```

Notice the cost estimate. Cloud infrastructure costs money. The system is transparent about this and lets the developer decide.

**Layer 4: Full E2E (60-90 min, CI)**

The system submits the PR and monitors CI. But it does not just wait and report red/green. It watches the CI logs in real time and provides analysis:

```
────────────────────────────────────────────────────────
CI STATUS: azure-e2e-disk-encryption

  [12:34] Cluster provisioning...
  [12:41] Control plane healthy
  [12:43] NodePool scaling to 2 workers...
  [12:47] ⚠ Worker node boot delayed — investigating...
  [12:48] Root cause: DiskEncryptionSet in different region
          than cluster. This is a test configuration issue,
          not a code issue. The test fixture hardcodes
          eastus but the CI cluster is in westus2.
  [12:48] Fixing test fixture and re-triggering...
────────────────────────────────────────────────────────
```

The system debugs CI failures in real time, distinguishes between code issues and environment issues, fixes what it can, and escalates what it cannot.

### Evening: The Handoff

Fatima's day is ending. The CI is still running. The system handles this gracefully:

```
────────────────────────────────────────────────────────
END OF DAY STATUS:

  PR #4892: Azure Disk Encryption with Customer-Managed Keys
  CI: Running (est. 25 min remaining)
  Reviews requested: Cesar (Azure), Dan (API)

  The system will:
  ✓ Monitor CI to completion
  ✓ Address review comments that match established patterns
  ✓ Flag review comments that need your judgment
  ✓ Update JIRA story status

  If CI fails:
  ✓ Analyze failure
  ✓ Fix if confidence > 90% and push
  ✓ Otherwise, have analysis ready for tomorrow's briefing

  Your tomorrow briefing will include full results.
────────────────────────────────────────────────────────
```

This is the autonomy spectrum in action. The developer configured this level of overnight autonomy. Another developer might say "never push without my approval." Another might say "handle everything unless it requires an API change."

---

## Part 4: The Seven Workflow Aspects, Realized

### 1. Spec-Driven Development: The Living Spec

The spec is not a static document written before coding and forgotten after. It is a **living artifact** that evolves with the implementation.

```
SPEC: HOSTEDCLUSTER-2847/Story-3

REQUIREMENT:
  NodePool spec accepts DiskEncryptionSetID for Azure platform.
  Workers boot with CMK-encrypted OS disks.

API CONTRACT:
  // Auto-generated from spec, validated against implementation
  type AzureNodePoolPlatform struct {
      // ...existing fields...
      DiskEncryptionSetID string `json:"diskEncryptionSetID,omitempty"`
  }

INVARIANTS:
  1. If DiskEncryptionSetID is set, all worker OS disks MUST use CMK
  2. DiskEncryptionSetID must be in the same subscription as the cluster
  3. Empty DiskEncryptionSetID means platform-managed encryption (default)
  4. DiskEncryptionSetID is immutable after NodePool creation

VALIDATION EVIDENCE:
  Invariant 1: Verified by TestDiskEncryptionSetAdapt_ValidDESID ✓
  Invariant 2: Verified by TestDiskEncryptionSetAdapt_DESInDifferentSubscription ✓
  Invariant 3: Verified by TestDiskEncryptionSetAdapt_EmptyDESID_NoOp ✓
  Invariant 4: Verified by webhook test (follow-up story) ○

DECISION LOG:
  - Separate adapt function (Option B) — per CPOv2 convention
  - Webhook deferred to follow-up story — developer decision
  - Field name matches Azure SDK naming convention
```

The spec links requirements to implementation to tests. A reviewer can read the spec and know, with evidence, whether the implementation matches the intent. This is how you build confidence in agentic code without line-by-line review.

### 2. Context Engineering: The Platform Knowledge Graph

This is where the cloud provider expertise becomes critical. The system maintains a structured knowledge graph of each cloud platform:

```
AZURE KNOWLEDGE GRAPH (excerpt):

DiskEncryptionSet
├── requires: KeyVault + Key
├── requires: ManagedIdentity with Reader role on DES
├── region: must match resources using it
├── API version: 2023-07-01 (Compute)
├── known issues:
│   ├── OCPBUGS-18234: expired key returns 409
│   ├── OCPBUGS-15789: cross-subscription DES needs explicit auth
│   └── CI-FLAKE-342: Azure API throttling during concurrent DES reads
├── related features:
│   ├── AWS equivalent: KMS Key ARN on RootVolume
│   ├── IBM equivalent: HPCS Key (planned)
│   └── KubeVirt: N/A (host-level encryption)
└── ClusterAPI mapping:
    └── AzureMachineTemplate.Spec.OSDisk.ManagedDisk.DiskEncryptionSet
```

This graph is built from:
- Azure documentation (automatically ingested and updated)
- HyperShift codebase analysis
- Historical PRs and bug fixes
- CI failure patterns
- Team members' review comments and Slack discussions (opt-in)

When a developer works on a feature involving DiskEncryptionSet, the system loads this context automatically. The developer does not need to know all of this -- the system uses it to make better suggestions, catch errors earlier, and generate more accurate code.

### 3. Prompt Engineering: Platform-Aware Prompts

The system does not use generic "write Go code" prompts. It uses platform-specific, architecture-aware prompt templates:

```yaml
# team-prompts/cpo-adapt-function.yaml
name: "CPOv2 Adapt Function"
trigger: "Creating or modifying a CPO adapt function"
context_required:
  - cpo_v2_patterns
  - target_platform_knowledge
  - existing_adapt_functions
constraints:
  - "Adapt functions MUST be pure transformations"
  - "No API calls, no side effects, no error handling beyond validation"
  - "Input: the resource being adapted + configuration"
  - "Output: the mutated resource"
  - "Follow existing naming: adapt{Resource}{Platform}.go"
validation:
  - "Function signature matches adapt pattern"
  - "No net/http imports"
  - "No client-go dynamic calls"
  - "Unit test covers empty/nil input"
platform_variants:
  aws:
    additional_context: "AWS resources use ARN format"
    examples: ["kas/aws_kms.go", "nodepool/aws_machine_template.go"]
  azure:
    additional_context: "Azure resources use ARM resource IDs"
    examples: ["kas/azure_kms.go", "nodepool/azure_machine_template.go"]
```

These prompts are version-controlled, reviewed by the team, and evolved over time. When someone discovers that the system consistently makes a mistake (say, using the wrong Azure API version), they update the prompt, and every team member benefits immediately.

### 4. Agent Memories: The Institutional Memory

The system remembers across sessions, across developers, across time:

```yaml
# .hypershift-agent/memories/azure-disk-encryption.yaml
created: 2025-08-14
last_accessed: 2026-02-11
contributors: [fatima, cesar, dan]

learnings:
  - id: azure-des-region-match
    source: CI failure in PR #4201
    lesson: "DiskEncryptionSet must be in same region as cluster"
    applied_in: [PR-4201-fix, PR-4892-test-fixture]

  - id: azure-des-expired-key
    source: OCPBUGS-18234
    lesson: "Azure returns HTTP 409 when key version in DES is disabled"
    applied_in: [PR-4892-edge-case-test]

  - id: azure-des-cross-sub
    source: Cesar's review on PR #4150
    lesson: "Cross-subscription DES requires explicit Azure AD auth setup"
    applied_in: [PR-4892-validation-webhook-spec]

patterns:
  - "Azure encryption features consistently require identity/RBAC validation"
  - "Always check region matching for Azure resources"
  - "Azure API throttling is more aggressive for encryption operations"
```

This is not a log file. It is structured, queryable knowledge that the system actively uses when working on related features.

### 5. Agent Tasks: The Dependency-Aware Pipeline

Tasks are not a flat list. They form a dependency graph that the system understands:

```
HOSTEDCLUSTER-2847: Azure CMK Disk Encryption
├── Story 1: API Design ✓ (merged)
├── Story 2: NodePool Controller ✓ (merged overnight)
├── Story 3: CPO Adapt Function ← CURRENT
│   ├── Task 3.1: Extend Azure NodePool platform types ✓
│   ├── Task 3.2: Write adapt function (in progress)
│   │   └── depends on: 3.1 (API types)
│   ├── Task 3.3: Unit tests
│   │   └── depends on: 3.2 (adapt function)
│   ├── Task 3.4: E2E test extension
│   │   └── depends on: 3.2, Story 2 merged ✓
│   └── Task 3.5: PR review + CI
│       └── depends on: 3.1, 3.2, 3.3, 3.4
├── Story 4: Documentation + CLI flags
│   └── depends on: Story 3
└── Story 5: Validation webhook
    └── depends on: Story 3
```

The system executes tasks respecting dependencies. It can parallelize where possible (write unit tests while e2e test fixture is being set up). It surfaces the critical path so the developer knows what actually matters for delivery.

### 6. Team Knowledge: The Compounding Advantage

This is the most important aspect and the hardest to get right.

**How knowledge compounds across the team:**

```
Week 1: Fatima implements Azure CMK encryption.
         System learns Azure DES patterns, RBAC requirements,
         region-matching constraints.

Week 2: Marcus implements IBM HPCS encryption for PowerVS.
         System suggests: "Azure CMK implementation (PR #4892)
         solved similar problems. Key patterns that transfer:
         - Separate adapt function for encryption config
         - Region/location matching validation
         - Key lifecycle edge cases (disabled, expired, rotated)
         Key patterns that DO NOT transfer:
         - Azure uses DES resource IDs; IBM uses HPCS instance CRN
         - Azure RBAC is per-resource; IBM IAM is per-service-instance
         - IBM HPCS requires explicit key unwrap for boot"

Week 3: Junior developer Amir works on OpenStack Barbican integration.
         System provides: "Two prior encryption implementations exist.
         Here is a COMPARISON of approaches and how Barbican maps:

         Concept          | AWS KMS     | Azure DES      | IBM HPCS    | Barbican
         Key reference    | ARN         | ARM resource ID | CRN         | Secret UUID
         Auth model       | IAM role    | Managed Identity| IAM policy  | Keystone
         HyperShift field | KMSKeyARN  | DESId          | HPCSCrn     | ?
         Adapt pattern    | kas/aws_kms | kas/azure_kms  | kas/ibm_kms | kas/osp_barbican

         Recommended field name: BarbicanSecretRef (matches OpenStack conventions)
         Recommended pattern: follow IBM HPCS (closest auth model to Keystone)"
```

Amir, on day one of this feature, has the synthesized knowledge of three prior implementations. He is not starting from zero. He is not reading three old PRs trying to extract the pattern. The system has already done that work and presented it in a form that accelerates his specific task.

**The knowledge is git-portable:**

```
.hypershift-agent/
├── team-knowledge/
│   ├── platform-patterns/
│   │   ├── encryption-at-rest.yaml      # Cross-platform encryption patterns
│   │   ├── identity-and-auth.yaml       # How each cloud handles auth
│   │   ├── machine-template-mapping.yaml # CAPI template per platform
│   │   └── failure-modes.yaml           # Known failure patterns per cloud
│   ├── architecture/
│   │   ├── cpov2-adapt-conventions.yaml  # CPOv2 rules and examples
│   │   ├── split-brain-patterns.yaml     # Management vs data plane patterns
│   │   └── nodepool-reconciler.yaml      # NodePool controller patterns
│   ├── ci/
│   │   ├── flaky-tests.yaml             # Known flakes and workarounds
│   │   ├── platform-test-matrix.yaml    # What runs where
│   │   └── debug-patterns.yaml          # Common CI failure → fix mappings
│   └── prompts/
│       ├── cpo-adapt-function.yaml
│       ├── api-type-extension.yaml
│       ├── nodepool-platform-feature.yaml
│       └── cross-platform-validation.yaml
├── memories/
│   ├── azure-disk-encryption.yaml
│   ├── aws-kms-rotation.yaml
│   └── ...per-feature memories
└── config/
    ├── autonomy-levels.yaml             # Per-developer autonomy preferences
    ├── platform-credentials.yaml        # Reference to credential stores
    └── ci-integration.yaml              # CI system configuration
```

All of this lives in the repo. It is reviewed like code. It evolves with the project. When someone leaves the team, their knowledge does not leave with them.

### 7. Agent Personas: Platform-Aware Roles

The system does not have one generic persona. It has **platform-aware specialist personas** that activate based on context:

```yaml
# personas/azure-specialist.yaml
name: "Azure Platform Specialist"
activates_when:
  - files_touched match "*/azure*"
  - JIRA labels include "azure"
  - NodePool platform type is "Azure"
expertise:
  - Azure Resource Manager APIs and conventions
  - Azure identity model (Managed Identity, Service Principal, RBAC)
  - Azure networking (VNet, NSG, Load Balancer, Private Link)
  - AzureMachine / AzureMachineTemplate CAPI types
  - Azure-specific HyperShift patterns and historical issues
review_focus:
  - ARM resource ID format validation
  - RBAC chain completeness
  - Region consistency
  - API version currency
  - Azure SDK error handling patterns
```

When a PR touches Azure code, the Azure specialist persona reviews it. When it touches shared code, the architecture persona reviews it. When it touches both, both review. This is not just prompting -- it is loading the right knowledge graph, the right memories, the right validation rules.

---

## Part 5: The Interaction Surfaces

### Primary: The Workspace (IDE-Integrated, Persistent)

The workspace is not a chat window. It is a persistent, structured document that lives alongside the code. Think of it as a living engineering notebook.

```
┌─────────────────────────────────────────────────────────────┐
│ WORKSPACE: HOSTEDCLUSTER-2847/Story-3                       │
│                                                             │
│ ┌─────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────┐ │
│ │  Briefing   │ │   Spec     │ │  Platform  │ │   CI     │ │
│ │  (current)  │ │  (living)  │ │  Matrix    │ │  Status  │ │
│ └─────────────┘ └────────────┘ └────────────┘ └──────────┘ │
│                                                             │
│ [Conversation thread below — scrollable, searchable]        │
│                                                             │
│ System: Generated adapt function. 3 tests passing.          │
│         Waiting for your review of the error handling        │
│         approach in adaptAzureDiskEncryption.go:47           │
│                                                             │
│ > [Your response here]                                      │
│                                                             │
│ ────────────────────────────────────────────                 │
│ QUICK ACTIONS:                                              │
│ [Review Code] [Run Tests] [Submit PR] [Ask Question]        │
│ [Adjust Autonomy] [View Platform Graph] [Team Knowledge]    │
└─────────────────────────────────────────────────────────────┘
```

The conversation is one part of the workspace, not the whole thing. The tabs at the top provide quick access to structured artifacts. The quick actions at the bottom surface the most likely next steps.

### Secondary: The Mobile Brief (Async, Notification-Driven)

When CI finishes at 11 PM, the developer does not need to open a laptop. They get a structured notification:

```
HYPERSHIFT AGENT — CI Complete

PR #4892: Azure CMK Disk Encryption
Result: ✓ All green (47 min)
Reviews: Cesar approved, Dan has 1 comment (non-blocking)

Dan's comment: "Should we add a release note for this?"
→ System suggestion: Yes, generated draft release note.

Action needed: Merge when ready (or reply "merge" to auto-merge)
```

This is not a Slack bot. It is a purpose-built notification that contains exactly the information needed to make a decision, with the ability to act directly from the notification.

### Tertiary: The Team Dashboard (Strategic, Weekly)

For leads and managers, a team-level view:

```
HYPERSHIFT MULTI-PLATFORM STATUS — Week of Feb 10

PLATFORM HEALTH:
  AWS:       ████████░░ 82% — 2 active features, 1 blocked on CI flake
  Azure:     █████████░ 91% — 1 feature in progress (CMK encryption)
  KubeVirt:  ██████████ 100% — stable, no active changes
  OpenStack: ███░░░░░░░ 30% — Barbican integration early stage
  PowerVS:   ██████░░░░ 60% — HPCS encryption in review

KNOWLEDGE GROWTH:
  12 new patterns learned this week
  3 cross-platform patterns identified
  1 pattern promoted to team-wide convention

RISK:
  AWS CI flake (TestNodePoolAutoRepair) — 3rd week. Recommend investigation.
  OpenStack Barbican — no team member has deep Barbican expertise.
    Suggestion: pair Amir with external Barbican SME.
```

---

## Part 6: The Confidence Model

This is the core question: how do you trust agentic code that touches cloud infrastructure?

### The Confidence Stack

Every agentic change carries a confidence score, built from multiple signals:

```
CONFIDENCE ASSESSMENT — PR #4892

PATTERN MATCH:        HIGH (92%)
  This change follows established patterns from 3 prior
  encryption implementations. No novel architecture.

CODE QUALITY:         HIGH (88%)
  Adapt function is pure. Tests cover core and edge cases.
  Naming follows conventions. No new dependencies.

PLATFORM CORRECTNESS: MEDIUM-HIGH (78%)
  Azure API usage matches SDK documentation.
  RBAC chain validated in simulation.
  Region constraint tested.
  Gap: no real Azure API call yet (will be in e2e).

CROSS-PLATFORM RISK:  LOW (95%)
  Changes are Azure-scoped. Shared code modified in 1 place
  (platform switch statement). Other platforms untouched.

CI VALIDATION:        HIGH (90%)
  All unit tests pass. Azure e2e pass. Other platform e2e
  not re-run (no shared code risk).

OVERALL CONFIDENCE:   HIGH (85%)

REVIEW RECOMMENDATION:
  Focus review on: adaptAzureDiskEncryption.go (new code)
  Skim: nodepool_types.go (standard API addition)
  Skip: test files (generated, pattern-following)
```

This confidence model lets reviewers allocate attention efficiently. Instead of reviewing every line, they focus on the areas where confidence is lower. The system has earned trust for pattern-following code; human attention goes to novel decisions.

### The Trust Ladder

Trust is built incrementally:

```
Level 0: SUGGEST
  System proposes changes, developer implements.
  Used for: new team members, novel architecture, risky changes.

Level 1: IMPLEMENT + REVIEW
  System writes code, developer reviews every change.
  Used for: established patterns, standard features.

Level 2: IMPLEMENT + SPOT-CHECK
  System writes code, developer reviews flagged areas only.
  Used for: high-confidence pattern-following changes.

Level 3: IMPLEMENT + VALIDATE
  System writes code, runs tests, developer validates results.
  Used for: routine fixes, test additions, documentation.

Level 4: AUTONOMOUS
  System handles the full cycle. Developer is notified of results.
  Used for: CI fixes, known-flaky-test workarounds, dependency updates.
```

Each developer configures their trust level per category. Each category's trust level can be different per platform:

```yaml
# Fatima's autonomy config
autonomy:
  api_changes: IMPLEMENT_REVIEW          # Always review API changes
  azure_platform_code: IMPLEMENT_SPOT_CHECK  # She knows Azure well
  aws_platform_code: IMPLEMENT_REVIEW    # Less familiar with AWS
  unit_tests: IMPLEMENT_VALIDATE         # Trust system for tests
  ci_debugging: AUTONOMOUS               # System can fix CI independently
  cross_platform_shared: IMPLEMENT_REVIEW # Always review shared code
```

---

## Part 7: The New Developer Experience

Day 1 for a new HyperShift developer today is overwhelming. The codebase is massive, multi-platform, and the architecture is non-obvious. The dream system transforms this.

### Day 1: Guided Orientation

```
────────────────────────────────────────────────────────
WELCOME TO HYPERSHIFT — Guided Orientation for Amir

Your first task: HOSTEDCLUSTER-3050
"Add Barbican secret reference to OpenStack NodePool spec"

Before we start, let me build your mental model.

HYPERSHIFT ARCHITECTURE (your feature's path):
  1. User creates NodePool with .spec.platform.openstack.barbicanSecretRef
  2. NodePool controller reads this field
  3. Controller generates CAPI OpenStackMachineTemplate with encryption config
  4. CAPI OpenStack provider creates the VM with encrypted disk
  5. This all happens on the MANAGEMENT cluster
  6. The worker node boots on the DATA PLANE (hosted cluster)
  7. The control plane and data plane communicate unidirectionally

YOUR FEATURE touches steps 1-4. Steps 5-7 are handled by existing machinery.

SIMILAR FEATURES TO STUDY:
  I have prepared a guided code walkthrough of the Azure CMK
  feature (most similar to yours). Want to start there?

YOUR TEAM:
  Azure encryption: Fatima (recently shipped, available for questions)
  OpenStack platform: Li Wei (primary maintainer)
  CPOv2 patterns: Dan (architect)

  Based on their calendars, Li Wei is available now if you want
  to pair on the OpenStack-specific aspects.
────────────────────────────────────────────────────────
```

The system does not throw documentation at Amir. It builds a mental model specific to his task, connects him to relevant prior work, and identifies the right humans to talk to. It serves as a bridge between the institutional knowledge and the new team member.

### Week 1: Scaffolded Implementation

The system sets Amir's autonomy level to SUGGEST by default. It walks him through each step, explaining why, not just what:

```
────────────────────────────────────────────────────────
STEP 1: API Type Extension

I will guide you through adding the Barbican field to the
OpenStack NodePool platform types.

FILE: api/hypershift/v1beta1/nodepool_types.go

CONTEXT: This file defines the NodePool API. Each platform
has a struct (AWSNodePoolPlatform, AzureNodePoolPlatform, etc.)
where platform-specific fields live.

The OpenStack struct is at line 847. Currently it has:
  - Flavor (VM size)
  - ImageName (OS image)

You need to add BarbicanSecretRef. Looking at how other
platforms handle similar fields...

Azure uses: DiskEncryptionSetID string
AWS uses:   KMSKeyARN string

For OpenStack + Barbican, the convention would be a secret
reference (UUID in Barbican). Recommended:

  BarbicanSecretRef string `json:"barbicanSecretRef,omitempty"`

Shall I add this, or would you like to write it yourself?
(Writing it yourself is recommended for learning — I will
review your change.)
────────────────────────────────────────────────────────
```

The system adapts to the developer's experience level. For Amir, it explains context. For Fatima, it would just do it.

---

## Part 8: Cross-Platform Intelligence

This is the capability that does not exist anywhere today and would be transformative for HyperShift.

### The Platform Abstraction Map

The system maintains a conceptual mapping across all platforms:

```
PLATFORM ABSTRACTION MAP — Encryption at Rest

CONCEPT: "Customer-managed encryption key for worker node OS disks"

Platform   | Key Service    | Key Reference    | Auth Model        | CAPI Field
-----------|----------------|------------------|-------------------|------------------
AWS        | KMS            | ARN              | IAM Role          | AWSMachine.Spec
Azure      | Key Vault      | ARM Resource ID  | Managed Identity  | AzureMachine.Spec
IBM        | HPCS           | CRN              | IAM Policy        | IBMMachine.Spec
OpenStack  | Barbican       | UUID             | Keystone Token    | OSMachine.Spec
KubeVirt   | N/A            | N/A              | Host-level        | N/A

COMMON PATTERN:
  - NodePool.Spec.Platform.{Platform}.{KeyReference}
  - Threaded through NodePool controller → CAPI template
  - Separate CPOv2 adapt function per platform
  - Validation: key exists, identity has access, region/location matches

DIVERGENT PATTERNS:
  - AWS KMS supports key rotation natively; others may not
  - Azure requires DES as intermediary (not direct key reference)
  - Barbican secrets are opaque; need explicit payload type check
  - KubeVirt delegates entirely to host — no HyperShift-level config
```

This map powers several capabilities:

**1. Automatic Impact Analysis**: When someone modifies the shared NodePool reconciler, the system knows which platforms are affected and how.

**2. Pattern Drift Detection**: If one platform's implementation diverges from the established pattern without justification, the system flags it.

**3. Feature Parity Tracking**: The system knows which features exist on which platforms and can generate a feature parity matrix for product planning.

**4. Cross-Platform Test Inference**: If a bug is found in AWS's encryption handling, the system can check whether the same bug could exist in Azure or IBM implementations, even if the code is completely different.

### The Cross-Platform Review

When a PR touches shared code, the system performs a cross-platform review:

```
────────────────────────────────────────────────────────
CROSS-PLATFORM REVIEW — PR #5001
"Refactor NodePool reconciler to support encryption lifecycle events"

This PR modifies shared NodePool reconciler code. Analyzing
impact across all platforms:

AWS:
  ✓ KMS key rotation event handling — works with refactor
  ✓ No behavioral change detected
  ⚠ New lifecycle hook called during reconcile — ensure AWS
    handler is registered (currently is: aws_encryption.go:34)

AZURE:
  ✓ DES reference handling — works with refactor
  ✗ ISSUE: New lifecycle hook expects encryption key ID as string.
    Azure uses DES resource ID (which is not the key ID).
    The DES → key resolution happens inside Azure. The hook
    will receive the DES ID, not the key ID.
    RECOMMENDATION: Make the hook accept a generic "encryption
    reference" rather than "key ID" to accommodate Azure's
    indirection model.

IBM:
  ✓ HPCS CRN handling — works with refactor
  ✓ No issues detected

OPENSTACK:
  ○ Not yet implemented — no impact

KUBEVIRT:
  ○ No encryption support — no impact

BLOCKING ISSUE: Azure indirection model not accommodated.
Suggest API change before merge.
────────────────────────────────────────────────────────
```

This is the kind of review that today requires a human who understands all five platforms deeply. No single person on the team has this. The system does.

---

## Part 9: Credential and Infrastructure Management

Cloud provider work has a unique challenge: credentials. The system handles this with a clear model:

### Credential Tiers

```
TIER 1: LOCAL DEVELOPMENT (no real credentials needed)
  - Mock APIs and simulated responses
  - Contract tests against interfaces
  - Used for: 80% of development work

TIER 2: TARGETED VALIDATION (short-lived, scoped credentials)
  - Ephemeral credentials from team credential store
  - Scoped to specific resource types (e.g., "can create DES, nothing else")
  - Time-limited (1 hour max)
  - Audit logged
  - Used for: validating specific cloud interactions

TIER 3: CI/E2E (managed by CI system)
  - Full credentials managed by CI infrastructure
  - Not accessible to developers or agents
  - Used for: full e2e validation

TIER 4: PRODUCTION (never touched by agents)
  - Production credentials are never accessible to the agentic system
  - Hard boundary, not configurable
```

The system knows which tier is needed for each operation and requests the minimum. It never stores credentials -- it uses short-lived tokens from a vault.

```
────────────────────────────────────────────────────────
CREDENTIAL REQUEST:

To validate Azure DES creation, I need a short-lived token with:
  - Scope: Microsoft.Compute/diskEncryptionSets/read
  - Resource Group: hypershift-ci-westus2
  - Duration: 30 minutes
  - Justification: Validate DES API interaction for PR #4892

This is a Tier 2 (Targeted Validation) request.

Approve? [yes / no / use-mock-instead]
────────────────────────────────────────────────────────
```

---

## Part 10: What Makes This Actually Work

Let me be direct about what separates a real vision from a fantasy.

### It Works Because It Is Incremental

You do not build all of this at once. The adoption path:

```
PHASE 1 (Month 1-2): Team Knowledge Foundation
  - Set up .hypershift-agent/ knowledge directory
  - Document platform patterns from existing code
  - Create initial prompt templates for common tasks
  - Value: immediate productivity gain from codified knowledge

PHASE 2 (Month 3-4): Single-Platform Agent
  - Pick one platform (probably AWS, most mature)
  - Implement the workspace + briefing for one developer
  - Implement confidence scoring for that platform
  - Value: proof of concept, gather feedback

PHASE 3 (Month 5-8): Multi-Platform Intelligence
  - Extend to Azure, then other platforms
  - Build the Platform Abstraction Map
  - Implement cross-platform review
  - Value: the system starts catching cross-platform issues

PHASE 4 (Month 9-12): Full Pipeline
  - CI integration and autonomous debugging
  - New developer onboarding flow
  - Team dashboard and analytics
  - Value: full agentic pipeline operational
```

### It Works Because Trust Is Earned

The system starts at Level 0 (SUGGEST) for everything. It earns trust by being right. Every correct suggestion, every caught bug, every accurate CI diagnosis builds the track record. Trust levels ratchet up based on evidence, not configuration.

### It Works Because Knowledge Is Versioned

Everything the system learns is in git. It can be reviewed, reverted, debated. Bad knowledge can be corrected. The system does not have a magic black box of "training" -- it has a transparent, auditable knowledge base that the team owns.

### It Works Because It Respects Cloud Provider Reality

The system understands that:
- Cloud APIs are eventually consistent (you cannot just "check if it worked")
- Cloud APIs are expensive (do not test by creating real resources when a mock suffices)
- Cloud APIs have different failure modes (Azure throttles differently than AWS)
- Credentials are sensitive and should be treated as radioactive
- Multi-cloud is not "same thing, different API" -- each cloud has genuinely different models

---

## Part 11: What This Is NOT

To be clear about boundaries:

**It is not replacing humans.** It is augmenting a team of 15-20 engineers to operate like a team of 50, with better consistency and less knowledge loss.

**It is not a chatbot.** The conversation interface is one part of a larger workspace. Most of the system's value comes from its ambient awareness, its knowledge graph, and its automated analysis -- not from chat.

**It is not cloud-agnostic.** It is deeply, specifically cloud-aware. It knows the difference between an ARN and a CRN. It knows Azure uses managed identities while AWS uses IAM roles. This specificity is the point.

**It is not a one-size-fits-all tool.** A new developer and a principal engineer have fundamentally different needs. The system adapts to both, with the same underlying knowledge.

**It is not magic.** It is structured knowledge plus targeted automation plus thoughtful UX. Every capability described here is technically feasible today. The innovation is in the integration and the domain specificity.

---

## Summary: The Three Big Ideas

**1. The Platform Knowledge Graph replaces tribal knowledge.**
Instead of "ask Cesar about Azure," the system holds and synthesizes platform knowledge across the entire team. New developers get the benefit of years of accumulated expertise on day one.

**2. The Confidence Stack replaces line-by-line review.**
Instead of reviewing every line of agentic code, reviewers focus attention where confidence is low. This is how you scale agentic development without losing quality -- not by trusting blindly, but by measuring trust precisely.

**3. Cross-Platform Intelligence catches what no human can.**
No single engineer holds all five platforms in their head. The system does. When shared code changes, it reasons about impact across all platforms simultaneously. This is a capability that does not exist on the team today, not because the engineers are not good enough, but because the cognitive load is inhuman.

The future of multi-cloud development is not "AI writes code." It is "AI holds the complexity so humans can focus on judgment." HyperShift is the perfect domain for this because its complexity is genuine, structured, and well-documented -- exactly the kind of complexity that machines can manage and humans should not have to hold alone.
