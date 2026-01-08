---
description: Review current working memory state and context
model: inherit
---

# Review Working Memory

Understand what knowledge wm has captured and what context is currently active.

**Platform Support:**
- ✅ **Claude Code** - Full support (session discovery works)
- ✅ **Codex** - Partial support (state/working set work, session listing limited)

## When to Use

- **Session start** — Review what patterns and constraints wm knows about this project
- **Before major changes** — Check if existing knowledge suggests an approach
- **Debugging unexpected behavior** — See if wm is injecting context that's affecting decisions
- **After distillation** — Verify what knowledge was extracted
- **Planning work** — Understand project patterns before designing solution

## What to Review

### 1. Current State (`wm show state`)

View all accumulated tacit knowledge:

```bash
wm show state
```

**What you'll see:**
- Rationale behind past decisions
- Constraints discovered through work
- Preferences revealed by corrections
- Patterns followed in this project
- Paths rejected and why

This is the "long-term memory" — knowledge accumulated across all sessions.

### 2. Active Context (`wm show working`)

View what context was last injected:

```bash
wm show working                      # Global working set
wm show working --session-id <id>   # Session-specific working set
```

**What you'll see:**
- Filtered subset of state.md relevant to last intent
- What context Claude received on last turn
- Useful for debugging: "Why is Claude suggesting X?"

This is the "short-term memory" — what's active in current context.

### 3. All Sessions (`wm show sessions`)

**Claude Code only:**

List all sessions for this project:

```bash
wm show sessions
```

**What you'll see:**
- Session IDs and timestamps
- Transcript locations
- Useful for running distillation or reviewing history

**Note:** In Codex, session listing requires manual discovery:
```bash
ls ~/.codex/sessions/2026/01/*/rollout-*.jsonl
```

### 4. Dive Context (`wm dive show`)

View current dive session context (if active):

```bash
wm dive show
```

**What you'll see:**
- Intent (fix, plan, explore, review, ship)
- Focus (what specifically you're working on)
- Constraints from guardrails or .superego/
- Workflow steps for this intent
- Context sources (local, git, OH, etc.)

## Example Workflow

```bash
# Starting new work - review what wm knows
wm show state

# Output shows:
# - "Prefer functional composition over inheritance"
# - "API calls require retry logic (flaky network)"
# - "Use snake_case for file names in this project"

# Now you understand the project patterns before coding
```

## Integration with Other Commands

### Review → Distill
```bash
# Check current state
wm show state

# Seems sparse - run distillation
wm distill

# Review again
wm show state
```

### Review → Compress
```bash
# Check state size
wm show state | wc -l
# Output: 87 lines

# Too large - compress
wm compress

# Review result
wm show state | wc -l
# Output: 34 lines
```

### Review → Dive Prep
```bash
# Review project knowledge
wm show state

# Now prepare focused dive with that context in mind
/dive-prep --intent fix
```

## Understanding State vs Working Set

**State.md** (Long-term Memory):
- All accumulated knowledge
- May contain items not relevant to current task
- Read with: `wm show state`

**Working Set** (Short-term Memory):
- Filtered for current intent
- Only relevant items injected to Claude
- Read with: `wm show working`

**Example:**

If `state.md` contains:
```markdown
- Prefer functional style over OOP
- API calls need retry logic
- Use PostgreSQL for persistence
- Frontend uses React hooks pattern
```

And your intent is "implement API retry logic", then `working_set.md` might contain:
```markdown
- API calls need retry logic
- Prefer functional style over OOP
```

The PostgreSQL and React items weren't relevant to the intent, so they weren't injected.

## Pause Status

Check if wm operations are paused:

```bash
wm status
```

**Output:**
```
Working Memory Status
=====================

Extract:  active
Compile:  active

All operations running normally.
```

Or if paused:
```
Extract:  PAUSED
Compile:  active

Extract is paused. Run 'wm resume extract' to re-enable.
```

## When to Use Each Command

| Command | When to Use |
|---------|-------------|
| `wm show state` | Understand accumulated project knowledge |
| `wm show working` | Debug what context was injected |
| `wm show sessions` | Find sessions for distillation or audit |
| `wm dive show` | Review current dive context |
| `wm status` | Check if operations are paused |

## Agent Usage

As an agent, invoke this skill when you need to:
- Understand project patterns before suggesting approach
- Debug why certain context is appearing
- Verify knowledge was captured after distillation
- Check if dive context is active
