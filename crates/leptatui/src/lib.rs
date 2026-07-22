//! Public runtime crate for Leptatui.
//!
//! Leptatui combines Leptos reactive primitives with Ratatui rendering helpers
//! and a managed Crossterm terminal app loop.
//!
//! # Modules
//!
//! - [`action`] — Signal-backed async mutation state helpers.
//! - [`app`] — Terminal setup, event polling, and app-loop runtime APIs.
//! - [`mod@component`] — Component rendering contracts and frame contexts.
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
//! fn Root() -> View {
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
//! they create Leptatui [`View`] trees and [`Component`] implementations rather
//! than Leptos DOM nodes.
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
//! let document = column([
//!     h1("Guide"),
//!     ordered_list([list_item([
//!         paragraph("Parent item"),
//!         unordered_list([list_item([paragraph("Nested item")])]),
//!     ])])
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
//! ]);
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
//! extensions are deferred. Links retain their inline labels and participate
//! in normal focus traversal. Inside a file-backed Markdown view, local
//! `.md`/`.markdown` links open in the same reader and non-empty fragments
//! scroll to GitHub-style heading anchors. Shift+H and Shift+L move through
//! cached page history. Other local targets and all URLs use the system
//! handler; in-memory Markdown and standalone [`View::Link`] values keep that
//! external behavior. Images become descriptive text without fetching local
//! or remote targets, and raw HTML or unsupported blocks retain readable
//! fallbacks. File readers are infallible and render a path-aware page for
//! unreadable or non-UTF-8 input.
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
pub use component::{
    Children, ChildrenFn, ChildrenMut, Component, KeyControl, RenderCtx, use_key_event,
};
pub use leptatui_macros::{component, stylesheet, view};
pub use markdown::{
    MarkdownOptions, MarkdownView, markdown, markdown_file, markdown_file_with_options,
    markdown_with_options,
};
pub use resource::{Resource, ResourceState, create_resource};
pub use route::{RouteState, provide_route, use_navigate, use_route};
pub use style::{
    BorderType, Borders, Color, LayoutDirection, MediaQuery, Modifier, StyleDeclarations,
    StyleModule, StyleRule, StyleSelector, StyleValue, Stylesheet, ThemeValue, ThemeVariables,
    TuiSize, TuiSpacing, TuiStyle, ViewportSize, theme_color,
};
pub use view::{
    ButtonAction, CellAlignment, EditableState, FormAction, ImageSource, InputAction, LinkTarget,
    RichText, StyleMetadata, SyntaxTheme, View, ViewType, VimMode, block, button, code_block,
    column, component, dynamic, form, h1, h2, h3, h4, h5, h6, image, input, link, list_item,
    ordered_list, paragraph, progress_bar, row, table, table_body, table_cell, table_head,
    table_row, text, text_area, unordered_list,
};

#[doc(hidden)]
/// Hidden implementation details used by generated macro code.
pub mod __private {
    use crate::{StyleMetadata, View};

    pub use crate::component::{
        __register_stylesheet, __with_key_handler_registry, __with_stylesheet_registry,
        FocusedControl, KeyHandlerRegistry, StylesheetRegistry,
    };
    pub use crate::context::hooks::{__with_context_scope, __with_context_scope_if_missing};
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
    /// A [`View`] wrapping the generated component factory.
    pub fn __component_factory<C>(
        preserve_on_reconcile: bool,
        factory: impl FnOnce() -> C + 'static,
    ) -> View
    where
        C: crate::Component + 'static,
    {
        crate::view::component_factory(preserve_on_reconcile, factory)
    }

