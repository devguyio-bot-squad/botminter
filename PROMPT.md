# Reconcile Codebase with ADR-0007 (Domain/Command Layering)

## Objective

Every command in `crates/bm/src/commands/` conforms to `.planning/adrs/0007-domain-command-layering.md`. All domain logic lives in domain modules. Commands are thin: load config, construct domain objects, call one method, display the result.

## Context

Read these files before writing any code:

1. `.planning/adrs/0007-domain-command-layering.md` — the authoritative definition of the desired state, including the command pattern, domain module structure, testing rules, and compliance checklist
2. `.planning/adrs/0006-directory-modules.md` — all domain modules are directory modules

## Constraints

- No public CLI behavior changes — same commands, same flags, same output
- No new dependencies
- All new domain modules are directory modules per ADR-0006
- Each extraction is an atomic commit following `refactor(<scope>): <description>`

## Success Criteria

Evidence required — all must pass:

- [ ] `just unit` passes
- [ ] No command file exceeds ~100 non-test lines: `for f in crates/bm/src/commands/*.rs; do code=$(sed '/^#\[cfg(test)\]/,$d' "$f" | wc -l); echo "$code $f"; done | sort -rn`
- [ ] No private functions in commands that lack output formatting: `grep -rn 'fn ' crates/bm/src/commands/ --include='*.rs'` — every function is `run()`, contains println/eprintln/table/prompt code, or is a test
- [ ] No types defined in commands used by other modules: `grep -rn 'commands::.*::' crates/bm/src/ --include='*.rs' | grep -v 'commands/mod.rs' | grep -v '#\[cfg(test)\]'` returns empty
- [ ] No cross-command exports — no command module has `pub` items consumed by another command
- [ ] Domain modules model concepts with structs and methods, not bags of free functions
- [ ] Domain operations return named result structs, not `Result<()>` for operations with meaningful output
- [ ] Domain tests assert on structured return types field by field, not just `is_ok()`
- [ ] No duplicated logic across command files

## Notes

- The ADR's compliance checklist (section "Compliance checklist") is the definitive quality gate for both commands and domain modules.
- Commit messages follow the project convention: `refactor(<scope>): <description>`.
