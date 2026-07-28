/// Verifies focused input character keys emit inserted text through `on_input`.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// Char('!'), Char(' ')
/// ```
///
/// # Assertions
///
/// - The `!` key is handled.
/// - The space key is handled.
/// - The callback receives `Ada!`.
/// - The callback receives `Ada `.
#[test]
fn focused_input_emits_inserted_text_through_on_input() -> leptatui::app::Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_char = Rc::clone(&emitted);
    let mut char_view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_char.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut char_view).set_mode(VimMode::Insert);

    assert_eq!(
        char_view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_space = Rc::clone(&emitted);
    let mut space_view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_space.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut space_view).set_mode(VimMode::Insert);

    assert_eq!(
        space_view.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(
        emitted.borrow().as_slice(),
        &[String::from("Ada!"), String::from("Ada ")]
    );

    Ok(())
}

/// Verifies focused inputs leave insert mode on the `jk` key sequence.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// j, k
/// ```
///
/// # Assertions
///
/// - Both keys are handled.
/// - No input value is emitted.
/// - The input switches to normal mode with Esc-style cursor placement.
#[test]
fn focused_input_jk_returns_to_normal_mode_without_emitting_text() -> leptatui::app::Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("Ada", &emitted);

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
    assert_eq!(editable_state(&view).cursor(), 2);
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    Ok(())
}

/// Verifies a pending insert-mode `j` is inserted with the next non-escape key.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// j, x
/// ```
///
/// # Assertions
///
/// - The first `j` waits for the next key.
/// - The following `x` emits both inserted characters.
/// - The input remains in insert mode.
#[test]
fn focused_input_pending_j_inserts_with_next_non_escape_character() -> leptatui::app::Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("Ada", &emitted);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('x')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&view).cursor(), 5);
    assert_eq!(emitted.borrow().as_slice(), &[String::from("Adajx")]);

    Ok(())
}

/// Verifies slow insert-mode `jk` is inserted as literal text.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// j, wait past timeout, k
/// ```
///
/// # Assertions
///
/// - The first `j` waits for the next key.
/// - The later `k` emits literal `jk`.
/// - The input remains in insert mode.
#[test]
fn focused_input_slow_jk_inserts_literal_text() -> leptatui::app::Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("Ada", &emitted);

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
    assert_eq!(editable_state(&view).cursor(), 5);
    assert_eq!(emitted.borrow().as_slice(), &[String::from("Adajk")]);

    Ok(())
}

/// Verifies an expired pending insert-mode `j` is emitted without another key.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// j, wait past timeout, flush
/// ```
///
/// # Assertions
///
/// - The first `j` waits for the timeout.
/// - Flushing emits literal `j`.
/// - A second flush has nothing to emit.
#[test]
fn focused_input_idle_flush_emits_expired_pending_j() -> leptatui::app::Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("Ada", &emitted);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().as_slice(), &[] as &[String]);

    thread::sleep(Duration::from_millis(1100));

    assert_eq!(view.__flush_pending_input(), Some(AppControl::Continue));
    assert_eq!(editable_state(&view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&view).cursor(), 4);
    assert_eq!(emitted.borrow().as_slice(), &[String::from("Adaj")]);
    assert_eq!(view.__flush_pending_input(), None);

    Ok(())
}

/// Verifies focused input deletion keys emit shortened text through `on_input`.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// Backspace, Delete at cursor 1
/// ```
///
/// # Assertions
///
/// - The backspace key is handled.
/// - The delete key is handled.
/// - The callback receives `Ad` after backspace.
/// - The callback receives `Aa` after delete.
#[test]
fn focused_input_emits_deletions_through_on_input() -> leptatui::app::Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_backspace = Rc::clone(&emitted);
    let mut backspace_view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_backspace.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut backspace_view).set_mode(VimMode::Insert);

    assert_eq!(
        backspace_view.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let emitted_for_delete = Rc::clone(&emitted);
    let mut delete_view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_delete.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut delete_view).set_mode(VimMode::Insert);
    editable_state_mut(&mut delete_view).set_cursor(1);

    assert_eq!(
        delete_view.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(
        emitted.borrow().as_slice(),
        &[String::from("Ad"), String::from("Aa")]
    );

    Ok(())
}

/// Verifies focused input cursor keys move without emitting text.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// Left, Home, Right, End
/// ```
///
/// # Assertions
///
/// - Left moves the cursor to byte index `2`.
/// - Home moves the cursor to byte index `0`.
/// - Right moves the cursor to byte index `1`.
/// - End moves the cursor to byte index `3`.
/// - No input callback values are emitted.
#[test]
fn focused_input_cursor_keys_move_without_emitting_text() -> leptatui::app::Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let emitted_for_input = Rc::clone(&emitted);
    let mut view = input("Ada").with_focus(true).on_input(move |next| {
        emitted_for_input.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut view).set_mode(VimMode::Insert);

    view.handle_key_event(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 2);

    view.handle_key_event(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    view.handle_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 1);

    view.handle_key_event(KeyEvent::new(KeyCode::End, KeyModifiers::NONE))?;
    assert_eq!(editable_state(&view).cursor(), 3);
    assert!(emitted.borrow().is_empty());

    Ok(())
}

/// Verifies focused inputs without callbacks do not mutate displayed values.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// Char('!')
/// ```
///
/// # Assertions
///
/// - The character key is handled.
/// - The retained input value remains `Ada`.
/// - Rendering still shows `Ada`.
/// - The cell after the value remains blank.
#[test]
fn focused_input_without_callback_does_not_mutate_displayed_value() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("Ada").with_focus(true);

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    assert_eq!(view.value(), "Ada");

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "A");
    assert_eq!(cell_symbol(&terminal, 3, 1, 8), "a");
    assert_eq!(cell_symbol(&terminal, 4, 1, 8), " ");

    Ok(())
}