    /// Reconciles a generated view tree with a previous rendered tree.
    ///
    /// # Arguments
    ///
    /// * `next` — Newly generated view tree to update in place.
    /// * `previous` — Previously rendered view tree used as reconciliation input.
    pub fn __reconcile_view(next: &mut View, previous: &View) {
        if should_preserve_deferred_boundary(next, previous) {
            *next = previous.clone();
            return;
        }

        match (next, previous) {
            (
                View::Markdown { state: next_state },
                View::Markdown {
                    state: previous_state,
                },
            ) if next_state.can_reconcile_from(previous_state) => {
                *next_state = previous_state.clone();
            }
            (
                View::Block {
                    child: next_child, ..
                },
                View::Block {
                    child: previous_child,
                    ..
                },
            ) => __reconcile_view(next_child, previous_child),
            (
                View::Row {
                    children: next_children,
                    metadata: next_metadata,
                },
                View::Row {
                    children: previous_children,
                    metadata: previous_metadata,
                },
            )
            | (
                View::Column {
                    children: next_children,
                    metadata: next_metadata,
                },
                View::Column {
                    children: previous_children,
                    metadata: previous_metadata,
                },
            )
            | (
                View::Form {
                    children: next_children,
                    metadata: next_metadata,
                    ..
                },
                View::Form {
                    children: previous_children,
                    metadata: previous_metadata,
                    ..
                },
            ) => {
                reconcile_scroll_metadata(next_metadata, previous_metadata);
                for (next_child, previous_child) in
                    next_children.iter_mut().zip(previous_children.iter())
                {
                    __reconcile_view(next_child, previous_child);
                }
            }
            (
                View::OrderedList {
                    items: next_items, ..
                },
                View::OrderedList {
                    items: previous_items,
                    ..
                },
            )
            | (
                View::UnorderedList {
                    items: next_items, ..
                },
                View::UnorderedList {
                    items: previous_items,
                    ..
                },
            ) => {
                for (next_item, previous_item) in next_items.iter_mut().zip(previous_items.iter()) {
                    __reconcile_view(next_item, previous_item);
                }
            }
            (
                View::Table {
                    sections: next_children,
                    ..
                },
                View::Table {
                    sections: previous_children,
                    ..
                },
            )
            | (
                View::TableHead {
                    rows: next_children,
                    ..
                },
                View::TableHead {
                    rows: previous_children,
                    ..
                },
            )
            | (
                View::TableBody {
                    rows: next_children,
                    ..
                },
                View::TableBody {
                    rows: previous_children,
                    ..
                },
            )
            | (
                View::TableRow {
                    cells: next_children,
                    ..
                },
                View::TableRow {
                    cells: previous_children,
                    ..
                },
            ) => {
                for (next_child, previous_child) in
                    next_children.iter_mut().zip(previous_children.iter())
                {
                    __reconcile_view(next_child, previous_child);
                }
            }
            (
                View::ListItem {
                    children: next_children,
                    ..
                },
                View::ListItem {
                    children: previous_children,
                    ..
                },
            ) => {
                for (next_child, previous_child) in
                    next_children.iter_mut().zip(previous_children.iter())
                {
                    __reconcile_view(next_child, previous_child);
                }
            }
            (
                View::Button {
                    metadata: next_metadata,
                    ..
                },
                View::Button {
                    metadata: previous_metadata,
                    ..
                },
            ) => reconcile_focus_metadata(next_metadata, previous_metadata),
            (
                View::Link {
                    target: next_target,
                    metadata: next_metadata,
                    ..
                },
                View::Link {
                    target: previous_target,
                    metadata: previous_metadata,
                    ..
                },
            ) if next_target == previous_target => {
                reconcile_focus_metadata(next_metadata, previous_metadata);
            }
            (
                View::H1 {
                    content: next_content,
                    ..
                },
                View::H1 {
                    content: previous_content,
                    ..
                },
            )
            | (
                View::H2 {
                    content: next_content,
                    ..
                },
                View::H2 {
                    content: previous_content,
                    ..
                },
            )
            | (
                View::H3 {
                    content: next_content,
                    ..
                },
                View::H3 {
                    content: previous_content,
                    ..
                },
            )
            | (
                View::H4 {
                    content: next_content,
                    ..
                },
                View::H4 {
                    content: previous_content,
                    ..
                },
            )
            | (
                View::H5 {
                    content: next_content,
                    ..
                },
                View::H5 {
                    content: previous_content,
                    ..
                },
            )
            | (
                View::H6 {
                    content: next_content,
                    ..
                },
                View::H6 {
                    content: previous_content,
                    ..
                },
            )
            | (
                View::Paragraph {
                    content: next_content,
                    ..
                },
                View::Paragraph {
                    content: previous_content,
                    ..
                },
            )
            | (
                View::TableCell {
                    content: next_content,
                    ..
                },
                View::TableCell {
                    content: previous_content,
                    ..
                },
            ) => reconcile_rich_text(next_content, previous_content),
            (
                View::Input {
                    metadata: next_metadata,
                    editable_state: next_editable_state,
                    ..
                },
                View::Input {
                    metadata: previous_metadata,
                    editable_state: previous_editable_state,
                    ..
                },
            ) => {
                reconcile_focus_metadata(next_metadata, previous_metadata);
                *next_editable_state = previous_editable_state.clone();
            }
            (
                View::TextArea {
                    metadata: next_metadata,
                    editable_state: next_editable_state,
                    ..
                },
                View::TextArea {
                    metadata: previous_metadata,
                    editable_state: previous_editable_state,
                    ..
                },
            ) => {
                reconcile_focus_metadata(next_metadata, previous_metadata);
                *next_editable_state = previous_editable_state.clone();
            }
            _ => {}
        }
    }

    /// Copies focus metadata that should survive view reconciliation.
    ///
    /// # Arguments
    ///
    /// * `next_metadata` — Metadata on the newly generated view node.
    /// * `previous_metadata` — Metadata from the previously rendered view node.
    fn reconcile_focus_metadata(
        next_metadata: &mut StyleMetadata,
        previous_metadata: &StyleMetadata,
    ) {
        next_metadata.set_focused(previous_metadata.is_focused());
        if previous_metadata.scroll_into_view_requested() {
            next_metadata.request_scroll_into_view();
        }
    }

    /// Copies focus metadata for matching inline links in semantic rich text.
    ///
    /// # Arguments
    ///
    /// * `next` — Newly generated rich text to update.
    /// * `previous` — Previously rendered rich text used as reconciliation input.
    fn reconcile_rich_text(next: &mut crate::RichText, previous: &crate::RichText) {
        for (next_link, previous_link) in next.links_mut().iter_mut().zip(previous.links()) {
            if next_link.target() == previous_link.target() {
                reconcile_focus_metadata(next_link.metadata_mut(), previous_link.metadata());
            }
        }
    }

    /// Copies layout scroll metadata that should survive view reconciliation.
    ///
    /// # Arguments
    ///
    /// * `next_metadata` — Layout metadata on the newly generated view node.
    /// * `previous_metadata` — Layout metadata from the previous view node.
    fn reconcile_scroll_metadata(next_metadata: &StyleMetadata, previous_metadata: &StyleMetadata) {
        next_metadata.set_max_scroll_offset(previous_metadata.max_scroll_offset());
        next_metadata.set_scroll_offset(previous_metadata.scroll_offset());
    }

    /// Returns whether the previous deferred boundary should be preserved.
    ///
    /// # Arguments
    ///
    /// * `next` — Newly generated view node.
    /// * `previous` — Previously rendered view node.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether reconciliation should keep the previous node.
    fn should_preserve_deferred_boundary(next: &View, previous: &View) -> bool {
        match (next, previous) {
            (View::Component(next), View::Component(previous)) => next.can_reconcile_from(previous),
            (View::Dynamic(next), View::Dynamic(previous)) => next.ptr_eq(previous),
            _ => false,
        }
    }
}
