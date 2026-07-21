# Trait-Based View and Workspace Refactor Summary

## Overview

This refactor replaces the closed `View` enum with an open, object-safe `View`
trait. Built-in UI elements are now concrete structs, heterogeneous trees use
`AnyView` only at ownership boundaries, and related behavior is grouped into
capability traits.

A follow-up structural refactor now applies the same domain-oriented layout to
the rest of the workspace. Every concrete view has its own source file, shared
runtime concerns are grouped under substantive domain directories, proc-macro
syntax modules no longer pass through generic `model`, `utils`, or `expansion`
layers, and large integration-test targets are divided by behavior.

The organization follows one filesystem rule throughout: a directory with
child directories must own substantive source files in addition to `mod.rs`.
Directories that only forwarded to another directory were flattened, while
file-and-directory pairs such as `layout.rs` plus `layout/render.rs` were
consolidated as `layout/mod.rs` plus `layout/render.rs`.

The central architectural change is:

```text
Before
View enum
  -> every renderer, traversal operation, and helper matches every variant
  -> adding a view requires changing several exhaustive matches

After
View trait
  -> each concrete view implements its own rendering and specialized behavior
  -> AnyView erases concrete child types only where heterogeneous storage is needed
  -> adding an application-owned view does not require changing the runtime
```

Backward compatibility was intentionally not preserved. Code that names enum
variants, pattern-matches `View`, or returns `View` directly must migrate to the
new trait and concrete types.

## Goals Achieved

- Removed the closed set of render-tree node variants.
- Eliminated the major exhaustive `match` statements over `View`.
- Made application-defined view types possible without modifying Leptatui.
- Kept builder return values concrete so type-specific configuration remains
  available.
- Preserved heterogeneous child trees through explicit type erasure.
- Grouped shared styling, container, text, and editing behavior into capability
  traits.
- Unified generated components and ordinary views under one runtime protocol.
- Opened stylesheet type selectors to application-defined view names.
- Preserved focus, scrolling, input state, component state, and dynamic-view
  reconciliation behavior.
- Replaced the aggregate `model.rs`, `builders.rs`, and `render.rs` files with
  domain-oriented modules.
- Colocated every concrete view's data, constructor, accessors, and `View`
  implementation in one owning file.
- Split the shared editable-control concrete type into distinct `InputView` and
  `TextAreaView` types.
- Preserved a flat public API even though the internal files are now nested by
  domain.
- Split styling into `cascade` and `foundation` domains.
- Split Markdown parsing and terminal-image support into focused modules.
- Split rendering context, image rendering, and render targets under
  `component/render`.
- Flattened proc-macro `model` and `utils` pass-through directories.
- Divided component expansion and `view!` element expansion into substantive
  ownership modules.
- Split the largest integration-test targets into behavioral source files while
  retaining their existing Cargo test-target names.

## Module Organization

### View Runtime

The `view` directory now has the following structure:

```text
view/
├── mod.rs
├── reconciliation.rs
├── boundary/
│   ├── mod.rs
│   ├── component.rs
│   └── dynamic.rs
├── containers/
│   ├── mod.rs
│   ├── block.rs
│   ├── layout/
│   │   ├── mod.rs
│   │   └── render.rs
│   ├── lists/
│   │   ├── mod.rs
│   │   ├── list.rs
│   │   ├── list_item.rs
│   │   └── render.rs
│   └── tables/
│       ├── mod.rs
│       ├── table.rs
│       ├── table_section.rs
│       ├── table_row.rs
│       ├── table_cell.rs
│       └── render.rs
├── content/
│   ├── mod.rs
│   ├── text.rs
│   ├── heading.rs
│   ├── paragraph.rs
│   └── code_block/
│       ├── mod.rs
│       └── highlight.rs
├── controls/
│   ├── mod.rs
│   ├── button.rs
│   ├── form.rs
│   ├── progress_bar.rs
│   └── editable/
│       ├── mod.rs
│       ├── model.rs
│       ├── state.rs
│       ├── input.rs
│       ├── text_area.rs
│       ├── render.rs
│       ├── insert.rs
│       ├── normal.rs
│       ├── visual.rs
│       └── movement.rs
├── core/
│   ├── mod.rs
│   ├── contract.rs
│   ├── any_view.rs
│   ├── conversion.rs
│   ├── capabilities.rs
│   ├── events.rs
│   ├── metadata.rs
│   └── render.rs
└── media/
    ├── mod.rs
    └── image.rs
```

