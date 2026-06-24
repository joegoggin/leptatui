//! Common imports for Leptatui applications.
//!
//! This module gathers the runtime, component, view, style, context, and
//! Leptos reactive APIs most application code needs.
//! Low-level render metadata, callback aliases, and generated-code hooks stay
//! outside the default import.

pub use crate::{
    Action, ActionState, App, AppControl, AppRoot, BorderType, Borders, Children, ChildrenFn,
    ChildrenMut, Color, Component, Error, KeyControl, LayoutDirection, MediaQuery, Modifier,
    RenderCtx, Resource, ResourceState, Result, RouteState, StyleDeclarations, StyleModule,
    StyleRule, StyleSelector, StyleValue, Stylesheet, ThemeValue, ThemeVariables, TuiSpacing,
    TuiStyle, View, ViewType, ViewportSize, block, button, column,
    context::{expect_context, provide_context, use_context},
    create_action, create_resource, provide_route, row, text, theme_color, use_key_event,
    use_navigate, use_route,
    view::{component, dynamic},
};

pub use leptatui_macros::{component, stylesheet, view};

pub use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub use leptos::prelude::{
    Callback, Effect, Get, GetUntracked, IntoSignalSetter, Memo, Owner, ReadSignal, RenderEffect,
    RwSignal, Set, Signal, SignalSetter, Update, With, WithUntracked, WriteSignal, signal,
    signal_local, untrack,
};
