# /wm:init

Initialize working memory in the current project.

## Usage

```
/wm:init
```

## Execution

1. Check if wm binary exists:
   ```bash
   command -v wm
   ```

2. If wm not found, tell the user to install it:
   ```
   The wm CLI is not installed.

   Install with Homebrew:
     brew install open-horizon-labs/homebrew-tap/wm

   Or with Cargo:
     cargo install working-memory

   Then run this command again.
   ```

3. If wm exists, run:
   ```bash
   wm init
   ```

   This creates `.wm/` directory with initial state.
