// Concrete-node checks through internal reactive view boundaries.

/// Checks a materialized node without depending on the macro's return type.
///
/// # Arguments
///
/// * `view` — Concrete or reactive fixture root.
/// * `check` — Assertions applied to each matching node.
fn inspect<V: leptatui::View>(view: &dyn leptatui::View, mut check: impl FnMut(&V)) {
    /// Traverses structural boundaries until the requested node is reached.
    ///
    /// # Arguments
    ///
    /// * `view` — Node or structural boundary to inspect.
    /// * `ctx` — Context used to materialize lazy boundaries.
    /// * `check` — Assertions applied to matching nodes.
    /// * `found` — Whether any node matched the requested type.
    fn visit<V: leptatui::View>(
        view: &dyn leptatui::View,
        ctx: &mut leptatui::RenderCtx<'_, '_>,
        check: &mut dyn FnMut(&V),
        found: &mut bool,
    ) {
        if let Some(node) = view.as_any().downcast_ref::<V>() {
            *found = true;
            check(node);
        } else {
            view.__visit_layout_children(ctx, &mut |child, ctx| visit(child.as_view(), ctx, check, found));
        }
    }
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 20)).unwrap();
    let mut found = false;
    terminal.draw(|frame| {
        let mut ctx = leptatui::RenderCtx::new(frame);
        view.render(&mut ctx).unwrap();
        visit(view, &mut ctx, &mut check, &mut found);
    }).unwrap();
    assert!(found, "expected fixture node was not materialized");
}

/// Compares terminal output without depending on internal boundary types.
///
/// # Arguments
///
/// * `actual` — Declarative view under test.
/// * `expected` — Equivalent builder view.
fn assert_render_eq(actual: &dyn leptatui::View, expected: &dyn leptatui::View) {
    /// Renders a fixture to an owned buffer.
    ///
    /// # Arguments
    ///
    /// * `view` — Fixture root to render.
    ///
    /// # Returns
    ///
    /// An owned terminal buffer containing the rendered fixture.
    fn render(view: &dyn leptatui::View) -> ratatui::buffer::Buffer {
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 40)).unwrap();
        terminal.draw(|frame| {
            view.render(&mut leptatui::RenderCtx::new(frame)).unwrap();
        }).unwrap();
        terminal.backend().buffer().clone()
    }
    let actual = render(actual);
    let expected = render(expected);
    for y in 0..40 {
        for x in 0..80 {
            let actual = &actual[(x, y)];
            let expected = &expected[(x, y)];
            assert_eq!(actual.symbol(), expected.symbol(), "rendered symbol at ({x}, {y}) differs");
            assert_eq!(actual.bg, expected.bg, "background at ({x}, {y}) differs");
            if !actual.symbol().trim().is_empty() {
                assert_eq!(actual, expected, "visible cell at ({x}, {y}) differs");
            }
        }
    }
}
