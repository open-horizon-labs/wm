# AGENTS.md - Cloud Atlas AI Tool Guidance

## ba (Task Tracking)

**When to use:** Track work items, manage task lifecycle, coordinate with OH endeavors.

**Protocol:**
- `ba ready` - List available tasks
- `ba claim <id> --session $SESSION_ID` - Claim a task to work on
- `ba complete <id>` - Mark task done
- Tasks sync with GitHub issues and OH endeavors

## superego (Metacognitive Advisor)

**When to use:** Before commits, PRs, or when you want feedback on approach.

**Protocol:**
- `/superego:review` - Review staged changes
- `/superego:review pr` - Review full PR diff
- Mode is set to `pull` - reviews happen on-demand, not automatically

## wm (Working Memory)

**When to use:** Automatic - captures tacit knowledge from sessions.

**Protocol:**
- `wm distill` - Extract knowledge from Claude Code sessions
- `wm distill --codex` - Extract from Codex sessions
- `wm compile` - Inject context into current session
- Knowledge is automatically injected via hooks
