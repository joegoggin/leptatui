/// View that records the label visible from its render context.
struct MacroContextConsumer;

impl View for MacroContextConsumer {
    /// Records the context label visible during render.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        *MACRO_CONTEXT_OBSERVED
            .lock()
            .expect("context observation lock should be available") =
            leptatui::context::use_context::<MacroLabel>();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// View that provides context to a descendant component boundary.
#[component]
fn MacroContextProvider() -> impl leptatui::IntoView {
    leptatui::context::provide_context(MacroLabel("macro"));
    component(MacroContextConsumer)
}
