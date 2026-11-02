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

Replace `src/main.rs` with:

```rust
use leptatui::prelude::*;

#[component]
fn NotesApp() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q')
            && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            return KeyControl::Exit;
        }
        KeyControl::Pass
    });

    view! { <Text>"My notes — Ctrl+Q to quit"</Text> }
}

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
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

[Documentation index](../README.md) · [Next: Building a screen](02-building-a-screen.md)
