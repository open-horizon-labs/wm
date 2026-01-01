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
[User prompt] → UserPromptSubmit hook → wm compile → inject context
                                                          ↓
                                                [Claude processes]
                                                          ↓
                                                [Stop hook: sg evaluates]
                                                          ↓
                                                [sg calls: wm extract &]
                                                (background, graceful failure)
```

### Two Operations

| Operation | Trigger | Purpose |
|-----------|---------|---------|
| `compile` | UserPromptSubmit hook | Filter state.md for relevance to current intent, inject as context |
| `extract` | Stop hook (via sg) | Extract tacit knowledge from transcript, update state.md |

### Composition with Superego

sg and wm are separate tools composing via shell:

```bash
# sg's stop hook calls:
wm extract --transcript "$TRANSCRIPT_PATH" --session-id "$SESSION_ID" &
# Background, graceful failure, sg doesn't wait
```

No shared state. No coupling. Unix philosophy.

## Storage

```
.wm/
├── state.md                    # Accumulated tacit knowledge
├── checkpoint.json             # Legacy (unused)
├── working_set.md              # Last compiled context (global)
├── hook.log                    # Debug log
└── sessions/
    └── <session-id>/
        ├── extraction_state.json   # Last extraction timestamp
        └── working_set.md          # Per-session compiled context
```

## CLI Commands

```bash
wm init                                           # Create .wm/
wm compile [--intent "..."]                       # Compile working set manually
wm extract [--transcript PATH] [--session-id ID] # Extract from transcript
wm show [state|working]                           # Display current state
wm hook compile --session-id ID                   # Hook entry (stdin: JSON)
wm hook extract --session-id ID                   # Hook entry (called by sg)
```

## How It Works

### Extract (Knowledge Capture)

1. **Triggered** by sg's stop hook (background)
2. **Read transcript** from Claude Code's session JSONL
3. **Filter messages** since last extraction + 5 min carryover window
4. **Strip system-reminder blocks** (CLAUDE.md content is explicit, not tacit)
5. **Call LLM** with extraction prompt
6. **Parse response** using `HAS_KNOWLEDGE: YES|NO` markers
7. **Write to state.md** if knowledge found

**Extract Prompt Pattern:**
```
If you found tacit knowledge worth capturing, respond:
HAS_KNOWLEDGE: YES

<markdown content>

If nothing worth capturing, respond:
HAS_KNOWLEDGE: NO
```

### Compile (Context Injection)

1. **Hook fires** on UserPromptSubmit
2. **Read state.md** (accumulated knowledge)
3. **Read intent** from hook JSON stdin
4. **Call LLM** to filter state for relevance
5. **Parse response** using `HAS_RELEVANT: YES|NO` markers
6. **Return JSON** with `additionalContext` field
7. **Claude Code injects** into conversation

**Compile Prompt Pattern:**
```
If knowledge is relevant, respond:
HAS_RELEVANT: YES

<relevant knowledge items>

If nothing is relevant, respond:
HAS_RELEVANT: NO
```

### Response Parsing

Both extract and compile use the same robust parsing pattern from superego:

1. Search for marker line (`HAS_KNOWLEDGE:` or `HAS_RELEVANT:`)
2. Strip markdown prefixes (`#`, `>`, `*`) for lenient matching
3. Extract YES/NO value (case insensitive, also accepts TRUE/FALSE)
4. Content is everything after the marker line
5. Fallback: if no marker found, treat as NO (safe default)

```rust
fn strip_markdown_prefix(line: &str) -> &str {
    line.trim().trim_start_matches(['#', '>', '*']).trim()
}
```

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
├── compile.rs           # Working set compilation
├── extract.rs           # Knowledge extraction
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
# View accumulated knowledge
cat .wm/state.md

# View last compiled context
cat .wm/working_set.md

# Watch hooks in real-time
tail -f .wm/hook.log

# Check session-specific state
cat .wm/sessions/<session-id>/extraction_state.json
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
2. **Recursion prevention**: Set `WM_DISABLED=1` and `SUPEREGO_DISABLED=1` during LLM calls
3. **Session isolation**: Per-session state prevents cross-conversation bleed
4. **Prior art over invention**: Parsing patterns copied from superego, not reinvented
5. **Minimal dependencies**: No async runtime, standard library where possible
