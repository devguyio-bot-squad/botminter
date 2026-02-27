#!/usr/bin/env bash
set -euo pipefail

# DEPRECATED: This script uses the old .github-sim/ file-based issue system.
# The project has migrated to real GitHub issues via the gh CLI.
# To create a synthetic epic, use: gh issue create --label kind/epic --label status/po:triage
# See: skeletons/team-repo/agent/skills/gh/SKILL.md
#
# deploy.sh — Deploy synthetic fixtures into a team repo for M2 Sprint 2 validation.
#
# Usage: bash deploy.sh <team-repo-path> [--project-repo=<path>]
#
# This script:
#   1. Copies knowledge files (3 scopes) into the team repo
#   2. Copies invariant files (3 scopes) into the team repo
#   3. Deploys synthetic epic to .github-sim/issues/1.md (DEPRECATED — use gh issue create)
#   4. Creates a synthetic project repo (minimal git repo with Go stub)
#   5. Commits all changes to the team repo
#
# Idempotent: re-running produces no changes if already deployed.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEAM_REPO=""
PROJECT_REPO="/tmp/synth-hypershift"

# Parse arguments
for arg in "$@"; do
    case "$arg" in
        --project-repo=*) PROJECT_REPO="${arg#--project-repo=}" ;;
        -*)               echo "Error: Unknown flag '$arg'"; exit 1 ;;
        *)                TEAM_REPO="$arg" ;;
    esac
done

if [ -z "$TEAM_REPO" ]; then
    echo "Error: team repo path is required"
    echo "Usage: bash deploy.sh <team-repo-path> [--project-repo=<path>]"
    exit 1
fi

