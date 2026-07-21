/// Verifies editable controls support Vim mode transition keys.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// i, Esc, a, I, A
///
/// text_area("ab\ncd").with_focus(true)
/// I, A
/// ```
///
/// # Assertions
///
/// - Inputs start in normal mode.
/// - Esc from insert mode switches the input to normal mode and moves the cursor onto the
///   previous character.
/// - `i` and `a` switch the input to insert mode at the current and next
///   normal-mode positions.
/// - `I` and `A` move to the line start and line end for inputs and text
///   areas.
#[test]
fn focused_editable_controls_support_vim_mode_transitions() -> Result<()> {
    let mut input_view = input("Ada").with_focus(true);
    assert_eq!(editable_state(&input_view).mode(), VimMode::Normal);

    editable_state_mut(&mut input_view).set_mode(VimMode::Insert);
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&input_view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&input_view).cursor(), 2);

    editable_state_mut(&mut input_view).set_cursor(1);
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Char('i')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&input_view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&input_view).cursor(), 1);

    editable_state_mut(&mut input_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut input_view).set_cursor(1);
    input_view.handle_key_event(key_event(KeyCode::Char('a')))?;
    assert_eq!(editable_state(&input_view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&input_view).cursor(), 2);

    editable_state_mut(&mut input_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut input_view).set_cursor(2);
    input_view.handle_key_event(key_event(KeyCode::Char('I')))?;
    assert_eq!(editable_state(&input_view).cursor(), 0);

    editable_state_mut(&mut input_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut input_view).set_cursor(0);
    input_view.handle_key_event(key_event(KeyCode::Char('A')))?;
    assert_eq!(editable_state(&input_view).cursor(), 3);

    let mut text_area_view = text_area("ab\ncd").with_focus(true);
    editable_state_mut(&mut text_area_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut text_area_view).set_cursor(4);
    text_area_view.handle_key_event(key_event(KeyCode::Char('I')))?;
    assert_eq!(editable_state(&text_area_view).mode(), VimMode::Insert);
    assert_eq!(editable_state(&text_area_view).cursor(), 3);

    editable_state_mut(&mut text_area_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut text_area_view).set_cursor(4);
    text_area_view.handle_key_event(key_event(KeyCode::Char('A')))?;
    assert_eq!(editable_state(&text_area_view).cursor(), 5);

    Ok(())
}

