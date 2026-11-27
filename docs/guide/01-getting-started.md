# 1. Getting started

Prerequisites: Basic Rust and Cargo.

By the end of this chapter, you will have a running application that owns the
terminal and exits cleanly.

## Create your notes project

Install a current stable Rust toolchain with Cargo and use an interactive
terminal. Choose a directory for your own projects, outside the Leptatui
repository, then run:

```sh
cargo new leptatui-notes
cd leptatui-notes
```

You do not need to clone Leptatui. Cargo downloads the library directly from
GitHub when you build your application.

Replace the generated `Cargo.toml` with:

```toml
[package]
name = "leptatui-notes"
version = "0.1.0"
edition = "2024"

[dependencies]
leptatui = { git = "https://github.com/joegoggin/leptatui.git" }
tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread", "time"] }
```

The Git URL points at the repository root. Cargo finds the `leptatui` package
inside the workspace and resolves its companion macro crate automatically.
This setup uses the repository's default branch and does not require a
crates.io release.

Cargo records the selected commit in your application's `Cargo.lock`. Commit
that file with your application to keep builds on the same revision. To update
Leptatui later, run `cargo update -p leptatui`. To select a specific commit
explicitly, add `rev = "<full-commit-sha>"` to the dependency declaration,
replacing the placeholder with the desired commit SHA. See Cargo's
[Git dependency documentation](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories)
for more options.

## Organize the project from the start

Keep `src/main.rs` focused on startup and put application composition in
`src/app.rs`. Declare `mod app;` in the entry point and import
`crate::app::NotesApp`. The complete files at the end of this chapter include
both the startup code and the quit handler.

Create `src/app.rs` with the prelude import and an application component:

```rust
use leptatui::prelude::*;

#[component]
pub(crate) fn NotesApp() -> impl IntoView {
    view! { <Text>"My notes — Ctrl+Q to quit"</Text> }
}
```

`pub(crate)` lets the entry point use the component without exposing it outside
this application. Each Rust module has its own imports; importing the prelude
in `main.rs` does not import it into `app.rs`.

Replace the generated `src/main.rs` with the startup code:

```rust
mod app;

use leptatui::prelude::*;

use crate::app::NotesApp;

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

As the app grows, add files around responsibilities rather than accumulating
components, styles, and data operations in the entry point:

- `app.rs` composes the application and owns shared state and routing.
- `layouts/root` contains the persistent shell and its supporting components.
- `pages/<feature>` owns a routed page and any feature-specific components.
- `services` contains application data operations, separate from UI state.
- Styled components use `component.rs`, `style.rs`, and `mod.rs`; styled pages
  use `page.rs`, `style.rs`, and `mod.rs`. Small components without local styles
  can stay in a single file. A `mod.rs` declares private implementation modules
  and re-exports only what other parts of the application need.

Create these modules as they become useful; do not add empty directories now.
The [Markdown editor](../../examples/markdown-editor/README.md#architecture-and-data-flow)
shows this organization in a complete application. We will apply it throughout
this guide, including co-located styles when styling is introduced.

Before running the app, add the following handler at the start of `NotesApp`
in `src/app.rs`, before `view!`, so you can exit cleanly:

```rust
use_key_event(KeyEventKind::Press, |key| {
    if key.code == KeyCode::Char('q')
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        return KeyControl::Exit;
    }
    KeyControl::Pass
});
```

Run `cargo run` from the notes project. You should see the text, and Ctrl+Q
should return to your shell. If your terminal captures Ctrl+Q, change the quit
key combination to one it delivers to applications. Use an interactive terminal
rather than an editor's output-only panel.

`prelude::*` imports commonly used types, traits, and macros. `#[component]`
turns a function into a reusable terminal component. `view!` constructs its
interface. Tokio supplies the async runtime used by `App::run` and later data
operations. `App` manages terminal setup, event polling, redraws, and cleanup.

The quit behavior is explicit: this application registers it with
`use_key_event`. Keep this handler in `NotesApp` throughout the guide.

## Optional: Run library examples

Later chapters link to runnable library examples. To try them, clone Leptatui
into a separate directory and run example commands from that checkout:

```sh
git clone https://github.com/joegoggin/leptatui.git
cd leptatui
cargo run --example hello_world
```

You should see a greeting inside a bordered panel. Press `q` to return to your
shell. Commands starting with `cargo run --example` run from this **library
checkout**. Return to your **notes project** to continue the guide with plain
`cargo run`; its Git dependency works independently of the optional checkout.

## Complete files for this chapter

Create or replace the following files. Use the Cargo manifest above.

The project now contains:

```text
└── src/
    ├── app.rs
    └── main.rs
```

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/app.rs`

```rust
use leptatui::prelude::*;

#[component]
pub(crate) fn NotesApp() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return KeyControl::Exit;
        }
        KeyControl::Pass
    });

    view! { <Text>"My notes — Ctrl+Q to quit"</Text> }
}
```

### `src/main.rs`

```rust
mod app;

use leptatui::prelude::*;

use crate::app::NotesApp;

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

[Documentation index](../README.md) · [Next: Building a screen](02-building-a-screen.md)
