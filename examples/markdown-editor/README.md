# Markdown Editor

The Markdown editor is a standalone, multi-page Leptatui reference application.
It combines Leptatui's standalone file selector, a semantic Markdown viewer,
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

The file selector and preview remain fully usable when no editor is configured; an
editor is launched only after pressing `e` with a Markdown document open.

## Start the Application

Run from the current directory:

```sh
cargo run -p markdown-editor
```

Or open a Markdown file directly:

```sh
cargo run -p markdown-editor -- examples/markdown-editor/README.md
```

Pass `--help` after Cargo's separator to inspect the CLI:

```sh
cargo run -p markdown-editor -- --help
```

The command accepts zero or one positional `FILE_PATH`. With no path, the
application starts on Home and the file selector opens in the process current
directory when requested. A supplied relative path is made absolute and lexically normalized,
then encoded into the initial Viewer route. Startup does not require the target
to exist; read and Markdown-validation failures render as recoverable Viewer
diagnostics. Additional positional arguments are rejected.

## Pages and Controls

The application starts on Home and uses Leptatui's URL-like router to switch
between two component pages while Leptatui owns the standalone file selector:

- **Home** — Press `o` or activate **Open file** to open the file selector.
  Recent-file buttons open their documents directly in the Markdown Viewer.
- **File Selector** — Use `Up`/`k` and `Down`/`j` to highlight entries,
  `Enter` to enter a directory or select a Markdown file, `Left`/`h` to visit
  the parent directory, and `Esc` to cancel. It starts in the process current
  directory and can continue to the filesystem or drive root.
- **Markdown Viewer** — Use `Page Up`, `Page Down`, `Ctrl-U`, `Ctrl-D`, `gg`,
  or `G` to scroll; `e` to edit; `r` to reload; `h` for Home; and `b` to browse
  files. Focus a Markdown link and press `Enter` to activate it; `Shift-H` and
  `Shift-L` move backward and forward through the shared Markdown view's page
  history.
- **Global** — Use `Tab` and `Shift-Tab` to move between buttons, `Enter` to
  activate the focused button, and `q` to exit.

Opening a document successfully promotes it to the front of a ten-item recent
list. The list is stored as versioned JSON in the platform-local application
data directory for `io.github.joegoggin/leptatui-markdown-editor`. Missing and
unsupported entries are omitted when Home loads the global history. Storage
errors remain recoverable and appear as a warning on Home.

## Filesystem and Failure Behavior

The file selector lists directories before case-insensitive `.md` and `.markdown`
files using deterministic name ordering. The first entry in each non-empty
directory is selected automatically, and selection stops at listing boundaries.

File-selector discovery follows symlinks only when their canonical targets remain
on the current filesystem or drive root. Broken or escaping symlinks are
hidden. Failed directory reads preserve the last valid listing and render a
recoverable error.

The Viewer loads its initial UTF-8 source through a component-local Leptatui
filesystem handle, then constructs a path-identified Markdown view so relative
links, local navigation, and history keep their normal behavior. Missing-file
and invalid UTF-8 failures remain recoverable diagnostics. Pressing `r`
dispatches a fresh read and rebuilds the document after the problem is
corrected, while Home and the standalone file selector remain available.

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

- `cli` contains optional startup-file parsing.
- `layouts/root` owns the application shell component and its co-located
  `root-layout` stylesheet.
- `services` contains Markdown path validation and persistent recent-file
  storage. Leptatui owns external editor sessions, file selection, volume-root
  containment, and asynchronous filesystem I/O.
- `contexts` owns shared notification state and user-facing feedback.
- `app` defines `AppRouter`, provides application services, and declares `/`
  and `/view/*path`. Each routed page and supporting component owns
  its BEM-prefixed presentation classes. Styled components use co-located
  `component.rs`, `style.rs`, and `mod.rs` files; styled pages use `page.rs`,
  `style.rs`, and `mod.rs`.
- `pages` organizes each routed feature around a `page` module with co-located
  state, stylesheet, and child components. Components without local style
  rules remain flat files. Leptatui owns selector state; Viewer derives its
  document from the route and owns its reload revision and editor error. Viewer
  creates a component-local filesystem handle at the containing volume root,
  while Home loads and displays recent-file history.
- `main` parses the CLI, converts an optional file path into the initial route,
  constructs `<AppRouter />`, and starts Leptatui.

The normal data flow is optional CLI file path → encoded initial Viewer route →
operation-loaded Markdown source. The file selector routes selected absolute Markdown
paths through the same encoder. Successful Viewer reads record directly through
`RecentFilesStore`; Home loads, validates, and displays that global MRU list.
Editing passes the route-derived path to Leptatui's `use_editor()` hook, which
temporarily restores the terminal, appends `--` and the path to the resolved
editor command, and resumes the same Viewer component. The handle's reactive
status updates the Viewer-local revision and path-associated failure so the
document reloads in place, then the Viewer clears the consumed status.
Recoverable failures render inline or through the notification context.

## Verification

The package's tests use temporary filesystem trees, page-owned signals,
injectable stores, and Ratatui's test backend, so filesystem and representative
rendering behavior do not require an interactive terminal. Generic editor
behavior is covered by the Leptatui crate's injected process-boundary tests.

```sh
cargo test -p markdown-editor
cargo clippy -p markdown-editor --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p markdown-editor --no-deps
```