# Resolve to absolute path
case "$TEAM_REPO" in
    /*) ;; # absolute — keep as is
    *)  TEAM_REPO="$(pwd)/$TEAM_REPO" ;;
esac

case "$PROJECT_REPO" in
    /*) ;; # absolute — keep as is
    *)  PROJECT_REPO="$(pwd)/$PROJECT_REPO" ;;
esac

if [ ! -d "$TEAM_REPO" ]; then
    echo "Error: team repo not found at $TEAM_REPO"
    exit 1
fi

if [ ! -d "$TEAM_REPO/.git" ]; then
    echo "Error: $TEAM_REPO is not a git repository"
    exit 1
fi

echo "Deploying synthetic fixtures to $TEAM_REPO"

# --- 1. Knowledge files (3 scopes) ---

# Team-level knowledge
mkdir -p "$TEAM_REPO/knowledge"
cp "$SCRIPT_DIR/knowledge/commit-convention.md" "$TEAM_REPO/knowledge/commit-convention.md"
echo "  [ok] Team knowledge: knowledge/commit-convention.md"

# Project-level knowledge
mkdir -p "$TEAM_REPO/projects/hypershift/knowledge"
cp "$SCRIPT_DIR/projects/hypershift/knowledge/hcp-architecture.md" \
   "$TEAM_REPO/projects/hypershift/knowledge/hcp-architecture.md"
echo "  [ok] Project knowledge: projects/hypershift/knowledge/hcp-architecture.md"

# Member-level knowledge
mkdir -p "$TEAM_REPO/team/architect/knowledge"
cp "$SCRIPT_DIR/team/architect/knowledge/design-patterns.md" \
   "$TEAM_REPO/team/architect/knowledge/design-patterns.md"
echo "  [ok] Member knowledge: team/architect/knowledge/design-patterns.md"

# --- 2. Invariant files (3 scopes) ---

# Team-level invariant
mkdir -p "$TEAM_REPO/invariants"
cp "$SCRIPT_DIR/invariants/code-review-required.md" "$TEAM_REPO/invariants/code-review-required.md"
echo "  [ok] Team invariant: invariants/code-review-required.md"

# Project-level invariant
mkdir -p "$TEAM_REPO/projects/hypershift/invariants"
cp "$SCRIPT_DIR/projects/hypershift/invariants/upgrade-path-tests.md" \
   "$TEAM_REPO/projects/hypershift/invariants/upgrade-path-tests.md"
echo "  [ok] Project invariant: projects/hypershift/invariants/upgrade-path-tests.md"

# Member-level invariant
mkdir -p "$TEAM_REPO/team/architect/invariants"
cp "$SCRIPT_DIR/team/architect/invariants/design-quality.md" \
   "$TEAM_REPO/team/architect/invariants/design-quality.md"
echo "  [ok] Member invariant: team/architect/invariants/design-quality.md"

# --- 3. Synthetic epic ---

mkdir -p "$TEAM_REPO/.github-sim/issues"
cp "$SCRIPT_DIR/synthetic-epic-1.md" "$TEAM_REPO/.github-sim/issues/1.md"
echo "  [ok] Synthetic epic: .github-sim/issues/1.md"

# --- 4. Synthetic project repo ---

if [ -d "$PROJECT_REPO" ]; then
    echo "  [skip] Synthetic project repo already exists at $PROJECT_REPO"
else
    mkdir -p "$PROJECT_REPO"
    cd "$PROJECT_REPO"
    git init --quiet

    # Set identity for the synthetic repo
    git config user.email "synthetic@botminter"
    git config user.name "botminter-fixture"

    # README describing the reconciler pattern
    cat > README.md << 'READMEEOF'
# synth-hypershift

Synthetic HCP project repository for M2 Sprint 2 validation.

This project simulates a Hosted Control Planes (HCP) codebase that uses a
reconciler pattern with controller-runtime. The HCP controller manages the
lifecycle of hosted Kubernetes control planes.

## Architecture

- Reconciler-based controller using controller-runtime
- Composition-based design (struct embedding over inheritance)
- Custom resources (CRDs) for HCP and NodePool
READMEEOF

    # Stub Go code with reconciler + composition markers
    mkdir -p pkg/controllers/hcp
    cat > pkg/controllers/hcp/reconciler.go << 'GOEOF'
package hcp

// Reconciler manages the lifecycle of HostedControlPlane resources.
// It uses the reconciler pattern from controller-runtime to watch for
// changes and drive actual state toward desired state.
//
// Design note: prefer composition over inheritance — embed shared
// capabilities rather than extending base types.

import (
	"context"
)

// HealthChecker provides health-check capabilities via composition.
type HealthChecker struct {
	// lastCheck records the timestamp of the last successful check.
	lastCheck string
}

// CheckHealth reports the health status of a component.
func (h *HealthChecker) CheckHealth(ctx context.Context) (bool, error) {
	// TODO: implement actual health check
	return true, nil
}

// Reconciler is the main HCP reconciler. It uses composition to embed
// shared capabilities like health checking.
type Reconciler struct {
	HealthChecker // composition — embedded struct, not inheritance
}

// Reconcile performs a single reconciliation loop for an HCP resource.
func (r *Reconciler) Reconcile(ctx context.Context, name string) error {
	// The reconciler pattern: read desired state, compare with actual,
	// take corrective action.
	return nil
}
GOEOF

    git add -A
    git commit --quiet -m "Initial synthetic HCP project repo"
    echo "  [ok] Synthetic project repo created at $PROJECT_REPO"
fi

# --- 5. Commit fixtures to team repo ---

cd "$TEAM_REPO"

# Check if there are any changes to commit
if git diff --quiet && git diff --cached --quiet && [ -z "$(git ls-files --others --exclude-standard)" ]; then
    echo ""
    echo "No changes to commit — fixtures already deployed."
else
    git add -A
    git commit --quiet -m "feat(fixtures): deploy synthetic fixtures for M2 Sprint 2 validation

Adds knowledge (3 scopes), invariants (3 scopes), and synthetic epic
at status/po:triage for two-agent full lifecycle validation.

Ref: M2-Sprint-2-Step-5"
    echo ""
    echo "Fixtures committed to team repo."
fi

echo ""
echo "Deploy complete. Next steps:"
echo "  1. Add members:             cd $TEAM_REPO && just add-member human-assistant && just add-member architect"
echo "  2. Create workspaces:       cd $TEAM_REPO && just create-workspace human-assistant $PROJECT_REPO"
echo "                              cd $TEAM_REPO && just create-workspace architect $PROJECT_REPO"
echo "  3. Launch both agents:      cd $TEAM_REPO && just launch human-assistant &"
echo "                              cd $TEAM_REPO && just launch architect &"