The domain modules are internal organization details. `view/mod.rs` re-exports
the supported public builders, traits, concrete view types, and discriminator
types so users do not need to import from these nested paths.

Reconciliation lives directly under `view` because it coordinates all view
domains. This also keeps the top-level `view` facade from becoming a directory
that contains only `mod.rs` and child directories.

### Runtime Support

The other runtime areas now follow the same organization:

```text
component/
├── mod.rs
├── children.rs
├── contract.rs
├── key.rs
├── stylesheet.rs
└── render/
    ├── mod.rs
    ├── image.rs
    └── target.rs

markdown/
├── mod.rs
├── block.rs
├── code.rs
├── inline.rs
├── inline_events.rs
├── list.rs
├── table.rs
└── tests.rs

style/
├── mod.rs
├── style_module.rs
├── cascade/
│   ├── mod.rs
│   ├── declarations.rs
│   ├── media.rs
│   ├── selector.rs
│   └── stylesheet.rs
└── foundation/
    ├── mod.rs
    ├── layout.rs
    ├── size.rs
    ├── spacing.rs
    ├── theme.rs
    └── tui_style.rs

terminal_image/
├── mod.rs
├── backend.rs
├── cache.rs
├── fallback.rs
└── tests.rs
```

The former aggregate `component/model.rs`, `markdown.rs`, and
`terminal_image.rs` files are now facades with focused implementations. Style
declarations and cascade behavior are separated from foundational value types
without adding generic intermediate directories.

### Procedural Macros

The proc-macro crate now has the following source structure:

```text
src/
├── lib.rs
├── crate_path.rs
├── component/
│   ├── mod.rs
│   ├── signature.rs
│   └── expand/
│       ├── mod.rs
│       ├── constructors.rs
│       └── props.rs
├── stylesheet/
│   ├── mod.rs
│   ├── declaration.rs
│   ├── import.rs
│   ├── media.rs
│   ├── mixin.rs
│   ├── root.rs
│   ├── rule.rs
│   ├── selector.rs
│   ├── value.rs
│   └── variable.rs
└── view/
    ├── mod.rs
    ├── attr.rs
    ├── child.rs
    ├── root.rs
    ├── syntax.rs
    ├── text_content.rs
    └── element/
        ├── mod.rs
        ├── attributes.rs
        ├── builtins.rs
        ├── children.rs
        ├── custom_component.rs
        └── validation.rs
```

The stylesheet syntax files now live directly under `stylesheet` instead of
`stylesheet/model`. The `view!` syntax files likewise live directly under
`view`, and element parsing owns its expansion concerns without an
`element/expansion/builtins` pass-through chain. Shared crate-path resolution is
a direct crate module rather than a single-file `utils` directory.

### Integration Tests

The large `view`, `style`, `markdown`, and `component_macro` integration targets
retain thin top-level Cargo harnesses. Their test bodies now live in matching
directories and are grouped by behavior:

```text
tests/
├── view.rs
├── view/{boundary,code_block,content,editable_render,forms,interaction,
│         layout,lists,media,metadata,progress,semantic_builders,tables}.rs
├── style.rs
├── style/{cascade,foundation,macro_integration,modules,theme}.rs
├── markdown.rs
├── markdown/{code,core,fallback,rendering}.rs
├── component_macro.rs
├── component_macro/{alias,compile,context,input,key_events,lifecycle,
│                     rendering,routing,styling}.rs
└── support.rs
```

Each matching directory also contains a documented `mod.rs` suite facade. The
split preserves the original test function inventory and Cargo target names.

## New Core View Model

### `View` trait

`View` is now an object-safe trait implemented by every renderable node. A
render-only leaf must implement:

- `render(&self, ctx: &mut RenderCtx<'_, '_>)`.
- `as_any(&self)`.
- `as_any_mut(&mut self)`.

