/// Verifies focused inputs support Vim character-wise visual mode transitions.
///
/// # Example Under Test
///
/// ```text
/// input("abcd").with_focus(true)
/// v, l, h, Esc
/// ```
///
/// # Assertions
///
/// - `v` enters character-wise visual mode and anchors at the current cursor.
/// - Normal movement keys move the cursor while preserving the anchor.
/// - Esc returns to normal mode and clears the selection anchor.
#[test]
fn focused_input_supports_vim_visual_mode_transitions() -> Result<()> {
    let mut view = input("abcd").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('v')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Visual);
    assert_eq!(editable_state(&view).selection_anchor(), Some(1));

    view.handle_key_event(key_event(KeyCode::Char('l')))?;
    assert_eq!(editable_state(&view).cursor(), 2);
    assert_eq!(editable_state(&view).selection_anchor(), Some(1));

    view.handle_key_event(key_event(KeyCode::Char('h')))?;
    assert_eq!(editable_state(&view).cursor(), 1);
    assert_eq!(editable_state(&view).selection_anchor(), Some(1));

    view.handle_key_event(key_event(KeyCode::Esc))?;
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).selection_anchor(), None);

    Ok(())
}

/// Verifies character-wise visual yank and delete commands use the selection.
///
/// # Example Under Test
///
/// ```text
/// input("abcd").with_focus(true).on_input(...)
/// v, l, y, then v, l, d
/// ```
///
/// # Assertions
///
/// - `y` yanks the selected characters and exits visual mode.
/// - `d` deletes the selected characters, emits the controlled value, and
///   records undo history.
#[test]
fn focused_input_supports_visual_yank_and_delete() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("abcd", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(1);

    view.handle_key_event(key_event(KeyCode::Char('v')))?;
    view.handle_key_event(key_event(KeyCode::Char('l')))?;
    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).selection_anchor(), None);
    assert_eq!(editable_state(&view).yank_buffer(), "bc");
    assert_eq!(editable_state(&view).cursor(), 1);

    view.handle_key_event(key_event(KeyCode::Char('v')))?;
    view.handle_key_event(key_event(KeyCode::Char('l')))?;
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('d')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("ad"));
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).selection_anchor(), None);
    assert_eq!(editable_state(&view).yank_buffer(), "bc");
    assert_eq!(editable_state(&view).undo_stack(), &[String::from("abcd")]);

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("abcd", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(1);
    view.handle_key_event(key_event(KeyCode::Char('v')))?;
    view.handle_key_event(key_event(KeyCode::Char('l')))?;
    view.handle_key_event(key_event(KeyCode::Char('x')))?;
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("ad"));
    assert_eq!(editable_state(&view).yank_buffer(), "bc");

    Ok(())
}

/// Verifies visual-line text-area yank, paste, and delete work linewise.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo\nthree").with_focus(true).on_input(...)
/// V, j, y, G, p
/// V, j, d
/// ```
///
/// # Assertions
///
/// - Visual-line `y` stores selected logical lines in the linewise yank buffer.
/// - `p` pastes the linewise selection below the current line.
/// - Visual-line `d` removes all selected logical lines and records undo
///   history.
#[test]
fn focused_text_area_supports_visual_line_yank_paste_and_delete() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_text_area("one\ntwo\nthree", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(4);

    view.handle_key_event(key_event(KeyCode::Char('V')))?;
    view.handle_key_event(key_event(KeyCode::Char('j')))?;
    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).selection_anchor(), None);
    assert_eq!(editable_state(&view).yank_buffer(), "two\nthree");

    view.handle_key_event(key_event(KeyCode::Char('G')))?;
    view.handle_key_event(key_event(KeyCode::Char('p')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\ntwo\nthree\ntwo\nthree")
    );

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_text_area("one\ntwo\nthree", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(4);

    view.handle_key_event(key_event(KeyCode::Char('V')))?;
    view.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('d')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("one"));
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).selection_anchor(), None);
    assert_eq!(editable_state(&view).yank_buffer(), "two\nthree");
    assert_eq!(
        editable_state(&view).undo_stack(),
        &[String::from("one\ntwo\nthree")]
    );

    Ok(())
}

