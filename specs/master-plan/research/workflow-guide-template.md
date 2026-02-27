# Agentic Development Workflow for OpenShift Teams

> A practical guide for setting up Claude Code-assisted development workflows.
> Authored during [Shift Week Plan](shift-week-plan.md)

## Overview
<!-- One paragraph: what this guide covers and who it's for -->

## Prerequisites
- [ ] Claude Code installed and configured
- [ ] API key with sufficient credits
- [ ] Sandboxing solution: <!-- nono.sh or Zenzana -->
- [ ] Git repository with CLAUDE.md set up
- [ ] <!-- tool-specific prerequisites -->

## Architecture

### How It Works
<!-- Diagram or description of the workflow components and how they interact -->

```
[Requirements / Spec]
        ↓
  [Orchestrator]  ←→  [Sandbox / Container]
        ↓
  [Claude Code Agent(s)]
        ↓
  [Implementation → Tests → Debug]
        ↓
  [PR / Review]
```

### Components
| Component | Tool | Purpose |
|-----------|------|---------|
| Orchestrator | <!-- winner --> | Task planning, work breakdown, progress tracking |
| Sandbox | <!-- nono.sh or Zenzana --> | Secure, unattended agent execution |
| Context | CLAUDE.md + <!-- skills/hooks --> | Persistent knowledge, project-specific guidance |
| Knowledge Base | <!-- approach --> | Team-shared patterns, troubleshooting, domain knowledge |

## Setup

### Step 1: Install the Orchestrator
```bash
# TODO: fill in after tool is chosen
```

### Step 2: Configure Sandboxing
```bash
# TODO: nono.sh or Zenzana setup
```

### Step 3: Project-Level Configuration
```bash
# CLAUDE.md, skills, hooks, etc.
```

### Step 4: Verify the Setup
```bash
# Smoke test: run a simple task through the full pipeline
```

## Workflow: From Requirement to PR

### 1. Define the Task
<!-- How to write a spec / task definition that the agent can act on -->

### 2. Launch the Agent
<!-- Command(s) to start the agent on the task -->

### 3. Monitor Progress
<!-- How to check on unattended agent progress -->

### 4. Review Output
<!-- What to look for, how to iterate -->

### 5. Submit for Review
<!-- PR creation, CI integration -->

## Building Team Knowledge

### CLAUDE.md Structure
<!-- What goes in CLAUDE.md, how to organize it for the team -->

### Skills & Hooks
<!-- How to build and share reusable skills -->

### Sharing the Setup
<!-- How a new team member gets onboarded to this workflow -->

## HyperShift-Specific Notes
<!-- Domain-specific tips, common patterns, gotchas -->
<!-- Keep this section separate so the rest of the guide stays portable -->

## Troubleshooting
| Problem | Solution |
|---------|----------|
| Agent gets stuck in a loop | |
| Sandbox permissions issue | |
| Context too large | |
| Agent makes incorrect assumptions | |

## Appendix

### Evaluation: Why This Tool?
See [Tool Evaluation & Decision](tool-evaluation-and-decision.md) for the full comparison.

### References
- Ralph Orchestrator: https://mikeyobrien.github.io/ralph-orchestrator/
- multiclaude: https://github.com/dlorenc/multiclaude
- GSD: https://github.com/glittercowboy/get-shit-done
- claude-pilot: https://github.com/maxritter/claude-pilot
- nono.sh: https://nono.sh/
- Zenzana: https://github.com/devguyio/zenzana