The trait supplies defaults for the remaining behavior, including:

- Minimum-height measurement.
- Style metadata access.
- Child traversal.
- Top-level event and key handling.
- Application-defined `on_event` and `on_key_event` hooks.
- Retained-state reconciliation.
- Focus traversal and activation.
- Editable-control event forwarding.
- Form submit and cancel behavior.
- Overflow scrolling and `gg`/`G` state.

This makes a custom render-only view small while allowing interactive or
container views to opt into the additional hooks they need.

Rendering takes `&self` instead of `&mut self`. Views that need retained
render-time mutation can use interior mutability, as the dynamic and component
boundaries do.

### `AnyView`

`AnyView` owns a `Box<dyn View>` and is the type-erasure boundary for
heterogeneous trees. It provides:

- Construction from any concrete `View`.
- `is`, `downcast_ref`, and `downcast_mut` for concrete-type inspection.
- Shared and mutable access to the underlying `dyn View`.
- Style metadata and child access.
- Fluent id, class, inline-style, and focus configuration when the stored view
  exposes metadata.
- Internal forwarding for focus, events, forms, editing, and scrolling.
- Reconciliation with a previous `AnyView`.
- Structural equality for built-in concrete views and identity equality for
  dynamic and component boundaries.

`AnyView` is intentionally not the return type of every builder. It is used
when values enter a heterogeneous child collection, component boundary,
dynamic boundary, or other ownership location that requires type erasure.

### Conversion traits

`IntoView` converts view-compatible values into `AnyView`.

Implementations cover:

- Every concrete type implementing `View`.
- `AnyView` itself without adding another erasure layer.
- `String` and `&str`, which become `TextView` values.

`IntoViews` converts child collections into `Vec<AnyView>`. It supports:

- `Vec<V>` where `V: IntoView`.
- Arrays `[V; N]` where `V: IntoView`.
- The empty tuple `()`.
- Heterogeneous tuples containing one through twenty-six `IntoView` values.

Tuples are now the preferred builder syntax for heterogeneous static children:

```rust
let view = column((
    h1("Settings"),
    paragraph("Choose an option."),
    row((button("Save"), button("Cancel"))),
));
```

Homogeneous arrays remain concise:

```rust
let actions = row([button("Save"), button("Cancel")]);
```

Iterator pipelines should currently be collected before being passed to a
builder:

```rust
let items = column(values.into_iter().map(text).collect::<Vec<_>>());
```

## Capability Traits

The refactor introduces four public traits for behavior shared across related
concrete views.

### `StyledView`

`StyledView` exposes `StyleMetadata` and supplies fluent methods for:

- `with_id`.
- `with_classes`.
- `with_inline_style`.
- `with_focus`.

All styleable built-in views implement this trait. The concrete types also
provide inherent forwarding methods so normal builder chains do not require
explicit trait-qualified calls.

### `ContainerView`

`ContainerView` provides shared and mutable access to direct `AnyView`
children. It is implemented by blocks, layouts, forms, lists, list items,
tables, table sections, and table rows.

For a custom container, implementing `ContainerView` provides the shared
container API. Its `View` implementation should also forward `children` and
`children_mut` so default event, focus, scrolling, and reconciliation traversal
can enter the subtree.

### `TextualView`

`TextualView` exposes retained Ratatui rich text. It is shared by:

- `TextView`.
- `HeadingView`.
- `ParagraphView`.
- `TableCellView`.

This replaces repeated enum matching for text extraction.

### `EditableView`

`EditableView` groups fluent configuration shared by single-line inputs and
multiline text areas:

- `placeholder`.
- `on_input`.

The controls now have separate public concrete types: `InputView` and
`TextAreaView`. Their shared editing state and algorithms remain internal to
`controls/editable`, but callers and reconciliation can distinguish the two
controls directly through their Rust types. Input callbacks use the shared
`EditableAction` alias.

## Concrete Built-In Views

The old enum variants became the following concrete public types:

