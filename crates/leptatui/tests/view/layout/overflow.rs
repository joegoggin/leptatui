/// Verifies fitting columns do not render a scrollbar.
///
/// # Example Under Test
///
/// ```text
/// div([text("12345678")])
/// terminal size = 8x1
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rightmost cell remains the final text character.
#[test]
fn fitting_column_does_not_render_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = div([text("12345678")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), "8");

    Ok(())
}

/// Verifies clipped overflow does not create a scroll container.
///
/// # Example Under Test
///
/// ```text
/// div(["12345678", "Two", "Three"])
/// overflow: clip
/// terminal size: 8x2
/// PageDown
/// ```
///
/// # Assertions
///
/// - The first row retains all eight text columns without reserving a scrollbar.
/// - PageDown passes because clipped overflow is not scrollable.
#[test]
fn clipped_overflow_does_not_scroll_or_reserve_a_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = div(vec![text("12345678"), text("Two"), text("Three")])
        .with_inline_style(TuiStyle::new().overflow(Axes::all(Overflow::Clip)));
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), "8");
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Pass
    );
    Ok(())
}

/// Verifies visible overflow paints outside its box without scrolling.
///
/// # Example Under Test
///
/// ```text
/// outer div([
///   8x1 div(["One", "Two"]) with overflow: visible
/// ])
/// terminal size: 8x2
/// PageDown
/// ```
///
/// # Assertions
///
/// - The second child paints below the one-row inner container.
/// - The overflow does not render a scrollbar.
/// - PageDown passes because visible overflow is not scrollable.
#[test]
fn visible_overflow_paints_outside_its_box_without_scrolling() -> Result<()> {
    let inner = div(vec![text("One"), text("Two")]).with_inline_style(
        TuiStyle::new()
            .size(LayoutSize::new(
                Dimension::from(Length::cells(8.0)),
                Dimension::from(Length::cells(1.0)),
            ))
            .overflow(Axes::all(Overflow::Visible)),
    );
    let mut view = div([inner]);
    let mut terminal = Terminal::new(TestBackend::new(8, 2))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 1, 8), "T");
    assert_eq!(cell_symbol(&terminal, 7, 0, 8), " ");
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Pass
    );
    Ok(())
}

/// Verifies hidden overflow scrolls without rendering a scrollbar.
///
/// # Example Under Test
///
/// ```text
/// div(["12345678", "Two", "Three"])
/// overflow: hidden
/// terminal size: 8x2
/// PageDown
/// ```
///
/// # Assertions
///
/// - The first row retains all eight text columns without a scrollbar.
/// - PageDown is handled by the hidden scroll container.
/// - The second row of content moves to the top after scrolling.
#[test]
fn hidden_overflow_scrolls_without_rendering_a_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = div(vec![text("12345678"), text("Two"), text("Three")])
        .with_inline_style(TuiStyle::new().overflow(Axes::all(Overflow::Hidden)));

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 7, 0, 8), "8");

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "T");
    Ok(())
}

/// Verifies nested auto overflow reserves a scrollbar only when needed.
///
/// # Example Under Test
///
/// ```text
/// 8x1 auto div(["12345678"])
/// 8x1 auto div(["12345678", "Two"])
/// terminal size: 8x2
/// ```
///
/// # Assertions
///
/// - Fitting auto content retains all eight text columns.
/// - Overflowing auto content wraps before the reserved scrollbar column.
/// - The overflowing container renders its scrollbar in the eighth column.
#[test]
fn nested_auto_overflow_conditionally_reserves_a_scrollbar() -> Result<()> {
    let fixed_size = LayoutSize::new(
        Dimension::from(Length::cells(8.0)),
        Dimension::from(Length::cells(1.0)),
    );
    let fitting = div([text("12345678")]).with_inline_style(
        TuiStyle::new()
            .size(fixed_size)
            .overflow(Axes::all(Overflow::Auto)),
    );
    let overflowing = div(vec![text("12345678"), text("Two")]).with_inline_style(
        TuiStyle::new()
            .size(fixed_size)
            .overflow(Axes::all(Overflow::Auto)),
    );
    let view = div((fitting, overflowing));
    let mut terminal = Terminal::new(TestBackend::new(8, 2))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), "8");
    assert_eq!(cell_symbol(&terminal, 6, 1, 8), "7");
    assert_eq!(cell_symbol(&terminal, 7, 1, 8), symbol_block::FULL);
    Ok(())
}

