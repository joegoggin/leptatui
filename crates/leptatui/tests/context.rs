//! Context storage tests.
//!
//! These tests cover public context APIs at runtime boundaries.

use leptatui::{
    AppRoot, Component, RenderCtx, Result,
    context::{expect_context, provide_context, use_context},
};
use leptos::prelude::{GetUntracked, Owner, ReadSignal, Set, signal};
use ratatui::{Terminal, backend::TestBackend};

/// Component that records context values observed during rendering.
struct ContextRoot {
    /// String context observed through Leptatui render-scope storage.
    observed_text: Option<String>,
    /// Signal value observed through Leptos owner fallback storage.
    observed_count: Option<i32>,
}

impl Component for ContextRoot {
    /// Provides and reads context values during a render pass.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Rendering context supplied by the app root adapter.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        provide_context(String::from("from component"));
        self.observed_text = use_context::<String>();

        Owner::new().with(|| {
            let (count, set_count) = signal(1);
            leptos::context::provide_context(count);
            set_count.set(2);

            self.observed_count =
                use_context::<ReadSignal<i32>>().map(|count| count.get_untracked());
        });

        Ok(())
    }
}

/// Verifies component render scopes can provide and read typed context.
///
/// # Example Under Test
///
/// ```text
/// AppRoot::render(&mut ContextRoot, frame)
/// provide_context(String::from("from component"))
/// leptos::context::provide_context(ReadSignal<i32>)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The app root render call succeeds.
/// - The component reads the string context from the Leptatui render scope.
/// - The component reads the signal value from Leptos owner context fallback.
///
/// # Why
///
/// Component rendering bridges Leptatui render scopes and Leptos owner scopes.
#[test]
fn component_render_scope_can_provide_and_read_context() -> Result<()> {
    let backend = TestBackend::new(16, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut root = ContextRoot {
        observed_text: None,
        observed_count: None,
    };
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        render_result = AppRoot::render(&mut root, frame);
    })?;
    render_result?;

    assert_eq!(root.observed_text.as_deref(), Some("from component"));
    assert_eq!(root.observed_count, Some(2));

    Ok(())
}

/// Verifies Leptos owner context remains available through Leptatui lookup.
///
/// # Example Under Test
///
/// ```text
/// Owner::new().with(|| {
///     leptos::context::provide_context(String::from("from leptos"));
///     use_context::<String>()
/// })
/// ```
///
/// # Assertions
///
/// - `use_context` finds the Leptos-provided string.
/// - `expect_context` returns the Leptos-provided string.
///
/// # Why
///
/// Leptatui context helpers should not break existing Leptos owner context.
#[test]
fn leptos_owner_context_fallback_still_works() {
    Owner::new().with(|| {
        leptos::context::provide_context(String::from("from leptos"));

        assert_eq!(use_context::<String>().as_deref(), Some("from leptos"));
        assert_eq!(expect_context::<String>(), "from leptos");
    });
}
