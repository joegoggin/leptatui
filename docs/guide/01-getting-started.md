# 1. Getting started

This chapter assumes basic Rust and Cargo.

Let's build a notes app. You'll start with a line of text and a
keyboard shortcut that returns you to your shell.

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

## Run your first screen

The first screen will live in `src/app.rs`. Create that file and leave it
empty for now. Then add `mod app;` above the existing `main` function in
`src/main.rs` so Rust knows where to find it.

```rust
mod app;

fn main() {
    println!("Hello, world!");
}
```

Now fill in `src/app.rs` with `NotesApp`, the component that displays the
first screen.

```rust
use leptatui::prelude::*;

#[component]
pub(crate) fn NotesApp() -> impl IntoView {
    view! { <Text>"My notes — Ctrl+Q to quit"</Text> }
}
```

The prelude brings in the types, traits, and macros you'll use throughout the
guide. `#[component]` lets you use `NotesApp` as a tag inside `view!`, just
like the built-in `Text` tag. You'll see that when you wire up `main.rs` below.

Think of `view!` as a description of what goes on screen. Here it creates a
single `Text` view, and the quoted string between its opening and closing tags
is what `Text` displays. The return type, `impl IntoView`, tells Leptatui it
can use the value returned by `NotesApp` as part of the interface. In the next
chapter, you'll put several views together to build a bordered note.

The text says Ctrl+Q quits, so let's give that key a job. Add this handler
inside `NotesApp`, just above `view!`.

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

When Leptatui sets up `NotesApp`, `use_key_event` registers the closure as a
keyboard handler. `KeyEventKind::Press` makes it run on key presses. The
closure checks both parts of the shortcut: `key.code` identifies the `q` key,
and `key.modifiers` tells you whether Control was held down.

When both checks pass, `KeyControl::Exit` tells Leptatui to close the app.
A plain `q` returns `KeyControl::Pass`, so it can continue through normal
handling. That will matter when you add an editor that needs to accept `q`
as ordinary text. Keep this shortcut as you build on the app in later chapters.

Now replace the generated `src/main.rs` with the code that starts the app.

```rust
mod app;

use leptatui::prelude::*;

use crate::app::NotesApp;

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

`pub(crate)` makes `NotesApp` available inside this application, and
`use crate::app::NotesApp;`
brings the name into scope so you can write `<NotesApp />`. The tag closes
itself because you're passing it no nested content. Each file imports the
prelude for the names it uses.

`App::new` takes that view as the starting point for the whole interface.
`#[tokio::main]` starts the async runtime that runs `main`, and
`.run().await` keeps the app running while Leptatui draws the screen and waits
for input. Selecting `KeyControl::Exit` ends that loop and restores your
terminal. The `Result` from `run` becomes the result of `main`, so a startup
or runtime error can reach the shell.

From the notes project, run this in an interactive terminal.

```sh
cargo run
```

You should see “My notes — Ctrl+Q to quit”. Press Ctrl+Q to return to your
shell. If your terminal intercepts Ctrl+Q, change the key combination in
`NotesApp` to one your terminal passes to the app.

## As the app grows

These two files are enough for your first screen. You'll add modules as the
app needs them, while `main.rs` keeps the same job of starting it.

- `app.rs` composes the application and owns shared state and routing.
- `layouts/root` contains the persistent shell and its supporting components.
- `pages/<feature>` owns a routed page and any feature-specific components.
- `services` contains application data operations, separate from UI state.
- Styled components use `component.rs`, `style.rs`, and `mod.rs`; styled pages
  use `page.rs`, `style.rs`, and `mod.rs`. Small components without local styles
  can stay in a single file. A `mod.rs` declares private implementation modules
  and re-exports only what other parts of the application need.

The complete Markdown editor shows how these pieces fit together.
[Markdown editor architecture](../../examples/markdown-editor/README.md#architecture-and-data-flow)

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