/// Verifies scroll overflow always reserves and renders its scrollbar.
///
/// # Example Under Test
///
/// ```text
/// div(["1234567"])
/// overflow: scroll
/// terminal size: 8x1
/// PageDown
/// ```
///
/// # Assertions
///
/// - The fitting text occupies the first seven columns.
/// - The scrollbar occupies the eighth column.
/// - PageDown passes because no scroll offset is available.
#[test]
fn scroll_overflow_always_reserves_and_renders_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let mut view = div([text("1234567")])
        .with_inline_style(TuiStyle::new().overflow(Axes::all(Overflow::Scroll)));
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 6, 0, 8), "7");
    assert_eq!(cell_symbol(&terminal, 7, 0, 8), symbol_block::FULL);
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Pass
    );
    Ok(())
}

/// Verifies overflowing columns render a right-side scrollbar.
///
/// # Example Under Test
///
/// ```text
/// div([text("One"), text("Two"), text("Three")])
/// terminal size = 8x2
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The first scrollbar cell renders as the scroll thumb.
/// - The second scrollbar cell renders as the scrollbar track.
#[test]
fn overflowing_column_renders_right_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = div(vec![text("One"), text("Two"), text("Three")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), symbol_block::FULL);
    assert_eq!(
        cell_symbol(&terminal, 7, 1, 8),
        symbol_line::DOUBLE_VERTICAL
    );

    Ok(())
}

/// Verifies dynamic overflowing columns keep scroll metadata between refreshes.
///
/// # Example Under Test
///
/// ```text
/// dynamic(|| div([text("One"), text("Two"), text("Three")]))
/// terminal size = 8x2
/// ```
///
/// # Assertions
///
/// - Initial rendering measures overflow.
/// - The Down key is handled by the refreshed dynamic child.
/// - Rendering after the key shows the scrolled second row.
#[test]
fn dynamic_overflowing_column_scrolls_after_render() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = dynamic(|| div(vec![text("One"), text("Two"), text("Three")]));

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "O");

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Down))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "T");
    assert_eq!(cell_symbol(&terminal, 1, 0, 8), "w");

    Ok(())
}

/// Verifies overflowing columns reserve width for the scrollbar.
///
/// # Example Under Test
///
/// ```text
/// div([text("123456"), text("more"), text("tail")])
/// terminal size = 6x2
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - Text wraps before the scrollbar column.
/// - The scrollbar thumb occupies the rightmost column.
#[test]
fn overflowing_column_reserves_width_for_scrollbar() -> Result<()> {
    let backend = TestBackend::new(6, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = div(vec![text("123456"), text("more"), text("tail")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 4, 0, 6), "5");
    assert_eq!(cell_symbol(&terminal, 5, 0, 6), symbol_block::FULL);
    assert_eq!(cell_symbol(&terminal, 0, 1, 6), "6");

    Ok(())
}

/// Verifies overflowing columns update the scrollbar thumb after scrolling.
///
/// # Example Under Test
///
/// ```text
/// div([text("One"), text("Two"), text("Three")])
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial terminal draw succeeds.
/// - PageDown is handled by the view.
/// - The second terminal draw succeeds.
/// - The scrollbar thumb moves from the top cell to the bottom cell.
#[test]
fn overflowing_column_updates_scrollbar_position() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = div(vec![text("One"), text("Two"), text("Three")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

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

    assert_eq!(
        cell_symbol(&terminal, 7, 0, 8),
        symbol_line::DOUBLE_VERTICAL
    );
    assert_eq!(cell_symbol(&terminal, 7, 1, 8), symbol_block::FULL);

    Ok(())
}

/// Verifies overflowing column scrollbars reach the bottom at max scroll.
///
/// # Example Under Test
///
/// ```text
/// div(Line 0..Line 9)
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial terminal draw succeeds.
/// - PageDown is handled by the view.
/// - The second terminal draw succeeds.
/// - The scrollbar thumb reaches the bottom row.
#[test]
fn overflowing_column_scrollbar_reaches_bottom_at_max_scroll() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = div(children.collect::<Vec<_>>());
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

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

    assert_eq!(cell_symbol(&terminal, 7, 4, 8), symbol_block::FULL);

    Ok(())
}
