//! Common imports for Leptatui applications.
//!
//! This module gathers the runtime, component, view, style, context, and
//! Leptos reactive APIs most application code needs.

// Leptatui public API re-exports belong here as the APIs are implemented:
// - issue #10: App runner and runtime error/result types
// - issue #11: Component, RenderCtx, and basic view builders
// - issue #12: TuiStyle and related style helpers

pub use crate::{
    App, AppControl, AppRoot, BorderType, Borders, ButtonAction, Children, ChildrenFn, ChildrenMut,
    Color, Component, Error, KeyControl, LayoutDirection, MediaQuery, Modifier, RenderCtx, Result,
    StyleDeclarations, StyleMetadata, StyleModule, StyleRule, StyleSelector, StyleValue,
    Stylesheet, ThemeValue, ThemeVariables, TuiSpacing, TuiStyle, View, ViewType, ViewportSize,
    block, button, column,
    context::{expect_context, provide_context, use_context},
    row, text, theme_color, use_key_event,
    view::{component, dynamic},
};

pub use leptatui_macros::{component, stylesheet, view};

pub use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub use leptos::prelude::{
    Callback, Effect, Get, GetUntracked, IntoSignalSetter, Memo, Owner, ReadSignal, RenderEffect,
    RwSignal, Set, Signal, SignalSetter, Update, With, WithUntracked, WriteSignal, signal,
    signal_local, untrack,
};
