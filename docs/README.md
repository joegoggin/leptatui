# Leptatui documentation

Start here if you know basic Rust but are new to terminal interfaces or reactive
programming. Chapters 1–10 build a notes app: display a note, edit it, organize
its components and pages, then load and save data. Chapters 11–13 explore
additional capabilities using focused examples.

## Read in order

1. [Getting started](guide/01-getting-started.md)
2. [Building a screen](guide/02-building-a-screen.md)
3. [State and interaction](guide/03-state-and-interaction.md)
4. [Editing notes](guide/04-editing-notes.md)
5. [Reusable components](guide/05-reusable-components.md)
6. [Styling and layout](guide/06-styling-and-layout.md)
7. [Pages and navigation](guide/07-pages-and-navigation.md)
8. [Handling failures](guide/08-handling-failures.md)
9. [Async data](guide/09-async-data.md)
10. [Files and editors](guide/10-files-and-editors.md)
11. [Documents and media](guide/11-documents-and-media.md)
12. [Themes and reusable styles](guide/12-themes-and-reusable-styles.md)
13. [Extending Leptatui](guide/13-extending-leptatui.md)

## Complete checkpoints

Each checkpoint contains a complete `src/main.rs`. Use the same Cargo setup
from chapter 1; each later checkpoint replaces the earlier file.

- [Editable note, after chapter 4](checkpoints/04-editable-note.md)
- [Routed notes app, after chapter 7](checkpoints/07-routed-notes.md)
- [Async notes app, after chapter 9](checkpoints/09-async-notes.md)
- [File-backed notes app, after chapter 10](checkpoints/10-file-notes.md)

## Reference

Read these when you need exact behavior or a supported-property list.

- [Feature index](reference/features.md)
- [Standard components, editing keys, semantic documents, and Markdown](reference/components.md)
- [Layout values, overflow, flexbox, grid, positioning, and CSS differences](reference/layout.md)
- [Stylesheet syntax, cascade, imports, mixins, and themes](reference/stylesheets.md)
- [Routing and parameters](reference/routing.md)
- [Resources and actions](reference/async.md)
- [Filesystem, file selector, and external editors](reference/files-and-editors.md)

For signatures and individual methods, run `cargo doc -p leptatui --no-deps --open`
from the checkout. The prelude collects common imports; explicit module imports
remain available. APIs under `__private` are implementation details.

## Examples and contributing

- [Runnable library examples](../crates/leptatui/examples/README.md)
- [Complete Markdown editor application](../examples/markdown-editor/README.md)
- [Workspace, checks, conformance tests, and benchmarks](contributing.md)

[Project README](../README.md)
