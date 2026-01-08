---
description: Pause or resume wm operations (extract, compile, or both)
model: inherit
---

# Pause and Resume

Temporarily disable wm operations without uninstalling or removing configuration.

**Platform Support:**
- ✅ **Claude Code** - Full support (hooks respect pause state)
- ✅ **Codex** - Full support (manual extractions respect pause state)

## When to Use

### Pause Extract
- **Sensitive work**: Working on confidential code that shouldn't be extracted
- **Personal debugging**: Troubleshooting without knowledge capture
- **Temporary privacy**: Preventing certain conversations from being recorded

### Pause Compile
- **Simple tasks**: Skip context injection for trivial work (faster, lower cost)
- **Testing isolation**: Want to work without historical context
- **Debugging wm**: Isolate issues by disabling injection

### Pause Both
- **Extended break**: Disabling wm entirely for a period
- **Project handoff**: Pausing before transferring to someone else
- **Performance testing**: Measuring impact of wm on session speed/cost

## Usage

```bash
# Pause both operations
wm pause

# Pause only extraction
wm pause extract

# Pause only compilation
wm pause compile

# Resume both operations
wm resume

# Resume only extraction
wm resume extract

# Resume only compilation
wm resume compile

# Check current status
wm status
```

## How It Works

Pause/resume modifies `.wm/pause_state.json`:

```json
{
  "extract_paused": false,
  "compile_paused": false
}
```

**Hook behavior when paused:**
- **Extract paused**: Stop hook gracefully exits without extraction
- **Compile paused**: UserPromptSubmit hook returns empty context

No configuration files are modified. No data is lost. Just a flag.

## Status Check

```bash
wm status
```

**Active (normal):**
```
Working Memory Status
=====================

Extract:  active
Compile:  active

All operations running normally.
```

**Paused:**
```
Working Memory Status
=====================

Extract:  PAUSED
Compile:  active

Extract is paused. Run 'wm resume extract' to re-enable.
```

## Common Scenarios

### Scenario 1: Sensitive Code Review

```bash
# Working on security-critical code
wm pause extract

# Work on sensitive changes
# ... conversation happens ...

# Done with sensitive work
wm resume extract
```

### Scenario 2: Quick Fix Without Context

```bash
# Need fast fix, don't need historical context
wm pause compile

# Make quick change
# ... conversation is faster, no context injected ...

# Resume normal operation
wm resume compile
```

### Scenario 3: Complete Pause

```bash
# Taking break from project
wm pause

# ... time passes ...

# Resuming project work
wm resume
```

## Integration with Other Operations

### Pause + Distill

If you paused extract for a while and missed knowledge capture:

```bash
# Check status
wm status
# Extract: PAUSED (for last 2 weeks)

# Resume
wm resume extract

# Catch up with batch extraction
wm distill
```

### Pause + Dive Prep

Dive prep works even if compile is paused:

```bash
# Compile is paused
wm status
# Compile: PAUSED

# Dive prep still gathers context
/dive-prep --intent fix

# Creates .wm/dive_context.md (independent of pause state)
```

## State Persistence

Pause state persists across:
- Sessions (stays paused in new Claude Code sessions)
- Restarts (survives Claude Code restart)
- Directory changes (tied to project `.wm/` directory)

To fully reset:
```bash
rm .wm/pause_state.json
# Operations resume automatically
```

## Best Practices

- **Default to active**: Only pause when you have a specific reason
- **Resume after sensitive work**: Don't forget to resume when done
- **Check status regularly**: Run `wm status` if unsure
- **Document reasons**: If pausing long-term, note why (in .wm/README or similar)

## Agent Usage

As an agent, you might invoke pause/resume when:
- User mentions working on sensitive/confidential code → suggest `wm pause extract`
- User asks for faster responses on trivial task → suggest `wm pause compile`
- User is debugging wm behavior → suggest isolating with pause
- User finished sensitive work → remind to `wm resume`

**Example dialogue:**

User: "I need to work on some confidential API keys, don't want this captured"

Agent: "I'll pause wm extraction so this session isn't recorded."
```bash
wm pause extract
```

[... work on sensitive code ...]

Agent: "Done with the confidential work. Resuming wm extraction."
```bash
wm resume extract
```
