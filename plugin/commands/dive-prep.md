# /wm:dive-prep

Prepare working memory context for a focused session.

## Usage

```
/wm:dive-prep [intent]
```

## Execution

Run the wm dive-prep command:

```bash
wm dive-prep
```

If an intent is provided, pass it along:

```bash
wm dive-prep --intent "<intent>"
```

Pass through the output to the user. The command reads `.wm/state.md` and surfaces relevant context for the session.
