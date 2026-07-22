/// Verifies moving the mouse over a button focuses it.
#[test]
fn mouse_move_focuses_button_under_pointer() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(20, 3))?;
    let mut view = row([button("One"), button("Two")]);

    draw_view(&mut terminal, &view)?;
    view.handle_event(mouse(MouseEventKind::Moved, 12, 1))?;

    assert_eq!(button_focuses(&view), vec![false, true]);
    Ok(())
}

/// Verifies left-clicking a button invokes its action.
#[test]
fn mouse_click_activates_button_under_pointer() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(12, 3))?;
    let count = Rc::new(Cell::new(0));
    let count_for_button = Rc::clone(&count);
    let mut view = button("Run").on_press(move || {
        count_for_button.set(count_for_button.get() + 1);
        AppControl::Continue
    });

    draw_view(&mut terminal, &view)?;
    assert_eq!(
        view.handle_event(mouse(MouseEventKind::Down(MouseButton::Left), 1, 1))?,
        AppControl::Continue
    );

    assert_eq!(count.get(), 1);
    assert_eq!(button_focuses(&view), vec![true]);
    Ok(())
}

/// Verifies mouse wheel events scroll overflowing content under the pointer.
#[test]
fn mouse_wheel_scrolls_overflowing_column_under_pointer() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(12, 2))?;
    let mut view = column([text("one"), text("two"), text("three")]);

    draw_view(&mut terminal, &view)?;
    assert_eq!(scroll_offset(&view), 0);

    view.handle_event(mouse(MouseEventKind::ScrollDown, 0, 0))?;
    assert_eq!(scroll_offset(&view), 1);

    draw_view(&mut terminal, &view)?;
    view.handle_event(mouse(MouseEventKind::ScrollUp, 0, 0))?;
    assert_eq!(scroll_offset(&view), 0);
    Ok(())
}

/// Verifies moving over an inline Markdown link focuses it.
#[test]
fn mouse_move_focuses_inline_markdown_link_under_pointer() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(20, 3))?;
    let mut view = markdown("[Docs](https://example.com)");

    draw_view(&mut terminal, view.as_view())?;
    view.handle_event(mouse(MouseEventKind::Moved, 0, 0))?;

    assert_eq!(view.__focused_control(), Some(FocusedControl::Link));
    Ok(())
}
