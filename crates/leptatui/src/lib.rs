//! Public runtime crate for Leptatui.
//!
//! Leptatui combines Leptos reactive primitives with Ratatui rendering helpers
//! and a managed Crossterm terminal app loop.
//!
//! # Modules
//!
//! - [`action`] — Signal-backed async mutation state helpers.
//! - [`app`] — Terminal setup, event polling, and app-loop runtime APIs.
//! - [`mod@component`] — Component construction support and frame contexts.
//! - [`context`] — Typed render-scope context APIs with Leptos owner fallback.
//! - [`file_system`] — Root-scoped asynchronous filesystem operations.
//! - `executor` — Leptos executor integration for async reactive work.
//! - `markdown` — CommonMark parsing and semantic view conversion.
//! - [`route`] — Signal-backed route state helpers for page switches.
//! - [`resource`] — Signal-backed async resource state helpers.
//! - [`mod@view`] — Basic renderable view builders for hand-written terminal UI.
//! - [`prelude`] — Common imports for application code.
//! - [`style`] — Styling and spacing helpers built on Ratatui types.
//! - `terminal_image` — Terminal graphics detection, caching, and fallbacks.
//!
//! # Public API Shape
//!
//! Application code should normally import [`prelude`] and run a root component
//! with `App::new(view).run().await`. Explicit module or top-level imports remain
//! available for lower-level manual rendering and style inspection.
//!
//! ```no_run
//! use leptatui::prelude::*;
//!
//! #[component]
//! fn Root() -> impl IntoView {
//!     view! { <Text>"Hello from Leptatui"</Text> }
//! }
//!
//! #[tokio::main]
//! async fn main() -> leptatui::app::Result<()> {
//!     let view = view! { <Root /> };
//!     App::new(view).run().await
//! }
//! ```
//!
//! Component setup that can fail returns [`ViewResult<impl IntoView>`]. The
//! [`macro@component`] supports `?` and automatically wraps the final bare
//! view expression. [`view_error!`] returns an intentional custom error, with
//! an optional source-preserving form:
//!
//! ```
//! use leptatui::prelude::*;
//!
//! #[component]
//! fn Fallible() -> ViewResult<impl IntoView> {
//!     let result: std::io::Result<()> = Ok(());
//!     if let Err(error) = result {
//!         view_error!(error => "failed to prepare the view");
//!     }
//!     view! { <Text>"Ready"</Text> }
//! }
//! # let _ = Fallible::new();
//! ```
//!
//! Component bodies own their local signals and can provide shared signals or
//! services through typed context. [`use_app_handle`] returns the scoped
//! [`AppHandle`] when a component needs to queue synchronous work that must run
//! while the managed terminal is temporarily restored.
//!
//! The [`macro@view`] and [`macro@component`] macros are Leptatui terminal UI
//! macros. They use Leptos-style syntax and Leptos reactive primitives, but
//! they create values implementing Leptatui's [`View`] protocol rather than
//! Leptos DOM nodes.
//!
//! # Layout and Styling
//!
//! [`DivView`] is the generic multi-child layout container and defaults to
//! block flow. [`BlockView`] adds bordered single-child chrome while using the
//! same computed layout path. Typed [`TuiStyle`] values and
//! [`macro@stylesheet`] declarations configure block, flexbox, grid, overflow,
//! positioning, stacking, and terminal-relative geometry without exposing
//! layout-engine types.
//!
//! ```
//! use leptatui::prelude::*;
//!
//! let layout = div((
//!     text("Main"),
//!     text("Sidebar"),
//! ))
//! .with_inline_style(
//!     TuiStyle::new()
//!         .display(Display::Flex)
//!         .gap(Axes::all(Length::cells(1.0)))
//!         .size(LayoutSize::new(
//!             Dimension::from(Length::percent(100.0)),
//!             Dimension::Auto,
//!         )),
//! );
//! let _ = layout;
//! ```
//!
//! Visible styleable views retain a [`LayoutGeometry`] snapshot containing
//! border, padding, content, viewport, and accumulated clip rectangles.
//! Layout uses floating-point calculations and rounds sibling geometry
//! cumulatively into terminal cells. See [`style`] for the complete typed
//! vocabulary and current intrinsic-sizing differences.
//!
//! Standard components are available as builders such as [`input`],
//! [`text_area`], [`form`], [`image()`], and [`progress_bar`] and as PascalCase
//! tags in [`macro@view`]. `Input` and `TextArea` are controlled editable views:
//! callers pass the current value and handle proposed updates through
//! `on_input`. Editable controls start in Vim-style normal mode and support
//! insert, normal, and visual editing commands, including submit and cancel
//! integration when they are nested in a `Form`. `Image` uses path-backed
//! [`ImageSource`] values, renders through supported terminal graphics
//! protocols, and falls back to deterministic text when graphics support is
//! unavailable. `ProgressBar` renders a clamped `0.0..=1.0` gauge with an
//! optional label. [`link`] creates a standalone URL or local-path control;
//! the equivalent tag is `<Link href="https://example.com">"label"</Link>`.
//! Links receive focus with Tab or Shift+Tab and activate with Enter or Space.
//! Moving the pointer over an interactive control focuses it, left-clicking a
//! button or link activates it, and vertical or horizontal mouse-wheel events
//! scroll the overflowing layout under the pointer.
//!
//! # Semantic Documents and Markdown
//!
//! Semantic headings are available through [`h1`] through [`h6`], and
//! [`paragraph`] creates unmodified body text. [`code_block`] creates a
//! bordered, width-aware source view with optional syntax highlighting through
//! the terminal's ANSI palette and optional line numbers. [`ordered_list`] and
//! [`unordered_list`] group block-oriented [`list_item`] values with terminal
//! markers. [`table`] groups semantic table sections, rows, and aligned cells
//! into a bordered responsive grid. These semantic views accept
//! owned Ratatui rich text, wrap to the available width, and contribute their
//! wrapped height to parent layouts. Code wraps instead of scrolling
//! horizontally, unknown language tokens retain plain source, and table cells
//! contain inline text rather than nested block views in the v1 API.
//!
//! Builder configuration methods set ordered-list starts, table-cell
//! alignment, and code presentation:
//!
//! ```
//! use leptatui::prelude::*;
//!
//! let document = div((
//!     h1("Guide"),
//!     ordered_list([list_item((
//!         paragraph("Parent item"),
//!         unordered_list([list_item([paragraph("Nested item")])]),
//!     ))])
//!     .start(3),
//!     table([
//!         table_head([table_row([
//!             table_cell("Component"),
//!             table_cell("Status").alignment(CellAlignment::Center),
//!         ])]),
//!         table_body([table_row([
//!             table_cell("CodeBlock"),
//!             table_cell("Ready").alignment(CellAlignment::Right),
//!         ])]),
//!     ]),
//!     code_block("fn main() {}")
//!         .language("rust")
//!         .line_numbers(true),
//! ));
//! # let _ = document;
//! ```
//!
//! The [`macro@view`] and [`macro@stylesheet`] macros expose the same semantic
//! structure and type selectors:
//!
//! ```
//! use leptatui::prelude::*;
//!
//! let _styles = stylesheet! {
//!     H1 => { fg: Color::LightCyan }
//!     OrderedList => { fg: Color::LightGreen }
//!     TableHead => { fg: Color::LightCyan }
//!     CodeBlock => { fg: Color::LightBlue }
//! };
//! let document = view! {
//!     <Div>
//!         <H1>"Guide"</H1>
//!         <OrderedList start=3>
//!             <ListItem>
//!                 <Paragraph>"Parent item"</Paragraph>
//!                 <UnorderedList>
//!                     <ListItem><Paragraph>"Nested item"</Paragraph></ListItem>
//!                 </UnorderedList>
//!             </ListItem>
//!         </OrderedList>
//!         <Table>
//!             <TableHead>
//!                 <TableRow>
//!                     <TableCell>"Component"</TableCell>
//!                     <TableCell alignment={CellAlignment::Center}>"Status"</TableCell>
//!                 </TableRow>
//!             </TableHead>
//!             <TableBody>
//!                 <TableRow>
//!                     <TableCell>"CodeBlock"</TableCell>
//!                     <TableCell alignment={CellAlignment::Right}>"Ready"</TableCell>
//!                 </TableRow>
//!             </TableBody>
//!         </Table>
//!         <CodeBlock language="rust" line_numbers=true>
//!             "fn main() {}"
//!         </CodeBlock>
//!     </Div>
//! };
//! # let _ = document;
//! ```
//!
//! [`markdown()`] and [`markdown_with_options`] convert in-memory CommonMark.
//! [`markdown_file`] and [`markdown_file_with_options`] synchronously load
//! UTF-8 paths before returning a view, and `view!` provides the equivalent
//! path-backed `Markdown` tag:
//!
//! ```
//! use leptatui::prelude::*;
//!
//! let source = "# Guide\n\n```rust\nfn main() {}\n```";
//! let default_document = markdown(source);
//! let configured_document = markdown_with_options(
//!     source,
//!     MarkdownOptions::default().line_numbers(true),
//! );
//! let file_document = markdown_file("README.md");
//! let tagged_document = view! {
//!     <Markdown src="README.md" line_numbers=true />
//! };
//! # let _ = (default_document, configured_document, file_document, tagged_document);
//! ```
//!
//! Markdown compatibility covers CommonMark plus tables. Optional GFM
//! extensions are deferred. Links retain their inline labels and participate
//! in normal focus traversal. Inside a file-backed Markdown view, local
//! `.md`/`.markdown` links open in the same reader and non-empty fragments
//! scroll to GitHub-style heading anchors. Shift+H and Shift+L move through
//! cached page history. Other local targets and all URLs use the system
//! handler; in-memory Markdown and standalone [`LinkView`] values keep that
//! external behavior. Images become descriptive text without fetching local
//! or remote targets, and raw HTML or unsupported blocks retain readable
//! fallbacks. File readers are infallible and render a path-aware page for
//! unreadable or non-UTF-8 input.
//!
//! Shared app state is usually stored with typed context via
//! [`context::provide_context`], [`context::use_context`], and
//! [`context::expect_context`]. Multi-page apps declare URL-like paths with
//! [`Router`], `Routes`, and `Route` tags, read reactive location state with
//! [`use_location`], and navigate with [`use_navigate`]. Asynchronous reads and
//! mutations use [`Resource::new`] and [`Action::new`] to expose loading,
//! pending, and output state. Application errors remain ordinary typed
//! [`std::result::Result`] values returned by resource fetchers or action
//! handlers.
//!
//! # Deferred Scope
//!
//! The first baseline intentionally does not expose a Leptos DOM renderer or
//! raw terminal session customization APIs. Generated-code and runtime wiring
//! hooks live under `__private` and are not supported as user APIs.

