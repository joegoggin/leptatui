//! Context storage tests.
//!
//! These tests cover public context APIs at runtime boundaries.

use leptatui::{
    AppRoot, Component, RenderCtx, Result,
    context::{expect_context, provide_context, use_context},
};
use leptos::prelude::{GetUntracked, Owner, ReadSignal, Set, signal};
use ratatui::{Terminal, backend::TestBackend};

struct ContextRoot {
    observed_text: Option<String>,
    observed_count: Option<i32>,
}

impl Component for ContextRoot {
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        provide_context(String::from("from component"));
        self.observed_text = use_context::<String>();

        Owner::new().with(|| {
            let (count, set_count) = signal(1);
            provide_context(count);
            set_count.set(2);

            self.observed_count =
                use_context::<ReadSignal<i32>>().map(|count| count.get_untracked());
        });

        Ok(())
    }
}

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

#[test]
fn leptos_owner_context_fallback_still_works() {
    Owner::new().with(|| {
        leptos::context::provide_context(String::from("from leptos"));

        assert_eq!(use_context::<String>().as_deref(), Some("from leptos"));
        assert_eq!(expect_context::<String>(), "from leptos");
    });
}
