/// Verifies focused text-area insertion keys emit full next values.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\nLovelace").with_focus(true).on_input(...)
/// Char('!'), Enter
/// ```
///
/// # Assertions
///
/// - The character key is handled.
/// - The enter key is handled.
/// - The callbacks receive the full proposed multiline values.
#[test]
fn focused_text_area_emits_inserted_text_through_on_input() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_char = Rc::clone(&emitted);
    let mut char_view = text_area("Ada\nLovelace")
        .with_focus(true)
        .on_input(move |next| {
            emitted_for_char.borrow_mut().push(next);
            AppControl::Continue
        });
    editable_state_mut(&mut char_view).set_mode(VimMode::Insert);

    assert_eq!(
        char_view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_enter = Rc::clone(&emitted);
    let mut enter_view = text_area("Ada").with_focus(true).on_input(move |next| {
        emitted_for_enter.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut enter_view).set_mode(VimMode::Insert);

    assert_eq!(
        enter_view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(
        emitted.borrow().as_slice(),
        &[String::from("Ada\nLovelace!"), String::from("Ada\n")]
    );

    Ok(())
}

/// Verifies focused text areas leave insert mode on the `jk` key sequence.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\nLovelace").with_focus(true).on_input(...)
/// j, k
/// ```
///
/// # Assertions
///
/// - Both keys are handled.
/// - No input value is emitted.
/// - The text area switches to normal mode with Esc-style cursor placement.
#[test]
fn focused_text_area_jk_returns_to_normal_mode_without_emitting_text() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_text_area("Ada\nLovelace", &emitted);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Insert);
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).cursor(), 11);
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    Ok(())
}

/// Verifies slow text-area insert-mode `jk` is inserted as literal text.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\nLovelace").with_focus(true).on_input(...)
/// j, wait past timeout, k
/// ```
///
/// # Assertions
///
/// - The first `j` waits for the next key.
/// - The later `k` emits literal `jk`.
/// - The text area remains in insert mode.
#[test]
fn focused_text_area_slow_jk_inserts_literal_text() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_text_area("Ada\nLovelace", &emitted);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    thread::sleep(Duration::from_millis(1100));

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&view).cursor(), 14);
    assert_eq!(
        emitted.borrow().as_slice(),
        &[String::from("Ada\nLovelacejk")]
    );

    Ok(())
}

/// Verifies focused text-area deletion keys can remove line boundaries.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\nLovelace").with_focus(true).on_input(...)
/// Backspace after newline, Delete before newline
/// ```
///
/// # Assertions
///
/// - Backspace at the start of the second line is handled.
/// - Delete at the end of the first line is handled.
/// - Both callbacks receive the joined multiline value.
#[test]
fn focused_text_area_emits_line_boundary_deletions_through_on_input() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_backspace = Rc::clone(&emitted);
    let mut backspace_view = text_area("Ada\nLovelace")
        .with_focus(true)
        .on_input(move |next| {
            emitted_for_backspace.borrow_mut().push(next);
            AppControl::Continue
        });
    editable_state_mut(&mut backspace_view).set_mode(VimMode::Insert);
    editable_state_mut(&mut backspace_view).set_cursor(4);

    assert_eq!(
        backspace_view.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_delete = Rc::clone(&emitted);
    let mut delete_view = text_area("Ada\nLovelace")
        .with_focus(true)
        .on_input(move |next| {
            emitted_for_delete.borrow_mut().push(next);
            AppControl::Continue
        });
    editable_state_mut(&mut delete_view).set_mode(VimMode::Insert);
    editable_state_mut(&mut delete_view).set_cursor(3);

    assert_eq!(
        delete_view.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(
        emitted.borrow().as_slice(),
        &[String::from("AdaLovelace"), String::from("AdaLovelace")]
    );

    Ok(())
}

/// Verifies focused text-area cursor keys move without emitting values.
///
/// # Example Under Test
///
/// ```text
/// text_area("abc\nde\nfghi").with_focus(true).on_input(...)
/// Up, Up, Down, Down, Home, End
/// ```
///
/// # Assertions
///
/// - Up and down move between logical lines at the nearest available column.
/// - Home and End move within the current logical line.
/// - No input callback values are emitted.
#[test]
fn focused_text_area_cursor_keys_move_without_emitting_text() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_text_area = Rc::clone(&emitted);
    let mut view = text_area("abc\nde\nfghi")
        .with_focus(true)
        .on_input(move |next| {
            emitted_for_text_area.borrow_mut().push(next);
            AppControl::Continue
        });
    editable_state_mut(&mut view).set_mode(VimMode::Insert);
    editable_state_mut(&mut view).set_cursor(9);

    view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 6);

    view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 2);

    view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 6);

    view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 9);

    view.handle_key_event(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 7);

    view.handle_key_event(KeyEvent::new(KeyCode::End, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 11);
    assert!(emitted.borrow().is_empty());

    Ok(())
}
