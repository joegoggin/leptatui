/// Verifies focused text areas without callbacks do not mutate displayed values.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\nLovelace").with_focus(true)
/// Char('!')
/// ```
///
/// # Assertions
///
/// - The character key is handled.
/// - The retained text-area value remains unchanged.
/// - Rendering still shows the original value.
/// - The cell after the first line remains blank.
#[test]
fn focused_text_area_without_callback_does_not_mutate_displayed_value() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = text_area("Ada\nLovelace").with_focus(true);

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(view.value(), "Ada\nLovelace");

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 12), "A");
    assert_eq!(cell_symbol(&terminal, 3, 1, 12), "a");
    assert_eq!(cell_symbol(&terminal, 4, 1, 12), " ");

    Ok(())
}

/// Verifies focused input editing works inside component boundaries.
///
/// # Example Under Test
///
/// ```text
/// component(FocusPanel { view: input("Ada").on_input(...) })
/// Tab, A, Char('!')
/// ```
///
/// # Assertions
///
/// - Tabbing into the component boundary succeeds.
/// - The character key is handled by the focused input.
/// - The callback receives `Ada!`.
#[test]
fn focused_input_inside_component_boundary_handles_editing_keys() -> leptatui::app::Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_input = Rc::clone(&emitted);
    let input_view = input("Ada").on_input(move |next| {
        emitted_for_input.borrow_mut().push(next);
        AppControl::Continue
    });
    let mut view = component(FocusPanel {
        view: input_view.into_view(),
    });

    view.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(emitted.borrow().as_slice(), &[String::from("Ada!")]);

    Ok(())
}

/// Verifies focused text-area editing works inside component boundaries.
///
/// # Example Under Test
///
/// ```text
/// component(FocusPanel { view: text_area("Ada").on_input(...) })
/// Tab, A, Enter
/// ```
///
/// # Assertions
///
/// - Tabbing into the component boundary succeeds.
/// - The enter key is handled by the focused text area.
/// - The callback receives `Ada\n`.
#[test]
fn focused_text_area_inside_component_boundary_handles_editing_keys() -> leptatui::app::Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_text_area = Rc::clone(&emitted);
    let text_area_view = text_area("Ada").on_input(move |next| {
        emitted_for_text_area.borrow_mut().push(next);
        AppControl::Continue
    });
    let mut view = component(FocusPanel {
        view: text_area_view.into_view(),
    });

    view.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?;
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(emitted.borrow().as_slice(), &[String::from("Ada\n")]);

    Ok(())
}
