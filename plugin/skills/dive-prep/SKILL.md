---
name: dive-prep
description: Prepare a grounded dive session with context from multiple sources
---

# Dive Prep

Prepare working memory context for a focused session by gathering context from multiple sources.

## Invocation

`/wm:dive-prep [intent or context]`

## Execution

**DO NOT run any bash command.** This skill requires an AI agent.

Use the Task tool to spawn the wm:dive-prep agent:

```
Task(subagent_type: "wm:dive-prep", prompt: "<user's intent or context>")
```

If the user provides a GitHub issue URL, include it in the prompt:
```
Task(subagent_type: "wm:dive-prep", prompt: "Prepare dive for https://github.com/org/repo/issues/123")
```

## What the Agent Does

1. Detect OH connection and suggest linking endeavors
2. Gather local context (CLAUDE.md, git state, etc.)
3. Fetch OH context if available
4. Write `.wm/dive_context.md` with curated grounding

## Output

Pass through the agent's output to the user. The dive context will be saved to `.wm/dive_context.md`.

## Example

```
User: /wm:dive-prep fix the auth bug in the login flow

Action: Task(subagent_type: "wm:dive-prep", prompt: "fix the auth bug in the login flow")
```
