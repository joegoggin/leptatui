//! Regression coverage for navigation through a retained parent outlet.
//!
//! # Modules
//!
//! - [`support`] — Component rendering and key-event helpers.

use std::sync::atomic::{AtomicUsize, Ordering};

use crossterm::event::KeyCode;
use leptatui::prelude::*;

mod support;

/// Counts parent setup calls during the navigation sequence.
static PARENT_SETUPS: AtomicUsize = AtomicUsize::new(0);

/// Creates a router initially matching only its parent route.
///
/// # Returns
///
/// A routed view with navigation controls and a nested profile page.
#[component]
fn OutletRoot() -> impl IntoView {
    view! {
        <Router initial_path="/settings">
            <Div>
                <OutletNav />
                <Routes fallback={|| text("Missing")}>
                    <ParentRoute path="/settings" view=OutletLayout>
                        <Route path="profile" view={|| text("Profile page")} />
                    </ParentRoute>
                </Routes>
            </Div>
        </Router>
    }
}

/// Creates navigation controls outside the matched parent boundary.
///
/// # Returns
///
/// A view handling keys for parent and child navigation.
#[component]
fn OutletNav() -> impl IntoView {
    let navigate = use_navigate();
    use_key_event(KeyEventKind::Press, move |key| {
        match key.code {
            KeyCode::Char('p') => navigate("/settings/profile", NavigateOptions::default()),
            KeyCode::Char('s') => navigate("/settings", NavigateOptions::default()),
            _ => return KeyControl::Pass,
        }
        KeyControl::Handled
    });
    text("Navigation")
}

/// Creates the retained layout around its child outlet.
///
/// # Returns
///
/// A heading followed by the current child page.
#[component]
fn OutletLayout() -> impl IntoView {
    PARENT_SETUPS.fetch_add(1, Ordering::SeqCst);
    view! { <Div><Text>"Settings layout"</Text><Outlet /></Div> }
}

/// Verifies an initially empty outlet responds to later child navigation.
///
/// # Example Under Test
///
/// ```text
/// /settings -> /settings/profile -> /settings -> /settings/profile
/// ```
///
/// # Assertions
///
/// - The initial parent renders without child content.
/// - Each navigation succeeds and shows or clears the child as appropriate.
/// - The parent heading remains visible and its setup runs only once.
#[test]
fn retained_parent_outlet_tracks_child_navigation() -> leptatui::app::Result<()> {
    PARENT_SETUPS.store(0, Ordering::SeqCst);
    let mut component = OutletRoot::new();
    let initial = support::render_component(&mut component, 40, 8)?;
    let text = support::rendered_text(&initial);
    assert!(text.contains("Settings layout"), "rendered: {text:?}");
    assert!(!text.contains("Profile page"), "rendered: {text:?}");

    for (key, child_visible) in [('p', true), ('s', false), ('p', true)] {
        assert_eq!(
            View::handle_event(&mut component, support::key(KeyCode::Char(key)))?,
            AppControl::Continue,
        );
        let rendered = support::render_component(&mut component, 40, 8)?;
        let text = support::rendered_text(&rendered);
        assert!(text.contains("Settings layout"), "rendered: {text:?}");
        assert_eq!(
            text.contains("Profile page"),
            child_visible,
            "rendered: {text:?}"
        );
        assert_eq!(PARENT_SETUPS.load(Ordering::SeqCst), 1);
    }
    Ok(())
}
