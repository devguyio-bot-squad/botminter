# Dream-World UX Vision — HCP Architect SME Perspective

> Independent vision from the HCP Architect SME agent. No cross-pollination with other SME outputs.
> Part of [Shift Week Plan](shift-week-plan.md), input to [Living Codebase Vision](ux-living-codebase-vision.md)

---

## Preamble: Why Everything We Have Now Is Wrong

Let me be direct. Every agentic coding tool today — Claude Code, Copilot Workspace, Cursor, Devin — treats code as the primary artifact. They are text editors with intelligence bolted on. They start from files and work outward.

For HyperShift, this is fundamentally backwards.

HyperShift is not a codebase. It is a **distributed system specification** that happens to be expressed in Go, YAML, and shell scripts. The primary artifact is not code — it is **architectural intent**. A developer changing how OIDC provider configuration propagates from HostedCluster to the data plane is not "editing files." They are reshaping a data flow that crosses trust boundaries, traverses version skew, touches three different reconciliation loops, and must behave correctly under five different cloud provider implementations — each with its own identity model.

The dream-world system does not start from code. It starts from **architecture**.

---

## Part I: The Paradigm Shift — Architecture as the Primary Interface

### The Fundamental Insight

The developer's mental model when working on HyperShift is never "I need to edit this file." It is always one of:

1. "I need to add a new capability to the platform" (feature development)
2. "Something is wrong in this layer" (debugging)
3. "This architectural pattern needs to change" (refactoring)
4. "A new product needs to integrate" (platform extension)

All four start from **where in the architecture** the work lives. The system should too.

### The Living Architecture Model

The core of the system is not a file tree. It is a **living, queryable, version-aware architectural model** of the entire HCP stack. Think of it as a continuously maintained mental model — the one that currently exists only inside the heads of the senior engineers and is transmitted through years of code review and tribal knowledge.

This model understands:

**Vertical Layers:**
```
┌─────────────────────────────────────────────────────────────┐
│  Product Layer (ROSA / ARO / ROKS / Self-Hosted)            │
│  - SLA contracts, operational models, billing integration    │
│  - Product-specific controllers, webhooks, admission        │
├─────────────────────────────────────────────────────────────┤
│  Orchestration Layer (HyperShift Operator)                   │
│  - HostedCluster / NodePool API, platform abstraction        │
│  - Version management, upgrade orchestration                 │
├─────────────────────────────────────────────────────────────┤
│  Control Plane Layer (Control Plane Operator)                │
│  - Per-cluster control plane lifecycle                       │
│  - CPOv2 adapt functions, component manifests                │
│  - Version-skewed from orchestration layer                   │
├─────────────────────────────────────────────────────────────┤
│  Data Plane Layer (Hosted Cluster)                           │
│  - Operators running inside guest cluster                    │
│  - User workloads, user-facing APIs                          │
│  - MUST NOT leak management-side state                       │
├─────────────────────────────────────────────────────────────┤
│  Infrastructure Layer (Cloud Provider)                       │
│  - AWS / Azure / KubeVirt / OpenStack / PowerVS / Agent      │
│  - Credential management, resource lifecycle                 │
│  - Platform-specific networking, storage, identity           │
└─────────────────────────────────────────────────────────────┘
```

**Horizontal Concerns (crossing all layers):**
- Trust boundaries and credential flow
- Version skew tolerance
- Tenant isolation
- Upgrade sequencing
- API compatibility contracts

**Invariants (hard rules the system enforces):**
- Communication is management-to-data-plane only. Never reverse.
- Control plane namespace isolation via network policy and RBAC.
- Worker nodes run user workloads only. Nothing else.
- No mutable CRDs/CRs on data plane that can break HyperShift-managed features.
- Data plane changes never trigger management-side lifecycle actions.
- HyperShift components never own or manage user infrastructure credentials.
- Upgrade signal decoupling: management components upgrade independently of data plane components.

This is not a static diagram. It is a **live, code-derived model** that the system maintains by analyzing the actual codebase, API definitions, controller registrations, and RBAC manifests. When you change code, the model updates. When the model shows a violation, it tells you before you push.

---

## Part II: The Developer's Day — A Narrative

### Morning: Picking Up Work

Ahmed opens his laptop at 9am. He does not open a terminal. He does not open an IDE. He opens the **Workspace** — a persistent, spatial environment that looks nothing like a text editor.

The workspace shows his current context: three active stories, one blocked. The system has been working overnight. Not coding — **thinking**. It analyzed a Slack thread where Cesar mentioned that the Azure OIDC implementation diverges from AWS in how it handles token audience validation. The system flagged this as relevant to Ahmed's current story about OIDC provider consolidation and added it to the story's context, with a link to the specific code paths that diverge and an architectural annotation explaining why.

The workspace surface is organized around **the architecture, not the filesystem**. Ahmed sees:

```
╔══════════════════════════════════════════════════════════════════╗
║  ACTIVE: OCPBUGS-12345 — Consolidate OIDC provider config        ║
║                                                                  ║
║  ┌──────────┐    ┌───────────┐    ┌──────────┐    ┌──────────┐   ║
║  │ Product  │    │ HyperShift│    │ Control  │    │ Data     │   ║
║  │ Layer    │    │ Operator  │    │ Plane Op │    │ Plane    │   ║
║  │          │    │           │    │          │    │          │   ║
║  │ ROSA: ●  │───▶│ HC API  ● │───▶│ CPOv2  ● │───▶│ kube-  ○ │   ║
║  │ ARO:  ◐  │    │ NP API  ○ │    │ adapt  ● │    │ apisvr ○ │   ║
║  │ ROKS: ○  │    │ reconcile │    │ deploy ● │    │ oauth  ○ │   ║
║  │          │    │           │    │          │    │          │   ║
║  └──────────┘    └───────────┘    └──────────┘    └──────────┘   ║
║                                                                  ║
║  ● = changes required   ◐ = impact uncertain   ○ = read-only     ║
║                                                                  ║
║  INVARIANT ALERTS:                                               ║
║  ⚠ Version skew: CPO adapt function must handle both             ║
║    4.16 and 4.17 HostedCluster shapes                            ║
║  ⚠ ARO: Token audience validation uses different claim           ║
║    (from Cesar's Slack, 2026-02-10)                              ║
║                                                                  ║
║  CONFIDENCE: 62% — needs spec review, ARO impact unclear         ║
╚══════════════════════════════════════════════════════════════════╝
```

This is not a dashboard. It is the system's **understanding of the change** rendered spatially. Each box is interactive — Ahmed can zoom into any layer and see the specific controllers, API fields, and data flows affected. The lines between boxes represent the actual communication paths in the running system (ConfigMaps synced via the control plane operator, Secrets pushed from management to data plane, status propagated back through the token endpoint).

Ahmed taps the ARO box where it shows "impact uncertain." The system explains:

> "ARO uses a federated identity model where the OIDC issuer URL is tied to an Azure AD application registration. Your current spec proposes consolidating the OIDC provider configuration into a unified struct on HostedCluster.Spec.Configuration. This works for ROSA, where the OIDC provider is an S3 bucket with a CloudFront distribution. But for ARO, the OIDC issuer URL is managed by the ARO resource provider and has lifecycle coupling to the Azure AD app registration. I do not have enough context to determine whether your proposed API change preserves this coupling correctly. I recommend pulling in the ARO-specific OIDC controller code and the Azure platform type's CreateOrUpdate flow before proceeding."

Ahmed says: "Pull in that context and show me the divergence."

The system does not just show two files side-by-side. It renders a **data flow comparison**:

```
ROSA OIDC Flow:
  HostedCluster.Spec.Platform.AWS.OIDCProviderARN
    → HO reconciler creates S3 bucket + CloudFront
    → CPO deploys kube-apiserver with --oidc-issuer-url pointing to S3
    → Token signing key stored in management namespace Secret
    → Public key published to S3 /.well-known/openid-configuration

ARO OIDC Flow:
  HostedCluster.Spec.Platform.Azure.ManagedIdentities
    → ARO RP (external) creates Azure AD application
    → HO reconciler receives OIDC issuer URL from ARO RP via annotation
    → CPO deploys kube-apiserver with --oidc-issuer-url pointing to Azure AD
    → Token signing key managed jointly (rotation requires RP coordination)
    → DIVERGENCE: issuer URL is NOT owned by HyperShift, it is owned by ARO RP
```

Ahmed now understands the problem. The OIDC provider config cannot be fully consolidated because ownership differs across platforms. This is not something any file-level code tool would have caught. It requires understanding the **ownership model** across the product boundary.

### Mid-Morning: Spec-Driven Design

Ahmed decides to refine the spec before writing any code. He tells the system:

"I want to consolidate OIDC configuration into a single API surface that works across platforms, but preserves platform-specific ownership semantics."

The system enters **Spec Mode**. This is not "write me a design doc." This is an interactive architectural design session where the system acts as a co-architect — one who knows every API type, every controller, every platform-specific branch in the codebase.

The system proposes:

> **Option A: Unified struct with platform overrides**
> Add `HostedCluster.Spec.Configuration.Authentication.OIDCProvider` as the canonical location. Platform-specific controllers can set defaults, and the field carries an `ownership` annotation (similar to Server-Side Apply field management) indicating whether the value is platform-managed or user-managed.
>
> **Pros:** Single API surface. Clear ownership semantics. CPOv2 adapt functions only read from one location.
> **Cons:** Requires migration path for existing clusters. The ownership annotation is a new pattern — increases API surface complexity.
>
> **Option B: Platform-specific OIDC remains, with computed status**
> Keep platform-specific OIDC config where it is, but add a computed `HostedCluster.Status.Authentication.EffectiveOIDCProvider` that controllers read from. The HO reconciler populates this status field based on platform type.
>
> **Pros:** No migration. No API change for existing platforms. CPOv2 adapt functions read from status (already a common pattern for computed fields).
> **Cons:** Status fields are not settable by users. If a self-hosted user needs to customize OIDC config, they cannot use this path. Adds indirection.
>
> **Option C: Configuration.Authentication.OIDCProviders as a list**
> Model OIDC providers as a list with a `purpose` field (e.g., `ServiceAccount`, `UserAuth`). This anticipates the future where clusters may have multiple OIDC providers (external OIDC is already a thing).
>
> **Pros:** Future-proof. Clean API. Each provider entry carries its own source/ownership metadata.
> **Cons:** Larger API change. More complex CPOv2 adapt logic. Requires careful defaulting for single-provider clusters.
>
> **My recommendation is Option B** for the following reasons:
> 1. It respects the existing ownership boundaries. ARO RP does not need to change.
> 2. It follows the existing pattern of computed status fields (e.g., `HostedCluster.Status.Version`, `HostedCluster.Status.KubeConfig`).
> 3. CPOv2 adapt functions already prefer reading from status for derived values — this is consistent.
> 4. The version skew concern is minimized because status is always written by the current HO version.
> 5. Self-hosted users who need custom OIDC can still set it via the platform-specific path.