mod executor;
mod markdown;

pub mod action;
pub mod app;
pub mod component;
pub mod context;
pub mod file_system;
pub mod prelude;
pub mod resource;
pub mod route;
pub mod style;
pub mod view;

mod terminal_image;

extern crate self as leptatui;

pub use action::Action;
pub use anyhow::Error as ViewError;
pub use app::{App, AppControl, AppHandle, AppRoot, Error, Result, use_app_handle};
pub use component::{Children, ChildrenFn, ChildrenMut, KeyControl, RenderCtx, use_key_event};
pub use executor::{spawn, spawn_local};
pub use leptatui_macros::{component, stylesheet, view};
pub use markdown::{
    MarkdownOptions, MarkdownView, markdown, markdown_file, markdown_file_with_options,
    markdown_source_with_options, markdown_with_options,
};
pub use resource::Resource;
pub use route::{
    History, Location, Navigate, NavigateOptions, Outlet, ParamsMap, RouteViewFactory, Router,
    RouterProps, use_history, use_location, use_navigate, use_params_map, use_query_map,
};
pub use style::{
    AlignContent, AlignItems, AlignSelf, Axes, BorderType, Borders, BoxSizing, Color, Dimension,
    Display, Edges, FlexDirection, FlexWrap, Fraction, GridAutoFlow, GridLine, GridMaxTrackSize,
    GridMinTrackSize, GridPlacement, GridRepeat, GridTemplateTrack, GridTrackSize, JustifyContent,
    JustifyItems, JustifySelf, LayoutSize, Length, LengthAuto, MediaQuery, Modifier, Overflow,
    Position, StyleDeclarations, StyleModule, StyleRule, StyleSelector, StyleValue, Stylesheet,
    ThemeValue, ThemeVariables, TuiSize, TuiSpacing, TuiStyle, ViewportSize, ZIndex, theme_color,
};
pub use view::{
    AnyView, AvailableSpace, BlockView, ButtonAction, ButtonView, CellAlignment, CodeBlockView,
    ContainerView, DivView, DynamicView, EditableAction, EditableState, EditableView, FormAction,
    FormView, HeadingLevel, HeadingView, ImageSource, ImageView, InputView, IntoView, IntoViews,
    LayoutGeometry, LinkTarget, LinkView, ListItemView, ListKind, ListView, ParagraphView,
    ProgressBarView, RichText, RouteLinkView, StyleMetadata, StyledView, TableCellView,
    TableRowView, TableSectionKind, TableSectionView, TableView, TextAreaView, TextView,
    TextualView, View, ViewType, VimMode, block, button, code_block, component, div, dynamic, form,
    h1, h2, h3, h4, h5, h6, image, input, keyed, link, list_item, ordered_list, paragraph,
    progress_bar, route_link, table, table_body, table_cell, table_head, table_row, text,
    text_area, unordered_list,
};

