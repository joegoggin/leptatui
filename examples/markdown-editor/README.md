# Markdown Editor

The Markdown editor is a standalone Leptatui workspace application. This first
phase establishes the application boundary, validates its browsing root, and
renders a minimal terminal shell for the explorer, preview, and editor features
added by later phases.

## Run

Pass zero or one directory as the browsing root:

```sh
cargo run -p markdown-editor -- [ROOT]
```

When `ROOT` is omitted, the application uses the current directory. The path is
canonicalized and verified as a directory before Leptatui starts its managed
terminal session. Missing paths and regular files fail with a path-specific
startup error.

Press `q` to exit the initial application shell.

## Architecture

- `cli` parses the optional browsing root and resolves the current-directory
  default.
- `domain` contains application-owned values such as the validated workspace.
- `filesystem` validates paths and will own anchored Markdown discovery.
- `editor_process` reserves the external-editor process boundary.
- `controller` assembles services and owns application state.
- `ui` converts controller state into Leptatui views and input handling.

The binary entry point only coordinates parsing, controller initialization, and
terminal startup so application behavior remains in focused modules.
