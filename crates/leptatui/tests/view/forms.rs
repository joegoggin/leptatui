/// Verifies form views render children and participate in focus traversal.
///
/// # Example Under Test
///
/// ```text
/// form([text("Title"), input("Ada"), button("Save")])
/// Tab, Tab
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The title renders before the input value.
/// - The form reports two focusable descendant controls.
/// - Tab moves focus from the input to the button.
#[test]
fn renders_form_children_and_moves_focus_through_descendants() -> Result<()> {
    let backend = TestBackend::new(12, 7);
    let mut terminal = Terminal::new(backend)?;
    let mut view = form((text("Title"), input("Ada"), button("Save")));

    draw_view(&mut terminal, &view)?;
    let title_position = symbol_position(&terminal, "T", 12);
    let input_position = symbol_position(&terminal, "A", 12);
    assert_eq!(title_position, (0, 0));
    assert_eq!(input_position.0, 1);
    assert!(input_position.1 > title_position.1);
    assert_eq!(view.__focusable_count(), 2);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![true, false]);
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![false, true]);

    Ok(())
}

/// Verifies focusing an editable control starts it in normal mode.
///
/// # Example Under Test
///
/// ```text
/// form([input("Ada").with_focus(true), button("Save")])
/// Tab, Tab
/// ```
///
/// # Assertions
///
/// - Moving focus away does not discard retained editable state.
/// - Moving focus back to the input switches it to normal mode.
/// - Cursor and yank buffer state are preserved.
#[test]
fn focusing_editable_control_enters_normal_mode_without_resetting_state() -> Result<()> {
    let mut input_view = input("Ada").with_focus(true);
    editable_state_mut(&mut input_view).set_mode(VimMode::Insert);
    editable_state_mut(&mut input_view).set_cursor(1);
    editable_state_mut(&mut input_view).set_yank_buffer("copy");
    let mut view = form((input_view, button("Save")));

    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Insert);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![false, true]);
    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Insert);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![true, false]);
    assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Normal);
    assert_eq!(editable_state(form_child(&view, 0)).cursor(), 1);
    assert_eq!(editable_state(form_child(&view, 0)).yank_buffer(), "copy");

    Ok(())
}

/// Verifies form type stylesheet rules apply through rendered descendants.
///
/// # Example Under Test
///
/// ```text
/// Form { fg: Green }
/// form([text("Hi")])
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The rendered text cell inherits the form foreground color.
#[test]
fn form_type_styles_apply_to_descendants() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = form([text("Hi")]);
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::Form),
        TuiStyle::new().foreground(Color::Green),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, _) = cell_colors(&terminal, 0, 0, 8);
    assert_eq!(fg, Color::Green);

    Ok(())
}

/// Verifies controlled form edits update caller state and reconcile into views.
///
/// # Example Under Test
///
/// ```text
/// form([input(name), text_area(notes), button("Submit")])
/// Tab, A, Char('!'), reconcile, Tab, A, Enter, reconcile
/// :focus { fg: Black, bg: Yellow }
/// ```
///
/// # Assertions
///
/// - The form reports input, text area, and button as focusable controls.
/// - Input edits update caller-owned state without mutating the stale view.
/// - Reconciliation displays the latest caller-owned input value and retains
///   focus and cursor state.
/// - Text-area edits follow the same controlled update and reconciliation path.
/// - The focused text area receives focus stylesheet colors after reconciliation.
#[test]
fn controlled_form_reconciles_values_focus_and_rendering() -> Result<()> {
    let name = Rc::new(RefCell::new(String::from("Ada")));
    let notes = Rc::new(RefCell::new(String::from("Notes")));
    let submits = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let mut view = controlled_form_view(&name, &notes, &submits, &cancels);

    assert_eq!(view.__focusable_count(), 3);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![true, false, false]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('A')))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('!')))?,
        KeyControl::Handled
    );
    assert_eq!(name.borrow().as_str(), "Ada!");
    assert_eq!(input_value(form_child(&view, 0)), "Ada");

    view = reconcile_controlled_form(&view, &name, &notes, &submits, &cancels);

    assert_eq!(input_value(form_child(&view, 0)), "Ada!");
    assert_eq!(editable_state(form_child(&view, 0)).cursor(), 4);
    assert_eq!(control_focuses(&view), vec![true, false, false]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![false, true, false]);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('A')))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(notes.borrow().as_str(), "Notes\n");
    assert_eq!(text_area_value(form_child(&view, 1)), "Notes");

    view = reconcile_controlled_form(&view, &name, &notes, &submits, &cancels);

    assert_eq!(text_area_value(form_child(&view, 1)), "Notes\n");
    assert_eq!(editable_state(form_child(&view, 1)).cursor(), 6);
    assert_eq!(control_focuses(&view), vec![false, true, false]);

    let backend = TestBackend::new(20, 6);
    let mut terminal = Terminal::new(backend)?;
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

    assert!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.fg == Color::Black && cell.bg == Color::Yellow)
    );

    Ok(())
}