/// Result type returned by fallible component setup functions.
///
/// [`ViewResult`] uses [`ViewError`] so component bodies can propagate any
/// thread-safe standard error with the `?` operator. The [`macro@component`]
/// converts an error into Leptatui's default interactive error screen.
pub type ViewResult<T> = std::result::Result<T, ViewError>;

/// Returns a custom error from a fallible component.
///
/// The one-part form creates a new [`ViewError`] from a formatted message. The
/// source-preserving form converts its first expression into a [`ViewError`]
/// and attaches the formatted message as context.
///
/// ```
/// use leptatui::prelude::*;
///
/// # fn load() -> std::io::Result<()> { Ok(()) }
/// #[component]
/// fn Example() -> ViewResult<impl IntoView> {
///     if let Err(error) = load() {
///         view_error!(error => "failed to load the example");
///     }
///
///     view! { <Text>"Loaded"</Text> }
/// }
/// # let _ = Example::new();
/// ```
#[macro_export]
macro_rules! view_error {
    ($source:expr => $($message:tt)+) => {{
        let __leptatui_error: $crate::ViewError =
            ::core::convert::Into::into($source);
        return ::core::result::Result::Err(
            __leptatui_error.context(::std::format!($($message)+)),
        );
    }};
    ($($message:tt)+) => {{
        return ::core::result::Result::Err(
            $crate::ViewError::msg(::std::format!($($message)+)),
        );
    }};
}

