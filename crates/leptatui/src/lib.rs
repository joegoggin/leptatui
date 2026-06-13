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
//! - [`node`] — Basic renderable node builders for hand-written terminal UI.
//! - [`prelude`] — Common imports for application code.
//! - [`style`] — Styling and spacing helpers built on Ratatui types.

pub mod app;
pub mod component;
pub mod context;
pub mod node;
pub mod prelude;
pub mod style;

extern crate self as leptatui;

pub use app::{App, AppControl, AppRoot, Error, Result};
pub use component::{Component, KeyControl, RenderCtx, use_key_event};
pub use leptatui_macros::{component, stylesheet, view};
pub use node::{
    ButtonAction, Node, NodeType, StyleMetadata, block, button, column, component, dynamic, row,
    text,
};
pub use style::{
    BorderType, Borders, Color, Modifier, StyleDeclarations, StyleRule, StyleSelector, Stylesheet,
    ThemeValue, ThemeVariables, TuiSpacing, TuiStyle, theme_color,
};

#[doc(hidden)]
pub mod __private {
    use crate::Node;

    pub use crate::component::{
        __register_stylesheet, __with_key_handler_registry, __with_stylesheet_registry,
        KeyHandlerRegistry, StylesheetRegistry,
    };
    pub use crossterm::event::{Event, KeyEvent};

    pub fn __reconcile_node(next: &mut Node, previous: &Node) {
        match (next, previous) {
            (
                Node::Block {
                    child: next_child, ..
                },
                Node::Block {
                    child: previous_child,
                    ..
                },
            ) => __reconcile_node(next_child, previous_child),
            (
                Node::Row {
                    children: next_children,
                    ..
                },
                Node::Row {
                    children: previous_children,
                    ..
                },
            )
            | (
                Node::Column {
                    children: next_children,
                    ..
                },
                Node::Column {
                    children: previous_children,
                    ..
                },
            ) => {
                for (next_child, previous_child) in
                    next_children.iter_mut().zip(previous_children.iter())
                {
                    __reconcile_node(next_child, previous_child);
                }
            }
            (
                Node::Button {
                    metadata: next_metadata,
                    ..
                },
                Node::Button {
                    metadata: previous_metadata,
                    ..
                },
            ) => next_metadata.set_focused(previous_metadata.is_focused()),
            (next_node @ Node::Component(_), Node::Component(_)) => {
                *next_node = previous.clone();
            }
            _ => {}
        }
    }
}
