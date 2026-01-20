# wm — Working Memory for AI Coding Assistants

**wm** automatically captures tacit knowledge from your coding sessions and surfaces relevant context for each new task. It's the memory layer that helps AI assistants learn how *you* work.

**Supported platforms:**
- **Claude Code** - Full support via plugin
- **OpenAI Codex CLI** - Alpha support via skill (agent-invoked)

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

## Quickstart: Claude Code

### Prerequisites

- [Claude Code](https://claude.com/code) CLI
- [superego](https://github.com/cloud-atlas-ai/superego) (optional, but recommended for extraction triggers)

### Option 1: Homebrew (macOS)

```bash
brew tap cloud-atlas-ai/wm
brew install wm
```

### Option 2: Cargo (all platforms)

```bash
cargo install wm
```

### Option 3: From Source

```bash
git clone https://github.com/cloud-atlas-ai/wm.git
cd wm
cargo install --path .
```

### Install the Claude Code Plugin

```bash
claude plugin install wm
```

### Initialize in Your Project

```bash
cd /your/project
wm init
```

This creates a `.wm/` directory to store accumulated knowledge.

## Quickstart: OpenAI Codex CLI (Alpha)

Codex support uses agent skills that can be invoked at decision points. Most features work, but session auto-discovery is limited due to Codex's different session storage format.

**What works:**
- ✅ Manual knowledge capture/review (state.md, working_set.md)
- ✅ Dive prep with context gathering
- ✅ Compress and pause operations
- ⚠️ Distill requires manual transcript paths (no auto-discovery yet)

```bash
# 1. Install the binary (choose one)
brew install cloud-atlas-ai/wm/wm           # macOS
cargo install wm                            # All platforms

# 2. Install the skills
mkdir -p ~/.codex/skills
ln -s /path/to/wm/plugin ~/.codex/skills/wm

# Or download agents individually:
mkdir -p ~/.codex/skills/wm/agents
BASE_URL="https://raw.githubusercontent.com/cloud-atlas-ai/wm/main/plugin/agents"
curl -L -o ~/.codex/skills/wm/agents/dive-prep.md $BASE_URL/dive-prep.md
curl -L -o ~/.codex/skills/wm/agents/review.md $BASE_URL/review.md
curl -L -o ~/.codex/skills/wm/agents/distill.md $BASE_URL/distill.md
curl -L -o ~/.codex/skills/wm/agents/compress.md $BASE_URL/compress.md
curl -L -o ~/.codex/skills/wm/agents/pause.md $BASE_URL/pause.md
```

**Available Agent Skills:**

| Skill | Support | When to Use |
|-------|---------|-------------|
| `$wm:dive-prep` | ✅ Full | Prepare focused work session with intent, context, and workflow |
| `$wm:review` | ✅ Full | Review accumulated knowledge and current context |
| `$wm:compress` | ✅ Full | Synthesize state.md to higher-level abstractions |
| `$wm:pause` | ✅ Full | Pause/resume operations (extract, compile, or both) |
| `$wm:distill` | ⚠️ Manual | Batch extract (requires manual transcript paths) |

**Typical Workflows:**

```bash
# Session start - prepare dive context
$wm:dive-prep --intent fix

# Mid-session - review what wm knows
$wm:review

# Maintenance - compress accumulated knowledge
$wm:compress

# Sensitive work - pause extraction
$wm:pause extract

# Manual extraction from Codex session (since auto-discovery isn't supported yet)
# Find your sessions:
ls ~/.codex/sessions/2026/01/*/rollout-*.jsonl
# Extract from specific transcript:
wm extract --transcript ~/.codex/sessions/2026/01/06/rollout-<timestamp>-<uuid>.jsonl
```

**Why limited session discovery?**

Codex stores sessions in `~/.codex/sessions/YYYY/MM/DD/` with different naming and structure than Claude Code's `~/.claude/projects/<project-id>/`. Auto-discovery support for Codex sessions is tracked in [#11](https://github.com/cloud-atlas-ai/wm/issues/11).

**Manual Commands:**

All CLI commands work normally: `wm init`, `wm show state`, `wm show working`, etc.

See [plugin/agents/](plugin/agents/) for detailed documentation on each skill.

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

# List all sessions
wm show sessions

# View session-specific working set
wm show working --session-id <id>

# Manually trigger extraction
wm extract

# Manually compile for a specific intent
wm compile --intent "implement authentication"

# Compress state.md (synthesize to higher abstractions)
wm compress
```

### Compressing Knowledge

Over time, `state.md` accumulates knowledge and can grow unwieldy. The `compress` command distills it down by:

- **Merging** related items into broader principles
- **Abstracting** specific instances into general patterns
- **Removing** obsolete or superseded knowledge
- **Preserving** critical constraints and preferences

```bash
wm compress
# Compressed: 42 → 18 lines (57% reduction)
# Backup saved to .wm/state.md.backup
```

Run periodically when state feels bloated, not after every session.

## Dive Sessions

A **dive** is a focused work session with explicit grounding. The metaphor comes from scuba diving: you prep before you dive, you don't just splash in. You check your gear, review your plan, know your limits. The 30 seconds of setup prevents 30 minutes of drift.

**No dive is too small for a dive prep.** Even a quick bug fix benefits from explicit intent. The act of stating what you're doing grounds you.

### Terminology

| Term | What It Is |
|------|------------|
| **dive** | A focused work session with explicit grounding |
| **dive-prep** | The action of preparing a dive (`/dive-prep`, `wm dive-prep`) |
| **dive pack** | A reusable context bundle stored in Open Horizons |
| **dive context** | The manifest file for the current session (`.wm/dive_context.md`) |

**Without a dive**, AI sessions often drift:
- You start coding without clarity on the goal
- Constraints surface mid-work ("oh wait, we can't do that because...")
- Related knowledge sits unused because nothing surfaced it
- The session ends without capturing what was learned

**With a dive**, you start grounded:
- **Intent** is explicit (fix, plan, explore, review, ship)
- **Context** is curated (relevant knowledge, constraints, mission)
- **Workflow** is suggested (what steps this intent typically follows)
- **Focus** is documented (what specifically you're working on)

This isn't overhead—it's the 30 seconds of setup that saves 30 minutes of drift.

### The `/dive-prep` Skill

Invoke `/dive-prep` in Claude Code to prepare a focused work session:

```bash
/dive-prep                          # Interactive - prompts for intent
/dive-prep --intent fix             # Fix a bug
/dive-prep --intent plan            # Design an approach
/dive-prep --intent explore         # Understand something
/dive-prep --intent review          # Reflect on recent work
/dive-prep --intent ship            # Get something deployed
```

**What it does:**

1. **Detects context** — Reads CLAUDE.md, git state, existing `.wm/` knowledge
2. **Checks for OH** — If [Open Horizons](https://github.com/cloud-atlas-ai/open-horizons) MCP is connected, offers to link to an endeavor for strategic context (missions, guardrails, learnings)
3. **Asks for intent** — If not provided, prompts for what you're trying to accomplish
4. **Builds workflow** — Suggests steps based on intent type
5. **Writes manifest** — Creates `.wm/dive_context.md` with curated grounding

**With Open Horizons (recommended):**
```bash
/dive-prep --intent fix --oh bd9d6ace
```

OH provides the strategic layer: *why* you're doing this (mission), *what not to do* (guardrails), and *what you've learned* (metis).

**Without OH:**
```bash
/dive-prep --intent explore
# Prompts: "What are you exploring?"
```

Still valuable—you get explicit intent, workflow guidance, and documented focus.

### The `wm dive` Commands

Manage dive context directly:

```bash
wm dive load <pack-id>    # Load a pre-built dive pack from OH
wm dive show              # Display current dive context
wm dive clear             # Remove dive context
```

**Dive packs** are curated context bundles stored in Open Horizons. They're useful for recurring work patterns—load a pack instead of rebuilding context each time.

**Configuration:**
```bash
# Set OH API key (required for wm dive load)
export OH_API_KEY=your-key

# Or configure in ~/.config/openhorizons/config.json:
{
  "api_key": "your-key",
  "api_url": "https://app.openhorizons.me"
}
```

## Batch Distillation

The `distill` command extracts knowledge from all your Claude Code sessions at once, instead of per-turn extraction:

```bash
wm distill                    # Process all sessions
wm distill --dry-run          # Preview what would be processed
wm distill --force            # Re-extract even cached sessions
```

**How it works:**

1. **Discovers sessions** — Finds all Claude Code transcripts for this project
2. **Extracts incrementally** — Caches results, only processes new/changed sessions
3. **Accumulates knowledge** — Writes raw extractions to `.wm/distill/raw_extractions.md`

**When to use:**
- Initial setup: extract knowledge from existing sessions
- Periodic catchup: if per-turn extraction was paused
- Audit: see what knowledge exists across all sessions

**Output:**
```
.wm/distill/
├── raw_extractions.md    # Accumulated knowledge from all sessions
├── cache.json            # Extraction cache (enables incremental runs)
└── errors.log            # Any extraction failures
```

## Pause and Resume

Temporarily disable wm operations without uninstalling:

```bash
wm pause                  # Pause both extract and compile
wm pause extract          # Pause only extraction
wm pause compile          # Pause only context injection

wm resume                 # Resume both operations
wm resume extract         # Resume only extraction
wm resume compile         # Resume only context injection

wm status                 # Show current state
```

**When to use:**
- **Sensitive work**: Pause extraction when working on confidential code
- **Debugging**: Isolate issues by disabling one operation
- **Performance**: Skip context injection on simple tasks

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

## Releases

Releases are automated via GitHub Actions. To release a new version:

1. Create and push a git tag: `git tag v0.3.5 && git push origin v0.3.5`
2. Create a GitHub release from the tag

This automatically publishes to:
- **crates.io** — Rust CLI (`cargo install working-memory`)
- **npm** — OpenCode plugin (`wm-opencode`)

## License

Source-available. See [LICENSE.md](LICENSE.md) for details.

## Part of Cloud Atlas AI

wm is part of the [Cloud Atlas AI](https://github.com/cloud-atlas-ai) ecosystem:

- **[Open Horizons](https://github.com/cloud-atlas-ai/open-horizons)** — Strategic alignment platform
- **[superego](https://github.com/cloud-atlas-ai/superego)** — Metacognitive advisor
- **[wm](https://github.com/cloud-atlas-ai/wm)** — Working memory (this project)
- **[oh-mcp-server](https://github.com/cloud-atlas-ai/oh-mcp-server)** — MCP bridge to Open Horizons
