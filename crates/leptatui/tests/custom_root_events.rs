//! Regression tests for custom root dispatch and generated layout reuse.
//!
//! # Modules
//!
//! - [`support`] — Component rendering helpers.

use std::{any::Any, cell::Cell, rc::Rc};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use leptatui::{app::AppRoot, prelude::*};

mod support;

/// Custom root whose event override requests exit and counts calls.
struct CustomRoot {
    /// Number of events delivered to the custom handler.
    events: Rc<Cell<usize>>,
}

impl View for CustomRoot {
    /// Leaves the test frame unchanged.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Unused test rendering context.
    ///
    /// # Returns
    ///
    /// An empty result indicating successful rendering.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> leptatui::Result<()> {
        Ok(())
    }

    /// Counts the supplied event and requests exit.
    ///
    /// # Arguments
    ///
    /// * `_event` — Event delivered through the app root adapter.
    ///
    /// # Returns
    ///
    /// An exit control value.
    fn handle_event(&mut self, _event: Event) -> leptatui::Result<AppControl> {
        self.events.set(self.events.get() + 1);
        Ok(AppControl::Exit)
    }

    /// Returns the root for shared type inspection.
    ///
    /// # Returns
    ///
    /// A shared type-erased reference to this root.
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Returns the root for mutable type inspection.
    ///
    /// # Returns
    ///
    /// A mutable type-erased reference to this root.
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a generated component containing an overflowing built-in view.
///
/// # Returns
///
/// A scrollable view with twenty text rows.
#[component]
fn ScrollingRoot() -> impl IntoView {
    view! { <Div>{text("row\n".repeat(20))}</Div> }
}

/// Verifies app dispatch preserves custom handlers with and without erasure.
///
/// # Example Under Test
///
/// ```text
/// CustomRoot -> AppRoot::__handle_event(Resize)
/// CustomRoot.into_view() -> AppRoot::__handle_event(Resize)
/// ```
///
/// # Assertions
///
/// - Both root adapters invoke the override exactly once and propagate exit.
/// - Custom dispatch conservatively requests fresh layout.
#[test]
fn custom_root_event_overrides_survive_app_dispatch() -> leptatui::Result<()> {
    let events = Rc::new(Cell::new(0));
    let mut root = CustomRoot {
        events: events.clone(),
    };
    assert_eq!(
        AppRoot::__handle_event(&mut root, Event::Resize(80, 24))?,
        (AppControl::Exit, false),
    );
    assert_eq!(events.get(), 1);

    let mut root = CustomRoot {
        events: events.clone(),
    }
    .into_view();
    assert_eq!(
        AppRoot::__handle_event(&mut root, Event::Resize(80, 24))?,
        (AppControl::Exit, false),
    );
    assert_eq!(events.get(), 2);
    Ok(())
}

/// Verifies generated roots retain the built-in scroll dispatch optimization.
///
/// # Example Under Test
///
/// ```text
/// ScrollingRoot, 10x4 terminal, Down
/// ```
///
/// # Assertions
///
/// - The generated component renders successfully.
/// - Scrolling continues the app and permits retained-layout reuse.
#[test]
fn generated_root_scroll_dispatch_permits_layout_reuse() -> leptatui::Result<()> {
    let mut root = ScrollingRoot::new();
    support::render_component(&mut root, 10, 4)?;
    assert_eq!(
        AppRoot::__handle_event(
            &mut root,
            Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        )?,
        (AppControl::Continue, true),
    );
    Ok(())
}
