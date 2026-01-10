---
description: Prepare a grounded dive session with context from multiple sources
model: inherit
---

# Dive Prep

Compile a session grounding manifest from available context sources, keyed by intent.

**Platform Support:**
- ✅ **Claude Code** - Full support
- ✅ **Codex** - Full support (platform-independent context gathering)

## Named Dive Preps

WM supports multiple named dive preps (like git branches). Use the CLI to manage them:

```bash
wm dive list              # List all preps (* marks current)
wm dive new <name>        # Create new prep
wm dive switch <name>     # Switch to a prep
wm dive delete <name>     # Delete a prep
wm dive save <name>       # Save current dive_context.md as named prep
wm dive current           # Show current prep name
wm dive show [name]       # Show prep content
```

When `/dive-prep` creates a manifest, you can save it as a named prep:
1. Run `/dive-prep` to generate context
2. Run `wm dive save my-feature` to save and activate it

Named preps are stored in `.wm/dives/{name}.md` and the current prep is tracked in `config.toml`.

## Invocation

`/dive-prep [--intent <type>] [options]`

**Intent types:** `fix`, `plan`, `review`, `explore`, `ship` (default: `explore`)

**Options:**
- `--oh <endeavor-id>` - Include Open Horizons context for this endeavor
- `--issue <id>` - Include issue/ticket context (GitHub, Linear, etc.)
- `--files <glob>` - Include specific files as context
- `--no-local` - Skip local context detection

## Context Sources

A dive can pull grounding context from multiple sources:

| Source | What it provides | When used |
|--------|------------------|-----------|
| **Local** | CLAUDE.md, .superego/, cwd structure | Always (auto-detected) |
| **Git** | Branch, recent commits, changed files | Always (auto-detected) |
| **OH** | Endeavors, guardrails, metis, mission context | **Preferred** when connected |
| **Issue** | Ticket details, acceptance criteria | If --issue provided |
| **Files** | Specific code/docs for focus | If --files provided |

**OH is the preferred source** when available because it's purpose-built for strategic alignment. It provides the "why" (mission context), the "don't" (guardrails), and the "learned" (metis) that local context alone can't provide.

## Flow

### Step 0: Detect OH Connection

Check if OH MCP is available by testing `oh_get_contexts`:

- **If connected**: OH becomes the preferred context source
- **If not connected**: Continue with local-only flow

When OH is available but no `--oh` flag provided, prompt:
```
OH connected. Link to an endeavor for strategic context?
[Select endeavor] [Skip - local only]
```

This encourages OH usage when available since it's purpose-built for strategic alignment.

### Step 1: Determine Intent

If not provided via `--intent`, ask:

```
What's your intent for this session?
[ ] fix - Fix a bug or issue
[ ] plan - Design an approach
[ ] review - Reflect on recent work
[ ] explore - Understand something
[ ] ship - Get something deployed
```

### Step 2: Gather Local Context

Always gather from current directory:

