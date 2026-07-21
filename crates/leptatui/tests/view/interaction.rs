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

/// Verifies text-area editing scrolls an overflowing parent to the cursor.
///
/// # Example Under Test
///
/// ```text
/// column([text("Top"), focused text_area("one\ntwo\nthree\nfour"), text("Bottom")])
/// Enter, reconcile, render
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds with no parent scroll.
/// - Enter is handled by the focused text area.
/// - Reconciliation preserves editable state after the input callback updates.
/// - The next draw scrolls the parent to show the cursor.
/// - The terminal cursor lands on the expected row.
///
/// # Why
///
/// Controlled text-area edits can change child height and must keep the cursor
/// visible inside overflowing parents.
#[test]
fn text_area_editing_scrolls_overflowing_parent_to_cursor() -> Result<()> {
    let width = 12;
    let backend = TestBackend::new(width, 5);
    let mut terminal = Terminal::new(backend)?;
    let notes = Rc::new(RefCell::new(String::from("one\ntwo\nthree\nfour")));
    let build_view = |notes: &Rc<RefCell<String>>| {
        let value = notes.borrow().clone();
        let cursor = value.len();
        let notes_for_input = Rc::clone(notes);
        let mut notes_view = text_area(value).with_focus(true).on_input(move |next| {
            *notes_for_input.borrow_mut() = next;
            AppControl::Continue
        });
        editable_state_mut(&mut notes_view).set_mode(VimMode::Insert);
        editable_state_mut(&mut notes_view).set_cursor(cursor);

        column((text("Top"), notes_view, text("Bottom")))
    };
    let mut view = build_view(&notes);

    draw_view(&mut terminal, &view)?;
    assert_eq!(scroll_offset(&view), 0);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );

    let previous = view;
    let mut view = build_view(&notes);
    leptatui::__private::__reconcile_view(&mut view, &previous);
    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 2);
    terminal.backend_mut().assert_cursor_position((1, 4));

    Ok(())
}

/// Verifies normal-mode input boundary keys scroll an overflowing form.
///
/// # Example Under Test
///
/// ```text
/// form([text("Top"), focused normal-mode input("Ada"), trailing text rows])
/// j, k
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds with no form scroll.
/// - `j` is handled and scrolls the form down.
/// - `k` is handled and scrolls the form back to the top.
///
/// # Why
///
/// Single-line inputs at movement boundaries should pass normal-mode movement
/// intent to their overflowing parent form.
#[test]
fn normal_mode_input_boundary_keys_scroll_overflowing_form() -> Result<()> {
    let backend = TestBackend::new(12, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut input_view = input("Ada").with_focus(true);
    editable_state_mut(&mut input_view).set_mode(VimMode::Normal);
    let mut view = form((
        text("Top"),
        input_view,
        text("After 1"),
        text("After 2"),
        text("After 3"),
    ));

    draw_view(&mut terminal, &view)?;
    assert_eq!(scroll_offset(&view), 0);

    assert_eq!(
        view.handle_event(key(KeyCode::Char('j')))?,
        AppControl::Continue
    );
    assert_eq!(scroll_offset(&view), 1);

    assert_eq!(
        view.handle_event(key(KeyCode::Char('k')))?,
        AppControl::Continue
    );
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}

/// Verifies normal-mode text-area boundary keys scroll an overflowing form.
///
/// # Example Under Test
///
/// ```text
/// form([text("Top"), focused normal-mode text_area("one\ntwo"), trailing text rows])
/// j, j, k, k
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds with no form scroll.
/// - The first `j` moves within the text area without parent scrolling.
/// - The second `j` is handled at the boundary and scrolls the form down.
/// - The first `k` moves within the text area without parent scrolling up.
/// - The second `k` is handled at the boundary and scrolls the form to the top.
///
/// # Why
///
/// Multi-line text areas should only delegate normal-mode movement to the form
/// after reaching their own vertical boundaries.
#[test]
fn normal_mode_text_area_boundary_keys_scroll_overflowing_form() -> Result<()> {
    let backend = TestBackend::new(12, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut text_area_view = text_area("one\ntwo").with_focus(true);
    editable_state_mut(&mut text_area_view).set_mode(VimMode::Normal);
    editable_state_mut(&mut text_area_view).set_cursor(0);
    let mut view = form((
        text("Top"),
        text_area_view,
        text("After 1"),
        text("After 2"),
    ));

    draw_view(&mut terminal, &view)?;
    assert_eq!(scroll_offset(&view), 0);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(form_child(&view, 1)).cursor(), 4);
    assert_eq!(scroll_offset(&view), 0);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(form_child(&view, 1)).cursor(), 4);
    assert_eq!(scroll_offset(&view), 1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(form_child(&view, 1)).cursor(), 0);
    assert_eq!(scroll_offset(&view), 1);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(editable_state(form_child(&view, 1)).cursor(), 0);
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}

