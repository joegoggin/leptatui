# Markdown Editor

The Markdown editor is a standalone Leptatui reference application. It combines
an anchored filesystem explorer, semantic Markdown preview, responsive terminal
layout, recoverable errors, and restored-terminal editing behind focused project
layers.

## Prerequisites

- A current stable Rust toolchain with Cargo.
- An interactive terminal supported by Leptatui's Crossterm backend.
- A terminal editor for the optional edit command. The application uses the
  first non-empty `VISUAL` or `EDITOR` value and otherwise falls back to `vi`.

Neovim is one supported choice:

```sh
export VISUAL="nvim -f"
```

The explorer and preview remain fully usable when no editor is configured; an
editor is launched only after pressing `e` with a Markdown document open.

## Start the Application

Run from the current directory:

```sh
cargo run -p markdown-editor
```

Or provide an explicit browsing root:

```sh
cargo run -p markdown-editor -- examples/markdown-editor
```

Pass `--help` after Cargo's separator to inspect the CLI:

```sh
cargo run -p markdown-editor -- --help
```

The command accepts zero or one positional `ROOT`. An omitted root resolves to
the process current directory. Before terminal startup, the path is
canonicalized and verified as a directory; missing paths, regular files, and
additional positional arguments fail without entering managed terminal mode.

## Controls

- `Up` or `k` — Select the previous explorer entry.
- `Down` or `j` — Select the next explorer entry.
- `Enter` — Enter the selected directory or open the selected Markdown file.
- `Left` or `h` — Return to the parent directory without crossing the root.
- `Page Up` or `Page Down` — Scroll the Markdown preview.
- `Ctrl-U`, `Ctrl-D`, `gg`, or `G` — Use Vim-style preview scrolling.
- `e` — Edit the open Markdown file in the configured terminal editor.
- `r` — Reload the open Markdown file.
- `q` — Exit the application.

## Filesystem and Failure Behavior

The explorer lists directories before case-insensitive `.md` and `.markdown`
files using deterministic name ordering. The first entry in each non-empty
directory is selected automatically, and selection stops at listing boundaries.

Explorer discovery follows symlinks only when their canonical targets remain
below the configured root. Broken and escaping symlinks are hidden. Failed
directory reads preserve the last valid listing and render a recoverable error.

File-read, missing-file, and invalid UTF-8 errors replace the preview body with
a diagnostic while leaving explorer navigation available. Pressing `r` retries
the same document after the problem is corrected.

Editor values can contain shell-word quoted arguments, such as
`VISUAL="nvim -f"`, but they are executed directly without shell expansion,
pipelines, or functions. Invalid configuration, a missing executable, or a
non-zero exit becomes a recoverable preview error; pressing `e` retries the same
open path.

## Responsive Layout

The header keeps the canonical root visible and shows the current directory and
open document relative to it. Terminals wider than 60 columns place the explorer
beside the preview. At 60 columns or narrower, the panes stack vertically and
reduce decorative spacing so both remain usable.

## Architecture and Data Flow

- `cli` parses the optional root and resolves the current-directory default.
- `domain` owns the validated workspace, explorer entries, listings, and
  recoverable state.
- `filesystem` validates paths and performs anchored Markdown discovery and
  reads.
- `editor_process` resolves, parses, and launches the configured editor through
  injectable environment and process boundaries.
- `controller` coordinates filesystem and editor services while applying
  selection, navigation, preview, reload, and edit transitions.
- `ui` maps keyboard input to controller transitions and renders responsive
  explorer and semantic Markdown views.
- `main` validates startup, runs managed terminal sessions, and invokes the
  external editor only after Leptatui restores raw mode, mouse capture, and the
  alternate screen.

The normal data flow is CLI root → validated workspace → safe directory listing
→ controller transition → rendered explorer or preview. Editing temporarily
exits the managed TUI, appends `--` and the canonical document path to the
resolved editor command, then starts a new TUI session with the same controller.
A successful editor exit reloads the document from disk without moving the
explorer selection.

## Verification

The package's tests use temporary filesystem trees, injectable editor services,
and Ratatui's test backend, so filesystem, controller, editor, and representative
rendering behavior do not require an interactive terminal.

```sh
cargo test -p markdown-editor
cargo clippy -p markdown-editor --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p markdown-editor --no-deps
```
