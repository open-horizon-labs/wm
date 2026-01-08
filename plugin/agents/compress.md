---
description: Compress and synthesize state.md by abstracting to higher-level patterns
model: inherit
---

# Compress Knowledge

Synthesize accumulated knowledge in `state.md` to higher-level abstractions, reducing size while preserving critical insights.

**Platform Support:**
- ✅ **Claude Code** - Full support
- ✅ **Codex** - Full support (platform-independent)

## When to Use

- **State.md is growing unwieldy** — Too many specific items, hard to navigate
- **After distillation** — Batch extraction added many items that need synthesis
- **Periodic maintenance** — Every few weeks/months as knowledge accumulates
- **Before sharing** — Clean up state before committing to repo or sharing with team

## What It Does

1. **Reads state.md** — Analyzes all accumulated knowledge
2. **Identifies patterns** — Finds related items that can be merged
3. **Abstracts specifics** — Converts concrete instances into general principles
4. **Removes obsolete items** — Discards superseded or no-longer-relevant knowledge
5. **Preserves critical constraints** — Keeps important guardrails and preferences
6. **Creates backup** — Saves original to `.wm/state.md.backup` before modifying

## Usage

```bash
# Compress state.md
wm compress

# Example output:
# Compressed: 42 → 18 lines (57% reduction)
# Backup saved to .wm/state.md.backup
```

## Compression Strategies

The LLM applies these strategies when compressing:

### 1. Merge Related Items
**Before:**
```markdown
- Prefer `Result<T, E>` for recoverable errors in Rust
- Use `Option<T>` for nullable values in Rust
- Return early with `?` operator in Rust
```

**After:**
```markdown
- Rust error handling: Use `Result<T, E>` for recoverable errors, `Option<T>` for nullables, early return with `?` operator
```

### 2. Abstract to Principles
**Before:**
```markdown
- Don't use `unwrap()` in production code
- Don't use `expect()` without clear reason
- Don't ignore error cases
```

**After:**
```markdown
- Error handling principle: Always handle errors explicitly; never panic in production
```

### 3. Remove Obsolete
**Before:**
```markdown
- Use old API version 1.x (deprecated)
- Migrate to API version 2.x (in progress)
- API 3.x is now stable and preferred
```

**After:**
```markdown
- Use API version 3.x (current stable)
```

### 4. Preserve Critical Constraints
These are NEVER compressed away:
- Security constraints (e.g., "never log passwords")
- Architecture decisions (e.g., "use event sourcing for audit trail")
- User preferences (e.g., "prefer functional style over OOP")

## Integration with Distill

Compress works well after distillation:

```bash
# 1. Batch extract from all sessions
wm distill

# 2. Review raw extractions
cat .wm/distill/raw_extractions.md

# 3. Compress both raw extractions and state.md
wm compress

# Result: synthesized knowledge in state.md
```

## Recovery

If compression removes something important:

```bash
# Restore from backup
cp .wm/state.md.backup .wm/state.md

# Or manually cherry-pick from backup
cat .wm/state.md.backup  # Review what was removed
```

Backups are timestamped, so multiple compressions create multiple backups.

## Best Practices

- **Review after compression**: Check that critical knowledge wasn't lost
- **Run periodically**: Don't wait until state.md is massive (harder to compress effectively)
- **Manual curation first**: Remove obviously obsolete items manually before compressing
- **Backup awareness**: Know that `.wm/state.md.backup` exists for recovery

## When NOT to Use

- **State.md is small** — No need to compress if it's under ~30-40 lines
- **Just started wm** — Let knowledge accumulate first before compressing
- **Uncertain about contents** — Review state.md manually before running automated compression
