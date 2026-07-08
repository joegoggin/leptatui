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
pub use resource::{Resource, ResourceState, create_resource};
pub use route::{RouteState, provide_route, use_navigate, use_route};
pub use style::{
    BorderType, Borders, Color, LayoutDirection, MediaQuery, Modifier, StyleDeclarations,
    StyleModule, StyleRule, StyleSelector, StyleValue, Stylesheet, ThemeValue, ThemeVariables,
    TuiSpacing, TuiStyle, ViewportSize, theme_color,
};
pub use view::{
    ButtonAction, EditableState, FormAction, ImageSource, InputAction, StyleMetadata, View,
    ViewType, VimMode, block, button, column, component, dynamic, form, image, input, row, text,
    text_area,
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