| Area | Concrete type | Relevant semantic discriminator |
| --- | --- | --- |
| Bordered container | `BlockView` | None. |
| Plain text | `TextView` | None. |
| Headings | `HeadingView` | `HeadingLevel::H1` through `HeadingLevel::H6`. |
| Paragraphs | `ParagraphView` | None. |
| Source code | `CodeBlockView` | Language, syntax theme, and line-number settings. |
| Ordered and unordered lists | `ListView` | `ListKind::Ordered` or `ListKind::Unordered`. |
| List items | `ListItemView` | None. |
| Tables | `TableView` | None. |
| Table head and body | `TableSectionView` | `TableSectionKind::Head` or `TableSectionKind::Body`. |
| Table rows | `TableRowView` | None. |
| Table cells | `TableCellView` | `CellAlignment`. |
| Rows and columns | `LayoutView` | `LayoutDirection::Row` or `LayoutDirection::Column`. |
| Forms | `FormView` | Submit and cancel callbacks. |
| Buttons | `ButtonView` | Activation callback. |
| Single-line inputs | `InputView` | Single-line editing behavior. |
| Multiline text areas | `TextAreaView` | Multiline editing behavior. |
| Images | `ImageView` | `ImageSource` and optional fallback text. |
| Progress bars | `ProgressBarView` | Clamped value and optional label. |
| Reactive children | `DynamicView` | Deferred child closure. |

Several related enum variants intentionally share one concrete struct. The
semantic discriminator is included in reconciliation checks so state cannot
leak between different meanings that happen to share a Rust type.

## Typed Builders

Every public builder now retains a concrete return type:

| Builder family | Return type |
| --- | --- |
| `block` | `BlockView`. |
| `text` | `TextView`. |
| `h1` through `h6` | `HeadingView`. |
| `paragraph` | `ParagraphView`. |
| `code_block` | `CodeBlockView`. |
| `ordered_list`, `unordered_list` | `ListView`. |
| `list_item` | `ListItemView`. |
| `table` | `TableView`. |
| `table_head`, `table_body` | `TableSectionView`. |
| `table_row` | `TableRowView`. |
| `table_cell` | `TableCellView`. |
| `row`, `column` | `LayoutView`. |
| `form` | `FormView`. |
| `button` | `ButtonView`. |
| `input` | `InputView`. |
| `text_area` | `TextAreaView`. |
| `image` | `ImageView`. |
| `progress_bar` | `ProgressBarView`. |
| `dynamic` | `DynamicView`. |
| `component` | `AnyView`. |

This removes the need for methods that silently do nothing on unrelated enum
variants. For example, `language` exists on `CodeBlockView`, `alignment` exists
on `TableCellView`, and `on_press` exists on `ButtonView`.

Public accessors were added for inspecting concrete state without matching an
enum. These cover metadata, children, rich text, heading levels, list and table
roles, code-block options, editable state, callback presence, image settings,
and progress-bar settings.

## Rendering and Traversal

Rendering moved from one central enum dispatcher into `View` implementations
for each concrete built-in type. Shared layout, list, table, editor, and focus
algorithms remain helper functions where they represent common mechanics, but
adding a new view no longer requires adding a new arm to a global `View` match.

Default traversal is defined by the `View` trait's child accessors. A custom
container that exposes children automatically participates in:

- Non-key event traversal.
- Key-event traversal.
- Pending-input flushing.
- Focus counting and focus movement.
- Focused-button activation.
- Editable-control key forwarding.
- Form key forwarding.
- Overflow scrolling.
- Recursive reconciliation.

Built-in controls override only the operations that are specific to their
behavior.

## Custom View Styling

`RenderCtx` now exposes `resolve_style`, allowing custom views to resolve the
same cascade as built-ins. Resolution includes:

- Semantic defaults.
- Component-scoped stylesheets.
- Type, class, id, focus, compound, and descendant selectors.
- Inline styles.
- `!important` declarations.
- Media queries using the root viewport.
- Runtime theme variables.
- Inherited text styles.

The existing area helpers used by built-in containers are public where custom
containers need them:

- `with_area`.
- `with_area_and_inherited_style`.
- `with_area_inherited_style_and_selector_ancestor`.