/// Verifies form-owned submit and cancel keys route around editable controls.
///
/// # Example Under Test
///
/// ```text
/// form([input(name), text_area(notes), button("Submit")])
/// Input focus: Enter, i, Esc, Esc
/// TextArea focus: A, Enter, reconcile, Ctrl+Enter
/// ```
///
/// # Assertions
///
/// - Enter submits a form when the input is focused.
/// - Esc leaves input insert mode without canceling the form.
/// - Esc in normal mode invokes the form cancel callback.
/// - Plain Enter inserts a newline when the text area is in insert mode.
/// - Ctrl+Enter submits a form when the text area is focused.
#[test]
fn controlled_form_routes_submit_and_cancel_keys() -> Result<()> {
    let name = Rc::new(RefCell::new(String::from("Ada")));
    let notes = Rc::new(RefCell::new(String::from("Notes")));
    let submits = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let mut input_view = controlled_form_view(&name, &notes, &submits, &cancels);

    input_view.handle_key_event(key_event(KeyCode::Tab))?;
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(submits.get(), 1);
    assert_eq!(cancels.get(), 0);

    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Char('i')))?,
        KeyControl::Handled
    );
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(
        editable_state(form_child(&input_view, 0)).mode(),
        VimMode::Normal
    );
    assert_eq!(cancels.get(), 0);

    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Esc))?,
        KeyControl::Handled
    );
    assert_eq!(cancels.get(), 1);

    let name = Rc::new(RefCell::new(String::from("Ada")));
    let notes = Rc::new(RefCell::new(String::from("Notes")));
    let submits = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let mut text_area_view = controlled_form_view(&name, &notes, &submits, &cancels);

    text_area_view.handle_key_event(key_event(KeyCode::Tab))?;
    text_area_view.handle_key_event(key_event(KeyCode::Tab))?;
    assert_eq!(
        text_area_view.handle_key_event(key_event(KeyCode::Char('A')))?,
        KeyControl::Handled
    );
    assert_eq!(
        text_area_view.handle_key_event(key_event(KeyCode::Enter))?,
        KeyControl::Handled
    );
    assert_eq!(notes.borrow().as_str(), "Notes\n");
    assert_eq!(submits.get(), 0);

    text_area_view = reconcile_controlled_form(&text_area_view, &name, &notes, &submits, &cancels);

    assert_eq!(
        text_area_view.handle_key_event(ctrl_enter_key_event())?,
        KeyControl::Handled
    );
    assert_eq!(notes.borrow().as_str(), "Notes\n");
    assert_eq!(submits.get(), 1);

    Ok(())
}

/// Verifies controlled form Vim edits survive reconciled redraws.
///
/// # Example Under Test
///
/// ```text
/// Input focus: 0, l, x, reconcile, u, reconcile, Ctrl+r
/// TextArea focus: gg, j, dd, reconcile, u
/// ```
///
/// # Assertions
///
/// - Normal-mode input deletion updates caller-owned state.
/// - Reconciliation retains input undo and redo history.
/// - Normal-mode text-area line deletion updates caller-owned state.
/// - Reconciliation retains the linewise yank buffer for text-area undo.
#[test]
fn controlled_form_preserves_vim_state_across_reconciliation() -> Result<()> {
    let name = Rc::new(RefCell::new(String::from("abc")));
    let notes = Rc::new(RefCell::new(String::from("notes")));
    let submits = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let mut input_view = controlled_form_view(&name, &notes, &submits, &cancels);

    input_view.handle_key_event(key_event(KeyCode::Tab))?;
    input_view.handle_key_event(key_event(KeyCode::Char('0')))?;
    input_view.handle_key_event(key_event(KeyCode::Char('l')))?;
    assert_eq!(
        input_view.handle_key_event(key_event(KeyCode::Char('x')))?,
        KeyControl::Handled
    );
    assert_eq!(name.borrow().as_str(), "ac");

    input_view = reconcile_controlled_form(&input_view, &name, &notes, &submits, &cancels);
    assert_eq!(input_value(form_child(&input_view, 0)), "ac");
    assert_eq!(
        editable_state(form_child(&input_view, 0)).undo_stack(),
        &[String::from("abc")]
    );

    input_view.handle_key_event(key_event(KeyCode::Char('u')))?;
    assert_eq!(name.borrow().as_str(), "abc");

    input_view = reconcile_controlled_form(&input_view, &name, &notes, &submits, &cancels);
    assert_eq!(
        input_view.handle_key_event(ctrl_key_event('r'))?,
        KeyControl::Handled
    );
    assert_eq!(name.borrow().as_str(), "ac");

    let name = Rc::new(RefCell::new(String::from("Ada")));
    let notes = Rc::new(RefCell::new(String::from("one\ntwo\nthree")));
    let submits = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let mut text_area_view = controlled_form_view(&name, &notes, &submits, &cancels);

    text_area_view.handle_key_event(key_event(KeyCode::Tab))?;
    text_area_view.handle_key_event(key_event(KeyCode::Tab))?;
    text_area_view.handle_key_event(key_event(KeyCode::Char('g')))?;
    text_area_view.handle_key_event(key_event(KeyCode::Char('g')))?;
    text_area_view.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(
        text_area_view.handle_key_event(key_event(KeyCode::Char('d')))?,
        KeyControl::Handled
    );
    assert_eq!(
        text_area_view.handle_key_event(key_event(KeyCode::Char('d')))?,
        KeyControl::Handled
    );
    assert_eq!(notes.borrow().as_str(), "one\nthree");

    text_area_view = reconcile_controlled_form(&text_area_view, &name, &notes, &submits, &cancels);
    assert_eq!(
        text_area_value(form_child(&text_area_view, 1)),
        "one\nthree"
    );
    assert_eq!(
        editable_state(form_child(&text_area_view, 1)).yank_buffer(),
        "two"
    );

    text_area_view.handle_key_event(key_event(KeyCode::Char('u')))?;
    assert_eq!(notes.borrow().as_str(), "one\ntwo\nthree");

    Ok(())
}
