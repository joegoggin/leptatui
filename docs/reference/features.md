# Feature index

Use this table to locate a lesson, exact behavior, and working source. Examples
run from the checkout with `cargo run --example NAME`; their full controls are
in the [examples README](../../crates/leptatui/examples/README.md).

For individual methods and types, run `cargo doc -p leptatui --no-deps --open`.
This index covers supported public feature families, including advanced
extension interfaces; internal generated-code hooks are excluded.

| Feature and main APIs | Learn | Reference / source | Example |
| --- | --- | --- | --- |
| Startup, terminal lifecycle, `App`, `AppControl`, runtime `Result`/`Error` | [Getting started](../guide/01-getting-started.md) | [App API](../../crates/leptatui/src/app/mod.rs) | `hello_world` |
| `#[component]`, `view!`, builders, `IntoView` | [Building a screen](../guide/02-building-a-screen.md) | [Components](components.md) | `hello_world` |
| Signals, tracked/untracked reads, memos, effects, reactive redraw | [State and interaction](../guide/03-state-and-interaction.md) | [Prelude exports](../../crates/leptatui/src/prelude.rs) | `counter`, `async_redraw` |
| Keyboard handlers, `KeyControl`, focus, pointer activation | [State and interaction](../guide/03-state-and-interaction.md) | [Controls](components.md) | `counter`, `standard_library_showcase` |
| Buttons, `Input`, `TextArea`, forms, editable state and Vim modes | [Editing notes](../guide/04-editing-notes.md) | [Controls and editing keys](components.md) | `controlled_form` |
| Props, defaults, conversions, `Children`/`ChildrenFn`/`ChildrenMut` | [Reusable components](../guide/05-reusable-components.md) | [Component macro](../../crates/leptatui-macros/src/lib.rs) | [Routed checkpoint](../checkpoints/07-routed-notes.md) |
| Typed context, component ownership, sharing across routes | [Reusable components](../guide/05-reusable-components.md) | [Context API](../../crates/leptatui/src/context/mod.rs) | `multi_page_demo` |
| `dynamic`, `keyed`, `IntoViews`, `AnyView`, mixed collections | [Reusable components](../guide/05-reusable-components.md) | [View exports](../../crates/leptatui/src/view/mod.rs) | [Keyed view docs](../../crates/leptatui/src/view/boundary/dynamic.rs) |
| `Div`, `Block`, display, box sizing, lengths, dimensions, spacing | [Styling and layout](../guide/06-styling-and-layout.md) | [Layout values and geometry](layout.md) | `block_layout` |
| Flex direction, wrapping, grow/shrink/basis, alignment, gap | [Styling and layout](../guide/06-styling-and-layout.md) | [Flexbox contract](layout.md#flexbox-contract) | `responsive_flex` |
| Grid templates, tracks, repeats, fractions, placement, auto flow | [Styling and layout](../guide/06-styling-and-layout.md) | [Grid contract](layout.md#grid-contract) | `responsive_grid` |
| Overflow, scrolling, clipping, wheel targeting, focus scrolling | [Styling and layout](../guide/06-styling-and-layout.md) | [Overflow contract](layout.md#overflow-contract) | `nested_overflow` |
| Static/relative/absolute/fixed/sticky positioning, insets, z-index | [Styling and layout](../guide/06-styling-and-layout.md) | [Positioning contract](layout.md#positioning-contract) | `positioning_showcase` |
| Media queries, viewport units, resizing | [Styling and layout](../guide/06-styling-and-layout.md) | [Stylesheets](stylesheets.md), [layout differences](layout.md#supported-css-differences) | `responsive_flex`, `responsive_grid` |
| `TuiStyle`, `stylesheet!`, rules/selectors/declarations, inheritance, specificity | [Styling and layout](../guide/06-styling-and-layout.md) | [Stylesheets](stylesheets.md) | `style_cascade_showcase` |
| Pseudo-classes, nested selectors, parent class suffixes, `!important` | [Themes and reusable styles](../guide/12-themes-and-reusable-styles.md) | [Stylesheets](stylesheets.md) | `style_cascade_showcase` |
| Variables, mixins, imports, aliases, `StyleModule`/`StyleValue` | [Themes and reusable styles](../guide/12-themes-and-reusable-styles.md) | [Modules and mixins](stylesheets.md#modules-mixins-and-themes) | `stylesheet_imports` |
| `ThemeVariables`, `ThemeValue`, `theme_color`, theme switching | [Themes and reusable styles](../guide/12-themes-and-reusable-styles.md) | [Themes](stylesheets.md#modules-mixins-and-themes) | `theme_switcher`, `default_styles_showcase` |
| `Router`, `Routes`, `Route`, `A`/`route_link`, active anchors, fallback | [Pages and navigation](../guide/07-pages-and-navigation.md) | [Routing](routing.md) | `multi_page_demo` |
| `ParentRoute`, `Outlet`, retained nested layouts | [Pages and navigation](../guide/07-pages-and-navigation.md) | [Nested route source](../../crates/leptatui/tests/route_outlet.rs) | [Nested route test](../../crates/leptatui/tests/route_outlet.rs) |
| `Location`, `Navigate`, `NavigateOptions`, `History`, routing hooks | [Pages and navigation](../guide/07-pages-and-navigation.md) | [Routing API](../../crates/leptatui/src/route.rs) | `multi_page_demo` |
| `RouteParams`, `QueryParams`, `ParamsMap`, `ParamsError`, `with_query` | [Handling failures](../guide/08-handling-failures.md) | [Routing](routing.md) | [Markdown editor](../../examples/markdown-editor/README.md) |
| `ViewResult`, `ViewError`, `view_error!`, error screen and panic cleanup | [Handling failures](../guide/08-handling-failures.md) | [View errors](../../crates/leptatui/src/view/error.rs) | `error_handling` |
| `Resource`, refetch, loading and stale completions | [Async data](../guide/09-async-data.md) | [Resources and actions](async.md) | `async_redraw`, `async_crud` |
| `Action`, dispatch, pending/input/value/version, clear | [Async data](../guide/09-async-data.md) | [Resources and actions](async.md) | `async_crud` |
| `spawn`, `spawn_local` | [Async data](../guide/09-async-data.md) | [Executor API](../../crates/leptatui/src/executor.rs) | [Executor examples](../../crates/leptatui/src/executor.rs) |
| `FileSystem`, options, root initialization and path containment | [Files and editors](../guide/10-files-and-editors.md) | [Filesystem](files-and-editors.md#root-scoped-file-system) | `file_system_showcase` |
| Metadata, entries, path resolution, directory listings, binary/UTF-8 reads | [Files and editors](../guide/10-files-and-editors.md) | [Filesystem operation table](../../crates/leptatui/examples/README.md#file-system-showcase) | `file_system_showcase` |
| Directory creation, write/append/replace, copy, rename, deletion | [Files and editors](../guide/10-files-and-editors.md) | [Filesystem API](../../crates/leptatui/src/file_system.rs) | `file_system_showcase` |
| `FileOperation`, pending/results/version and retries | [Files and editors](../guide/10-files-and-editors.md) | [Filesystem](files-and-editors.md) | `file_system_showcase` |
| `FileSelector`, options, filtering, selection and clearing | [Files and editors](../guide/10-files-and-editors.md) | [File selector](files-and-editors.md#file-selector) | [Markdown editor](../../examples/markdown-editor/README.md) |
| `Editor`, `EditorStatus`, file/text editing, editor configuration | [Files and editors](../guide/10-files-and-editors.md) | [External editor](files-and-editors.md#external-editor) | `external_editor` |
| `AppHandle`, `use_app_handle`, terminal suspension | [Files and editors](../guide/10-files-and-editors.md) | [App handle API](../../crates/leptatui/src/app/handle.rs) | `external_editor` |
| Text, rich text, headings, paragraphs | [Documents and media](../guide/11-documents-and-media.md) | [Semantic documents](components.md#semantic-documents) | `document_showcase` |
| Ordered/unordered lists and nested list items | [Documents and media](../guide/11-documents-and-media.md) | [Semantic documents](components.md#semantic-documents) | `document_showcase` |
| Tables, sections, rows, inline cells and alignment | [Documents and media](../guide/11-documents-and-media.md) | [Semantic documents](components.md#semantic-documents) | `document_showcase` |
| Code blocks, syntax languages, line numbers and wrapping | [Documents and media](../guide/11-documents-and-media.md) | [Semantic documents](components.md#semantic-documents) | `document_showcase` |
| Markdown functions/options, source identity, synchronous files, async tag | [Documents and media](../guide/11-documents-and-media.md) | [Markdown readers](components.md#markdown-readers) | `markdown_reader` |
| Markdown links, anchors, history, editing/reloading and fallbacks | [Documents and media](../guide/11-documents-and-media.md) | [Markdown readers](components.md#markdown-readers) | `markdown_reader` |
| `Link`, `LinkTarget`, external opening and visited state | [Documents and media](../guide/11-documents-and-media.md) | [Components](components.md) | `standard_library_showcase` |
| `Image`, `ImageSource`, terminal protocols and fallback text | [Documents and media](../guide/11-documents-and-media.md) | [Components](components.md) | `standard_library_showcase` |
| `ProgressBar`, clamping and labels | [Documents and media](../guide/11-documents-and-media.md) | [Components](components.md) | `standard_library_showcase` |
| Custom `View`, measurement, `RenderCtx`, metadata, `ViewType`, geometry | [Extending Leptatui](../guide/13-extending-leptatui.md) | [View contract](../../crates/leptatui/src/view/core/contract.rs) | [Rendering test helpers](../../crates/leptatui/tests/view/support/render.rs) |
| `StyledView`, `TextualView`, `ContainerView`, concrete builders | [Extending Leptatui](../guide/13-extending-leptatui.md) | [View capabilities](../../crates/leptatui/src/view/core/capabilities.rs) | [Text implementation](../../crates/leptatui/src/view/content/text.rs) |
| Custom `AppRoot`, frame rendering and event dispatch | [Extending Leptatui](../guide/13-extending-leptatui.md) | [AppRoot contract](../../crates/leptatui/src/app/root.rs) | [Custom root test](../../crates/leptatui/tests/custom_root_events.rs) |

[Documentation index](../README.md)
