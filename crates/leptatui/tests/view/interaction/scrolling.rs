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

/// Verifies focus scrolling follows the word wrapping used for Markdown links.
///
/// # Example Under Test
///
/// ```text
/// 123456 [Link](https://example.com), viewport = 10x1
/// Tab, render
/// ```
///
/// # Assertions
///
/// - Focusing the wrapped link scrolls the Markdown view down one row.
/// - The focused link is visible at the start of the viewport.
#[test]
fn focused_word_wrapped_markdown_link_scrolls_into_view() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(10, 1))?;
    let mut view = markdown("123456 [Link](https://example.com)");

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, view.as_view())?;

    assert_eq!(scroll_offset(view.as_view()), 1);
    assert_eq!(cell_symbol(&terminal, 0, 0, 10), "L");

    Ok(())
}
