//! Common imports for Leptatui applications.
//!
//! Application code should normally start with `use leptatui::prelude::*`.
//! This module gathers the runtime, component, view, style, context, routing,
//! async state, keyboard event, terminal key, and Leptos reactive APIs most
//! application code needs.
//!
//! The prelude exports the Leptatui `#[component]`, `view!`, and `stylesheet!`
//! macros. These macros build terminal components and [`crate::View`] trees;
//! they are not Leptos DOM macros.
//!
//! ```no_run
//! use leptatui::prelude::*;
//!
//! #[component]
//! fn Root() -> View {
//!     view! { <Text>"Hello"</Text> }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     App::new(Root::new()).run().await
//! }
//! ```
//!
//! Low-level render metadata and generated-code hooks stay outside the default
//! import.

pub use crate::{
    Action, ActionState, App, AppControl, AppRoot, BorderType, Borders, Children, ChildrenFn,
    ChildrenMut, Color, Component, Error, FormAction, ImageSource, InputAction, KeyControl,
    LayoutDirection, MediaQuery, Modifier, RenderCtx, Resource, ResourceState, Result, RouteState,
    StyleDeclarations, StyleModule, StyleRule, StyleSelector, StyleValue, Stylesheet, ThemeValue,
    ThemeVariables, TuiSpacing, TuiStyle, View, ViewType, ViewportSize, block, button, column,
    context::{expect_context, provide_context, use_context},
    create_action, create_resource, form, image, input, progress_bar, provide_route, row, text,
    text_area, theme_color, use_key_event, use_navigate, use_route,
    view::{component, dynamic},
};

pub use leptatui_macros::{component, stylesheet, view};

pub use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub use leptos::prelude::{
    Callback, Effect, Get, GetUntracked, IntoSignalSetter, Memo, Owner, ReadSignal, RenderEffect,
    RwSignal, Set, Signal, SignalSetter, Update, With, WithUntracked, WriteSignal, signal,
    signal_local, untrack,
};
