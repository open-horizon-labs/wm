# /wm:pause

Pause wm operations (extract, compile, or both).

## Usage

```
/wm:pause [extract|compile]
```

## Execution

Run the wm pause command:

```bash
wm pause              # Pause both extract and compile
wm pause extract      # Pause only extract
wm pause compile      # Pause only compile
```

To resume operations later:

```bash
wm resume             # Resume both
wm resume extract     # Resume only extract
wm resume compile     # Resume only compile
```

Check current status with:

```bash
wm status
```

Pass through the output to the user.