`TuiStyle::inherited_values` is also public so a custom container can propagate
the same inheritable values as a built-in container.

A styleable custom view typically:

1. Stores `StyleMetadata::new(ViewType::new("CustomName"))`.
2. Implements `StyledView`.
3. Returns its metadata from the `View` metadata hooks.
4. Calls `ctx.resolve_style(&self.metadata)` while rendering.

## Open Stylesheet Type Selectors

`ViewType` changed from a closed enum to a copyable, hashable newtype around a
static name.

Built-in names remain available as associated constants such as:

- `ViewType::Text`.
- `ViewType::Column`.
- `ViewType::Button`.

Application-owned names use `ViewType::new`:

```rust
const BADGE: ViewType = ViewType::new("Badge");
```

The `stylesheet!` macro now accepts any identifier as a type selector and lowers
it to `ViewType::new(stringify!(...))`:

```rust
let styles = stylesheet! {
    Badge => { fg: Color::LightMagenta }
};
```

The previous compile-fail fixture for an unsupported type selector was removed
and replaced with a passing custom-selector fixture.

## Event Handling

`View` now owns the complete event protocol that was previously split between
`View` and `Component`.

Public entry points are:

- `handle_event` for full terminal events.
- `handle_key_event` for key events with explicit `KeyControl` propagation.
- `on_event` for custom non-key behavior on a node.
- `on_key_event` for custom key behavior on a node.

Default dispatch visits child views before invoking a node's custom hook.
Built-in focus, editing, form, activation, and scrolling behavior runs after
custom key dispatch when the result is `KeyControl::Pass`.

Generated components retain their registered `use_key_event` handlers and
continue to preserve child-before-parent propagation and short-circuiting.

## Component Unification

The public `Component` trait was removed. Generated `#[component]` types now
implement `View` directly.

The component macro now:

- Stores its rendered tree as `AnyView`.
- Converts component body results through `IntoView`.
- Implements `View::render`, measurement, event traversal, focus traversal,
  form behavior, scrolling, and reconciliation hooks.
- Preserves the component's Leptos owner, key-handler registry, and scoped
  stylesheet.
- Returns `AnyView` from the generated inherent `into_view` method.

`ComponentView` remains an internal boundary type. It supplies persistent
context scope and shared mutable component storage when a generated component
is embedded in another tree.

`App::new` now accepts any `IntoView`. `AppRoot` adapts both `AnyView` and
concrete `View` implementations, so built-ins, generated components, and
application-defined views can all be roots.

## `view!` Macro Changes

The `view!` macro now lowers child expressions through `IntoView` instead of
converting them into an enum.

Consequences include:

- Application-defined `View` values can appear directly in braced child
  expressions.
- Built-in element expansions retain their concrete builder type at the root.
- Children are converted to `AnyView` when collected into heterogeneous
  vectors.
- Component children are converted through the same protocol.
- Dynamic child closures may return any `IntoView` type.

The element grammar remains intentionally closed for angle-bracket built-in
tags. Custom Rust views are embedded as expressions, while PascalCase component
tags continue to use generated props builders.

## Reconciliation

Reconciliation is now trait-driven rather than an exhaustive enum match.

The core process is:

1. Ask the new node whether it can reconcile from the previous node through
   `can_reconcile_from`.
2. Preserve transient style metadata for compatible nodes.
3. Invoke the concrete view's `reconcile` hook.
4. Reconcile corresponding direct children recursively.

The default compatibility rule requires identical concrete Rust types.
Specialized rules handle cases where type equality is insufficient:

- `InputView` reconciles only from `InputView`, and `TextAreaView` reconciles
  only from `TextAreaView`. Their distinct concrete types prevent editing state
  from leaking between single-line and multiline controls.
- `DynamicView` preserves a deferred boundary only when both values share the
  same allocation identity. A newly produced nested dynamic closure therefore
  replaces the previous closure.
- `ComponentView` preserves a generated boundary only when reconciliation is
  enabled and the component types match.

Compatible metadata retains focus requests, scroll offsets, maximum scroll
offsets, and pending `gg` state while leaving newly authored ids, classes, and
inline styles intact.

