# Commit Convention

All commits must reference an issue number using `Ref: #<number>` in the commit
body. This ensures traceability between code changes and tracked work items.

## Format

```
<type>(<scope>): <subject>

<body>

Ref: #<issue-number>
```

## Types

- `feat` — new feature
- `fix` — bug fix
- `docs` — documentation only
- `refactor` — code restructuring without behavior change
- `test` — adding or updating tests
- `chore` — maintenance tasks

## Rules

1. Subject line under 72 characters
2. Body explains the **why**, not the **what**
3. Every commit MUST include `Ref: #<number>` linking to the relevant issue number
4. One logical change per commit