/// Verifies focused text areas support Vim normal-mode open-line commands.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo").with_focus(true).on_input(...)
/// o, O
///
/// text_area("").with_focus(true).on_input(...)
/// o, O
/// ```
///
/// # Assertions
///
/// - `o` opens a blank line below the current logical line, enters insert mode,
///   and places the cursor on that blank line.
/// - `O` opens a blank line above the current logical line, enters insert mode,
///   and places the cursor on that blank line.
/// - Opening below the final line appends a trailing blank line.
/// - Opening above the first line prepends a blank line.
/// - Empty text areas enter insert mode without emitting a changed value.
#[test]
fn focused_text_area_supports_vim_open_line_commands() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut below_middle = emitting_text_area("one\ntwo", &emitted);
    editable_state_mut(&mut below_middle).set_mode(VimMode::Normal);
    editable_state_mut(&mut below_middle).set_cursor(1);

    assert_eq!(
        below_middle.handle_key_event(key_event(KeyCode::Char('o')))?,
        KeyControl::Handled
    );
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\n\ntwo")
    );
    assert_eq!(editable_state(&below_middle).mode(), VimMode::Insert);
    assert_eq!(editable_state(&below_middle).cursor(), 4);
    assert_eq!(
        editable_state(&below_middle).undo_stack(),
        &[String::from("one\ntwo")]
    );

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut above_middle = emitting_text_area("one\ntwo", &emitted);
    editable_state_mut(&mut above_middle).set_mode(VimMode::Normal);
    editable_state_mut(&mut above_middle).set_cursor(5);

    above_middle.handle_key_event(key_event(KeyCode::Char('O')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\n\ntwo")
    );
    assert_eq!(editable_state(&above_middle).mode(), VimMode::Insert);
    assert_eq!(editable_state(&above_middle).cursor(), 4);

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut below_final = emitting_text_area("one\ntwo", &emitted);
    editable_state_mut(&mut below_final).set_mode(VimMode::Normal);
    editable_state_mut(&mut below_final).set_cursor(4);

    below_final.handle_key_event(key_event(KeyCode::Char('o')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\ntwo\n")
    );
    assert_eq!(editable_state(&below_final).mode(), VimMode::Insert);
    assert_eq!(editable_state(&below_final).cursor(), 8);

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut above_first = emitting_text_area("one\ntwo", &emitted);
    editable_state_mut(&mut above_first).set_mode(VimMode::Normal);
    editable_state_mut(&mut above_first).set_cursor(1);

    above_first.handle_key_event(key_event(KeyCode::Char('O')))?;
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("\none\ntwo")
    );
    assert_eq!(editable_state(&above_first).mode(), VimMode::Insert);
    assert_eq!(editable_state(&above_first).cursor(), 0);

    above_first = reconcile_text_area_value(&above_first, "\none\ntwo", &emitted);
    assert_eq!(
        above_first.handle_key_event(key_event(KeyCode::Backspace))?,
        KeyControl::Handled
    );
    assert_eq!(
        emitted.borrow().last().map(String::as_str),
        Some("one\ntwo")
    );
    assert_eq!(editable_state(&above_first).mode(), VimMode::Insert);
    assert_eq!(editable_state(&above_first).cursor(), 0);

    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut empty_below = emitting_text_area("", &emitted);
    editable_state_mut(&mut empty_below).set_mode(VimMode::Normal);

    empty_below.handle_key_event(key_event(KeyCode::Char('o')))?;
    assert!(emitted.borrow().is_empty());
    assert_eq!(editable_state(&empty_below).mode(), VimMode::Insert);
    assert_eq!(editable_state(&empty_below).cursor(), 0);

    let mut empty_above = emitting_text_area("", &emitted);
    editable_state_mut(&mut empty_above).set_mode(VimMode::Normal);

    empty_above.handle_key_event(key_event(KeyCode::Char('O')))?;
    assert!(emitted.borrow().is_empty());
    assert_eq!(editable_state(&empty_above).mode(), VimMode::Insert);
    assert_eq!(editable_state(&empty_above).cursor(), 0);

    Ok(())
}

/// Verifies focused inputs handle Vim open-line keys as no-ops.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// o, O
/// ```
///
/// # Assertions
///
/// - `o` and `O` are handled so they do not leak to parent key handling.
/// - Inputs do not emit values or leave normal mode for multiline-only
///   open-line commands.
#[test]
fn focused_input_handles_vim_open_line_commands_without_mutation() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let mut view = emitting_input("Ada", &emitted);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('o')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).cursor(), 1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('O')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).cursor(), 1);
    assert!(emitted.borrow().is_empty());

    Ok(())
}

