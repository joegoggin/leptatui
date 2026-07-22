/// Verifies tab navigation moves between static buttons.
///
/// # Example Under Test
///
/// ```text
/// column([button("One"), text("Gap"), button("Two")])
/// Tab, Tab, BackTab
/// ```
///
/// # Assertions
///
/// - The first tab event succeeds and focuses the first button.
/// - The second tab event succeeds and focuses the second button.
/// - The back-tab event succeeds and returns focus to the first button.
///
/// # Why
///
/// Non-focusable text views should be skipped during keyboard focus movement.
#[test]
fn tab_focus_moves_between_static_buttons() -> Result<()> {
    let mut view = column((button("One"), text("Gap"), button("Two")));

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(button_focuses(&view), vec![true, false]);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(button_focuses(&view), vec![false, true]);

    view.handle_event(key(KeyCode::BackTab))?;
    assert_eq!(button_focuses(&view), vec![true, false]);

    Ok(())
}

/// Verifies tab navigation includes editable controls.
///
/// # Example Under Test
///
/// ```text
/// column([button("Save"), text("Gap"), Input, TextArea, button("Submit")])
/// Tab x4, BackTab
/// ```
///
/// # Assertions
///
/// - The view reports four focusable controls.
/// - Each tab event succeeds and moves focus in render order.
/// - The back-tab event succeeds and moves focus back one control.
/// - Non-editable text is skipped.
#[test]
fn tab_focus_moves_across_buttons_and_editable_controls() -> Result<()> {
    let mut view = column((
        button("Save"),
        text("Gap"),
        editable_input("Ada"),
        editable_text_area("Notes"),
        button("Submit"),
    ));

    assert_eq!(view.__focusable_count(), 4);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![true, false, false, false]);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![false, true, false, false]);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![false, false, true, false]);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![false, false, false, true]);

    view.handle_event(key(KeyCode::BackTab))?;
    assert_eq!(control_focuses(&view), vec![false, false, true, false]);

    Ok(())
}

/// Verifies tab focus scrolls an overflowing column to the focused button.
///
/// # Example Under Test
///
/// ```text
/// column([button("A1"), button("B2"), button("C3")])
/// height = 4
/// Tab, Tab, render
/// ```
///
/// # Assertions
///
/// - The second button receives focus.
/// - Rendering scrolls the column by the minimum amount needed.
/// - The focused button label is visible in the terminal buffer.
///
/// # Why
///
/// Keyboard focus should not move to an offscreen button without bringing that
/// button into view.
#[test]
fn tab_focus_scrolls_overflowing_column_to_focused_button() -> Result<()> {
    let width = 18;
    let backend = TestBackend::new(width, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column([button("A1"), button("B2"), button("C3")]);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(button_focuses(&view), vec![false, true, false]);
    assert_eq!(scroll_offset(&view), 2);
    assert!(symbol_position_opt(&terminal, "B", width).is_some());

    Ok(())
}

/// Verifies back-tab focus scrolls upward to a previously offscreen button.
///
/// # Example Under Test
///
/// ```text
/// column([button("A1"), button("B2"), button("C3")])
/// height = 4
/// Tab x3, render, BackTab, render, BackTab, render
/// ```
///
/// # Assertions
///
/// - Forward tabbing scrolls down to the third button.
/// - Back-tab to the second button scrolls just enough to reveal it.
/// - Back-tab to the first button returns to the top.
///
/// # Why
///
/// Reverse focus movement should use the same focus visibility rule as forward
/// movement.
#[test]
fn backtab_focus_scrolls_overflowing_column_up_to_focused_button() -> Result<()> {
    let width = 18;
    let backend = TestBackend::new(width, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column([button("A1"), button("B2"), button("C3")]);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;
    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 5);
    assert!(symbol_position_opt(&terminal, "C", width).is_some());

    view.handle_event(key(KeyCode::BackTab))?;
    draw_view(&mut terminal, &view)?;

    assert_eq!(button_focuses(&view), vec![false, true, false]);
    assert_eq!(scroll_offset(&view), 3);
    assert!(symbol_position_opt(&terminal, "B", width).is_some());

    view.handle_event(key(KeyCode::BackTab))?;
    draw_view(&mut terminal, &view)?;

    assert_eq!(button_focuses(&view), vec![true, false, false]);
    assert_eq!(scroll_offset(&view), 0);
    assert!(symbol_position_opt(&terminal, "A", width).is_some());

    Ok(())
}

/// Verifies focus scrolling does not pin later manual scroll movement.
///
/// # Example Under Test
///
/// ```text
/// column([button("A1"), button("B2"), button("C3")])
/// Tab, Tab, render, PageDown, render
/// ```
///
/// # Assertions
///
/// - Focus scrolling first reveals the second button.
/// - A later page-down scroll is preserved after rendering.
///
/// # Why
///
/// Automatic focus visibility should be a response to focus movement, not a
/// permanent constraint that prevents normal scrolling.
#[test]
fn focus_scroll_request_does_not_override_later_manual_scroll() -> Result<()> {
    let width = 18;
    let backend = TestBackend::new(width, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column([button("A1"), button("B2"), button("C3")]);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;
    draw_view(&mut terminal, &view)?;
    assert_eq!(scroll_offset(&view), 2);

    view.handle_event(key(KeyCode::PageDown))?;
    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 5);

    Ok(())
}
