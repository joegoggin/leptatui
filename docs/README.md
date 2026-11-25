# Leptatui documentation

Start here if you know basic Rust but are new to terminal interfaces or reactive
programming. Chapters 1–12 build a notes app: display a note, edit it, organize
its components and pages, load and save data, then add Markdown and themes.
Chapter 13 explores custom views in a standalone application.

Build application UI with `view!` and declare styles with `stylesheet!`.
Direct view and style builders are low-level escape hatches and an anti-pattern
for ordinary application code.

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

## Code in each chapter

Each chapter explains incremental edits and ends with a complete, runnable
`src/main.rs`. Use the Cargo manifest from chapter 1 throughout. Chapters 1–12
progressively build the notes app; chapter 13 is a standalone extension lesson.
Optional experiments are marked separately from the code to carry forward.

## Reference

Read these when you need exact behavior or a supported-property list.

- [Editor support inside `view!` and `stylesheet!`](reference/editor-support.md)
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
