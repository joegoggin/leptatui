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
expanded when it begins with exact `~` or `~/`, canonicalized, and verified as
a directory. Named-user forms such as `~alice` remain literal. Missing paths,
regular files, and additional positional arguments fail without entering
managed terminal mode.

## Pages and Controls

The application starts on Home and uses Leptatui's URL-like router to switch
between three component pages:

- **Home** — Press `o` or activate **Open file** to visit the File Explorer.
  Recent-file buttons open their documents directly in the Markdown Viewer.
- **File Explorer** — Use `Up`/`k` and `Down`/`j` to select entries, `Enter` to
  enter a directory or open a Markdown file, `Left`/`h` to visit the parent
  directory, and `Esc` to return Home. Explorer state belongs to the active
  page, so returning to File Explorer starts again at the workspace root.
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

The Viewer loads its initial UTF-8 source through a component-local Leptatui
filesystem handle, then constructs a path-identified Markdown view so relative
links, local navigation, and history keep their normal behavior. Missing-file
and invalid UTF-8 failures remain recoverable diagnostics. Pressing `r`
dispatches a fresh read and rebuilds the document after the problem is
corrected, while Home and File Explorer remain available as explicit
destinations.

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

- `cli` contains command-line parsing and browsing-root selection.
- `hooks` exposes two domain contexts: `use_workspace()` supplies the validated
  workspace root, while `use_files()` supplies recent-file
  signals, persistence diagnostics, and external-editor failures.
- `services` contains Markdown-specific filesystem filtering, persistent
  recent-file storage, external editor process boundaries, restored-terminal
  session coordination, and explorer result values. Leptatui owns root
  containment and asynchronous filesystem I/O.
- `contexts` owns shared notification state and user-facing feedback.
- `app` owns the application shell, provides typed contexts, and declares `/`,
  `/files`, and `/view/*path`. Each routed page and supporting component owns
  its `stylesheet!` rules and BEM-prefixed presentation classes; the app-level
  stylesheet contains only shell and route-container styling.
- `pages` organizes each routed feature around a `page` module with co-located
  state and child components. Explorer owns its listing, selection, and error
  signals; Viewer derives its document from the route and owns its reload
  revision. Explorer and Viewer each call `use_file_system(workspace.root())`
  locally, and Viewer builds a navigable Markdown view from operation-loaded
  source.
- `main` uses `anyhow` to add startup and runtime context while it parses the
  CLI, validates the workspace, initializes services and signals, constructs
  `<AppRouter />`, and starts Leptatui.

The normal data flow is CLI root validation → `use_workspace()` context →
component-local `use_file_system()` handles and page-owned operations → encoded
Viewer route → operation-loaded Markdown source. Successful Viewer reads update
and persist recent-file values through `use_files()`. Editing queues the
route-derived path through the contextual editor session, temporarily restores
the terminal, appends `--` and the path to the resolved editor command, and
resumes the same Viewer component. Completion updates the Viewer revision and
path-associated failure signal so the mounted document reloads in place.
Recoverable failures retain their `anyhow` source chains in shared pointers
until the page renders them inline or sends them through the notification
context.

## Verification

The package's tests use temporary filesystem trees, page-owned signals, the two
domain hooks, injectable editor services, and Ratatui's test backend, so
filesystem, editor, and representative rendering behavior do not require an
interactive terminal.

```sh
cargo test -p markdown-editor
cargo clippy -p markdown-editor --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p markdown-editor --no-deps
```
