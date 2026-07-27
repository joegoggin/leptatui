# Markdown Editor

The Markdown editor is a standalone Leptatui workspace application. It validates
an anchored browsing root and renders a safe explorer containing directories
and Markdown files. Keyboard selection opens directories or renders UTF-8
Markdown in a scrollable preview without allowing navigation outside the root.

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
files using deterministic name ordering. The first entry in each non-empty
directory is selected automatically, and selection stops at listing boundaries.

## Controls

- `Up` or `k` — Select the previous explorer entry.
- `Down` or `j` — Select the next explorer entry.
- `Enter` — Enter the selected directory or open the selected Markdown file.
- `Left` or `h` — Return to the parent directory without crossing the root.
- `Page Up` or `Page Down` — Scroll the Markdown preview.
- `Ctrl-U`, `Ctrl-D`, `gg`, or `G` — Use Vim-style preview scrolling.
- `r` — Reload the open Markdown file.
- `q` — Exit the application.

The header keeps the canonical root visible while showing the current directory
and open document relative to it. Terminals wider than 60 columns place the
explorer beside the preview; narrower terminals stack the panes. File-read,
missing-file, and invalid UTF-8 errors replace the preview body with a
recoverable diagnostic. Explorer navigation remains available, and `r` retries
the same path after the problem is corrected.

## Architecture

- `cli` parses the optional browsing root and resolves the current-directory
  default.
- `domain` contains the validated workspace, explorer entries, listings, and
  recoverable state.
- `filesystem` validates paths and owns anchored Markdown discovery.
- `editor_process` reserves the external-editor process boundary.
- `controller` assembles services, applies selection and activation commands,
  and preserves recoverable explorer and preview state.
- `ui` renders responsive explorer and semantic Markdown preview views.

The binary entry point only coordinates parsing, controller initialization, and
terminal startup so application behavior remains in focused modules.
