//! Signal-backed route state helpers.
//!
//! These helpers keep route state in typed context so components can read the
//! active route and navigate without requiring a global router.

use leptos::prelude::{ReadSignal, WriteSignal, signal};

use crate::context;

/// Signal-backed route state shared through Leptatui context.
pub struct RouteState<R> {
    route: ReadSignal<R>,
    navigate: WriteSignal<R>,
}

impl<R> Clone for RouteState<R> {
    /// Clones the route signal handles.
    fn clone(&self) -> Self {
        Self {
            route: self.route.clone(),
            navigate: self.navigate.clone(),
        }
    }
}

impl<R> RouteState<R> {
    /// Creates route state from existing read and write signals.
    pub const fn new(route: ReadSignal<R>, navigate: WriteSignal<R>) -> Self {
        Self { route, navigate }
    }

    /// Returns the read signal for the active route.
    pub fn route(&self) -> ReadSignal<R> {
        self.route.clone()
    }

    /// Returns the write signal used to navigate to another route.
    pub fn navigate(&self) -> WriteSignal<R> {
        self.navigate.clone()
    }
}

/// Creates route state and provides it to descendant components.
///
/// # Arguments
///
/// * `initial` — Initial route value for the app or subtree.
///
/// # Returns
///
/// A [`RouteState`] containing read and write signals for the route.
pub fn provide_route<R>(initial: R) -> RouteState<R>
where
    R: Clone + Send + Sync + 'static,
{
    let (route, navigate) = signal(initial);
    let state = RouteState::new(route, navigate);

    context::provide_context(state.clone());

    state
}

/// Returns the active route read signal from context.
///
/// # Returns
///
/// A [`ReadSignal`] for the nearest provided route state of type `R`.
///
/// # Panics
///
/// Panics if no matching [`RouteState<R>`] exists in context.
pub fn use_route<R>() -> ReadSignal<R>
where
    R: Clone + 'static,
{
    context::expect_context::<RouteState<R>>().route()
}

/// Returns the route navigation write signal from context.
///
/// # Returns
///
/// A [`WriteSignal`] for the nearest provided route state of type `R`.
///
/// # Panics
///
/// Panics if no matching [`RouteState<R>`] exists in context.
pub fn use_navigate<R>() -> WriteSignal<R>
where
    R: Clone + 'static,
{
    context::expect_context::<RouteState<R>>().navigate()
}
