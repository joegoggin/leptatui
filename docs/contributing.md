# Working on Leptatui

These instructions are for changing the library. To build an application, start with the [guide](guide/01-getting-started.md).

## Workspace Layout

- `crates/leptatui`: Public runtime crate with app, component, view, context,
  style, and prelude APIs.
- `crates/leptatui-lsp`: Rust-analyzer proxy providing markup and stylesheet editor support.
- `crates/leptatui-macros`: Internal proc-macro crate that implements
  `#[component]`, `view!`, `stylesheet!`, and the route/query parameter derives.
  The runtime crate defines `view_error!`.
- `crates/leptatui/examples`: Runnable examples for the public crate.
- `examples/markdown-editor`: Standalone reference application.
- `docs`: Progressive guide with complete chapter code and feature references.

Within each crate, domain facades keep public imports stable while substantive
subdirectories group related implementations. Generic pass-through layers such
as `model`, `utils`, or `expansion` are avoided; a directory either owns direct
source files or is flattened into its parent.

## Teaching examples

Use `view!` for application structure and `stylesheet!` for styling in guides,
references, rustdoc, and runnable examples. Direct constructors and fluent style
builders are an anti-pattern for ordinary application UI. Use tracked children
in markup for conditional content. Keep helpers without macro equivalents
inside markup, and explain their purpose. Inline style builders are appropriate
only when an example specifically teaches inline precedence or `!important`.

Progressive application guides should teach file organization from project
creation, following `examples/markdown-editor`: a startup entry point, application
composition, feature-owned pages and components, layouts, and data services as
needed. Keep styled components in `component.rs`, `style.rs`, and `mod.rs`;
use `page.rs` for routed pages. Small unstyled components can remain flat files.
Register co-located styles through a helper called during component setup.
Avoid accumulating the whole application in `main.rs` or another large file.
Show complete new or changed files with explicit paths, module declarations,
re-exports, and move/deletion instructions; retain unchanged files between
chapters. Do not create empty architectural layers ahead of their use.

## Quality Gates

CI validates the full workspace with the same commands expected for local
pre-review checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
cargo check --workspace --examples
```

## Editor protocol checks

The LSP suite requires Python 3, rust-analyzer, and rustfmt. An optional
`leptosfmt` executable enables the mixed view/stylesheet formatting check.
The suite creates temporary consumer
applications and validates completion, hover, navigation, references, and
compilable rename edits through the real language server:

```sh
cargo build -p leptatui-lsp
python3 tools/lsp/test_view.py
python3 tools/lsp/test_stylesheet.py
python3 tools/lsp/test_formatting.py
```

CI runs this separately with Rust/rust-analyzer 1.92.0. See
[editor support](reference/editor-support.md) for Neovim configuration.

## Layout conformance and benchmarks

The layout contract is protected by public-API integration tests that retain
terminal geometry and, where painting matters, compare complete rendered rows.
This matrix is the index for the supported property groups and the edge cases
that must remain covered:

| Contract | Properties and behavior | Edge-case coverage | Executable coverage |
| --- | --- | --- | --- |
| Flow and visibility | `display`, block flow, source ordering, `Display::None` | Automatic sizes, nested blocks, transparent boundaries, hidden subtrees | `cargo test -p leptatui --test view suite::layout::block_flow` |
| Box geometry | `box_sizing`, `size`, `min_size`, `max_size`, `aspect_ratio`, `margin`, `padding` | Content/border boxes, percentages, viewport units, invalid ratios, zero-sized boxes, cumulative rounding | `cargo test -p leptatui --test view suite::layout::box_model`<br>`cargo test -p leptatui --test view suite::layout::sizing` |
| Intrinsic measurement | Automatic dimensions and measured leaf/container contributions | Wrapped text, trailing newlines, component boundaries, nested/replaced views | `cargo test -p leptatui --test view suite::layout::measurement` |
| Overflow | `overflow` on both axes, retained viewport and clip geometry | Visible/hidden/clip/scroll/auto, conditional gutters, nested clips, wheel/focus scrolling, reconciliation | `cargo test -p leptatui --test view suite::layout::overflow` |
| Flexbox | `flex_direction`, `flex_wrap`, `flex_basis`, `flex_grow`, `flex_shrink`, `gap`, flex alignment | Reversed axes, wrapped lines, intrinsic bases, constraints, zero-sized items, odd remainders, terminal resize | `cargo test -p leptatui --test flexbox_conformance`<br>`cargo test -p leptatui --test view suite::layout::flex` |
| Grid sizing | Templates, automatic tracks, `minmax()`, repeats, fractions, `gap`, grid alignment | Intrinsic and nested grids, percentage/auto/fraction mixes, empty repeated tracks, cumulative rounding, terminal resize | `cargo test -p leptatui --test grid_sizing_conformance` |
| Grid placement | `grid_row`, `grid_column`, `grid_auto_flow` | Signed lines, forward/backward spans, implicit tracks, collisions, sparse/dense row and column flow | `cargo test -p leptatui --test grid_placement_conformance` |
| Positioning and stacking | `position`, `inset`, `z_index` | Static/relative/absolute/fixed/sticky, containing blocks, percentage resize, nested scrollports, clipping, atomic stacking contexts | `cargo test -p leptatui --test positioning_conformance` |
| Responsive recomputation | Viewport units, media queries, retained geometry rebuilds | Narrow/wide breakpoints, repeated resize, flex/grid reflow | `cargo test -p leptatui --test view suite::layout::responsive`<br>`cargo test -p leptatui --test flexbox_conformance responsive`<br>`cargo test -p leptatui --test grid_sizing_conformance responsive` |

The performance suite exercises the same public render path for cold layout
construction, intrinsic measurement, resize recomputation, deep trees, and
large flex/grid collections. Record a machine-local Criterion baseline before
making layout changes:

```sh
cargo bench -p leptatui --bench layout -- --save-baseline phase-18
```

Compare a later run on the same machine and toolchain:

```sh
cargo bench -p leptatui --bench layout -- --baseline phase-18
```

Criterion stores named baselines below the ignored `target/criterion`
directory. Timings are intentionally not committed because terminal backend,
host, power, and toolchain differences make cross-machine measurements
incomparable.

## Other workspace commands

```sh
cargo metadata --format-version 1
cargo check --workspace
cargo check --workspace --examples
cargo test --workspace
```

[Documentation index](README.md)
