# Invariant: Upgrade Path Tests

All upgrade paths must have integration tests that validate the transition
from version N to version N+1.

## Rules

1. Every new feature that changes persistent state must include an upgrade test
2. Upgrade tests must verify data migration from the previous schema
3. Rollback scenarios must be tested alongside forward upgrades
4. The upgrade test plan must be documented in the design doc

## Rationale

HCP manages long-lived clusters that undergo in-place upgrades. Untested
upgrade paths lead to data loss and control plane outages. Designs must include
an upgrade test plan to comply with this invariant.
