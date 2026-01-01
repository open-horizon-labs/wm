# wm — Working Memory for AI Coding Assistants

**wm** automatically captures tacit knowledge from your coding sessions and surfaces relevant context for each new task. It's the memory layer that helps AI assistants learn how *you* work.

## The Problem

LLMs have amnesia. Every conversation starts fresh. The patterns you've established, the constraints you've discovered, the preferences you've revealed—all forgotten.

You end up repeating yourself:
- "Remember, we always use X pattern here"
- "Don't forget the constraint about Y"
- "I prefer Z approach for this kind of problem"

## The Solution

wm runs silently in the background:

1. **Extract**: After each conversation turn, captures tacit knowledge—the wisdom that emerges from *how* you work, not just what you say
2. **Compile**: Before each new prompt, filters accumulated knowledge for relevance and injects it as context

The result: AI assistants that remember your patterns across sessions.

## What Gets Captured

**Tacit knowledge** is the unspoken wisdom in how someone works:

- Rationale behind decisions (WHY this approach, not just WHAT was done)
- Paths rejected and why (the judgment in pruning options)
- Constraints discovered through friction
- Preferences revealed by corrections
- Patterns followed without stating

**Not captured:**
- What happened ("Fixed X", "Updated Y")
- Explicit requests or questions
- Tool outputs or code snippets
- Content from CLAUDE.md (already explicit)

## Installation

### Prerequisites

- Rust toolchain (`cargo`)
- [Claude Code](https://claude.com/code) CLI
- [superego](https://github.com/cloud-atlas-ai/superego) (optional, but recommended)

### Build & Install

```bash
# Clone the repository
git clone https://github.com/cloud-atlas-ai/wm.git
cd wm

# Build and install
cargo install --path .

# Install the Claude Code plugin
claude plugin install ./plugin
```

### Initialize in Your Project

```bash
cd /your/project
wm init
```

This creates a `.wm/` directory to store accumulated knowledge.

## Usage

Once installed, wm works automatically:

1. **You write prompts** → wm injects relevant context
2. **You work with Claude** → conversation happens normally
3. **Turn ends** → wm extracts any tacit knowledge learned

### Manual Commands

```bash
# View accumulated knowledge
wm show state

# View what context would be injected
wm show working

# Manually trigger extraction
wm extract

# Manually compile for a specific intent
wm compile --intent "implement authentication"
```

### Debugging

```bash
# Watch hooks fire in real-time
tail -f .wm/hook.log

# Check what's being captured
cat .wm/state.md
```

## How It Works

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Claude Code Session                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  [User Prompt]                                              │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────┐                                        │
│  │ wm compile      │◄── Reads state.md                      │
│  │ (hook)          │    Filters for relevance               │
│  └────────┬────────┘    Injects as context                  │
│           │                                                 │
│           ▼                                                 │
│  [Claude Processes with Context]                            │
│           │                                                 │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ sg evaluate     │◄── Superego evaluates turn             │
│  │ (stop hook)     │    Calls wm extract in background      │
│  └────────┬────────┘                                        │
│           │                                                 │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ wm extract      │◄── Reads transcript                    │
│  │ (background)    │    Extracts tacit knowledge            │
│  └─────────────────┘    Updates state.md                    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Storage

```
.wm/
├── state.md              # Accumulated tacit knowledge (the "memory")
├── working_set.md        # Last compiled context
├── hook.log              # Debug log
└── sessions/
    └── <session-id>/     # Per-session state (prevents cross-session bleed)
```

## Integration with Superego

wm is designed to work with [superego](https://github.com/cloud-atlas-ai/superego), a metacognitive advisor for AI assistants. When both are installed:

- **superego** evaluates Claude's work and provides feedback
- **superego's stop hook** triggers wm extraction in the background
- **wm** captures knowledge, superego captures concerns—complementary roles

They compose via shell calls with no shared state (Unix philosophy).

## Configuration

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `WM_DISABLED=1` | Skip all wm operations |
| `CLAUDE_PROJECT_DIR` | Project root (auto-set by Claude Code) |

### What to Expect

- **First few sessions**: Little or no knowledge captured (normal)
- **Over time**: Patterns accumulate in state.md
- **Context injection**: Only relevant items surface per-task
- **LLM costs**: Each extract/compile makes one LLM call (~$0.01-0.05)

## Troubleshooting

### Hooks not firing

1. Check if wm is in PATH: `which wm`
2. Check if `.wm/` exists in project
3. Reinstall plugin after updates:
   ```bash
   claude plugin uninstall wm
   claude plugin install ./plugin
   # Restart Claude Code
   ```

### No knowledge being captured

- Most sessions genuinely have no tacit knowledge worth capturing
- Check `.wm/hook.log` for extraction activity
- Verify superego is installed (it triggers extraction)

### State.md has wrong content

- System reminders (CLAUDE.md content) should be stripped
- If seeing explicit instructions, check transcript reader is up to date

## License

MIT

## Part of Cloud Atlas AI

wm is part of the [Cloud Atlas AI](https://github.com/cloud-atlas-ai) ecosystem:

- **[Open Horizons](https://github.com/cloud-atlas-ai/open-horizons)** — Strategic alignment platform
- **[superego](https://github.com/cloud-atlas-ai/superego)** — Metacognitive advisor
- **[wm](https://github.com/cloud-atlas-ai/wm)** — Working memory (this project)
- **[oh-mcp-server](https://github.com/cloud-atlas-ai/oh-mcp-server)** — MCP bridge to Open Horizons
