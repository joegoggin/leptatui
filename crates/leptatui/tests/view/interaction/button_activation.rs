/// Verifies enter and space activate focused button actions.
///
/// # Example Under Test
///
/// ```text
/// column([button("Enter").on_press(...), button("Space").on_press(...)])
/// Tab, Enter, Tab, Space
/// ```
///
/// # Assertions
///
/// - The first tab event succeeds.
/// - The enter event succeeds and increments the action count to `1`.
/// - The second tab event succeeds.
/// - The space event succeeds and increments the action count to `2`.
///
/// # Why
///
/// Both common activation keys should trigger only the currently focused
/// button.
#[test]
fn enter_and_space_activate_focused_button() -> Result<()> {
    let count = Rc::new(Cell::new(0));
    let enter_count = Rc::clone(&count);
    let space_count = Rc::clone(&count);

    let mut view = column([
        button("Enter").on_press(move || {
            enter_count.set(enter_count.get() + 1);
            AppControl::Continue
        }),
        button("Space").on_press(move || {
            space_count.set(space_count.get() + 1);
            AppControl::Continue
        }),
    ]);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Enter))?;
    assert_eq!(count.get(), 1);

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Char(' ')))?;
    assert_eq!(count.get(), 2);

    Ok(())
}
