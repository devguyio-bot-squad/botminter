# GitHub Mutations: Hat-Only

**CRITICAL INVARIANT**

All GitHub issue mutations MUST happen inside a hat (subagent). The hatless orchestrator layer MUST NEVER modify GitHub issues directly.

## Forbidden at Orchestrator Layer

When operating as the hatless orchestrator (no active hat), the following operations are FORBIDDEN:

- ❌ Creating issues
- ❌ Commenting on issues
- ❌ Changing issue labels
- ❌ Transitioning issue statuses
- ❌ Assigning issues
- ❌ Closing/reopening issues
- ❌ Any other GitHub API mutations

## Allowed at Orchestrator Layer

The orchestrator MAY:

- ✅ Read issues (query, list, view)
- ✅ Read project board state
- ✅ Decide which issue to work on next
- ✅ Activate a hat to handle the work

## Rationale

**Separation of concerns:** The orchestrator decides **which** issue to work on; the hat does the **actual work** on it.

This separation ensures:
- Clear audit trail (all mutations attributed to a specific hat/role)
- Proper scoping of knowledge and invariants to the hat performing the work
- Clean boundaries between coordination logic and work execution

## Enforcement

When tempted to modify GitHub state:

1. Check: Am I currently in a hat?
2. If NO → Activate the appropriate hat first, then perform the mutation inside that hat
3. If YES → Proceed with the mutation

All GitHub operations use the `gh` skill. Before calling any mutation operation from the skill, verify you are inside a hat context.
