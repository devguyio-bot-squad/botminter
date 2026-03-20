# Additional Idea: Dual-Channel Interaction (GitHub + Chat)

## Problem

The current scrum-compact profile lost human interaction capability when the telegram-specific variant was removed. Members can only interact via GitHub issue comments — there's no chat-based interaction. This is unnatural.

## Desired Behavior

Members should interact like real human team members:
- Sometimes they comment on the GitHub issue (design reviews, code review feedback, status updates)
- Sometimes they reach out via chat/bridge (blocking questions, quick clarifications, progress updates, "hey I'm stuck on X")
- The choice of channel is contextual, not hardcoded

This is the full SDLC flow: the agent works autonomously, uses GitHub for formal artifacts (issues, PRs, comments), and uses chat for informal communication (questions, updates, discussions).

## How Chat-First Solves This

The brain naturally has both channels available:
1. **GitHub** — via `gh` CLI (already available through Bash)
2. **Chat/Bridge** — the brain IS the chat agent, so any response it generates goes to the bridge

The brain's system prompt should instruct it to:
- Use GitHub comments for formal status transitions, reviews, and decisions that need to be recorded
- Use chat for blocking questions, quick updates, and informal discussion
- Never force all communication through one channel

## Impact on Profile

The scrum-compact profile needs:
- Brain system prompt template that includes dual-channel interaction instructions
- Hat instructions that reference the brain's ability to chat (not just `human.interact` events)
- The board-scanner / coordinator hat needs awareness that it can discuss work with the human via chat before starting
