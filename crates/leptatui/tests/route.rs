//! Route state helper tests.
//!
//! These tests cover the public context-backed route helper API.

use leptatui::{context, provide_route, use_navigate, use_route};
use leptos::prelude::{GetUntracked, Owner, Update};

/// Route values used by route helper tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TestRoute {
    Home,
    Settings,
}

/// Verifies route helpers provide readable and writable route state.
///
/// # Example Under Test
///
/// ```text
/// provide_route(Home)
/// use_route::<TestRoute>()
/// use_navigate::<TestRoute>()
/// ```
///
/// # Assertions
///
/// - The provided route can be read from context.
/// - The navigation setter updates the active route.
/// - The returned route state observes the same updated signal.
#[test]
fn route_helpers_provide_and_update_route_state() {
    Owner::new().with(|| {
        context::__with_context_scope(|| {
            let state = provide_route(TestRoute::Home);
            let route = use_route::<TestRoute>();
            let navigate = use_navigate::<TestRoute>();

            assert_eq!(route.get_untracked(), TestRoute::Home);

            navigate.update(|route| *route = TestRoute::Settings);

            assert_eq!(route.get_untracked(), TestRoute::Settings);
            assert_eq!(state.route().get_untracked(), TestRoute::Settings);
        });
    });
}
