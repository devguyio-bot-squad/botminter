# Research: Claude Code Prompt Injection Mechanisms

> Understanding the prompt hierarchy in Claude Code for `bm chat` implementation.

## Summary

Claude Code has three distinct prompt layers with different authority levels. `--append-system-prompt` is a literal string concatenation to the system prompt — no wrapping, no framing, no metadata added. CLAUDE.md is injected as a user message, NOT as part of the system prompt.

## Prompt Hierarchy (Highest to Lowest Authority)

```
1. System prompt (Claude Code defaults + --append-system-prompt)   ← highest
2. CLAUDE.md content (injected as user message)                    ← medium
3. User conversation messages                                      ← lowest
```

**Critical distinction:** CLAUDE.md is NOT part of the system prompt. It's injected as a user message after the system prompt. This means `--append-system-prompt` content has higher authority than CLAUDE.md content.

## Flag Behavior

| Flag | Behavior | What it does internally |
|------|----------|----------------------|
| `--append-system-prompt` | Literal string concatenation to system prompt | No wrapping, no framing, no metadata. Just appended. |
| `--append-system-prompt-file` | Same, from file | Works in both interactive and print modes (verified experimentally) |
| `--system-prompt` | **Replaces** entire default system prompt | Destructive — loses built-in capabilities |
| `--system-prompt-file` | Same, from file | Works in both interactive and print modes |
| `-p` | Non-interactive execution mode | Runs prompt once, exits. NOT a prompt injection mechanism. |

## CLAUDE.md vs System Prompt

| | `--append-system-prompt` | CLAUDE.md |
|---|---|---|
| **Injected as** | Part of system prompt | User message |
| **Authority** | Higher | Lower |
| **Persistence** | Session only (CLI flag) | Reloaded each session from files |
| **Discovery** | Explicit via flag | Auto-discovered from directory tree |
| **User-visible** | No | Yes (via `/memory`) |

## How Ralph Invokes Claude Code

Ralph does NOT use `--system-prompt` or `--append-system-prompt`. It passes the entire constructed prompt as a **user message** via `-p`:

```rust
// ralph-adapters/src/cli_backend.rs
pub fn claude() -> Self {
    Self {
        command: "claude".to_string(),
        args: vec![
            "--dangerously-skip-permissions".to_string(),
            "--verbose".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--disallowedTools=TodoWrite,TaskCreate,TaskUpdate,TaskList,TaskGet".to_string(),
        ],
        prompt_mode: PromptMode::Arg,
        prompt_flag: Some("-p".to_string()),
        ..
    }
}
```

For large prompts (>7000 chars), Ralph writes to a temp file and passes:
`claude -p "Please read and execute the task in /tmp/xxx"`

All Ralph-injected content (orientation, workflow, hat instructions, guardrails, skills, scratchpad, tasks) is ONE string passed as a user-level prompt. No system prompt manipulation.

## Implications for `bm chat`

Using `--append-system-prompt` for `bm chat` gives the role framing **higher authority** than it has inside Ralph (where it's a user message). This is intentionally correct for interactive sessions — role identity should stick even as the conversation grows.

### Meta-Prompt Structure

`bm chat` generates a meta-prompt that wraps Ralph's content with interactive mode framing. The meta-prompt is structured so that:

1. **Role identity and interactive mode framing** — directly in the appended system prompt, tells Claude who it is and that it's in interactive mode
2. **Applicable Ralph prompts** — guardrails, hat instructions, PROMPT.md content — embedded in the right sections of the meta-prompt
3. **Runtime-behavioral Ralph prompts** (events, tasks, scratchpad, workflow cycles) — kept in a reference file and mentioned in the meta-prompt as "when running in operation mode, you use these" rather than injected as active instructions. These are context for understanding, not directives for the interactive session.

This prevents confusing Claude Code with instructions about event publishing, loop mechanics, and task management that don't apply in interactive mode.

### Example Meta-Prompt Skeleton

```markdown
# Interactive Session — [Role Name]

You are [member name], a [role] on the [team name] team.

You normally run autonomously inside Ralph Orchestrator, processing work items
from the team's GitHub project board. Right now you are in an interactive session
with the human who assumes the PO role.

## Your Capabilities

[Hat instructions from ralph.yml — applicable to interactive mode]

## Guardrails

[Guardrails from ralph.yml — always apply]

## Role Context

[PROMPT.md content — role identity and cross-hat rules]

## Reference: Operation Mode

For context, when running autonomously inside Ralph Orchestrator, you follow
the operational workflows described in: [path to ralph-prompts reference file]
These do not apply in interactive mode — the human drives the workflow.
```
