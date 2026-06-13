//! Public runtime crate for Leptatui.
//!
//! Leptatui combines Leptos reactive primitives with Ratatui rendering helpers
//! and a managed Crossterm terminal app loop.
//!
//! # Modules
//!
//! - [`app`] — Terminal setup, event polling, and app-loop runtime APIs.
//! - [`mod@component`] — Component rendering contracts and frame contexts.
//! - [`context`] — Typed render-scope context APIs with Leptos owner fallback.
//! - [`view`] — Basic renderable view builders for hand-written terminal UI.
//! - [`prelude`] — Common imports for application code.
//! - [`style`] — Styling and spacing helpers built on Ratatui types.

pub mod app;
pub mod component;
pub mod context;
pub mod prelude;
pub mod style;
pub mod view;

extern crate self as leptatui;

pub use app::{App, AppControl, AppRoot, Error, Result};
pub use component::{
    Children, ChildrenFn, ChildrenMut, Component, KeyControl, RenderCtx, use_key_event,
};
pub use leptatui_macros::{component, stylesheet, view};
pub use style::{
    BorderType, Borders, Color, Modifier, StyleDeclarations, StyleModule, StyleRule, StyleSelector,
    StyleValue, Stylesheet, ThemeValue, ThemeVariables, TuiSpacing, TuiStyle, theme_color,
};
pub use view::{
    ButtonAction, StyleMetadata, View, ViewType, block, button, column, component, dynamic, row,
    text,
};

#[doc(hidden)]
pub mod __private {
    use crate::View;

    pub use crate::component::{
        __register_stylesheet, __with_key_handler_registry, __with_stylesheet_registry,
        KeyHandlerRegistry, StylesheetRegistry,
    };
    pub use crossterm::event::{Event, KeyEvent};

    pub fn __reconcile_view(next: &mut View, previous: &View) {
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
                    ..
                },
                View::Row {
                    children: previous_children,
                    ..
                },
            )
            | (
                View::Column {
                    children: next_children,
                    ..
                },
                View::Column {
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
            ) => next_metadata.set_focused(previous_metadata.is_focused()),
            (next_view @ View::Component(_), View::Component(_)) => {
                *next_view = previous.clone();
            }
            _ => {}
        }
    }
}
