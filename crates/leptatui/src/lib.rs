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
//! - [`route`] — Signal-backed route state helpers for page switches.
//! - [`resource`] — Signal-backed async resource state helpers.
//! - [`mod@view`] — Basic renderable view builders for hand-written terminal UI.
//! - [`prelude`] — Common imports for application code.
//! - [`style`] — Styling and spacing helpers built on Ratatui types.
//!
//! # Public API Shape
//!
//! Application code should normally import [`prelude`] and run a root component
//! with `App::new(root).run().await`. Explicit module or top-level imports remain
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
//! async fn main() -> Result<()> {
//!     App::new(Root::new()).run().await
//! }
//! ```
//!
//! The [`macro@view`] and [`macro@component`] macros are Leptatui terminal UI
//! macros. They use Leptos-style syntax and Leptos reactive primitives, but
//! they create values implementing Leptatui's [`View`] protocol rather than
//! Leptos DOM nodes.
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
//! optional label.
//!
//! # Semantic Documents and Markdown
//!
//! Semantic headings are available through [`h1`] through [`h6`], and
//! [`paragraph`] creates unmodified body text. [`code_block`] creates a
//! bordered, width-aware source view with optional bundled syntax highlighting
//! and line numbers. [`ordered_list`] and
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
//! let document = column((
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
//!         .syntax_theme(SyntaxTheme::Dark)
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
//!     <Column>
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
//!         <CodeBlock
//!             language="rust"
//!             syntax_theme={SyntaxTheme::Dark}
//!             line_numbers=true
//!         >"fn main() {}"</CodeBlock>
//!     </Column>
//! };
//! # let _ = document;
//! ```
//!
//! [`markdown`] and [`markdown_with_options`] convert in-memory CommonMark.
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
//!     MarkdownOptions::default()
//!         .syntax_theme(SyntaxTheme::Light)
//!         .line_numbers(true),
//! );
//! let file_document = markdown_file("README.md");
//! let tagged_document = view! {
//!     <Markdown src="README.md" syntax_theme={SyntaxTheme::Dark} line_numbers=true />
//! };
//! # let _ = (default_document, configured_document, file_document, tagged_document);
//! ```
//!
//! Markdown compatibility covers CommonMark plus tables. Optional GFM
//! extensions are deferred. Links are readable but non-interactive; images
//! become descriptive text without fetching local or remote targets; and raw
//! HTML or unsupported blocks retain readable fallbacks. File readers are
//! infallible and render a path-aware paragraph for unreadable or non-UTF-8
//! input.
//!
//! Shared app state is usually stored with typed context via
//! [`context::provide_context`], [`context::use_context`], and
//! [`context::expect_context`]. Multi-page apps can store the active page with
//! [`provide_route`], read it with [`use_route`], and navigate with
//! [`use_navigate`]. Asynchronous reads and mutations use [`create_resource`]
//! and [`create_action`] to expose pending, ready, and error state to
//! components.
//!
//! # Deferred Scope
//!
//! The first baseline intentionally does not expose a Leptos DOM renderer, a
//! generalized router, or raw terminal session customization APIs. Generated-code
//! and runtime wiring hooks live under `__private` and are not supported as user
//! APIs.

mod executor;
mod markdown;

pub mod action;
pub mod app;
pub mod component;
pub mod context;
pub mod prelude;
pub mod resource;
pub mod route;
pub mod style;
pub mod view;

mod terminal_image;

extern crate self as leptatui;

pub use action::{Action, ActionState, create_action};
pub use app::{App, AppControl, AppRoot, Error, Result};
pub use component::{Children, ChildrenFn, ChildrenMut, KeyControl, RenderCtx, use_key_event};
pub use leptatui_macros::{component, stylesheet, view};
pub use markdown::{
    MarkdownOptions, markdown, markdown_file, markdown_file_with_options, markdown_with_options,
};
pub use resource::{Resource, ResourceState, create_resource};
pub use route::{RouteState, provide_route, use_navigate, use_route};
pub use style::{
    BorderType, Borders, Color, LayoutDirection, MediaQuery, Modifier, StyleDeclarations,
    StyleModule, StyleRule, StyleSelector, StyleValue, Stylesheet, ThemeValue, ThemeVariables,
    TuiSize, TuiSpacing, TuiStyle, ViewportSize, theme_color,
};
pub use view::{
    AnyView, BlockView, ButtonAction, ButtonView, CellAlignment, CodeBlockView, ContainerView,
    DynamicView, EditableKind, EditableState, EditableTextView, EditableView, FormAction, FormView,
    HeadingLevel, HeadingView, ImageSource, ImageView, InputAction, IntoView, IntoViews,
    LayoutView, ListItemView, ListKind, ListView, ParagraphView, ProgressBarView, StyleMetadata,
    StyledView, SyntaxTheme, TableCellView, TableRowView, TableSectionKind, TableSectionView,
    TableView, TextView, TextualView, View, ViewType, VimMode, block, button, code_block, column,
    component, dynamic, form, h1, h2, h3, h4, h5, h6, image, input, list_item, ordered_list,
    paragraph, progress_bar, row, table, table_body, table_cell, table_head, table_row, text,
    text_area, unordered_list,
};

#[doc(hidden)]
/// Hidden implementation details used by generated macro code.
pub mod __private {
    use crate::{AnyView, View};

    pub use crate::component::{
        __register_stylesheet, __with_key_handler_registry, __with_stylesheet_registry,
        FocusedControl, KeyHandlerRegistry, StylesheetRegistry,
    };
    pub use crate::context::hooks::{__with_context_scope, __with_context_scope_if_missing};
    pub use crossterm::event::{Event, KeyEvent};

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