Editable controls additionally retain cursor position, selection, Vim mode,
scrolling, yank state, undo history, and redo history when their concrete types
match.

## Public API and Prelude Changes

The top-level crate and prelude now export:

- `View`, `AnyView`, `IntoView`, and `IntoViews`.
- All concrete built-in view types.
- `StyledView`, `ContainerView`, `TextualView`, and `EditableView`.
- Semantic discriminator types such as `HeadingLevel`, `ListKind`,
  and `TableSectionKind`.
- `InputView`, `TextAreaView`, `EditableAction`, and `EditableState` for
  editable controls.
- `DynamicView`, `StyleMetadata`, and the open `ViewType`.

The removed `Component`, `EditableTextView`, `EditableKind`, and `InputAction`
exports are deliberate breaking changes. `EditableAction` replaces
`InputAction` as the callback alias shared by both editable controls.

Component function signatures in examples and documentation now normally use:

```rust
#[component]
fn Root() -> impl IntoView {
    view! { <Text>"Hello"</Text> }
}
```

## Migration Examples

### Returning views

Before:

```rust
fn content() -> View {
    text("Hello")
}
```

After, when callers do not need the concrete type:

```rust
fn content() -> impl IntoView {
    text("Hello")
}
```

After, when callers benefit from the concrete API:

```rust
fn content() -> TextView {
    text("Hello")
}
```

### Inspecting a built-in view

Before:

```rust
match view {
    View::Button { label, .. } => assert_eq!(label, "Save"),
    _ => panic!("expected button"),
}
```

After, when the builder result remains concrete:

```rust
let view = button("Save");
assert_eq!(view.label(), "Save");
```

After type erasure:

```rust
let view = button("Save").into_view();
let button = view
    .downcast_ref::<ButtonView>()
    .expect("expected button");
assert_eq!(button.label(), "Save");
```

### Building heterogeneous children

Before:

```rust
let view = column([
    h1("Title"),
    paragraph("Body"),
    button("Continue"),
]);
```

After:

```rust
let view = column((
    h1("Title"),
    paragraph("Body"),
    button("Continue"),
));
```

### Implementing a custom view

```rust
use std::any::Any;

use leptatui::prelude::*;
use ratatui::widgets::Paragraph;

struct Badge {
    label: String,
    metadata: StyleMetadata,
}

impl Badge {
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            metadata: StyleMetadata::new(ViewType::new("Badge")),
        }
    }
}

impl StyledView for Badge {
    fn metadata(&self) -> &StyleMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut StyleMetadata {
        &mut self.metadata
    }
}

impl View for Badge {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let style = ctx.resolve_style(&self.metadata);
        ctx.render_widget(
            Paragraph::new(self.label.clone()).style(style.to_ratatui_style()),
        );
        Ok(())
    }

    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }

    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

let view = column((Badge::new("New"), text("Built in")));
```

## Test and Fixture Migration

Tests that previously destructured enum variants now use one of three paths:

- Concrete accessors when the value has not been erased.
- `AnyView` downcasting when inspecting a heterogeneous child.
- Trait methods for metadata, child traversal, rendering, or event behavior.

New coverage includes:

- A custom styleable and interactive view composed beside a built-in view.
- Rendering a custom view through `RenderCtx`.
- Applying a custom `ViewType` stylesheet rule.
- Type erasure and downcasting.
- Public custom key-event hooks.
- A `view!` pass fixture containing an application-defined view expression.
- A `stylesheet!` pass fixture containing an application-defined type selector.
- Reconciliation isolation between inputs and text areas.
- Concrete builder return types for both `input` and `text_area`.
- `AnyView` equality for matching editable types and inequality across input and
  text-area boundaries.
- Replacement of newly created nested dynamic boundaries.
- Focus scrolling across component boundaries after the protocol unification.

Existing tests, examples, Markdown construction, app roots, routing, context,
resources, actions, forms, editable controls, tables, lists, code blocks,
images, progress bars, styling, media queries, themes, dynamic views, and
component macros were migrated to the new API.

The proc-macro source-layout test now checks the flattened `view` syntax paths.
It requires ordinary syntax types to keep their impls in their declaration
files while allowing `Element` expansion impls anywhere inside the substantive
`view/element` owner directory.

