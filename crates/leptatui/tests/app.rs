//! App runtime contract tests.
//!
//! These tests cover the public app traits and control values without entering
//! a real terminal session.

use std::{cell::Cell, time::Duration};

use crossterm::event::Event;
use leptatui::{App, AppControl, IntoView, RenderCtx, View, button};

/// Test view used to prove view-to-root adaptation.
struct TestRoot {
    /// Number of terminal events observed by the test view.
    events: Cell<usize>,
}

impl View for TestRoot {
    /// Renders the test view as an empty frame.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Rendering context supplied by the app contract.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> leptatui::app::Result<()> {
        Ok(())
    }

    /// Handles one event and requests app exit.
    ///
    /// # Arguments
    ///
    /// * `_event` — Terminal event supplied by the app contract.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value requesting exit.
    fn handle_event(&mut self, _event: Event) -> leptatui::app::Result<AppControl> {
        self.events.set(self.events.get().saturating_add(1));
        Ok(AppControl::Exit)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Verifies an application-defined view satisfies the app root contract.
///
/// # Example Under Test
///
/// ```text
/// App::new(TestRoot { events: 0 }).with_redraw_interval(50ms)
/// ```
///
/// # Assertions
///
/// - `TestRoot` type-checks as an [`AppRoot`].
/// - An [`App`] can be constructed with a non-default redraw interval.
#[test]
fn app_accepts_custom_view_contract() {
    /// Accepts any root type that converts into a view.
    ///
    /// # Arguments
    ///
    /// * `root` — Root value to pass into the app constructor.
    fn assert_app_root<R: IntoView>(root: R) {
        let _app = App::new(root).with_redraw_interval(Duration::from_millis(50));
    }

    assert_app_root(TestRoot {
        events: Cell::new(0),
    });
}

/// Verifies app control values implement equality.
///
/// # Example Under Test
///
/// ```text
/// AppControl::Continue
/// AppControl::Exit
/// ```
///
/// # Assertions
///
/// - `Continue` compares equal to itself.
/// - `Continue` compares unequal to `Exit`.
#[test]
fn app_control_is_comparable() {
    assert_eq!(AppControl::Continue, AppControl::Continue);
    assert_ne!(AppControl::Continue, AppControl::Exit);
}

/// Verifies view roots with button actions satisfy the app root contract.
///
/// # Example Under Test
///
/// ```text
/// button("Quit").on_press(|| AppControl::Exit)
/// ```
///
/// # Assertions
///
/// - A button view with an action type-checks as an [`AppRoot`].
/// - An [`App`] can be constructed with the view root.
///
/// # Why
///
/// Button action callbacks should not prevent view trees from being used as
/// app roots.
#[test]
fn app_accepts_view_root_with_button_action() {
    /// Accepts any root type that converts into a view.
    ///
    /// # Arguments
    ///
    /// * `root` — Root value to pass into the app constructor.
    fn assert_app_root<R: IntoView>(root: R) {
        let _app = App::new(root);
    }

    assert_app_root(button("Quit").on_press(|| AppControl::Exit));
}
