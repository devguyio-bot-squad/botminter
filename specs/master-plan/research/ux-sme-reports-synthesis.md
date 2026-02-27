# Dream-World UX — Agent Reports Comparison

> Comprehensive comparison of all agent visions for the ideal agentic development UX.
> Part of [Shift Week Plan](shift-week-plan.md), input to [Ralph vs Claude Code POC](ralph-vs-claude-code-poc.md)
> Source reports: [Karim's First Week UX](ux-karims-first-week.md), [Living Codebase Vision](ux-living-codebase-vision.md), [SME: API Evolution](sme-api-evolution.md), [SME: Data Plane NodePool](sme-data-plane-nodepool.md), [SME: Control Plane CPOv2](sme-control-plane-cpov2.md), [SME: Cloud Provider](sme-cloud-provider-multicloud.md), [SME: HCP Architecture](sme-hcp-architecture.md)

---

## Report Summaries

### 1. Ideal Workflow UX (Original Narrative)

**File:** [Karim's First Week UX](ux-karims-first-week.md)
**Core metaphor:** The practical onboarding story — Karim's first week

The **concrete UX baseline** written as a new developer's week-long story. Defines the target experience both POCs must deliver. Key elements:

- **`rh start JIRA-ID`** kicks off a full context-aware plan
- Agent **escalates** cleanly with structured options (Option A/B) when judgment is needed
- **Unattended mode** (`--unattended`) for simple stories
- **Knowledge captured via `rh learn`**, stored in `.devagent/knowledge/`, git-committed
- **CI triage** with flake detection and automated retrigger
- **Event-driven queuing**: "Start story X when Y merges"
- The agent stays scoped — 4-line PR for a 4-line story

**Distinctive contribution:** The most practical and testable of all reports. Defines the 7 "POC must validate" criteria.

---

### 2. Cloud Native Visionary — "The Living Codebase"

**File:** [Living Codebase Vision](ux-living-codebase-vision.md)
**Core metaphor:** The codebase is an autonomous entity you collaborate WITH

The most ambitious and philosophically sweeping report. Key elements:

- **The Bridge** — a "decision surface" (not IDE, not chat) showing what matters now
- **Confidence Model** with 5 dimensions: Evidence Strength, Precedent Matching, Blast Radius, Domain Complexity, Historical Track Record
- **Trust Spectrum** with configurable thresholds per subsystem, per change type
- **Evidence Reports** on every PR — structured proof that replaces 80% of mechanical review
- **Knowledge as Behavior** — team learnings become executable pattern rules (YAML), not wiki pages
- **Planning Cascade**: RFE (human) → Feature (collaborative) → Epic (collaborative) → Story (agentic)
- **The Wedge**: Start with one change type (adding API fields), prove the thesis
- **The Moat**: cumulative domain knowledge, confidence history, network effects

**Distinctive contribution:** The broadest vision. Frames the paradigm shift from "AI-assisted" to "AI-native" development. The confidence model with auto-merge thresholds is the most fully developed across all reports.

---

### 3. API SME — "The API Evolution Record"

**File:** [SME: API Evolution](sme-api-evolution.md)
**Core metaphor:** API changes as first-class lifecycle objects

Deeply focused on API changes as the highest-risk, highest-leverage area. Key elements:

- **API Evolution Record (AER)** — a living, structured lifecycle object from proposal to GA
- **Convention Corpus** — both written conventions AND unwritten team preferences extracted from review history
- **Platform Impact Analysis** — cross-platform implications surfaced for every API change
- **Graduated Autonomy** (Level 0-4) per engineer, per change type, per risk level
- **Version Skew** as first-class concern in every AER
- **Confidence dimensions** specific to API: Convention Compliance, Backward Compatibility, Test Coverage, Pattern Conformance, Naming Consistency, Version Skew Safety
- **Perspectives, not personas**: Implementer, Reviewer, Archaeologist, API Guardian

**Distinctive contribution:** The AER concept — treating an API change as a tracked lifecycle object from inception through GA. The most detailed treatment of how generated code (`make api`) should be handled (verified artifact, not reviewable code).

---

### 4. Data Plane SME — "The Situation Room"

**File:** [SME: Data Plane NodePool](sme-data-plane-nodepool.md)
**Core metaphor:** Developer works at the right abstraction level, system carries the rest

Focused on NodePool lifecycle, CAPI abstractions, and platform-specific complexity. Key elements:

- **Domain Model**, not file model — NodePool Lifecycle States, CAPI Abstraction Map, Platform Variation Matrix
- **Three-zone workspace**: Situation Zone (ambient), Work Zone (primary), Evidence Zone (expandable)
- **Three-tier knowledge**: Tier 1 (Pattern Library, curated), Tier 2 (Experience Logs, automated), Tier 3 (Individual Memory, personal)
- **Collaborative mode switching**: "Full auto" → "Pair with me" seamlessly within a session
- **Four-level confidence**: Structural → Behavioral → Architectural → Human
- **CAPI abstraction boundary enforcement** — catches violations like reading AzureMachine directly in NodePool controller
- **Platform Variation Matrix** with boot times, flake rates, test durations per platform

**Distinctive contribution:** The most domain-specific. The NodePool lifecycle state machine and CAPI abstraction map are concrete, immediately useful artifacts. The three-tier knowledge system (curated vs automated vs personal) is the most nuanced knowledge architecture.

---

### 5. Control Plane SME — "The Forge"

**File:** [SME: Control Plane CPOv2](sme-control-plane-cpov2.md)
**Core metaphor:** The developer is a smith, the system is the forge

Focused on CPOv2 framework, adapt function purity, and component lifecycle. Key elements:

- **Contract-driven development** — developers think in contracts (purity, RBAC scope, resource budget, version skew), not files
- **Topology View** — reconciliation graph showing component dependencies and contract satisfaction status
- **Six contracts per component**: deployment, RBAC, adapt purity, resource budget, platform matrix, version skew
- **Voice input** in the IDE for quick corrections
- **Contract satisfaction dashboard** for team visibility
- **Version Skew Knowledge Graph** — auto-generated, field-level compatibility tracking across HO and CPO versions
- **Autonomy spectrum** per phase (planning, implementation, testing, review, release)
- **Pattern learning from CI failures** → team knowledge entries automatically

**Distinctive contribution:** The contract-as-primary-artifact concept. Instead of "review my code," it's "verify my contracts." The reconciliation graph topology view is unique to this report. The version skew knowledge graph (auto-generated from API analysis) is the most technically concrete approach to version skew.

---

### 6. Cloud Provider SME — "The Platform Knowledge Graph"

**File:** [SME: Cloud Provider](sme-cloud-provider-multicloud.md)
**Core metaphor:** Making distributed cloud cognition durable and composable

Focused on multi-cloud complexity, credential management, and cross-platform intelligence. Key elements:

- **Platform Knowledge Graph** — structured, queryable graph per cloud provider (DiskEncryptionSet → requires KeyVault, requires ManagedIdentity, etc.)
- **Platform Abstraction Map** — conceptual mapping of equivalent features across all 5 platforms
- **Three-layer validation**: Contract Testing (local) → Simulated Cloud (sandboxed) → Targeted Cloud ($2.40) → Full E2E (CI)
- **Credential Tiers**: Local (mock) → Targeted (scoped, 1hr) → CI (managed) → Production (never)
- **Cross-Platform Review** — when shared code changes, system reasons about impact across ALL platforms simultaneously
- **Platform-aware personas** that activate based on context (Azure specialist, AWS specialist, etc.)
- **Incremental adoption path**: Phase 1 (knowledge, months 1-2) → Phase 2 (single platform, 3-4) → Phase 3 (multi-platform, 5-8) → Phase 4 (full pipeline, 9-12)
- **Trust Ladder**: Suggest → Implement+Review → Implement+Spot-Check → Implement+Validate → Autonomous

**Distinctive contribution:** The most practical on cloud-specific challenges. The credential tier model and the simulated cloud validation layer are unique. The cross-platform review capability ("no single engineer holds all five platforms — the system does") is the strongest argument for the system's value.

---

### 7. HCP Architect SME — "Architecture as the Primary Interface"

**File:** [SME: HCP Architecture](sme-hcp-architecture.md)
**Core metaphor:** The filesystem is not the interface — architecture is

The most architecturally opinionated, focused on HyperShift's unique product-level complexity. Key elements:

- **Living Architecture Model** — vertical layers (Product → Orchestration → Control Plane → Data Plane → Infrastructure) + horizontal concerns (trust, version skew, isolation, upgrades, API compat)
- **Seven core invariants** as hard rules the system enforces
- **Spec-Driven Development** with executable YAML specs (kind: FeatureSpec) that drive code generation, test selection, and review
- **Cross-Product Impact Analysis** — ROSA/ARO/ROKS/Self-Hosted simulation before PR creation
- **Confidence as a vector**, not a number — per-platform, per-dimension
- **Architecture-aware task decomposition** following layers, not files
- **The system says "No"** — refuses to generate code that violates architectural invariants
- **"Confidence is the product, not code"** — the most radical reframing
- **Phase 0-5 implementation roadmap**: Knowledge Graph → Invariant Enforcement → Spec-Driven Dev → Context-Aware Agent → Confidence Scoring → Workspace

**Distinctive contribution:** The strongest on product-level concerns (ROSA SLA, ARO RP dependencies). The cross-product impact analysis is unique — simulating a change across ROSA/ARO/ROKS/Self-Hosted. The spec YAML format (kind: FeatureSpec) is the most concrete spec-driven artifact. The "system says No" opinion is the boldest take on guardrails.

---

## Comparison Tables

### Core Metaphor & Philosophy

| Report | Core Metaphor | Developer Role | System Role |
|--------|--------------|---------------|-------------|
| **Ideal Workflow** | Competent assistant | Decision maker | Executor + advisor |
| **Visionary** | Living Codebase | Strategic partner | Autonomous collaborator |
| **API SME** | Junior-to-mid engineer on team | API architect | Lifecycle tracker + convention enforcer |
| **Data Plane SME** | Situation Room | Works at right abstraction | Carries the abstraction stack |
| **Control Plane SME** | The Forge (smith & forge) | Contract verifier | Contract satisfier + evidence producer |
| **Cloud Provider SME** | Durable distributed cognition | Intent expresser | Platform knowledge holder |
| **HCP Architect** | Architecture-first workspace | Architectural judge | Invariant guardian + impact analyzer |

### Primary Artifact (What the Developer Works With)

| Report | Primary Artifact | Instead of... |
|--------|-----------------|---------------|
| **Ideal Workflow** | Plan + code diff | Raw files |
| **Visionary** | Evidence Report + Confidence Score | PR diff |
| **API SME** | API Evolution Record (AER) | JIRA ticket + PR |
| **Data Plane SME** | Domain Model (lifecycle states, CAPI map) | File tree |
| **Control Plane SME** | Contracts (purity, RBAC, resources, skew) | Code |
| **Cloud Provider SME** | Platform Knowledge Graph + Living Spec | Platform docs + tribal knowledge |
| **HCP Architect** | Executable FeatureSpec YAML | Design docs |

### Knowledge System Architecture

| Report | Structure | Storage | Compounding Mechanism |
|--------|-----------|---------|----------------------|
| **Ideal Workflow** | Flat `.devagent/knowledge/` files | Git (in repo) | `rh learn` → PR → sync |
| **Visionary** | Pattern rules (YAML) with fire counts | Git (in repo) | Every interaction → pattern extraction |
| **API SME** | Convention Corpus + Review Patterns + Failure Archaeology | Git + queryable graph | AER post-merge → knowledge capture |
| **Data Plane SME** | 3 tiers: Curated Patterns, Auto Logs, Personal Memory | Git (T1, T2) + local (T3) | Experience log → promoted to pattern |
| **Control Plane SME** | 3 layers: Personal, Team, Domain | Git (team/domain) + local (personal) | CI failure → pattern → knowledge entry |
| **Cloud Provider SME** | Platform Knowledge Graph + Memories + Prompts | Git (`.hypershift-agent/`) | Cross-platform pattern inference |
| **HCP Architect** | Architectural Knowledge Graph (code-derived + enriched) | Git + GraphQL-queryable | Continuous re-analysis on merge |

### Confidence / Trust Model

| Report | Confidence Type | Dimensions | Trust Levels |
|--------|----------------|------------|--------------|
| **Ideal Workflow** | Implicit (CI pass + flake detection) | N/A | Attended vs Unattended |
| **Visionary** | Single score (0-100%) with breakdown | Evidence, Precedent, Blast Radius, Complexity, Track Record | Configurable threshold per subsystem |
| **API SME** | Per-dimension (0-1.0) | Convention, Backward Compat, Coverage, Pattern, Naming, Version Skew | 5 levels (L0-L4) per engineer x change type |
| **Data Plane SME** | 4-level stack | Structural, Behavioral, Architectural, Human | Full Auto <-> Pair Programming (per session) |
| **Control Plane SME** | Evidence Bundle per contract | Per-contract satisfaction (pass/fail/partial) | Per-phase autonomy (manual -> autonomous) |
| **Cloud Provider SME** | Confidence Stack (%) | Pattern Match, Code Quality, Platform Correctness, Cross-Platform Risk, CI Validation | 5-level Trust Ladder per category x platform |
| **HCP Architect** | Vector (% per dimension) | API, Invariants, Platform Compat (per platform), Version Skew, Test Coverage, Spec Compliance | Confidence-driven escalation policy |

### Unique Contributions (What Only This Report Brings)

| Report | Unique Contribution |
|--------|-------------------|
| **Ideal Workflow** | The 7 POC validation criteria; concrete CLI UX (`rh start`, `rh learn`, `--unattended`) |
| **Visionary** | The Wedge strategy (start with API field additions); The Moat (cumulative knowledge as competitive advantage); Mobile-first decision surface |
| **API SME** | API Evolution Record (AER) lifecycle tracking; generated code as verified artifact (not reviewable); de facto convention drift detection |
| **Data Plane SME** | NodePool lifecycle state machine; CAPI abstraction boundary enforcement; Platform Variation Matrix with boot times and flake rates |
| **Control Plane SME** | Contract-driven development; CPOv2 reconciliation graph topology view; auto-generated version skew knowledge graph; voice input |
| **Cloud Provider SME** | Credential tier model; simulated cloud validation (cost estimates); cross-platform review reasoning; 12-month adoption roadmap |
| **HCP Architect** | Cross-product impact analysis (ROSA/ARO/ROKS simulation); executable FeatureSpec YAML; "system says No" guardrails; invariant enforcement as CI checks |

---

## Agreement Points (Where All Reports Converge)

1. **Knowledge must compound via git** — all reports insist knowledge lives in the repo, version-controlled, reviewable
2. **Evidence replaces line-by-line review** — every report proposes some form of structured evidence that shifts review from "find problems" to "verify reasoning"
3. **The system must understand HyperShift-specific architecture** — not generic coding AI, but domain-specific (split-brain, CPOv2 purity, CAPI abstractions, version skew)
4. **Autonomy is a spectrum, not a switch** — configurable per developer, per change type, per risk level
5. **New developer onboarding is a prime use case** — every report includes a "Day 1" narrative showing dramatic improvement
6. **CI debugging should be automated** — flake detection, root cause analysis, auto-retrigger
7. **Platform isolation must be enforced** — cross-platform impact analysis on every change
8. **The sequential pipeline (not swarm) is correct** — Ralph-Wiggum style orchestration

---

## Tension Points (Where Reports Disagree)

| Tension | Position A | Position B |
|---------|-----------|-----------|
| **Custom UI?** | Visionary, CP SME, HCP Architect: Yes, build "The Bridge"/"The Forge"/Workspace | API SME: No, integrate with existing tools (VS Code, GitHub, Slack) |
| **How explicit should specs be?** | HCP Architect: Formal YAML specs (kind: FeatureSpec) | Ideal Workflow: Plans in plain text, no formal spec format |
| **Voice input?** | CP SME: Yes, for quick corrections | Others: Not mentioned |
| **Agent as "team member" vs "tool"?** | API SME: Agent is a junior-to-mid engineer on the team | Ideal Workflow: Agent is a competent tool you invoke |
| **How much product context?** | HCP Architect: Full ROSA/ARO/ROKS product modeling | Others: Focus on codebase, less on product boundaries |
| **Starting point?** | HCP Architect: Build knowledge graph first (Phase 0) / Visionary: Build the wedge first (one change type) / Cloud Provider SME: Build team knowledge directory first |

---

## Synthesis: Composing the Visions

These are not competing visions — they are **complementary layers**:

- The **Ideal Workflow UX** is the CLI surface and developer experience contract
- The **Visionary** provides the philosophical frame, confidence model, and go-to-market (wedge + moat)
- Each **SME** adds domain-specific depth:
  - **API SME**: AER lifecycle tracking, convention enforcement
  - **Data Plane SME**: NodePool lifecycle model, CAPI boundary enforcement
  - **Control Plane SME**: Contract-driven CPOv2 development, topology view
  - **Cloud Provider SME**: Platform knowledge graph, credential management, cross-platform reasoning
  - **HCP Architect**: Invariant enforcement, product-level impact analysis, executable specs

### The Most Actionable Synthesis

1. **Start with the knowledge base** (all agree) — `.devagent/knowledge/` with team-curated patterns, constraints, and architecture docs
2. **Prove with one change type** (Visionary's wedge + API SME's API field addition) — adding a new field to HostedCluster or NodePool
3. **Build confidence incrementally** (all agree on graduated trust) — start at Level 0 (agent proposes, human implements), earn trust empirically
4. **Instrument everything** — track confidence accuracy, review time reduction, post-merge bugs to calibrate the system
5. **Keep it git-portable** — every artifact the system produces or learns must live in the repo