## Documentation Changes

- Updated crate-level and prelude examples to return `impl IntoView`.
- Updated heterogeneous builder examples to use tuples.
- Documented the open `View` extension model in the README.
- Updated Markdown API links that previously referenced enum variants.
- Added formal Rustdoc for the new traits, builders, conversion behavior,
  custom styling APIs, and public accessors.
- Updated editable-control documentation for `InputView`, `TextAreaView`, and
  `EditableAction`.
- Added module maps for the new runtime and proc-macro facades.
- Documented the domain-directory and no-pass-through organization rule in the
  README.
- Updated the README to describe the two distinct editable control types.
- Updated example applications to compile against concrete builder results and
  the unified component protocol.

## Validation

The structural refactor passes:

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo doc --workspace --no-deps` without warnings.
- `cargo check --workspace --examples`.
- All workspace doctests.
- The proc-macro source-layout architecture test.
- An inventory comparison confirming all 153 original `view` integration tests
  remain present after the split.
- All workspace tests unrelated to the existing editable-control cursor
  failures.
- `git diff --check`.

An unfiltered `cargo test --workspace` still reports twenty editable-control
cursor and insertion failures. A representative failure was reproduced against
the original `HEAD`, confirming that behavior predates this structural change.

## Primary Files Changed

The follow-up structural work is concentrated in:

- `crates/leptatui/src/view/mod.rs` for the flat public re-export surface.
- `crates/leptatui/src/view/core/` for the `View` contract, `AnyView`,
  conversions, capabilities, events, shared rendering helpers, and metadata.
- `crates/leptatui/src/view/reconciliation.rs` for cross-domain retained-state
  reconciliation.
- `crates/leptatui/src/view/content/` for text, headings, paragraphs, and code
  blocks.
- `crates/leptatui/src/view/containers/` for blocks, layouts, lists, and tables.
- `crates/leptatui/src/view/controls/` for buttons, forms, progress bars, and
  editable controls.
- `crates/leptatui/src/view/controls/editable/input.rs` and `text_area.rs` for
  the distinct public editable view types.
- `crates/leptatui/src/view/boundary/` for dynamic and component ownership
  boundaries.
- `crates/leptatui/src/view/media/image.rs` for image data, construction, and
  rendering.
- `crates/leptatui/src/component/render/` for rendering context, image methods,
  and frame or buffer targets.
- `crates/leptatui/src/style/` for cascade and foundation style domains.
- `crates/leptatui/src/markdown/` and `src/terminal_image/` for focused parsing,
  backend, caching, fallback, and test modules.
- `crates/leptatui-macros/src/stylesheet/`, `src/view/`, and
  `src/component/expand/` for flattened syntax ownership and focused code
  generation.
- `crates/leptatui/src/lib.rs` and `src/prelude.rs` for the updated public
  exports.
- `crates/leptatui/tests/{view,style,markdown,component_macro}/` for the split
  behavioral integration suites.
- `crates/leptatui-macros/tests/model_impl_layout.rs` for the updated syntax
  ownership invariant.

The former aggregate files `view/model.rs`, `view/builders.rs`,
`view/render.rs`, `view/render/table.rs`, `view/dynamic.rs`,
`view/component_view.rs`, and `view/metadata.rs` were removed after their
responsibilities moved to the owning modules above.

The workspace-wide pass additionally removed `component/model.rs`,
`markdown.rs`, `terminal_image.rs`, the flat style implementation files, and
the proc-macro `stylesheet/model`, `view/model`, `view/utils`, and top-level
`utils` directories after moving their responsibilities to the structures
documented above.

## Result

The render tree is no longer limited to a library-owned enum. Built-ins retain
ergonomic, typed builder APIs; components and ordinary views share one
protocol; and application code can introduce new renderable, styleable,
interactive, and container views without editing Leptatui's central rendering
or traversal dispatch. Internally, each built-in view now has a focused owner
file, closely related rendering and editing algorithms are grouped in small
domain helpers, and new views can be added without growing another central
model, builder, or renderer file.