Ahmed pushes back: "But the platform-specific fields are a mess. AWS has OIDCProviderARN, Azure has it buried in managed identities, KubeVirt does not have it at all. I want to clean this up."

The system does not capitulate. It responds:

> "You are right that the platform-specific fields are inconsistent. But cleaning them up is a multi-release migration with API deprecation policy implications. You would need:
> 1. New field introduced in 4.18
> 2. Both old and new fields supported in 4.18, 4.19 (minimum two releases per OpenShift deprecation policy)
> 3. Old field removed in 4.20 at earliest
>
> Your current story is scoped to OIDC consolidation for CPOv2 adapt function simplification. If you pursue Option A or C, the story expands to include API design review (architecture call), migration controller implementation, e2e test updates across all platforms, and doc updates.
>
> I recommend: pursue Option B now (story-scoped), file a follow-up RFE for API cleanup (Option C) targeting 4.19."

This is the system acting as an architect, not a code generator. It knows the OpenShift deprecation policy. It knows the release cadence. It knows the JIRA hierarchy (this story should not balloon into an RFE). It knows the team process (architecture calls for API changes).

Ahmed agrees. He asks the system to write the spec.

The system produces a **structured spec** — not a Google Doc, but a machine-readable, version-controlled artifact that will drive implementation:

```yaml
# specs/oidc-consolidation/spec.yaml
kind: FeatureSpec
metadata:
  story: OCPBUGS-12345
  epic: HOSTEDCP-2345
  author: ahmed
  reviewers: [cesar, alberto, enxebre]
  status: draft

architecture:
  layers_affected:
    - hypershift-operator  # writes Status.Authentication.EffectiveOIDCProvider
    - control-plane-operator  # reads from status in adapt functions
  layers_read_only:
    - product  # ROSA/ARO/ROKS controllers not changed
    - data-plane  # no data plane changes

  invariants:
    unidirectional_communication: preserved  # status is management-side only
    version_skew_tolerance: |
      Older CPO versions ignore the new status field and continue reading
      from platform-specific spec fields. No breakage.
    tenant_isolation: preserved  # no cross-namespace reads
    credential_handling: not_affected

api_changes:
  - group: hypershift.openshift.io
    version: v1beta1
    kind: HostedCluster
    field: status.authentication.effectiveOIDCProvider
    type: new_field
    go_type: OIDCProviderStatus
    description: |
      Computed field populated by the HostedCluster reconciler.
      Contains the effective OIDC provider configuration regardless
      of platform type.

  - go_type_definition:
      name: OIDCProviderStatus
      fields:
        - name: issuerURL
          type: string
          description: "The OIDC issuer URL for this cluster's service account tokens"
        - name: source
          type: OIDCProviderSource  # enum: AWS, Azure, KubeVirt, Manual
          description: "Indicates which platform path determined this value"

implementation:
  phases:
    - name: api
      description: "Add OIDCProviderStatus type and status field"
      generated_artifacts: [deepcopy, client, informer]

    - name: ho-reconciler
      description: "Populate status.authentication.effectiveOIDCProvider in HC reconciler"
      per_platform: true
      platforms: [aws, azure, kubevirt, openstack, powervs, agent, none]

    - name: cpo-adapt
      description: "Update CPOv2 adapt functions to prefer status field with fallback"
      components: [kube-apiserver, oauth-server]

    - name: tests
      description: "Unit tests for each platform, e2e for ROSA and self-hosted"

validation:
  unit_tests:
    - "Each platform's HC reconciler correctly populates the status field"
    - "CPOv2 adapt functions use status field when present"
    - "CPOv2 adapt functions fall back to platform-specific fields when status absent"
    - "OIDCProviderStatus type round-trips through serialization"
  e2e_tests:
    - platform: aws
      test: "Create HC, verify kube-apiserver OIDC config matches status field"
    - platform: none
      test: "Create HC with manual OIDC config, verify status reflects it"
  invariant_checks:
    - "No data plane resources reference the new status field"
    - "No management-to-data-plane communication pattern changed"
```

This spec is the **contract**. It drives everything downstream. The system will not generate code that deviates from it. When reviewing PRs, the system validates against it. When running tests, it knows which tests matter for which phase.

### Afternoon: Implementation with Architectural Guardrails

Ahmed is ready to implement. He tells the system to start with the API phase.

The system does not just generate Go types. It:

1. **Checks the spec** for the exact type definition
2. **Analyzes existing API patterns** in the HyperShift codebase (how other status subresources are structured, naming conventions, godoc patterns)
3. **Generates the type** following those patterns exactly
4. **Runs `make api`** to generate deepcopy, clients, informers
5. **Validates** that the generated code compiles
6. **Runs the API linter** (if one exists) or applies known API review criteria
7. **Shows Ahmed the diff** — not as a raw git diff, but as an **architectural diff**:

```
ARCHITECTURAL DIFF: API Phase
─────────────────────────────
  HostedCluster (hypershift.openshift.io/v1beta1)
    .status
      + .authentication              (new group)
        + .effectiveOIDCProvider     (OIDCProviderStatus)
          + .issuerURL               string
          + .source                  OIDCProviderSource (enum)

  New Types:
    OIDCProviderStatus              (api/hypershift/v1beta1/types.go)
    OIDCProviderSource              (enum: AWS, Azure, KubeVirt, Manual)

  Generated Artifacts:
    zz_generated.deepcopy.go        (updated)
    client/...                       (updated)
    informers/...                    (updated)

  Invariant Check: PASS
    ✓ No new CRDs exposed on data plane
    ✓ Status field (not spec) — no user-settable mutation surface
    ✓ No new RBAC requirements
```

Ahmed reviews this in seconds. Not line-by-line code review — architectural review. He confirms. The system commits the API phase separately (clean git history, one phase per commit, reviewable in isolation).

Now the HO reconciler phase. The system knows this is per-platform. It:

1. Reads the existing platform-specific reconciliation code for each platform type
2. Identifies where OIDC configuration is currently set
3. Generates a status population function that runs after the existing reconciliation
4. For each platform, it ensures the function reads only from data already available in the reconciliation context (no new API calls, no new client permissions)
5. **Critical**: It checks that the new code does not violate the unidirectional communication invariant — the status field must be computed entirely from management-side data

For the CPOv2 adapt phase, the system understands the framework's constraints:

- Adapt functions must be pure transformations: `(existingResource, ownerRef, releaseImage) -> desiredResource`
- No side effects (no API calls, no status writes)
- The adapt function receives a deployment/config context that includes relevant status

The system generates adapt function changes that read from the status field with a fallback to the old platform-specific path, maintaining backward compatibility for older HO versions that have not populated the new status field yet.

### Late Afternoon: The Cross-Product Impact Check

Before Ahmed creates a PR, the system runs something unique: a **Cross-Product Impact Analysis**.

This is not a test. It is a **simulation**. The system has models of how ROSA, ARO, ROKS, and self-hosted deployments differ. It runs the change through each model:

```
CROSS-PRODUCT IMPACT ANALYSIS
══════════════════════════════

ROSA (AWS):
  OIDCProviderARN → status.authentication.effectiveOIDCProvider.issuerURL
  Source: AWS
  Risk: LOW — straightforward mapping, OIDC provider fully owned by HyperShift
  SLA Impact: None — no change to steady-state behavior

ARO (Azure):
  ManagedIdentities.OIDCIssuerURL → status.authentication.effectiveOIDCProvider.issuerURL
  Source: Azure
  Risk: MEDIUM — OIDC issuer URL is set by ARO RP via annotation
  Question: Does the HO reconciler run AFTER the ARO RP sets the annotation?
    Analysis: Yes — HO reconciler watches HostedCluster, ARO RP writes the
    annotation, HO reconciler re-reconciles and will populate the status field.
    BUT: There is a window where the status field is empty (between HC creation
    and ARO RP annotation). CPOv2 fallback handles this.
  SLA Impact: None — steady-state unchanged, transient empty status is handled

ROKS (IBM Cloud):
  PowerVS/IBMCloud OIDC configuration path: NOT FOUND IN CODEBASE
  Risk: UNKNOWN — I cannot find where ROKS configures OIDC
  Recommendation: Check with the IBM team or search in the downstream fork

Self-Hosted (None/Agent):
  Manual OIDC configuration → status.authentication.effectiveOIDCProvider.issuerURL
  Source: Manual
  Risk: LOW — user sets spec, HO reconciler copies to status
  SLA Impact: None

OVERALL CONFIDENCE: 78%
BLOCKERS: ROKS OIDC path unknown — cannot confirm compatibility
```

The system just prevented a potential production incident on ROKS by identifying a blind spot before the PR was even created. It does not pretend to know what it does not know. It says "I cannot find this — ask the IBM team."

### Evening: The PR and CI

Ahmed creates the PR. The system:

1. Generates a PR description that includes the architectural diff, the cross-product impact analysis, and links to the spec
2. Annotates the PR with **review hints** — not for humans, but for the system itself and other team members' systems:

```markdown
## Review Focus Areas

### Architectural Review (recommended: Alberto, Cesar)
- New status subresource pattern: `status.authentication.effectiveOIDCProvider`
- Is `OIDCProviderSource` enum extensible enough for future platforms?
- Version skew handling: fallback in CPOv2 adapt functions

### Platform Review (recommended: per-platform owners)
- AWS: `pkg/controllers/hostedcluster/aws/oidc.go` — status population
- Azure: `pkg/controllers/hostedcluster/azure/oidc.go` — annotation dependency
- KubeVirt: `pkg/controllers/hostedcluster/kubevirt/oidc.go` — new file

### NOT in scope (do not review for this PR)
- API cleanup / deprecation of platform-specific OIDC fields (future RFE)
- Data plane changes (none in this PR)
```

