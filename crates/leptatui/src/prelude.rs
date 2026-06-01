//! Common imports for Leptatui applications.

// Leptatui public API re-exports belong here as the APIs are implemented:
// - issue #10: App runner and runtime error/result types
// - issue #11: Component, RenderCtx, and basic node builders
// - issue #12: TuiStyle and related style helpers

pub use crate::context::{provide_context, use_context};

pub use leptos::prelude::{
    Callback, Effect, Get, GetUntracked, IntoSignalSetter, Memo, Owner, ReadSignal, RenderEffect,
    RwSignal, Set, Signal, SignalSetter, Update, With, WithUntracked, WriteSignal, signal,
    signal_local, untrack,
};
