# /wm:review

Review current working memory state and context.

## Usage

```
/wm:review [state|working|sessions]
```

## Execution

Use the `wm show` command to display working memory contents:

```bash
wm show state      # Show .wm/state.md
wm show working    # Show compiled working set
wm show sessions   # List available sessions
```

Without arguments, show the state:

```bash
wm show
```

For a comprehensive review, you can also:
1. Show distilled knowledge: `cat .wm/distill/guardrails.md .wm/distill/metis.md`
2. Show dive context: `cat .wm/dive_context.md`

Pass through the output to the user.
