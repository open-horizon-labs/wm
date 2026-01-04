# CLAUDE.md

Working Memory (WM) — automatic tacit knowledge extraction and context management for AI coding assistants.

## Project Overview

WM solves LLM amnesia by automatically extracting tacit knowledge from conversations and surfacing relevant context each turn. It's the "working memory" layer in the Cloud Atlas AI ecosystem.

**Binary:** `wm`

## Build & Test

```bash
cargo build              # Development build
cargo build --release    # Release build
cargo test               # Run tests
cargo install --path .   # Install to ~/.cargo/bin/
```

## Architecture

### Execution Flow

```
[wm distill]  ──────────────────────────────────────────┐
  (on-demand batch extraction from all sessions)        │
                                                        ▼
                                              .wm/distill/
                                              ├── guardrails.md
                                              └── metis.md
                                                        │
[User prompt] → UserPromptSubmit hook → wm compile ─────┘
                                              │
                                              ▼
                                        inject context
                                              │
                                              ▼
                                    [Claude processes]
```

### Primary Operations

| Operation | Trigger | Purpose |
|-----------|---------|---------|
| `distill` | Manual (on-demand) | Batch extract from all sessions → categorize into guardrails.md + metis.md |
| `compile` | UserPromptSubmit hook | Read distill/ files directly, inject as context (no LLM filtering) |

### Knowledge Flow

1. **Distill** (batch, on-demand): Process all session transcripts → extract tacit knowledge → categorize into guardrails vs metis
2. **Compile** (per-turn, automatic): Read pre-curated distill/ files → inject into conversation

All content is pre-curated by distill. Compile does no filtering—distilled knowledge is always relevant.

## Storage

```
.wm/
├── config.yaml                 # Pause/resume settings
├── working_set.md              # Last compiled context
├── hook.log                    # Debug log
├── dive_context.md             # Optional session grounding (from dive-prep)
├── distill/
│   ├── cache.json              # Session extraction cache (for incremental runs)
│   ├── raw_extractions.md      # Pass 1 output (intermediate)
│   ├── guardrails.md           # Pass 2 output: hard constraints
│   └── metis.md                # Pass 2 output: wisdom/patterns
└── sessions/
    └── <session-id>/
        └── working_set.md      # Per-session compiled context
```

## CLI Commands

```bash
wm init                           # Create .wm/
wm distill [--dry-run] [--force]  # Batch extract + categorize (primary extraction)
wm compile                        # Compile working set (reads distill/ directly)
wm show [working|sessions]        # Display working set or available sessions
wm status                         # Show operation status (running/paused)
wm pause [extract|compile]        # Pause operations
wm resume [extract|compile]       # Resume operations
wm hook compile --session-id ID   # Hook entry (stdin: JSON)

# Deprecated (use distill instead):
wm extract [--transcript PATH]    # Per-turn extraction (legacy)
```

## How It Works

### Distill (Batch Knowledge Extraction)

Run `wm distill` to extract and categorize tacit knowledge from all sessions.

**When to run:** After significant work sessions, or when you want to refresh the knowledge base. Distill is incremental—it only processes new/changed sessions, so running it frequently is cheap.

**Pass 1 - Extract:**
1. Discover all session transcripts in `~/.claude/projects/<project-id>/`
2. For each session (incremental—skips already-processed unless `--force`):
   - Read JSONL transcript
   - Call LLM to extract tacit knowledge
   - Cache result for incremental runs
3. Accumulate all extractions to `raw_extractions.md`

**Pass 2 - Categorize:**
1. Read accumulated raw extractions
2. Call LLM to categorize each insight:
   - **Guardrails**: Hard constraints that must NEVER be violated (rules)
   - **Metis**: Wisdom/patterns about HOW to work effectively (advice)
3. Write categorized output to `guardrails.md` and `metis.md`

**Guardrails vs Metis:**
- Guardrails: "Never commit .env files", "Always run tests before pushing"
- Metis: "Prefer functional approaches", "User likes concise commit messages"

### Compile (Context Injection)

1. **Hook fires** on UserPromptSubmit
2. **Read directly** from `distill/guardrails.md` + `distill/metis.md`
3. **Read optional** `dive_context.md` (session grounding from dive-prep)
4. **Combine** all sources (no LLM filtering—all content is pre-curated)
5. **Return JSON** with `additionalContext` field
6. **Claude Code injects** into conversation

No LLM call during compile—distilled content is always relevant by design.

### Extract (Deprecated)

The per-turn `wm extract` command is deprecated. Use `wm distill` for batch extraction instead.

Previously, superego's stop hook called `wm extract &` after each turn. This is no longer needed—distill replaces per-turn extraction with on-demand batch processing. The extract command remains for backwards compatibility but prints a deprecation warning.

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `WM_DISABLED=1` | Skip all wm operations |
| `SUPEREGO_DISABLED=1` | Set by wm during LLM calls (prevents sg recursion) |
| `CLAUDE_PROJECT_DIR` | Project root (hook scripts use this) |
| `CLAUDE_SESSION_ID` | Current session ID |
| `CLAUDE_TRANSCRIPT_PATH` | Path to session transcript |

## Module Structure

```
src/
├── main.rs              # CLI (clap)
├── init.rs              # Initialize .wm/
├── compile.rs           # Working set compilation (reads distill/)
├── distill.rs           # Batch extraction + categorization
├── extract.rs           # Per-turn extraction (deprecated)
├── llm.rs               # LLM calls + response parsing
├── session.rs           # Session discovery
├── state.rs             # File I/O helpers
├── types.rs             # Data structures
└── transcript/
    ├── mod.rs
    ├── types.rs         # TranscriptEntry enum
    └── reader.rs        # JSONL parsing, filtering, formatting
```

## Plugin Structure

```
plugin/
├── .claude-plugin/
│   └── plugin.json      # Plugin metadata
├── hooks/
│   └── hooks.json       # Hook definitions
└── scripts/
    └── compile.sh       # UserPromptSubmit hook script
```

## Debugging

```bash
# View distilled guardrails
cat .wm/distill/guardrails.md

# View distilled metis (wisdom/patterns)
cat .wm/distill/metis.md

# View last compiled context
cat .wm/working_set.md

# Watch hooks in real-time
tail -f .wm/hook.log

# Check distill cache (session extraction status)
cat .wm/distill/cache.json | jq .

# Preview distill without writing
wm distill --dry-run
```

### Common Issues

**Plugin not updating:** Claude Code caches plugins. After local changes:
```bash
# Reinstall plugin
claude plugin uninstall wm
claude plugin install /path/to/wm/plugin
# Restart Claude Code
```

**Hooks not firing:** Check hook.log for errors. Verify:
- `wm` binary is in PATH
- `.wm/` directory exists in project
- `CLAUDE_PROJECT_DIR` is set correctly

## Design Principles

1. **Graceful failure**: Hooks never block Claude, return empty on error
2. **Pre-curation over filtering**: Distill categorizes once, compile serves instantly (no per-turn LLM)
3. **Incremental by default**: Distill caches session extractions, re-processes only changed files
4. **Recursion prevention**: Set `WM_DISABLED=1` and `SUPEREGO_DISABLED=1` during distill LLM calls
5. **Minimal dependencies**: No async runtime, standard library where possible
