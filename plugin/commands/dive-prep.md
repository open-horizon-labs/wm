# /wm:dive-prep

Prepare working memory context for a focused session.

## Usage

```
/wm:dive-prep [intent or context]
```

## Execution

This command uses the `wm:dive-prep` Task agent to gather context from multiple sources and write `.wm/dive_context.md`.

**Do NOT run a CLI command.** Instead, invoke the Task tool:

```
Task(subagent_type: "wm:dive-prep", prompt: "<user's intent or context>")
```

If the user provides a GitHub issue URL, include it in the prompt:
```
Task(subagent_type: "wm:dive-prep", prompt: "Prepare dive for https://github.com/org/repo/issues/123")
```

The agent will:
1. Detect OH connection and suggest linking endeavors
2. Gather local context (CLAUDE.md, git state, etc.)
3. Fetch OH context if available
4. Write `.wm/dive_context.md` with curated grounding

Pass through the agent's output to the user.
