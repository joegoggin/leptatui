/// Verifies Vim `G` scrolls an overflowing column to the bottom.
///
/// # Example Under Test
///
/// ```text
/// div(Line 0..Line 9)
/// G
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds.
/// - The initial scroll offset is zero.
/// - `G` is handled by the view.
/// - The scroll offset moves to the bottom.
#[test]
fn overflowing_column_scrolls_to_bottom_with_vim_g() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = div(children.collect::<Vec<_>>());

    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 0);
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 5);

    Ok(())
}

/// Verifies Vim control keys page through an overflowing column.
///
/// # Example Under Test
///
/// ```text
/// div(Line 0..Line 9)
/// Ctrl-D, Ctrl-D, Ctrl-U, Ctrl-U
/// ```
///
/// # Assertions
///
/// - `Ctrl-D` scrolls down five rows and is handled.
/// - A second `Ctrl-D` at the bottom leaves the offset clamped and passes.
/// - `Ctrl-U` scrolls up five rows and is handled.
/// - A second `Ctrl-U` at the top leaves the offset clamped and passes.
#[test]
fn overflowing_column_pages_with_vim_control_keys() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = div(children.collect::<Vec<_>>());

    draw_view(&mut terminal, &view)?;

    assert_eq!(
        view.handle_key_event(ctrl_key_event('d'))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 5);
    assert_eq!(view.handle_key_event(ctrl_key_event('d'))?, KeyControl::Pass);
    assert_eq!(scroll_offset(&view), 5);

    assert_eq!(
        view.handle_key_event(ctrl_key_event('u'))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 0);
    assert_eq!(view.handle_key_event(ctrl_key_event('u'))?, KeyControl::Pass);
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}

/// Verifies Vim `gg` scrolls an overflowing column to the top.
///
/// # Example Under Test
///
/// ```text
/// div(Line 0..Line 9)
/// G, g, g
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds.
/// - `G` scrolls to the bottom.
/// - The first `g` keeps the pending top-scroll prefix.
/// - The second `g` scrolls to the top.
#[test]
fn overflowing_column_scrolls_to_top_with_vim_gg() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = div(children.collect::<Vec<_>>());

    draw_view(&mut terminal, &view)?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 5);

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 5);
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}

/// Verifies the Vim `gg` prefix resets after an unrelated key.
///
/// # Example Under Test
///
/// ```text
/// G, g, Down, g, g
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds.
/// - `G` scrolls to the bottom.
/// - `g`, `Down`, `g` leaves the scroll offset at the bottom.
/// - A fresh `g` completes the prefix and scrolls to the top.
#[test]
fn vim_scroll_to_top_prefix_resets_on_unrelated_key() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = div(children.collect::<Vec<_>>());

    draw_view(&mut terminal, &view)?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 5);

    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 5);

    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}
