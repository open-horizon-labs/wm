---
description: Batch extract tacit knowledge from all sessions in this project
model: inherit
---

# Distill Knowledge

Extract tacit knowledge from all sessions at once, instead of relying on per-turn extraction.

**Platform Support:**
- ✅ **Claude Code** - Full support (auto-discovers sessions)
- ⚠️ **Codex** - Manual mode only (requires explicit transcript paths)

In Codex, you'll need to provide transcript paths manually since auto-discovery isn't yet implemented.

## When to Use

- **Project onboarding**: Extract knowledge from existing sessions when first setting up wm
- **Catchup after pause**: Process sessions that weren't extracted due to pause or missing hooks
- **Audit existing knowledge**: Review what patterns exist across all past work
- **Migration**: Moving from another system and want to bootstrap knowledge

## What It Does

**In Claude Code:**
1. **Discovers sessions** — Finds all transcripts in `~/.claude/projects/<project>/`
2. **Extracts incrementally** — Uses cache to only process new/changed sessions
3. **Accumulates knowledge** — Writes raw extractions to `.wm/distill/raw_extractions.md`
4. **Preserves existing state** — Doesn't overwrite `.wm/state.md` automatically

**In Codex (manual mode):**
1. Guide user to find transcripts in `~/.codex/sessions/YYYY/MM/DD/`
2. Extract from specific transcript: `wm extract --transcript <path>`
3. Repeat for multiple sessions
4. Review `.wm/distill/raw_extractions.md`

## Usage

**Claude Code (automatic):**
```bash
# Standard distillation (process new/changed sessions only)
wm distill

# Preview what would be processed
wm distill --dry-run

# Re-extract everything (ignores cache)
wm distill --force
```

**Codex (manual):**
```bash
# Find transcripts
ls ~/.codex/sessions/2026/01/*/rollout-*.jsonl

# Extract from specific sessions
wm extract --transcript ~/.codex/sessions/2026/01/06/rollout-2026-01-06T10-30-00-<uuid>.jsonl
wm extract --transcript ~/.codex/sessions/2026/01/05/rollout-2026-01-05T14-15-00-<uuid>.jsonl

# Review accumulated extractions
cat .wm/state.md
```

## Output Structure

```
.wm/distill/
├── raw_extractions.md    # All extracted knowledge (chronological)
├── cache.json            # Extraction cache (session ID → last processed)
└── errors.log            # Any extraction failures
```

## After Distillation

Review the raw extractions and decide what to do:

1. **Review raw_extractions.md** — See what knowledge was found
2. **Curate to state.md** — Manually copy relevant items to `.wm/state.md`, or run `wm compress` to auto-synthesize
3. **Compress if needed** — If state.md grows large, run `wm compress` to synthesize to higher abstractions

## Integration Notes

- **Complementary to per-turn extraction**: Distill catches what hooks missed, but hooks are still the primary mechanism
- **Idempotent**: Running multiple times is safe (uses cache)
- **Session isolation**: Each session's extraction is independent

## Example Workflow

```bash
# 1. Initialize wm in existing project
cd /your/project
wm init

# 2. Run distillation
wm distill

# 3. Review what was found
cat .wm/distill/raw_extractions.md

# 4. Check if anything worth adding to state
wm show state

# 5. If raw extractions have value, manually curate or compress
# Manual: copy relevant items from raw_extractions.md → state.md
# Auto: wm compress (synthesizes both files)
```

## When NOT to Use

- **Active sessions**: Distill is for batch processing, not real-time extraction
- **Every session**: Only run when you need catchup or audit—not after every conversation
- **As primary mechanism**: Hooks provide better per-turn extraction; distill is backup/catchup