/// Verifies focus scrolling works through component boundaries.
///
/// # Example Under Test
///
/// ```text
/// column([button("A1"), component(FocusPanel(button("B2"))), button("C3")])
/// height = 4
/// Tab, Tab, render
/// ```
///
/// # Assertions
///
/// - Tabbing into the component boundary succeeds.
/// - Rendering scrolls the parent column to the component's focused button.
/// - The component button label is visible in the terminal buffer.
///
/// # Why
///
/// Component boundaries should preserve the built-in focus visibility behavior.
#[test]
fn tab_focus_scrolls_to_focused_button_inside_component_boundary() -> Result<()> {
    let width = 18;
    let backend = TestBackend::new(width, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column((
        button("A1"),
        component(FocusPanel {
            view: button("B2").into_view(),
        }),
        button("C3"),
    ));

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 2);
    assert!(symbol_position_opt(&terminal, "B", width).is_some());

    Ok(())
}

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
fn focused_input_emits_inserted_text_through_on_input() -> Result<()> {
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
fn focused_input_jk_returns_to_normal_mode_without_emitting_text() -> Result<()> {
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
fn focused_input_pending_j_inserts_with_next_non_escape_character() -> Result<()> {
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
fn focused_input_slow_jk_inserts_literal_text() -> Result<()> {
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
fn focused_input_idle_flush_emits_expired_pending_j() -> Result<()> {
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
fn focused_input_emits_deletions_through_on_input() -> Result<()> {
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
fn focused_input_cursor_keys_move_without_emitting_text() -> Result<()> {
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
fn focused_input_without_callback_does_not_mutate_displayed_value() -> Result<()> {
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

/// Verifies insert-mode Enter keeps inputs single-line and text areas multiline.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true).on_input(...)
/// Enter
///
/// text_area("Ada").with_focus(true).on_input(...)
/// Enter
/// ```
///
/// # Assertions
///
/// - Enter is handled for a focused input without emitting values.
/// - Enter is handled for a focused text area.
/// - The text-area callback emits the value with a trailing newline.
#[test]
fn insert_mode_keeps_input_single_line_and_text_area_multiline() -> Result<()> {
    let input_emitted = Rc::new(RefCell::new(Vec::new()));
    let mut input_view = emitting_input("Ada", &input_emitted);
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert!(input_emitted.borrow().is_empty());

    let text_area_emitted = Rc::new(RefCell::new(Vec::new()));
    let mut text_area_view = emitting_text_area("Ada", &text_area_emitted);
    assert_eq!(
        text_area_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(
        text_area_emitted.borrow().as_slice(),
        &[String::from("Ada\n")]
    );

    Ok(())
}

/// Verifies focused inputs submit forms on Enter in insert and normal mode.
///
/// # Example Under Test
///
/// ```text
/// form([input("Ada").with_focus(true)]).on_submit(...)
/// Enter
/// ```
///
/// # Assertions
///
/// - Insert-mode Enter is handled.
/// - Insert-mode Enter invokes the submit callback once.
/// - Normal-mode Enter is handled.
/// - Normal-mode Enter invokes the submit callback once.
#[test]
fn form_submits_focused_input_on_enter_in_insert_and_normal_mode() -> Result<()> {
    let insert_submits = Rc::new(Cell::new(0));
    let insert_submits_for_form = Rc::clone(&insert_submits);
    let mut insert_input = input("Ada").with_focus(true);
    editable_state_mut(&mut insert_input).set_mode(VimMode::Insert);
    let mut insert_view = form([insert_input]).on_submit(move || {
        insert_submits_for_form.set(insert_submits_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        insert_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(insert_submits.get(), 1);

    let normal_submits = Rc::new(Cell::new(0));
    let normal_submits_for_form = Rc::clone(&normal_submits);
    let mut normal_input = input("Ada").with_focus(true);
    editable_state_mut(&mut normal_input).set_mode(VimMode::Normal);
    let mut normal_view = form([normal_input]).on_submit(move || {
        normal_submits_for_form.set(normal_submits_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        normal_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(normal_submits.get(), 1);

    Ok(())
}

/// Verifies text areas keep multiline Enter behavior inside forms.
///
/// # Example Under Test
///
/// ```text
/// form([text_area("Ada").with_focus(true).on_input(...)])
/// Enter, Ctrl+Enter
/// ```
///
/// # Assertions
///
/// - Plain Enter is handled by the text area.
/// - Plain Enter emits a value with a trailing newline.
/// - Plain Enter does not submit the form.
/// - Ctrl+Enter is handled by the form.
/// - Ctrl+Enter submits the form without emitting another input value.
#[test]
fn form_text_area_uses_plain_enter_for_newlines_and_ctrl_enter_for_submit() -> Result<()> {
    let emitted = Rc::new(RefCell::new(Vec::new()));
    let submits = Rc::new(Cell::new(0));
    let submits_for_form = Rc::clone(&submits);
    let mut view = form([emitting_text_area("Ada", &emitted)]).on_submit(move || {
        submits_for_form.set(submits_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().as_slice(), &[String::from("Ada\n")]);
    assert_eq!(submits.get(), 0);

    assert_eq!(
        view.handle_key_event(ctrl_enter_key_event())?,
        KeyControl::Handled
    );
    assert_eq!(emitted.borrow().len(), 1);
    assert_eq!(submits.get(), 1);

    Ok(())
}

/// Verifies Esc leaves editable insert mode before canceling a form.
///
/// # Example Under Test
///
/// ```text
/// form([input("Ada").with_focus(true)]).on_cancel(...)
/// Esc, Esc
/// ```
///
/// # Assertions
///
/// - The first Esc is handled by the focused input.
/// - The first Esc does not invoke the cancel callback.
/// - The second Esc is handled by the form.
/// - The second Esc invokes the cancel callback once.
#[test]
fn form_esc_leaves_insert_mode_before_canceling() -> Result<()> {
    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut input_view = input("Ada").with_focus(true);
    editable_state_mut(&mut input_view).set_mode(VimMode::Insert);
    let mut view = form([input_view]).on_cancel(move || {
        cancels_for_form.set(cancels_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 0);
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 1);

    Ok(())
}

/// Verifies `jk` leaves editable insert mode before Esc cancels a form.
///
/// # Example Under Test
///
/// ```text
/// form([input("Ada").with_focus(true)]).on_cancel(...)
/// j, k, Esc
/// ```
///
/// # Assertions
///
/// - The `jk` sequence is handled by the focused input.
/// - The `jk` sequence does not invoke the cancel callback.
/// - A later Esc is handled by the form.
#[test]
fn form_jk_leaves_insert_mode_without_canceling() -> Result<()> {
    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut input_view = input("Ada").with_focus(true);
    editable_state_mut(&mut input_view).set_mode(VimMode::Insert);
    let mut view = form([input_view]).on_cancel(move || {
        cancels_for_form.set(cancels_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('k')))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 0);
    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Normal);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 1);

    Ok(())
}

/// Verifies Esc leaves editable visual modes before canceling a form.
///
/// # Example Under Test
///
/// ```text
/// form([input("Ada").with_focus(true)]).on_cancel(...)
/// v, Esc, Esc
///
/// form([text_area("one\ntwo").with_focus(true)]).on_cancel(...)
/// V, Esc, Esc
/// ```
///
/// # Assertions
///
/// - The first Esc is handled by the focused editable visual mode.
/// - The first Esc does not invoke the cancel callback.
/// - The second Esc is handled by the form and invokes cancel.
#[test]
fn form_esc_leaves_visual_modes_before_canceling() -> Result<()> {
    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut input_view = input("Ada").with_focus(true);
    editable_state_mut(&mut input_view).set_mode(VimMode::Visual);
    editable_state_mut(&mut input_view).set_selection_anchor(Some(0));
    let mut view = form([input_view]).on_cancel(move || {
        cancels_for_form.set(cancels_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 0);
    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Normal);
    assert_eq!(
        editable_state(form_child(&view, 0)).selection_anchor(),
        None
    );

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 1);

    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut text_area_view = text_area("one\ntwo").with_focus(true);
    editable_state_mut(&mut text_area_view).set_cursor(4);
    editable_state_mut(&mut text_area_view).set_mode(VimMode::VisualLine);
    editable_state_mut(&mut text_area_view).set_selection_anchor(Some(4));
    let mut view = form([text_area_view]).on_cancel(move || {
        cancels_for_form.set(cancels_for_form.get() + 1);
        AppControl::Continue
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 0);
    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Normal);
    assert_eq!(
        editable_state(form_child(&view, 0)).selection_anchor(),
        None
    );

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 1);

    Ok(())
}

/// Verifies forms inside component boundaries handle submit keys.
///
/// # Example Under Test
///
/// ```text
/// component(FocusPanel { view: form([focused input]).on_submit(...) })
/// Enter
/// ```
///
/// # Assertions
///
/// - Enter is handled through the component boundary.
/// - The nested form submit callback runs once.
#[test]
fn form_inside_component_boundary_handles_submit_key() -> Result<()> {
    let submits = Rc::new(Cell::new(0));
    let submits_for_form = Rc::clone(&submits);
    let view = form([input("Ada").with_focus(true)]).on_submit(move || {
        submits_for_form.set(submits_for_form.get() + 1);
        AppControl::Continue
    });
    let mut view = component(FocusPanel {
        view: view.into_view(),
    });

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(submits.get(), 1);

    Ok(())
}

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
fn focused_text_area_without_callback_does_not_mutate_displayed_value() -> Result<()> {
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
fn focused_input_inside_component_boundary_handles_editing_keys() -> Result<()> {
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
fn focused_text_area_inside_component_boundary_handles_editing_keys() -> Result<()> {
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

/// Verifies activation keys do not activate focused editable controls.
///
/// # Example Under Test
///
/// ```text
/// column([Input, button("Submit")])
/// Tab, Enter, Space, Tab, Enter
/// ```
///
/// # Assertions
///
/// - The editable input receives focus.
/// - Enter and space return [`AppControl::Continue`] without running callbacks
///   while the editable input is focused.
/// - The button receives focus after another tab event.
/// - Enter returns [`AppControl::Continue`] and runs the focused button callback.
#[test]
fn enter_and_space_do_not_activate_focused_editable_controls() -> Result<()> {
    let count = Rc::new(Cell::new(0));
    let submit_count = Rc::clone(&count);
    let mut view = column((
        editable_input("Ada"),
        button("Submit").on_press(move || {
            submit_count.set(submit_count.get() + 1);
            AppControl::Continue
        }),
    ));

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![true, false]);

    assert_eq!(
        view.handle_event(key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(
        view.handle_event(key(KeyCode::Char(' ')))?,
        AppControl::Continue
    );
    assert_eq!(count.get(), 0);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![false, true]);
    assert_eq!(
        view.handle_event(key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(count.get(), 1);

    Ok(())
}

/// Verifies focused button actions can request app exit.
///
/// # Example Under Test
///
/// ```text
/// button("Quit").on_press(|| AppControl::Exit)
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - The tab event succeeds and focuses the button.
/// - The enter event returns [`AppControl::Exit`].
#[test]
fn focused_button_action_can_exit_app_loop() -> Result<()> {
    let mut view = button("Quit").on_press(|| AppControl::Exit);

    view.handle_event(key(KeyCode::Tab))?;

    assert_eq!(view.handle_event(key(KeyCode::Enter))?, AppControl::Exit);

    Ok(())
}

/// Verifies focused buttons render with focus stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// row([button("One"), button("Two")])
/// Stylesheet::new().rule(StyleSelector::focus(), black on yellow)
/// with_focus(true)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rendered focused button label exists.
/// - The focused cell has a black foreground.
/// - The focused cell has a yellow background.
///
/// # Why
///
/// Focus selector state should affect rendered button styling.
#[test]
fn renders_focused_button_with_focus_stylesheet_rule() -> Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let view = row([button("One").with_focus(true), button("Two")]);
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::focus(),
        TuiStyle::new()
            .foreground(Color::Black)
            .background(Color::Yellow),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let focused_cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "O")
        .expect("rendered focused button label");

    assert_eq!(focused_cell.fg, Color::Black);
    assert_eq!(focused_cell.bg, Color::Yellow);

    Ok(())
}

/// Component that forwards rendering and built-in focus traversal to a child view.
struct FocusPanel {
    /// Child view owned by this component boundary.
    view: AnyView,
}

impl View for FocusPanel {
    /// Renders the child view.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the view boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        ctx.render_view(&self.view)
    }

    /// Returns the minimum useful render height of the child view.
    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        self.view.__min_height(ctx)
    }

    /// Returns the number of focusable controls inside the child view.
    #[doc(hidden)]
    fn __focusable_count(&self) -> usize {
        self.view.__focusable_count()
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.view.__focused_index_inner(index)
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        self.view.__set_focus_by_index_inner(target, index);
    }

    /// Returns the focused control's vertical span inside the child view.
    #[doc(hidden)]
    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        self.view.__focused_button_span(ctx)
    }

    /// Activates the focused control inside the child view, if any.
    #[doc(hidden)]
    fn __activate_focused_button(&self) -> Option<AppControl> {
        self.view.__activate_focused_button()
    }

    /// Handles keys for a focused input inside the child view, if any.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to apply to the focused child input.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the key control result when an input handles
    /// the key.
    #[doc(hidden)]
    fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.view.__handle_focused_input_key(key)
    }

    /// Returns the focused built-in control inside the child view.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing focused control metadata when a supported
    /// built-in control is focused.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<leptatui::__private::FocusedControl> {
        self.view.__focused_control()
    }

    /// Handles form-owned submit or cancel keys inside the child view.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to evaluate for nested form behavior.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing key traversal control when a nested form
    /// handles the key.
    #[doc(hidden)]
    fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.view.__handle_form_key(key)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Component that renders text and exits on any event.
struct EventExit;

impl View for EventExit {
    /// Renders the component's child text.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the view boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        View::render(&text("Child"), ctx)
    }

    /// Handles an event by requesting app exit.
    ///
    /// # Arguments
    ///
    /// * `_event` — Event dispatched through the view tree.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value requesting exit.
    fn __dispatch_event(&mut self, _event: &Event) -> Result<AppControl> {
        Ok(AppControl::Exit)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Component that counts how many events it receives.
struct EventCounter {
    /// Shared event count updated by event handling.
    count: Rc<Cell<usize>>,
}

impl View for EventCounter {
    /// Renders nothing for event-only tests.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Rendering context supplied by the view boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }

    /// Handles an event by incrementing the shared count.
    ///
    /// # Arguments
    ///
    /// * `_event` — Event dispatched through the view tree.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value requesting continued traversal.
    fn __dispatch_event(&mut self, _event: &Event) -> Result<AppControl> {
        self.count.set(self.count.get() + 1);
        Ok(AppControl::Continue)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Component that records selector metadata from a rendered child view.
struct MetadataRecorder {
    /// Shared slot receiving child selector metadata.
    seen: Rc<RefCell<Option<StyleMetadata>>>,
}

impl View for MetadataRecorder {
    /// Renders a child view and records its selector metadata.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the component boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let view = text("Child")
            .with_id("inside")
            .with_classes("component-child");
        *self.seen.borrow_mut() = view.style_metadata().cloned();
        View::render(&view, ctx)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
