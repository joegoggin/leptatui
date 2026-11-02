# 1. Getting started

Prerequisites: Basic Rust and Cargo.

By the end of this chapter, you will have a running application that owns the
terminal and exits cleanly.

## Try an existing application

Install a current stable Rust toolchain with Cargo and use an interactive
terminal. From the repository checkout, run:

```sh
cargo run --example hello_world
```

You should see a greeting inside a bordered panel. Press `q` to return to your
shell. Run this in a terminal, rather than an editor's output-only panel.

## Create your notes project

The following commands create a sibling application. They assume the checkout
is called `leptatui` and you are currently inside it:

```sh
cd ..
cargo new leptatui-notes
cd leptatui-notes
```

Replace the generated `Cargo.toml` with:

```toml
[package]
name = "leptatui-notes"
version = "0.1.0"
edition = "2024"

[dependencies]
leptatui = { path = "../leptatui/crates/leptatui" }
tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread", "time"] }
```

The path points at the runtime crate inside the checkout, not at the workspace
root. Adjust it if you chose a different checkout location. This guide uses
the local source and does not depend on a crates.io release.

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
key combination to one it delivers to applications.

`prelude::*` imports commonly used types, traits, and macros. `#[component]`
turns a function into a reusable terminal component. `view!` constructs its
interface. Tokio supplies the async runtime used by `App::run` and later data
operations. `App` manages terminal setup, event polling, redraws, and cleanup.

The quit behavior is explicit: this application registers it with
`use_key_event`. Keep this handler in `NotesApp` throughout the guide.

Commands starting with `cargo run --example` in later chapters run from the
**library checkout**. Plain `cargo run` runs your **notes project**.

[Documentation index](../README.md) · [Next: Building a screen](02-building-a-screen.md)