/// Verifies focused inputs support Vim normal-mode movement.
///
/// # Example Under Test
///
/// ```text
/// input("one two three").with_focus(true)
/// l, Left, Right, h, w, e, b, $, 0, G, gg
/// ```
///
/// # Assertions
///
/// - Character movement keys and arrows update the input cursor.
/// - Word motions move to the expected word start and end positions.
/// - Line and value boundary motions move to the first or last character.
/// - `gg` moves the cursor back to the first character.
#[test]
fn focused_input_supports_vim_normal_mode_movement() -> Result<()> {
    let mut view = input("one two three").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(0);

    view.handle_key_event(key_event(KeyCode::Char('l')))?;
    assert_eq!(editable_state(&view).cursor(), 1);

    view.handle_key_event(key_event(KeyCode::Left))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    view.handle_key_event(key_event(KeyCode::Right))?;
    assert_eq!(editable_state(&view).cursor(), 1);

    view.handle_key_event(key_event(KeyCode::Char('h')))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    view.handle_key_event(key_event(KeyCode::Char('w')))?;
    assert_eq!(editable_state(&view).cursor(), 4);

    view.handle_key_event(key_event(KeyCode::Char('e')))?;
    assert_eq!(editable_state(&view).cursor(), 6);

    view.handle_key_event(key_event(KeyCode::Char('b')))?;
    assert_eq!(editable_state(&view).cursor(), 4);

    view.handle_key_event(key_event(KeyCode::Char('$')))?;
    assert_eq!(editable_state(&view).cursor(), 12);

    view.handle_key_event(key_event(KeyCode::Char('0')))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    view.handle_key_event(key_event(KeyCode::Char('G')))?;
    assert_eq!(editable_state(&view).cursor(), 12);

    view.handle_key_event(key_event(KeyCode::Char('g')))?;
    view.handle_key_event(key_event(KeyCode::Char('g')))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    Ok(())
}

/// Verifies focused text areas support Vim normal-mode movement.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo\nthree").with_focus(true)
/// j, k, Down, Up, $, 0, G, gg
/// ```
///
/// # Assertions
///
/// - `j`, `k`, Down, and Up move between logical lines.
/// - Vertical movement preserves the nearest available column.
/// - `$` and `0` move to the current line end and start.
/// - `G` and `gg` move to the last and first characters in the text area.
#[test]
fn focused_text_area_supports_vim_normal_mode_movement() -> Result<()> {
    let mut view = text_area("one\ntwo\nthree").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Normal);
    editable_state_mut(&mut view).set_cursor(4);

    view.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(editable_state(&view).cursor(), 8);

    view.handle_key_event(key_event(KeyCode::Char('k')))?;
    assert_eq!(editable_state(&view).cursor(), 4);

    view.handle_key_event(key_event(KeyCode::Down))?;
    assert_eq!(editable_state(&view).cursor(), 8);

    view.handle_key_event(key_event(KeyCode::Up))?;
    assert_eq!(editable_state(&view).cursor(), 4);

    editable_state_mut(&mut view).set_cursor(5);
    view.handle_key_event(key_event(KeyCode::Char('k')))?;
    assert_eq!(editable_state(&view).cursor(), 1);

    editable_state_mut(&mut view).set_cursor(8);
    view.handle_key_event(key_event(KeyCode::Char('$')))?;
    assert_eq!(editable_state(&view).cursor(), 12);

    view.handle_key_event(key_event(KeyCode::Char('0')))?;
    assert_eq!(editable_state(&view).cursor(), 8);

    view.handle_key_event(key_event(KeyCode::Char('G')))?;
    assert_eq!(editable_state(&view).cursor(), 12);

    view.handle_key_event(key_event(KeyCode::Char('g')))?;
    view.handle_key_event(key_event(KeyCode::Char('g')))?;
    assert_eq!(editable_state(&view).cursor(), 0);

    Ok(())
}

/// Verifies focused text areas keep trailing blank lines reachable in normal mode.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo\n").with_focus(true)
/// Insert at trailing blank line, Esc, k, j
/// ```
///
/// # Assertions
///
/// - Esc enters normal mode without moving off the trailing blank line.
/// - `k` moves to the previous logical line.
/// - `j` returns to the trailing blank line.
#[test]
fn focused_text_area_supports_trailing_blank_line_normal_mode_movement() -> Result<()> {
    let value = "one\ntwo\n";
    let trailing_blank_cursor = value.len();
    let mut view = text_area(value).with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Insert);
    editable_state_mut(&mut view).set_cursor(trailing_blank_cursor);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(&view).mode(), VimMode::Normal);
    assert_eq!(editable_state(&view).cursor(), trailing_blank_cursor);

    view.handle_key_event(key_event(KeyCode::Char('k')))?;
    assert_eq!(editable_state(&view).cursor(), 4);

    view.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(editable_state(&view).cursor(), trailing_blank_cursor);

    Ok(())
}
