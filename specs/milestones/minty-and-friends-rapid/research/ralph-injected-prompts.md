# Research: Ralph Orchestrator Injected Prompts

> Comprehensive inventory of all prompts Ralph hardcodes in its Rust codebase and injects at runtime — invisible to profiles today.

## Summary

Ralph injects ~13 categories of hardcoded prompts into the coding agent session. These are compiled into the Rust binary and NOT configurable via ralph.yml or PROMPT.md. For `bm chat` to replicate what the agent sees inside Ralph, these must be extracted and shipped with BotMinter profiles.

## Injection Flow

```
EventLoop::build_prompt()
  1. Auto-inject skills (memories, ralph-tools, robot interaction, custom skills)
  2. Scratchpad content (<scratchpad> XML tags)
  3. Ready tasks (<ready-tasks> XML tags)
  4. HatlessRalph::build_prompt() — core prompt + workflow + hats + events
```

## Injected Prompt Categories

### 1. Orientation (hatless_ralph.rs)

```
### 0a. ORIENTATION
You are Ralph. You are running in a loop. You have fresh context each iteration.
You MUST complete only one atomic task for the overall objective.
```

Tells the agent it's in a loop with fresh context each iteration.

### 2. Scratchpad Instructions (hatless_ralph.rs)

Instructions for using the scratchpad file as a thinking journal — what to use it for (reasoning, analysis, plan narrative) and what NOT to use it for (task tracking).

### 3. State Management (hatless_ralph.rs)

Defines four state channels: Tasks (ralph tools task), Scratchpad (thinking), Memories (persistent learning), Context Files (research artifacts). Clear separation of concerns.

### 4. Guardrails Framing (hatless_ralph.rs)

Wraps guardrails from ralph.yml config with numbered section (999+).

### 5. Workflow Sections (hatless_ralph.rs)

Four workflow variants depending on mode:
- **Solo mode (scratchpad-only)**: Study → Plan → Implement → Commit → Repeat
- **Solo mode (with memories)**: Study → Plan → Implement → Verify & Commit → Exit (one task per iteration)
- **Multi-hat coordinating**: Plan → Delegate (publish ONE event)
- **Multi-hat fast path**: Publish starting_event immediately

### 6. Objective Section (hatless_ralph.rs)

Dynamic — injects the current objective with framing: "This is your primary goal. All work must advance this objective."

### 7. Robot Guidance (hatless_ralph.rs)

Dynamic — injects human guidance messages from RObot.

### 8. Pending Events (hatless_ralph.rs)

Dynamic — lists events that must be handled this iteration.

### 9. Hats Section (hatless_ralph.rs)

Two modes:
- **Coordinating (no active hat)**: Shows hat topology table + mermaid event flow diagram + valid events constraint
- **Active hat**: Shows hat instructions + event publishing guide + tool restrictions (if any)

### 10. Event Writing (hatless_ralph.rs)

Instructions for using `ralph emit` to publish events. Constraints: stop working after publishing, don't continue with additional work.

### 11. Done Section (hatless_ralph.rs)

Instructions for emitting the completion event. Pre-completion checklist: verify all tasks closed before declaring done.

### 12. Custom Hat Instruction Template (instructions.rs)

For hats with custom configurations, wraps their instructions with:
- "You are <hat_name>. You have fresh context each iteration."
- Orientation, Execute, Verify, Report phases
- Guardrails
- Event context

### 13. Auto-Injected Skills (data/)

Two built-in skills injected as XML tags:
- **ralph-tools.md**: Task and memory management CLI commands
- **robot-interaction-skill.md**: Human-in-the-loop interaction via `ralph emit human.interact`

## What `bm chat` Needs

For interactive mode, NOT all injected prompts apply. Categorized by relevance:

### Applicable to interactive mode
- Guardrails (always apply)
- Hat instructions (the role's capabilities)
- State management concepts (scratchpad, memories)
- Event context (what the agent is working on)

### NOT applicable to interactive mode
- Loop orientation ("You are running in a loop") — replaced by interactive mode framing
- Workflow sections (Study/Plan/Implement/Commit cycle) — human drives the workflow
- Done section (completion events) — human decides when done
- Event writing (ralph emit) — no event loop in interactive mode
- Fast path — no event dispatch

### Needs adaptation
- Hats section — show available hats but don't auto-dispatch
- Pending events — show context but don't require handling

## Source Files

| File | Content |
|------|---------|
| `crates/ralph-core/src/hatless_ralph.rs` | Core prompt, workflow, hats, events, done sections |
| `crates/ralph-core/src/instructions.rs` | Custom hat instruction template, derived behaviors |
| `crates/ralph-core/data/ralph-tools.md` | Built-in ralph-tools skill |
| `crates/ralph-core/data/robot-interaction-skill.md` | Built-in robot interaction skill |
| `crates/ralph-core/src/memory_store.rs` | Memory formatting and injection |
| `crates/ralph-core/src/event_loop/mod.rs` | Injection orchestration, XML tag wrapping |
