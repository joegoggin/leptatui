//! Pass fixture for application-defined views in `view!` expressions.

use std::any::Any;

use leptatui::prelude::*;

/// Minimal application-owned render node.
struct Badge;

impl View for Badge {
    /// Renders nothing in this compile-only fixture.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }

    /// Exposes the concrete type for view-tree downcasting.
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Exposes the mutable concrete type for view-tree downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Embeds a custom view beside a built-in view without manual erasure.
fn main() {
    let view = view! {
        <Div>
            {Badge}
            <Text>"Built in"</Text>
        </Div>
    };

    assert!(view.children()[0].is::<Badge>());
    assert!(view.children()[1].is::<TextView>());
}