CI runs. A test fails. Not a flaky test — a real failure. The system does not just show the log. It **diagnoses architecturally**:

> "e2e test `TestCreateClusterAWS` failed at step 'verify kube-apiserver OIDC configuration.' The kube-apiserver pod has `--oidc-issuer-url` set to the old platform-specific value, not the value from the new status field. Root cause: the CPOv2 adapt function for kube-apiserver is reading the status field, but the test's mock HostedCluster does not have the status field populated (the test creates the HC directly without running the HO reconciler). Fix: update the test fixture to include `status.authentication.effectiveOIDCProvider`, or modify the adapt function to fall back when the status field is empty (which it already should — checking...)
>
> Found: the fallback IS present, but it checks `len(status.Authentication.EffectiveOIDCProvider.IssuerURL) > 0`, and the test fixture has the field present but with an empty string. The fallback condition should be `status.Authentication != nil && status.Authentication.EffectiveOIDCProvider != nil && len(status.Authentication.EffectiveOIDCProvider.IssuerURL) > 0`.
>
> This is a nil-safety issue, not an architectural issue. Fixing."

The system understands the difference between "the architecture is wrong" and "the code has a nil check bug." It does not waste Ahmed's time with the latter.

---

## Part III: The Interaction Surfaces

### The Workspace (Primary Surface)

Not an IDE. Not a browser. A **spatial, persistent environment** that can manifest on any screen.

On a laptop, it is a desktop app with a spatial canvas. On a phone, it is a focused, single-story view with swipe navigation. On a large monitor, it is a multi-pane architectural view.

The key insight: **the workspace is organized around the architecture, not the filesystem**. You do not see `pkg/controllers/hostedcluster/`. You see the HyperShift Operator layer, with its controllers organized by function (lifecycle, networking, storage, identity) and by platform.

When you zoom into a controller, you see it in architectural context: what it reads, what it writes, what it watches, what it owns. The code is there — you can read it and edit it — but it is presented as an implementation of an architectural component, not as a standalone file.

### The Conversation (Secondary Surface)

Voice-first when appropriate. Text when precise. The system supports natural conversation:

"What happens if the ARO resource provider goes down while we are trying to populate the OIDC status?"

The system responds with both text and a visual — a sequence diagram showing the reconciliation flow with the failure point highlighted, the retry behavior, and the steady-state recovery.

The conversation is contextual. It knows what story Ahmed is working on, what phase he is in, what files he has open, what tests have run. Ahmed never has to re-explain context.

### The Review Surface (Team Surface)

When reviewing someone else's PR, the system does not show a diff. It shows an **architectural impact assessment** and lets the reviewer choose their depth:

- **Architectural Review**: "Does this change respect the invariants? Does the API design follow conventions?" (5 minutes)
- **Implementation Review**: "Is the code correct, efficient, well-tested?" (15 minutes)
- **Deep Review**: "Show me everything — every line, every generated file, every test." (60 minutes)

The reviewer's system has its own architectural model and can independently verify invariants. If Cesar reviews Ahmed's PR, Cesar's system checks the ARO-specific impact using Cesar's accumulated knowledge of the ARO platform. Knowledge compounds across the team.

---

## Part IV: The Seven Aspects — Reimagined for HyperShift

### 1. Spec-Driven Development: Architecture-First Specs

Specs are not documents. They are **executable architectural contracts**. The spec YAML shown earlier is not just documentation — it drives code generation, test selection, review guidance, and CI validation.

The spec knows about layers, invariants, platform-specific behavior, and version skew. It is the single source of truth for what the change IS, and everything else (code, tests, PRs) is derived from it or validated against it.

Specs are versioned, reviewable, and diffable. When a spec changes, the system knows which downstream artifacts need to change too.

### 2. Context Engineering: The Architectural Knowledge Graph

Context is not "load these files into the prompt." Context is a **knowledge graph** of the entire HCP architecture.

The graph contains:
- Every API type and its fields, with platform-specific semantics
- Every controller and its reconciliation targets
- Every communication path between management and data plane
- Every platform-specific divergence point
- Every architectural invariant and its enforcement mechanism
- Every known product constraint (ROSA SLA, ARO RP dependencies, ROKS requirements)

When the system needs to load context for a task, it traverses the graph to find the relevant subgraph. For Ahmed's OIDC change, it pulls in: the HostedCluster API type (but only the OIDC-relevant fields), the platform-specific OIDC controllers (but not the networking controllers), the CPOv2 adapt functions for kube-apiserver and oauth-server (but not for etcd or kube-controller-manager), and the relevant e2e test fixtures.

This is surgical context loading. Not "stuff 200 files into the prompt and hope for the best."

### 3. Prompt Engineering: Architectural Prompts

Prompts are organized by architectural layer and concern, not by generic task type.

Not: "Write a unit test"
But: "Write a unit test for a CPOv2 adapt function that handles version skew between HO 4.17 and CPO 4.16"

Not: "Review this PR"
But: "Review this PR for invariant violations in the management-to-data-plane communication boundary"

