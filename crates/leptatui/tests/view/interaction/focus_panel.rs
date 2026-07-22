/// Component that forwards rendering and built-in focus traversal to a child view.
struct FocusPanel {
    /// Child view owned by this component boundary.
    view: AnyView,
}

impl View for FocusPanel {
    /// Renders the child view.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the view boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        ctx.render_view(&self.view)
    }

    /// Returns the minimum useful render height of the child view.
    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        self.view.__min_height(ctx)
    }

    /// Returns the number of focusable controls inside the child view.
    #[doc(hidden)]
    fn __focusable_count(&self) -> usize {
        self.view.__focusable_count()
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.view.__focused_index_inner(index)
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        self.view.__set_focus_by_index_inner(target, index);
    }

    /// Returns the focused control's vertical span inside the child view.
    #[doc(hidden)]
    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        self.view.__focused_button_span(ctx)
    }

    /// Activates the focused control inside the child view, if any.
    #[doc(hidden)]
    fn __activate_focused_button(&self) -> Result<Option<AppControl>> {
        self.view.__activate_focused_button()
    }

    /// Handles keys for a focused input inside the child view, if any.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to apply to the focused child input.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the key control result when an input handles
    /// the key.
    #[doc(hidden)]
    fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.view.__handle_focused_input_key(key)
    }

    /// Returns the focused built-in control inside the child view.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing focused control metadata when a supported
    /// built-in control is focused.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<leptatui::__private::FocusedControl> {
        self.view.__focused_control()
    }

    /// Handles form-owned submit or cancel keys inside the child view.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to evaluate for nested form behavior.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing key traversal control when a nested form
    /// handles the key.
    #[doc(hidden)]
    fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.view.__handle_form_key(key)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
