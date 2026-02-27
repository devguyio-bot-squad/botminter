# HCP Architecture

HCP (Hosted Control Planes) uses a reconciler pattern with controller-runtime
to manage the lifecycle of hosted Kubernetes control planes.

## Core Pattern

Each HCP resource is managed by a dedicated reconciler that watches for changes
and drives the actual state toward the desired state. The reconciler loop:

1. Receives an event (create, update, delete) for an HCP resource
2. Reads the desired state from the HCP spec
3. Compares with actual infrastructure state
4. Takes corrective action to converge

## Key Components

- **HCP Controller** — top-level reconciler managing the HCP resource
- **NodePool Controller** — reconciler for worker node pools
- **Control Plane Operator** — runs inside the hosted cluster, manages control
  plane components

## Technology Stack

- Go with controller-runtime framework
- Kubernetes custom resources (CRDs)
- etcd for state storage (hosted control plane etcd)

## Design Principles

- Each reconciler is independent and idempotent
- State is declarative — the HCP spec is the source of truth
- Failure recovery through re-reconciliation, not manual intervention
