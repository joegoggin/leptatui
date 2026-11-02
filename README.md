# Leptatui

Leptatui is an experimental Rust library for building interactive terminal
applications. It combines Leptos reactive state, Ratatui rendering, and
Crossterm input in a managed terminal runtime.

Describe your interface with components and `view!` markup or Rust builders.
Signals keep displayed data up to date, and the runtime manages input, redraws,
and restoring the terminal when your app exits.

You can build applications with:

- Reusable components, shared state, buttons, and editable forms.
- Typed styles, responsive block/flex/grid layouts, scrolling, and themes.
- Multiple pages with routing, parameters, queries, and navigation history.
- Async reads and mutations with loading and error states.
- Markdown documents, tables, highlighted code, images, and progress bars.
- File operations, a file selector, and integration with external editors.

Leptatui is under active development. Its macros build terminal views; they
use Leptos-style syntax without producing browser DOM nodes.

## Try it

With a current stable Rust toolchain and an interactive terminal:

```sh
git clone https://github.com/joegoggin/leptatui.git
cd leptatui
cargo run --example hello_world
```

Press `q` to exit. The [first chapter](docs/guide/01-getting-started.md)
explains how to create your own application using this checkout.

## Learn Leptatui

The guide assumes basic Rust, but no terminal UI or Leptos experience. Build a
small notes app one step at a time, then explore specialized features.

1. [Getting started](docs/guide/01-getting-started.md)
2. [Building a screen](docs/guide/02-building-a-screen.md)
3. [State and interaction](docs/guide/03-state-and-interaction.md)
4. [Editing notes](docs/guide/04-editing-notes.md)
5. [Reusable components](docs/guide/05-reusable-components.md)
6. [Styling and layout](docs/guide/06-styling-and-layout.md)
7. [Pages and navigation](docs/guide/07-pages-and-navigation.md)
8. [Handling failures](docs/guide/08-handling-failures.md)
9. [Async data](docs/guide/09-async-data.md)
10. [Files and editors](docs/guide/10-files-and-editors.md)
11. [Documents and media](docs/guide/11-documents-and-media.md)
12. [Themes and reusable styles](docs/guide/12-themes-and-reusable-styles.md)
13. [Extending Leptatui](docs/guide/13-extending-leptatui.md)

Use the [documentation index](docs/README.md) for checkpoints and reference pages.

## Look things up

- [Feature index](docs/reference/features.md): find a guide, reference, or example for each feature.
- [Standard components and keyboard controls](docs/reference/components.md)
- [Layout](docs/reference/layout.md) and [stylesheets](docs/reference/stylesheets.md)
- [Routing](docs/reference/routing.md), [async work](docs/reference/async.md), and [files and editors](docs/reference/files-and-editors.md)
- [Runnable examples and their controls](crates/leptatui/examples/README.md)
- [Complete Markdown editor application](examples/markdown-editor/README.md)
- [Contributor instructions and quality gates](docs/contributing.md)

For generated API documentation, run `cargo doc -p leptatui --no-deps --open`.
