# Markdown Editor

The Markdown editor is a standalone Leptatui workspace application. It validates
an anchored browsing root and renders a safe explorer containing directories
and Markdown files. Selection, preview, and editor controls are added by later
phases.

## Run

Pass zero or one directory as the browsing root:

```sh
cargo run -p markdown-editor -- [ROOT]
```

When `ROOT` is omitted, the application uses the current directory. The path is
canonicalized and verified as a directory before Leptatui starts its managed
terminal session. Missing paths and regular files fail with a path-specific
startup error. Explorer discovery follows symlinks only when their canonical
targets remain below the configured root. Broken and escaping symlinks are
hidden, and directory-read failures render as recoverable errors.

The explorer lists directories before case-insensitive `.md` and `.markdown`
files using deterministic name ordering. Press `q` to exit the application
shell.

## Architecture

- `cli` parses the optional browsing root and resolves the current-directory
  default.
- `domain` contains the validated workspace, explorer entries, listings, and
  recoverable state.
- `filesystem` validates paths and owns anchored Markdown discovery.
- `editor_process` reserves the external-editor process boundary.
- `controller` assembles services and preserves the last valid listing across
  navigation failures.
- `ui` renders current, empty, and error explorer states as Leptatui views.

The binary entry point only coordinates parsing, controller initialization, and
terminal startup so application behavior remains in focused modules.
