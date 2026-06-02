//! Common imports for Leptatui applications.
//!
//! This module gathers the runtime, component, node, style, context, and
//! Leptos reactive APIs most application code needs.

// Leptatui public API re-exports belong here as the APIs are implemented:
// - issue #10: App runner and runtime error/result types
// - issue #11: Component, RenderCtx, and basic node builders
// - issue #12: TuiStyle and related style helpers

pub use crate::{
    App, AppControl, AppRoot, BorderType, Borders, Color, Component, Error, Modifier, Node,
    RenderCtx, Result, TuiSpacing, TuiStyle, block, button, column,
    context::{expect_context, provide_context, use_context},
    row, text,
};

pub use leptatui_macros::{component, view};

pub use leptos::prelude::{
    Callback, Effect, Get, GetUntracked, IntoSignalSetter, Memo, Owner, ReadSignal, RenderEffect,
    RwSignal, Set, Signal, SignalSetter, Update, With, WithUntracked, WriteSignal, signal,
    signal_local, untrack,
};