/// Verifies focused inputs support normal-mode mutation and history commands.
///
/// # Example Under Test
///
/// ```text
/// input("abc").with_focus(true).on_input(...)
/// x, yy, p, dd, u, Ctrl+r
/// ```
///
/// # Assertions
///
/// - `x` emits `ac` and records the original value in undo history.
/// - `yy` yanks the current input value.
/// - `p` emits the pasted `acac` value.
/// - `dd` emits an empty value.
/// - `u` emits the previous value and records redo history.
/// - Ctrl+r emits the redone empty value.
/// - The full emitted value sequence matches the expected mutation order.
///
/// # Why
///
/// Undo and redo history must survive controlled-value reconciliation between
/// emitted input values.
#[test]
fn focused_input_supports_vim_delete_yank_paste_undo_and_redo() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("abc", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('x')))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("ac"));
    assert_eq!(editable_state(&view).undo_stack(), &[String::from("abc")]);

    view = reconcile_input_value(&view, "ac", &emitted);
    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    assert_eq!(editable_state(&view).yank_buffer(), "ac");

    view.handle_key_event(key_event(KeyCode::Char('p')))?;
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("acac"));

    view = reconcile_input_value(&view, "acac", &emitted);
    view.handle_key_event(key_event(KeyCode::Char('d')))?;
    view.handle_key_event(key_event(KeyCode::Char('d')))?;
    assert_eq!(emitted.borrow().last().map(String::as_str), Some(""));

    view = reconcile_input_value(&view, "", &emitted);
    view.handle_key_event(key_event(KeyCode::Char('u')))?;
    assert_eq!(emitted.borrow().last().map(String::as_str), Some("acac"));
    assert_eq!(editable_state(&view).redo_stack(), &[String::new()]);

    view = reconcile_input_value(&view, "acac", &emitted);
    view.handle_key_event(ctrl_key_event('r'))?;
    assert_eq!(emitted.borrow().last().map(String::as_str), Some(""));

    assert_eq!(
        emitted.borrow().as_slice(),
        &[
            String::from("ac"),
            String::from("acac"),
            String::new(),
            String::from("acac"),
            String::new(),
        ]
    );

    Ok(())
}

/// Verifies focused text areas support linewise yank, delete, paste, and history.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo\nthree").with_focus(true).on_input(...)
/// yy, G, p, dd, u, Ctrl+r
/// ```
///
/// # Assertions
///
/// - `yy` yanks the current logical line without a trailing newline.
/// - `p` after `G` appends the yanked line below the final line.
/// - `dd` deletes the selected logical line and keeps that line in the yank
///   buffer.
/// - `u` emits the previous text-area value.
/// - Ctrl+r emits the redone line-deleted value.
///
/// # Why
///
/// Linewise operations need different paste ranges than character-wise input
/// operations.
#[test]
fn focused_text_area_supports_linewise_yank_delete_paste_undo_and_redo() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_text_area("one\ntwo\nthree", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(4);

    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    view.handle_key_event(key_event(KeyCode::Char('y')))?;
    assert_eq!(editable_state(&view).yank_buffer(), "two");

    view.handle_key_event(key_event(KeyCode::Char('G')))?;
    view.handle_key_event(key_event(KeyCode::Char('p')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\ntwo\nthree\ntwo")
    );

    view = reconcile_text_area_value(&view, "one\ntwo\nthree\ntwo", &emitted);
    editable_state_mut(&mut view).set_cursor(4);
    view.handle_key_event(key_event(KeyCode::Char('d')))?;
    view.handle_key_event(key_event(KeyCode::Char('d')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\nthree\ntwo")
    );
    assert_eq!(editable_state(&view).yank_buffer(), "two");

    view = reconcile_text_area_value(&view, "one\nthree\ntwo", &emitted);
    view.handle_key_event(key_event(KeyCode::Char('u')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\ntwo\nthree\ntwo")
    );

    view = reconcile_text_area_value(&view, "one\ntwo\nthree\ntwo", &emitted);
    view.handle_key_event(ctrl_key_event('r'))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\nthree\ntwo")
    );

    Ok(())
}