These prompts encode deep domain knowledge. They are version-controlled, team-maintained, and continuously refined based on what actually catches bugs. When a prompt fails to catch an issue that a human reviewer caught, the prompt gets updated. The prompt library is a **living encoding of the team's review expertise**.

### 4. Agent Memories: Architectural Decision Records

Memories are not "facts the system remembers." They are **architectural decisions with context**.

> "On 2026-01-15, we decided that CPOv2 adapt functions should read OIDC configuration from status rather than spec, because the status field is always populated by the current HO version and avoids version skew issues. This was discussed in the architecture call. Cesar raised the concern about ARO RP timing, and we agreed that the fallback path handles it."

These memories are linked to the architectural knowledge graph. When someone later tries to move OIDC configuration back to spec, the system surfaces this decision and its rationale. Architectural decisions are not lost when people leave the team.

### 5. Agent Tasks: Architecture-Aware Task Decomposition

Task decomposition follows the architectural layers. A story like "Consolidate OIDC configuration" does not decompose into "edit file X, edit file Y." It decomposes into:

1. **API Phase**: Add status type (layer: API)
2. **HO Reconciler Phase**: Populate status per-platform (layer: Orchestration, per-platform)
3. **CPO Adapt Phase**: Read from status with fallback (layer: Control Plane)
4. **Test Phase**: Unit + e2e (cross-layer)

Each phase has:
- A clear architectural scope (which layers it touches)
- Pre-conditions (API phase must complete before reconciler phase)
- Invariant checks (what must be true after this phase completes)
- Validation criteria (how we know this phase is correct)

The system tracks task dependencies not just at the task level, but at the architectural level. It knows that the CPO adapt phase depends on the API phase because the adapt function needs the new type. But it also knows that the CPO adapt phase can be developed in parallel with the HO reconciler phase, as long as the API types are available — because adapt functions are pure transformations that do not depend on the reconciler's runtime behavior.

### 6. Team Knowledge: The Shared Architectural Model

This is where the compound effect happens. Every team member's system contributes to a shared knowledge base:

- **Pattern Library**: "When adding a new status subresource, follow this pattern" (extracted from the 15 times the team has done this before)
- **Platform Expertise**: Cesar's system knows deep ARO internals. When anyone on the team makes an ARO-affecting change, Cesar's accumulated ARO knowledge is available (with appropriate attribution and access controls)
- **Bug Patterns**: "Every time someone adds a new field to HostedCluster without updating the printer columns, the field is invisible in `oc get hc`. Here is the fix pattern."
- **Review Patterns**: "Alberto consistently catches version skew issues in CPOv2 adapt functions. His review pattern checks for: (1) nil safety on status fields that may not exist in older versions, (2) fallback behavior when the field is absent, (3) test coverage for the upgrade path."

This knowledge is git-portable. It lives in the repository. New team members get it on day one — not after six months of osmosis.

### 7. Agent Personas: Architectural Roles

Personas map to architectural concerns, not generic roles.

- **The API Designer**: Knows OpenShift API conventions, deprecation policy, CEL validation, structural schema requirements. Reviews API changes for consistency and extensibility.
- **The Platform Specialist** (one per platform): Deep knowledge of AWS/Azure/KubeVirt/OpenStack/PowerVS/Agent internals. Understands platform-specific constraints and can simulate platform behavior.
- **The Invariant Guardian**: Watches for invariant violations. Every change is checked against the seven core invariants. This persona is adversarial — it tries to find ways the change could break isolation, leak credentials, or introduce reverse communication paths.
- **The Version Skew Analyst**: Understands the matrix of HO versions x CPO versions and checks that changes are compatible across the support matrix.
- **The CI Archaeologist**: Knows the test suite, knows which tests are flaky, knows which tests are slow, knows which tests actually validate the behavior you care about. When CI fails, it distinguishes "this is a real failure caused by your change" from "this is FLAKY-1234 again."

---

## Part V: Confidence — The Core Innovation

The single biggest barrier to agentic development adoption in a project like HyperShift is **confidence**. Not "does the code compile?" but "can I ship this without a senior engineer reviewing every line?"

The dream-world system makes confidence **measurable, composable, and incrementally buildable**.

### Confidence Dimensions

Confidence is not a single number. It is a vector across multiple dimensions:

```
CONFIDENCE ASSESSMENT: PR #4567 — OIDC Status Consolidation
════════════════════════════════════════════════════════════

  API Correctness:          92%  (type follows known patterns, linter passes)
  Invariant Compliance:     95%  (all 7 invariants verified by static analysis)
  Platform Compatibility:
    AWS:                    90%  (unit tests pass, e2e green)
    Azure:                  75%  (unit tests pass, e2e not run, ARO RP timing uncertain)
    KubeVirt:               88%  (unit tests pass, e2e green)
    OpenStack:              60%  (no platform-specific test, inferred from KubeVirt)
    PowerVS:                40%  (no test coverage, IBM team not consulted)
    Agent:                  85%  (unit tests pass)
  Version Skew:             88%  (fallback tested for N-1, not N-2)
  Test Coverage:            82%  (unit: 95%, e2e: 60%, integration: not applicable)
  Spec Compliance:          100% (all spec requirements have corresponding code)

  OVERALL:                  78%  (bottleneck: PowerVS coverage, ARO timing)

  TO REACH 90%:
    1. Run Azure e2e suite (+8%)
    2. Confirm PowerVS OIDC path with IBM team (+5%)
    3. Add N-2 version skew test (+2%)
```

