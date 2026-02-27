# Review — Sprint 5: Plan + PROMPT

> Findings from parallel review against `specs/design-principles.md`.
> Reviewed: `plan.md`, `PROMPT.md`
> Reference: `design.md`, `design-principles.md`, prior sprint PROMPTs/plans

---

## Resolved

- [x] **Supervised mode toggle pattern** — `SUPERVISED MODE: ENABLED` declared in PROMPT.md but no hat instruction checks it. Unlike training mode (every hat has `If TRAINING MODE is ENABLED`), supervised mode is baked into backlog_manager/review_gater core workflows. Decision needed: drop the toggle or add conditionals. *(discussed, pending decision)*
- [x] **Member rename** — `all-in-one` → `superman` across plan.md, PROMPT.md, design.md.

---

## Plan Issues

### P1 — Hat count inconsistency (medium)

Plan header and Step 3 title say "12 hats" but Step 3 body (line ~270) correctly identifies 14 (12 + writer + content_reviewer). The design.md itself is inconsistent — Section 4.1 table has 12 numbered rows, but Section 9.3 says "7 actually" for the 6 new hats. Plan should be self-consistent.

**Fix:** Update header and Step 3 title to say "14 hats" or "12 core + writer/content_reviewer".
 >>> Approved. 

### P2 — RObot configuration missing (medium)

Step 3 (ralph.yml) never mentions `RObot: enabled: true, timeout_seconds: 600, checkin_interval_seconds: 300`. Step 5 launches with `--telegram-bot-token`. Sprint 3 explicitly added RObot config to both agents' ralph.yml.

**Fix:** Add RObot config subsection to Step 3.
>>> Approved. 

### P3 — ralph.yml top-level config incomplete (low)

Plan mentions `persistent: true` and `board.scan` starting event but omits other config keys that Sprint 1 specified: `tasks`, `memories`, `skills`, `max_iterations`, `max_runtime`.

**Fix:** Add a brief ralph.yml config preamble to Step 3.

### P4 — Content_reviewer terminal status not design-backed (low)

Plan says content_reviewer transitions to `status/done` on approval. Design Section 4.5 only addresses `arch:sign-off` and `po:merge` auto-advance — doesn't specify the content flow terminal status. Reasonable assumption but not explicitly designed.

**Fix:** Note in plan that this is a plan-level decision filling a design gap, or update design.md.

### P5 — Knowledge paths for implementer/test_designer unexplained (low)

Plan lists these as having `### Knowledge` sections, which aligns with design Section 4.4 but goes beyond the design-principles Section 8 reference examples (which only show designer/planner with knowledge). These hats need knowledge because they read parent epic design and project context.

**Fix:** Add a brief note explaining why these hats need knowledge paths.

### P6 — Fixture adaptation underspecified (low)

Step 5 says "adapt M2 fixtures" but the specific changes needed (profile path, member name, seeding status, no write-lock fixtures) could be more explicit.

**Fix:** List the specific adaptations: (a) profile `compact` not `rh-scrum`, (b) member `superman` not `architect`/`human-assistant`, (c) seed at `po:triage` not `arch:design`, (d) no write-lock fixtures.

### P7 — Design.md internal contradiction inherited (low)

Design Section 3.3 annotation says "Every hat returns to the board scanner — no hat-to-hat direct dispatch" but the design's own event flow, hat table, and lifecycle sequence all show direct chain dispatch. Plan correctly follows the actual design (direct chain). Contradiction is in the design, not the plan.

**Fix:** Note only — fix in design.md if desired, not in plan.

---

## PROMPT Issues

### R1 — No RFC 2119 language (critical)

Zero MUST/MUST NOT/SHOULD/MAY anywhere in the document. Design-principles Section 11 explicitly requires this. Sprints 3 and 4 use RFC 2119 heavily. The anti-pattern table lists "casual language for hard constraints" as something to avoid.

**Fix:** Add RFC 2119 keywords throughout. Examples:
- Req 1: "PROCESS.md MUST be copied and adapted from `rh-scrum`"
- Req 3: "Story TDD hats MUST use direct chain dispatch"
- ACs: "then the generated repo MUST contain..."
>>>Approved. 
### R2 — Incomplete Given-When-Then in ACs 3, 4, 10 (medium)

ACs 3, 4, and 10 use "Given X, then Y" without a "When" clause. Design principles require Given-When-Then format.

**Fix:** Add "When" clauses:
- AC 3: "Given the compact ralph.yml, **when the dispatch table is inspected**, then it covers all statuses..."
- AC 4: "Given the compact ralph.yml, **when the story TDD hats are inspected**, then they use direct chain dispatch..."
- AC 10: "Given comments produced during the lifecycle, **when inspected**, then each comment uses the correct role header..."
>>> Approved. 
### R3 — No verification tier split (medium)

Sprint 3 split ACs into Tier 1 (Ralph verifies by file inspection) and Tier 2 (manual with live Telegram). Sprint 5 has a similar mix: ACs 1-5 are file-inspection checks, ACs 6-10 require a running agent. The PROMPT doesn't distinguish between them.

**Fix:** Either add a tier split (like Sprint 3) or add a note clarifying that ACs 6-10 require live agent execution.
>>>  Not needed
### R4 — Hat count ambiguity (medium)

Requirement 3 lists "auxiliary hats (infra_setup, writer, content_reviewer)" but doesn't state the total hat count. Combined with design.md's "12 hats" vs actual 14, Ralph could be confused about how many hats to build.

**Fix:** State explicitly: "14 hats total (12 from design Section 4.1 plus writer and content_reviewer from Section 4.4)."
>>> Approved
### R5 — No dedicated AC for auto-advance (medium)

Auto-advance at `arch:sign-off` → `po:merge` → `done` is mentioned in the story lifecycle AC but not tested in isolation. A separate AC would verify the board scanner handles these statuses correctly.

**Fix:** Add: "Given a story at `status/arch:sign-off`, when the board scanner processes it, then it auto-advances to `status/po:merge` and then to `status/done` without dispatching a hat."
>>> approved
### R6 — No regression AC for rh-scrum (low)

Sprint 5 adds a new profile alongside `rh-scrum`. No AC verifies the existing profile still works. Sprints 3 and 4 include `(Regression)` ACs.

**Fix:** Add: "(Regression) Given the compact profile exists, when `just init --profile=rh-scrum` runs, then the rh-scrum profile still generates correctly."

### R7 — No AC for auxiliary hats (low)

Writer, content_reviewer, and infra_setup are in Requirement 3 but have no acceptance criteria verifying they exist or function.

**Fix:** Add AC for auxiliary hats, or note them as deferred validation (not exercised by the synthetic epic fixture).

### R8 — No poll-log AC (low)

Previous sprints (Sprint 1, Sprint 2) include ACs for `poll-log.txt`. Sprint 5 does not.

**Fix:** Add: "Given the superman agent completes a scan cycle, then `poll-log.txt` contains clean scan entries."

### R9 — No error handling AC (low)

Design Section 6.1 describes "Attempt N/3" failure pattern and escalation to `status/error`. No AC tests this.

**Fix:** Consider adding, or note as deferred — error handling is inherited from the rh-scrum pattern and doesn't need re-validation unless the compact profile changes the behavior.

### R10 — Key References includes requirements.md (low)

Sprint 5 lists `requirements.md` in Key References. Sprints 1-4 don't. Not wrong — actually a good addition — but a minor format inconsistency.

**Fix:** Keep it (it's useful), or remove for consistency. Low priority.
