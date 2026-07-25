/// Verifies parent backgrounds fill the bottom row after scrolling down.
///
/// # Example Under Test
///
/// ```text
/// div([text("Top"), button("Launch"), text("Tail")]).surface
/// Down
/// ```
///
/// # Assertions
///
/// - The initial styled render succeeds.
/// - The down key is handled by the view.
/// - The second styled render succeeds.
/// - Empty and occupied cells on the bottom row keep the parent background.
#[test]
fn overflowing_column_keeps_parent_background_on_bottom_row_after_scrolling_down() -> Result<()> {
    let backend = TestBackend::new(12, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = div((text("Top"), button("Launch"), text("Tail"))).with_classes("surface");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("surface"),
        TuiStyle::new().background(Color::Blue),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(cell_colors(&terminal, 0, 1, 12).1, Color::Blue);
    assert_eq!(cell_colors(&terminal, 5, 1, 12).1, Color::Blue);

    Ok(())
}

/// Verifies parent backgrounds fill the top row after scrolling up.
///
/// # Example Under Test
///
/// ```text
/// div([text("Top"), button("Launch"), text("Tail")]).surface
/// PageDown, Up
/// ```
///
/// # Assertions
///
/// - The initial styled render succeeds.
/// - PageDown and Up are handled by the view.
/// - The second styled render succeeds.
/// - Empty and occupied cells on the top row keep the parent background.
#[test]
fn overflowing_column_keeps_parent_background_on_top_row_after_scrolling_up() -> Result<()> {
    let backend = TestBackend::new(12, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = div((text("Top"), button("Launch"), text("Tail"))).with_classes("surface");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("surface"),
        TuiStyle::new().background(Color::Blue),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(cell_colors(&terminal, 0, 0, 12).1, Color::Blue);
    assert_eq!(cell_colors(&terminal, 5, 0, 12).1, Color::Blue);

    Ok(())
}