This is not a gimmick. Each dimension is computed from actual evidence: test results, static analysis, spec traceability, platform coverage. The system does not hallucinate confidence — it earns it through verification.

### Confidence Accumulation

When Alberto reviews the API design and approves it, the API Correctness confidence goes up — not because "a human looked at it," but because Alberto's system verified specific API properties that the automated checks could not. Alberto's review carries weight because his review history shows he catches API issues that automated tools miss.

When the Azure e2e suite passes, the Azure platform compatibility confidence jumps. The system tracks which tests validate which confidence dimensions. Not all tests are equal — the system knows that `TestCreateClusterAzure` validates the end-to-end OIDC flow, while `TestAzureResourceCreation` does not touch OIDC at all.

### Confidence Contracts

Teams can define confidence thresholds for different types of changes:

```yaml
confidence_policy:
  api_changes:
    minimum_overall: 85%
    require:
      api_correctness: 90%
      invariant_compliance: 95%
      version_skew: 85%
    require_human_review:
      - api_designer_persona

  platform_specific_changes:
    minimum_overall: 80%
    require:
      affected_platform_compatibility: 85%
    require_human_review: []  # pure agentic if confidence met

  cpo_adapt_changes:
    minimum_overall: 82%
    require:
      version_skew: 90%  # higher bar for CPO changes
      invariant_compliance: 95%
    require_human_review:
      - version_skew_analyst_persona
```

This is how the system bridges "full agentic" and "human-in-the-loop." It is not a binary switch. It is a **confidence-driven escalation policy**. Simple, well-understood changes with high confidence ship without human review. Cross-cutting architectural changes with lower confidence require specific expert review — and the system tells you exactly which expert and why.

---

## Part VI: Day One — The New Developer Experience

Sara joins the HyperShift team. Today is her first day.

She opens the workspace. The system does not dump the README on her. It does not show her a file tree with 2,000 files. It shows her **the architecture**.

```
Welcome to HyperShift.

You are looking at a system that runs OpenShift control planes
as workloads on a management cluster. Here is how it is organized:

[Interactive architectural diagram]

Your first story is OCPBUGS-13456: "Add validation for NodePool
replica count on PowerVS platform."

This story lives in the Orchestration Layer, specifically in the
NodePool admission webhook. It affects only the PowerVS platform.
Here is what you need to know:

1. NodePool API: the type definition (I will show you the relevant fields)
2. Admission webhooks: where they live and how they work in HyperShift
3. PowerVS constraints: why this platform has different replica limits
4. The pattern: here are 3 similar validations for other platforms
   that you can follow as a template

You do not need to understand the Control Plane Operator,
the data plane, or the upgrade system for this story.
Those layers are not affected.

Ready to start?
```

Sara does not spend two weeks reading code to figure out where things are. The system gives her exactly the architectural context she needs for her specific story, at the right level of detail, with working examples from the same codebase.

When Sara inevitably asks "but what does the control plane operator do?", the system explains it in the context of her story:

> "The Control Plane Operator (CPO) runs inside each hosted cluster's control plane namespace. It takes the HostedCluster spec and turns it into running Kubernetes control plane components — etcd, kube-apiserver, kube-controller-manager, etc. Your NodePool validation change does not affect the CPO because NodePool replica count is handled entirely in the HyperShift Operator layer, before the CPO ever sees it. The CPO only cares about the control plane components, not the worker node count. You will work with the CPO when you get stories about control plane configuration — but not today."

The system is not just onboarding Sara. It is **teaching her the architecture through her work**. Every story she completes adds to her architectural understanding, guided by the system. After 20 stories across different layers, Sara has a working mental model of the entire system — not from reading docs, but from experiencing it.

---

## Part VII: What Makes This Different — The Hard Opinions

### Opinion 1: The Filesystem Is Not the Interface

Every current tool starts from files. This is wrong for HyperShift. The architecture is the interface. Files are implementation details. When you navigate the system, you navigate layers, controllers, API types, data flows, and invariants. Files appear when you need to read or edit code, but they are always in architectural context.

### Opinion 2: The System Must Say "No"

Current agentic tools are eager to please. They will generate whatever you ask for. The dream-world system refuses to generate code that violates architectural invariants. It will not write a controller that reads data plane state from the management cluster. It will not generate a CPOv2 adapt function with side effects. It will not create an API type that exposes mutable CRDs on the data plane.

This is not a limitation. It is a feature. The system embodies the team's architectural standards and enforces them mechanically. This is how you get confidence in agentic code — not by reviewing every line, but by making entire categories of mistakes impossible.

### Opinion 3: Confidence Is the Product, Not Code

The output of the system is not code. It is **confidence that the code is correct**. The code is a means to an end. The end is a shipped change that works correctly across all platforms, respects all invariants, handles version skew, and passes all relevant tests.

Current tools measure success by "did the code compile?" or "did the tests pass?" The dream-world system measures success by "how confident are we that this change is safe to ship?" This is a fundamentally different metric, and it changes everything about how the system operates.