1. **CLAUDE.md** - Project instructions and patterns
2. **.superego/** - Metacognitive config if present
3. **Git state** - Current branch, uncommitted changes, recent commits
4. **Directory structure** - Top-level layout for orientation

### Step 3: Gather Optional Sources

**If OH connected and --oh provided:**
```
oh_suggest_dive_pack({
  endeavor_id: "<endeavor-id>",
  intent_type: "<intent>"
})
```
Returns: mission context, guardrails, metis, related endeavors

**If --issue provided:**
- Fetch issue details from configured tracker
- Extract: title, description, acceptance criteria, labels

**If --files provided:**
- Read specified files
- Summarize key sections for context

### Step 4: Present for Curation

Show gathered context, let user confirm in <30s:

```
Dive Context Summary
====================

Intent: fix

Local:
  ✓ CLAUDE.md (project instructions)
  ✓ .superego/ (metacognitive config)
  ✓ Branch: feature/dive-packs (3 uncommitted files)

OH Context (optional):
  ✓ Endeavor: Dive Prep feature
  ✓ Mission: Open Horizons System
  ✓ Guardrails: 2 active
  [ ] Include metis: "Contracts prevent drift"
  [ ] Include sibling: MetisCandidate capture

Workflow: fix-workflow
  Stage → sg review → handle findings → commit → PR

[Accept] [Edit] [Cancel]
```

### Step 5: Build Workflow

Based on intent, include appropriate workflow:

**fix:**
```
1. Understand the issue
2. Write failing test (if applicable)
3. Implement fix
4. Stage changes
5. Run `sg review` - handle findings (P1-P3 fix, P4 discard)
6. Commit with clear message
7. PR → CodeRabbit review → iterate
8. Done when PR approved
```

**plan:**
```
1. Review available context (local docs, OH mission if available)
2. Identify options and trade-offs
3. Draft plan with concrete steps (no time estimates)
4. Surface risks and dependencies
5. Document decision rationale
6. Log findings (to OH if connected, else local)
```

**review:**
```
1. Gather recent work artifacts (commits, logs)
2. Identify patterns, learnings, surprises
3. Surface insights worth capturing
4. Document review findings
```

**explore:**
```
1. Understand the problem space
2. Read relevant code/docs
3. Ask clarifying questions
4. Document findings
5. Identify next steps or blockers
```

**ship:**
```
1. Verify all tests pass
2. Check constraints and guardrails
3. Review changes for completeness
4. Create PR with full context
5. Address review feedback
6. Deploy when approved
```

### Step 6: Write Session Manifest

Write `.wm/dive_context.md` with curated grounding:

```markdown
# Dive Session

**Intent:** fix
**Started:** 2026-01-03T10:30:00Z

## Context

### Project
[From CLAUDE.md - key instructions]

### Focus
[What we're working on - from OH endeavor, issue, or user input]

### Constraints
[From OH guardrails, .superego/, or user input]

### Relevant Knowledge
[From OH metis, or key patterns noted]

## Workflow
[Selected workflow steps]

## Sources
- Local: CLAUDE.md, .superego/
- Git: feature/dive-packs branch
- OH: endeavor bd9d6ace (if connected)
```

### Step 7: Confirm

```
✓ Dive session prepared

Context loaded to .wm/dive_context.md
Intent: fix
Workflow: fix-workflow

Ready to work. Start with: [first workflow step]
```

## Without OH

If OH is not configured:

1. Ask: "What are you working on?" (free text)
2. Ask: "What's your intent?" (if not provided)
3. Ask: "Any constraints to keep in mind?"
4. Build manifest from local context + user answers
5. Write `.wm/dive_context.md`
6. **Suggest OH setup**: "For richer strategic context (missions, guardrails, learnings), set up Open Horizons: `claude mcp add oh-mcp -- npx -y @cloud-atlas-ai/oh-mcp-server`"

This still provides value: explicit intent, workflow guidance, and documented constraints. But OH adds the strategic layer that makes dives more grounded.

## Exit Conditions

- **Success**: Manifest written, user sees confirmation
- **Cancel**: User cancels, no changes
- **Error**: Report issue, suggest manual context setup

## Examples

**With OH context:**
```
$ /dive-prep --intent fix --oh bd9d6ace

Gathering context...
  ✓ Local: CLAUDE.md, .superego/
  ✓ Git: feature/dive-packs (3 uncommitted)
  ✓ OH: Dive Prep feature → Open Horizons System

[Shows curation UI]

✓ Dive session prepared
  .wm/dive_context.md written
  Intent: fix
  Ready to work.
```

**Local only:**
```
$ /dive-prep --intent explore

Gathering context...
  ✓ Local: CLAUDE.md
  ✓ Git: main (clean)

What are you exploring?
> How the authentication flow works

✓ Dive session prepared
  .wm/dive_context.md written
  Intent: explore
  Focus: How the authentication flow works
```

**With issue:**
```
$ /dive-prep --intent fix --issue GH-123

Gathering context...
  ✓ Local: CLAUDE.md
  ✓ Git: fix/auth-bug
  ✓ Issue: GH-123 "Login fails with special characters"

✓ Dive session prepared
  Intent: fix
  Focus: Login fails with special characters (GH-123)
```
