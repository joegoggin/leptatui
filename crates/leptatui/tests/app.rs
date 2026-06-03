//! App runtime contract tests.
//!
//! These tests cover the public app traits and control values without entering
//! a real terminal session.

use std::time::Duration;

use crossterm::event::Event;
use leptatui::{
    App, AppControl, AppRoot, Color, Component, RenderCtx, Result, StyleSelector, Stylesheet,
    TuiStyle, button,
};

/// Test component used to prove component-to-root adaptation.
struct TestRoot {
    /// Number of terminal events observed by the test component.
    events: usize,
}

impl Component for TestRoot {
    /// Renders the test component as an empty frame.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Rendering context supplied by the app contract.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
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
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        self.events += 1;
        Ok(AppControl::Exit)
    }
}

/// Verifies a component satisfies the app root contract.
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
fn app_accepts_component_contract() {
    /// Accepts any root type that implements [`AppRoot`].
    ///
    /// # Arguments
    ///
    /// * `root` — Root value to pass into the app constructor.
    fn assert_app_root<R: AppRoot>(root: R) {
        let _app = App::new(root)
            .with_redraw_interval(Duration::from_millis(50))
            .with_stylesheet(Stylesheet::new().rule(
                StyleSelector::class("root"),
                TuiStyle::new().foreground(Color::White),
            ));
    }

    assert_app_root(TestRoot { events: 0 });
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

/// Verifies node roots with button actions satisfy the app root contract.
///
/// # Example Under Test
///
/// ```text
/// button("Quit").on_press(|| AppControl::Exit)
/// ```
///
/// # Assertions
///
/// - A button node with an action type-checks as an [`AppRoot`].
/// - An [`App`] can be constructed with the node root.
///
/// # Why
///
/// Button action callbacks should not prevent node trees from being used as
/// app roots.
#[test]
fn app_accepts_node_root_with_button_action() {
    /// Accepts any root type that implements [`AppRoot`].
    ///
    /// # Arguments
    ///
    /// * `root` — Root value to pass into the app constructor.
    fn assert_app_root<R: AppRoot>(root: R) {
        let _app = App::new(root);
    }

    assert_app_root(button("Quit").on_press(|| AppControl::Exit));
}