### Opinion 4: Sequential Pipeline, Not Swarm — But With Parallelism Where Architecture Allows

The pipeline is sequential at the architectural level: Spec → API → Implementation → Test → Review → Ship. You do not write implementation before the spec is approved. You do not ship before confidence thresholds are met.

But within each phase, the system exploits architectural parallelism. The HO reconciler changes for AWS and Azure can be developed in parallel because they are in different platform packages with no shared state. The CPOv2 adapt function changes for kube-apiserver and oauth-server can be developed in parallel because adapt functions are pure transformations with no interdependence.

The system knows where parallelism is safe because it knows the architecture.

### Opinion 5: Team Knowledge Is the Moat

Any tool can generate code. The dream-world system's competitive advantage is that it accumulates, structures, and applies **team-specific architectural knowledge**. After a year of use, the system knows:

- Every architectural decision and its rationale
- Every platform-specific gotcha
- Every API convention and exception
- Every developer's expertise area
- Every common bug pattern and its fix
- Every test's reliability and coverage
- Every CI flake and its workaround

This knowledge compounds. It makes the 100th PR faster and more confident than the 1st. It makes the 5th team member as effective as the 1st. It makes onboarding a multiplier, not a tax.

### Opinion 6: The System Must Understand Products, Not Just Code

HyperShift does not exist in isolation. It exists to serve ROSA, ARO, ROKS, and self-hosted OpenShift. A change that is correct at the code level but breaks ROSA's SLA guarantee is not correct. A change that works for all platforms but violates ARO's resource provider contract is not safe.

The system must model product-level constraints: SLA requirements, operational models, billing integration points, support boundaries. These are not in the code — they are in the product documentation, the architecture calls, the team's collective knowledge. The system must internalize them and apply them during development and review.

### Opinion 7: The IDE Is Dead for This Domain

An IDE is designed for editing files. HyperShift development is about understanding and modifying a distributed system. The workspace is not an IDE with AI bolted on. It is a **distributed system development environment** that happens to let you edit files when you need to.

The primary interaction is not typing code. It is:
- Querying the architectural model ("what controllers watch HostedCluster?")
- Simulating changes ("if I add this field, what breaks on Azure?")
- Reviewing impact ("show me the data flow from this spec field to the data plane")
- Building confidence ("what tests do I need to pass to reach 90% confidence?")

Code editing is a small fraction of the actual work. The system should reflect that.

---

## Part VIII: Practical First Steps

This vision is ambitious. Here is how I would sequence building toward it:

### Phase 0: The Architectural Knowledge Graph (Foundation)

Build the living model of HyperShift's architecture. This is the foundation everything else depends on. It should be:
- Code-derived (analyze API types, controller registrations, RBAC, network policies)
- Manually enriched (architectural decisions, platform constraints, invariants)
- Continuously updated (re-analyze on every merge to main)
- Queryable (GraphQL or similar — "what controllers are affected by changes to HostedCluster.Spec.Platform.AWS?")

This graph is useful immediately, even without any agentic capabilities. It is a queryable architectural model that any developer can use to understand the system.

### Phase 1: Invariant Enforcement (Confidence Foundation)

Implement the seven core invariants as automated checks. Run them on every PR. This is a CI check, not an agentic feature, but it builds the foundation for confidence scoring.

- Static analysis for unidirectional communication violations
- RBAC analysis for cross-namespace access
- API analysis for mutable CRDs exposed on data plane
- Data flow analysis for management-side lifecycle triggers from data plane changes

### Phase 2: Spec-Driven Development (Process Foundation)

Introduce the spec format. Start simple — a structured YAML that captures which layers are affected, which platforms are impacted, and what invariants are relevant. The spec does not need to drive code generation yet. It just needs to exist as a reviewable, machine-readable artifact that the team uses to plan implementation.

### Phase 3: Context-Aware Agentic Development (First Agentic Capability)

Use the knowledge graph and specs to provide surgically precise context to an agent (RH-A). Instead of "here is the whole codebase," give the agent exactly the subgraph relevant to the current story. This alone will dramatically improve agentic code quality.

### Phase 4: Confidence Scoring (Closing the Loop)

Implement confidence scoring based on test results, invariant checks, spec compliance, and platform coverage. Define team confidence policies. Start routing low-confidence changes to human reviewers and high-confidence changes through automated merge.

### Phase 5: The Workspace (Full Vision)

Build the spatial, architecture-first development environment. This is the hardest part and the last to build, because it depends on everything else being in place.

---

## Closing

The fundamental insight is this: HyperShift is not a codebase that happens to have architecture. It is an architecture that happens to be expressed in code. The dream-world system starts from the architecture and works inward to the code, not the other way around.

Every current tool — every IDE, every AI coding assistant, every CI system — starts from files and tries to infer architecture. The dream-world system starts from a living architectural model and uses files as the implementation medium.

This is the paradigm shift. The architecture is the interface. The code is the detail. Confidence is the product.

---

*Wazir's final note: This vision is deliberately opinionated and ambitious. Not everything here is buildable today. But the sequencing in Part VIII is practical — each phase delivers standalone value while building toward the full vision. Start with the knowledge graph. Everything else follows from having a machine-readable architectural model.*
