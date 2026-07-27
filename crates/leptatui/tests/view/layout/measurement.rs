/// Verifies flex-row intrinsic size uses the tallest child.
///
/// # Example Under Test
///
/// ```text
/// div([text("Hello World"), text("Side")])
/// terminal width = 12
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The flex row reports the available width and tallest child height.
#[test]
fn flex_row_intrinsic_size_uses_tallest_child() -> Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = div(vec![text("Hello World"), text("Side")])
        .with_inline_style(TuiStyle::new().display(Display::Flex));
    let mut measured = LayoutSize::all(0.0);

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        measured = measure_view_in_area(&view, &mut ctx);
    })?;

    assert_eq!(measured, LayoutSize::new(12.0, 1.0));

    Ok(())
}

/// Verifies text-area intrinsic size counts trailing newline rows.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\n")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The text area size includes the available width, trailing blank line, and border.
#[test]
fn text_area_intrinsic_size_counts_trailing_newline() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("Ada\n");
    let mut measured = LayoutSize::all(0.0);

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        measured = measure_view_in_area(&view, &mut ctx);
    })?;

    assert_eq!(measured, LayoutSize::new(8.0, 4.0));

    Ok(())
}

/// Verifies component boundaries report the wrapped view's two-axis size.
///
/// # Example Under Test
///
/// ```text
/// component(div([text("One"), text("Two"), text("Three")])).measure(...)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component boundary reports the available width and wrapped three-row height.
///
/// # Why
///
/// Parent layouts use component measurements to resolve child geometry and
/// overflow.
#[test]
fn component_view_intrinsic_size_uses_wrapped_view_size() -> Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = component(div([text("One"), text("Two"), text("Three")]));
    let mut measured = LayoutSize::all(0.0);

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        measured = measure_view_in_area(view.as_view(), &mut ctx);
    })?;

    assert_eq!(measured, LayoutSize::new(12.0, 3.0));

    Ok(())
}

/// Verifies overflowing columns scroll rows produced by wrapped text.
///
/// # Example Under Test
///
/// ```text
/// div([text("Hello World"), text("Bottom")])
/// terminal size = 7x2
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial render succeeds and hides the later child.
/// - PageDown is handled by the view.
/// - The second render succeeds.
/// - The wrapped text row and later child become visible after scrolling.
#[test]
fn overflowing_column_scrolls_wrapped_text_rows() -> Result<()> {
    let backend = TestBackend::new(7, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = div(vec![text("Hello World"), text("Bottom")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "B", 7).is_none());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "W", 7).1, 0);
    assert_eq!(symbol_position(&terminal, "B", 7).1, 1);

    Ok(())
}
