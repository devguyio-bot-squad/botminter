# Rough Idea: Loop Inbox

Source: `.planning/plans/loop-inbox-plan.md`

The brain (ACP-wrapped Claude Code) can observe Ralph loop events via the EventWatcher, but has no way to send feedback back to running loops. This creates a one-way observation channel. When a human says "stop refactoring, fix CI" on the bridge, the brain can acknowledge it but cannot steer its own working loops.

The goal is a per-loop inbox mechanism where the brain can write messages that are delivered to the coding agent inside the loop, allowing real-time steering of running work.
