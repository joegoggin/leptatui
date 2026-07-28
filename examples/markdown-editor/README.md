# Markdown Editor

The Markdown editor is a standalone, multi-page Leptatui reference application.
It combines an anchored filesystem explorer, semantic Markdown viewer,
persistent recent files, recoverable errors, and restored-terminal editing
behind focused project layers.

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

## Pages and Controls

The application starts on Home and uses Leptatui's URL-like router to switch
between three component pages:

- **Home** — Press `o` or activate **Open file** to visit the File Explorer.
  Recent-file buttons open their documents directly in the Markdown Viewer.
- **File Explorer** — Use `Up`/`k` and `Down`/`j` to select entries, `Enter` to
  enter a directory or open a Markdown file, `Left`/`h` to visit the parent
  directory, and `Esc` to return Home.
- **Markdown Viewer** — Use `Page Up`, `Page Down`, `Ctrl-U`, `Ctrl-D`, `gg`,
  or `G` to scroll; `e` to edit; `r` to reload; `h` for Home; and `b` to browse
  files. Focus a Markdown link and press `Enter` to activate it; `Shift-H` and
  `Shift-L` move backward and forward through the shared Markdown view's page
  history.
- **Global** — Use `Tab` and `Shift-Tab` to move between buttons, `Enter` to
  activate the focused button, and `q` to exit.

Opening a document successfully promotes it to the front of a ten-item recent
list. The list is stored as versioned JSON in the platform-local application
data directory for `io.github.joegoggin/leptatui-markdown-editor`. Missing,
unsupported, and out-of-workspace entries are omitted when the application
starts. Storage errors remain recoverable and appear as a warning on Home.

## Filesystem and Failure Behavior

The explorer lists directories before case-insensitive `.md` and `.markdown`
files using deterministic name ordering. The first entry in each non-empty
directory is selected automatically, and selection stops at listing boundaries.

Explorer discovery follows symlinks only when their canonical targets remain
below the configured root. Broken and escaping symlinks are hidden. Failed
directory reads preserve the last valid listing and render a recoverable error.

The Viewer delegates file loading, semantic rendering, local Markdown
navigation, history, and file-read diagnostics to Leptatui's existing
path-backed `<Markdown />` view. Missing-file and invalid UTF-8 failures
therefore use the same diagnostic presentation as every other file-backed
Markdown reader. Pressing `r` rebuilds that view and retries the same document
after the problem is corrected, while Home and File Explorer remain available
as explicit destinations.

Editor values can contain shell-word quoted arguments, such as
`VISUAL="nvim -f"`, but they are executed directly without shell expansion,
pipelines, or functions. Invalid configuration, a missing executable, or a
non-zero exit becomes a recoverable preview error; pressing `e` retries the same
open path.

## Responsive Layout

Each page uses the full terminal area and shows only the context needed for its
task. At 60 columns or narrower, page actions stack vertically and decorative
spacing is reduced.

## Architecture and Data Flow

- `cli` parses the optional root and resolves the current-directory default.
- `domain` owns the validated workspace, explorer entries, listings, open
  document path and revision, editor errors, and recent-file state.
- `filesystem` validates paths and performs anchored Markdown discovery.
- `recent_files` loads and saves the versioned recent-file document through an
  injectable storage boundary.
- `editor_process` resolves, parses, and launches the configured editor through
  injectable environment and process boundaries.
- `controller` coordinates filesystem, recent-file, and editor services while
  applying selection, navigation, open, view invalidation, and edit
  transitions.
- `ui` keeps the application shell and shared presentation helpers separate
  from page-focused `home`, `explorer`, `viewer`, and `not_found` modules. The
  shell declares `/`, `/files`, and `/view/*path`, and Viewer delegates document
  rendering to the existing `<Markdown />` component.
- `main` validates startup, runs managed terminal sessions, and invokes the
  external editor only after Leptatui restores raw mode, mouse capture, and the
  alternate screen.

The normal data flow is CLI root → validated workspace and recent paths → routed
Home or Explorer action → controller path transition → `<Markdown />` Viewer.
Editing temporarily exits the managed TUI, appends `--` and the canonical
document path to the resolved editor command, then starts a new TUI session on
Viewer with the same controller. A successful editor exit invalidates and
rebuilds the Markdown view without moving the explorer selection.

## Verification

The package's tests use temporary filesystem trees, injectable editor services,
and Ratatui's test backend, so filesystem, controller, editor, and representative
rendering behavior do not require an interactive terminal.

```sh
cargo test -p markdown-editor
cargo clippy -p markdown-editor --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p markdown-editor --no-deps
```
