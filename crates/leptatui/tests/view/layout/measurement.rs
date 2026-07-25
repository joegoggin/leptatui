/// Verifies flex-row minimum height uses the tallest child.
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
/// - The flex row reports the tallest child's intrinsic height.
#[test]
fn flex_row_min_height_uses_tallest_child() -> Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = div(vec![text("Hello World"), text("Side")])
        .with_inline_style(TuiStyle::new().display(Display::Flex));
    let mut min_height = 0;

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = view.__min_height(&mut ctx);
    })?;

    assert_eq!(min_height, 1);

    Ok(())
}

/// Verifies text-area minimum height counts trailing newline rows.
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
/// - The text area minimum height includes the trailing blank line and border.
#[test]
fn text_area_min_height_counts_trailing_newline() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("Ada\n");
    let mut min_height = 0;

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = view.__min_height(&mut ctx);
    })?;

    assert_eq!(min_height, 4);

    Ok(())
}

/// Verifies component boundaries backed by [`View`] report wrapped view height.
///
/// # Example Under Test
///
/// ```text
/// component(div([text("One"), text("Two"), text("Three")])).__min_height(ctx)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component boundary reports the wrapped column's three-row height.
///
/// # Why
///
/// Parent layouts use component minimum heights to decide whether children need
/// fixed height or overflow scrolling.
#[test]
fn component_view_min_height_uses_wrapped_view_height() -> Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = component(div([text("One"), text("Two"), text("Three")]));
    let mut min_height = 0;

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = view.__min_height(&mut ctx);
    })?;

    assert_eq!(min_height, 3);

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