#[doc(hidden)]
/// Hidden implementation details used by generated macro code.
pub mod __private {
    use crate::{AnyView, View};

    pub use crate::component::{
        __register_stylesheet, __with_key_handler_registry, __with_stylesheet_registry,
        FocusedControl, KeyHandlerRegistry, StylesheetRegistry,
    };
    pub use crate::context::hooks::{
        __with_component_setup_context, __with_context_scope, __with_context_scope_if_missing,
    };
    pub use crate::route::{__outlet, __route_definition, __routes};
    pub use crate::view::error::__view_error;
    pub use crossterm::event::{Event, KeyEvent, MouseEvent};

    /// Creates a component view from a generated component factory.
    ///
    /// # Arguments
    ///
    /// * `preserve_on_reconcile` — Whether reconciliation should retain the
    ///   previous component boundary.
    /// * `factory` — Closure that builds the component instance.
    ///
    /// # Returns
    ///
    /// An [`AnyView`] wrapping the generated component factory.
    pub fn __component_factory<C>(
        preserve_on_reconcile: bool,
        factory: impl FnOnce() -> C + 'static,
    ) -> AnyView
    where
        C: crate::View + 'static,
    {
        crate::view::component_factory(preserve_on_reconcile, factory)
    }

    /// Reconciles a generated view tree with a previous rendered tree.
    ///
    /// # Arguments
    ///
    /// * `next` — Newly generated view tree to update in place.
    /// * `previous` — Previously rendered view tree used as reconciliation input.
    pub fn __reconcile_view<N, P>(next: &mut N, previous: &P)
    where
        N: View,
        P: View,
    {
        crate::view::reconcile_views(next, previous);
    }
}
