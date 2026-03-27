# Additional Ideas (captured during planning)

## Blocking Human Interaction Mode

**Current behavior:** When Ralph hits `human.interact`, it sends the interaction and continues. The response gets appended as `human.guidance` on the next iteration — async, non-blocking.

**Desired new mode:** For blocking issues, Ralph should stop midway. The interaction reaches the brain first. The brain discusses the feedback with the human on the bridge. Once an answer is agreed upon, the brain injects the response into the next Ralph iteration.

### Possible Implementation Approaches

1. **Convention file:** Ralph's prompt includes a convention section: "Check `.ralph/pending-human-messages.md` for responses from the human." The brain writes agreed-upon answers there before starting the next iteration.

2. **`ralph run --continue`:** A new Ralph CLI flag that resumes a stopped loop with injected context. The brain calls `ralph run --continue --context "the human said X"`.

3. **BotMinter message injection:** Add support in the profile/hat instructions for a "messages from human" section that gets populated by the brain before re-launching the loop.

4. **Brain mediates all human interaction:** The brain intercepts ALL `human.interact` events. For non-blocking questions, it answers from knowledge/memory. For blocking questions, it stops the loop, discusses with the human, and restarts with the answer.

### Key Insight

The brain acts as an intelligent mediator between Ralph loops and the human. It doesn't forward every question — it uses judgment, like a good team member would. This is the core value proposition of the chat-first model.

## Constraints

- No pushing to any repository — commit only, review before push
- Upstream branches OK for Ralph Orchestrator and ACP modifications
- Full E2E, exploratory tests, docs, specs, ADRs required
