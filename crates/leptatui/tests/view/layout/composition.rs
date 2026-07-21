/// Verifies overflowing columns can scroll to later children by default.
///
/// # Example Under Test
///
/// ```text
/// column([text rows..., row([button("Launch"), button("Quit")]).focus-actions])
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial styled render succeeds and hides the later button.
/// - PageDown is handled by the view.
/// - The second styled render succeeds and shows the later button.
#[test]
fn overflowing_column_scrolls_to_later_children_by_default() -> Result<()> {
    let backend = TestBackend::new(12, 6);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column((
        text("One"),
        text("Two"),
        text("Three"),
        text("Four"),
        row(vec![button("Launch"), button("Quit")]).with_classes("focus-actions"),
    ));
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("focus-actions"),
        TuiStyle::new().direction(LayoutDirection::Column),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "Q", 12).is_none());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "Q", 12).1, 4);

    Ok(())
}

/// Verifies page scrolling handles stacked buttons without nested scrolling.
///
/// # Example Under Test
///
/// ```text
/// column([block(text("Top")), row(buttons).stack, text("End")])
/// PageDown, PageDown
/// ```
///
/// # Assertions
///
/// - The first render shows the top block and hides the last button.
/// - The first PageDown scrolls the parent page and reveals middle buttons.
/// - The second PageDown keeps the top block hidden and reveals the last button.
///
/// # Why
///
/// Parent overflow should manage page scrolling when nested stacked content is
/// taller than the viewport.
#[test]
fn overflowing_page_scrolls_stacked_buttons_without_nested_scroll() -> Result<()> {
    let backend = TestBackend::new(12, 6);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column((
        block(text("Top")),
        row(vec![button("A"), button("B"), button("C"), button("D")]).with_classes("stack"),
        text("End"),
    ));
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("stack"),
        TuiStyle::new().direction(LayoutDirection::Column),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "T", 12).is_some());
    assert!(symbol_position_opt(&terminal, "D", 12).is_none());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "T", 12).is_none());
    assert!(symbol_position_opt(&terminal, "C", 12).is_some());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "T", 12).is_none());
    assert!(symbol_position_opt(&terminal, "D", 12).is_some());

    Ok(())
}

/// Verifies row layout stays horizontal without a direction override.
///
/// # Example Under Test
///
/// ```text
/// row([text("A"), text("B")])
/// terminal width = 4
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The child text views render on the same row in separate columns.
#[test]
fn row_layout_stays_horizontal_without_direction_override() -> Result<()> {
    let backend = TestBackend::new(4, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = row(vec![text("A"), text("B")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "A", 4), (0, 0));
    assert_eq!(symbol_position(&terminal, "B", 4), (2, 0));

    Ok(())
}
