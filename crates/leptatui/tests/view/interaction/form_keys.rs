/// Verifies insert-mode Enter keeps inputs single-line and text areas multiline.
///
/// # Example Under Test
///
/// ```text
/// input(RwSignal::new(String::from("Ada"))).with_focus(true).on_input(...)
/// Enter
///
/// text_area(RwSignal::new(String::from("Ada"))).with_focus(true).on_input(...)
/// Enter
/// ```
///
/// # Assertions
///
/// - Enter is handled for a focused input without emitting values.
/// - Enter is handled for a focused text area.
/// - The text-area callback emits the value with a trailing newline.
#[test]
fn insert_mode_keeps_input_single_line_and_text_area_multiline() -> leptatui::app::Result<()> {
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
/// form([input(RwSignal::new(String::from("Ada"))).with_focus(true)]).on_submit(...)
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
fn form_submits_focused_input_on_enter_in_insert_and_normal_mode() -> leptatui::app::Result<()> {
    let insert_submits = Rc::new(Cell::new(0));
    let insert_submits_for_form = Rc::clone(&insert_submits);
    let mut insert_input = input(RwSignal::new(String::from("Ada"))).with_focus(true);
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
    let mut normal_input = input(RwSignal::new(String::from("Ada"))).with_focus(true);
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

/// Verifies insert-mode text areas keep multiline Enter behavior inside forms.
///
/// # Example Under Test
///
/// ```text
/// form([text_area(RwSignal::new(String::from("Ada"))).with_focus(true).on_input(...)])
/// Insert mode: Enter, Ctrl+Enter
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
fn form_text_area_uses_plain_enter_for_newlines_and_ctrl_enter_for_submit() -> leptatui::app::Result<()> {
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
/// form([input(RwSignal::new(String::from("Ada"))).with_focus(true)]).on_cancel(...)
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
fn form_esc_leaves_insert_mode_before_canceling() -> leptatui::app::Result<()> {
    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut input_view = input(RwSignal::new(String::from("Ada"))).with_focus(true);
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
/// form([input(RwSignal::new(String::from("Ada"))).with_focus(true)]).on_cancel(...)
/// j, k, Esc
/// ```
///
/// # Assertions
///
/// - The `jk` sequence is handled by the focused input.
/// - The `jk` sequence does not invoke the cancel callback.
/// - A later Esc is handled by the form.
#[test]
fn form_jk_leaves_insert_mode_without_canceling() -> leptatui::app::Result<()> {
    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut input_view = input(RwSignal::new(String::from("Ada"))).with_focus(true);
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
/// form([input(RwSignal::new(String::from("Ada"))).with_focus(true)]).on_cancel(...)
/// v, Esc, Esc
///
/// form([text_area(RwSignal::new(String::from("one\ntwo"))).with_focus(true)]).on_cancel(...)
/// V, Esc, Esc
/// ```
///
/// # Assertions
///
/// - The first Esc is handled by the focused editable visual mode.
/// - The first Esc does not invoke the cancel callback.
/// - The second Esc is handled by the form and invokes cancel.
#[test]
fn form_esc_leaves_visual_modes_before_canceling() -> leptatui::app::Result<()> {
    let cancels = Rc::new(Cell::new(0));
    let cancels_for_form = Rc::clone(&cancels);
    let mut input_view = input(RwSignal::new(String::from("Ada"))).with_focus(true);
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
    let mut text_area_view = text_area(RwSignal::new(String::from("one\ntwo"))).with_focus(true);
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
fn form_inside_component_boundary_handles_submit_key() -> leptatui::app::Result<()> {
    let submits = Rc::new(Cell::new(0));
    let submits_for_form = Rc::clone(&submits);
    let view = form([input(RwSignal::new(String::from("Ada"))).with_focus(true)]).on_submit(move || {
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

/// Verifies normal-mode text areas submit on Enter without editing their signal.
///
/// # Example Under Test
///
/// ```text
/// Form containing a focused normal-mode TextArea bound to "Ada".
/// on_submit returns Continue or Exit; press Enter, then Ctrl+Enter.
/// ```
///
/// # Assertions
///
/// - Each submit key invokes the form callback exactly once.
/// - The callback's application control propagates to the caller.
/// - Submission preserves the text, cursor, mode, and input-notification count.
#[test]
fn form_text_area_normal_enter_submits_without_editing() -> leptatui::Result<()> {
    Owner::new().with(|| {
        for result in [AppControl::Continue, AppControl::Exit] {
            let value = RwSignal::new(String::from("Ada"));
            let edits = RwSignal::new(0);
            let submits = RwSignal::new(0);
            let editor = text_area(value).with_focus(true).on_input(move || {
                edits.set(edits.get_untracked() + 1);
            });
            let cursor = editor.editable_state().cursor();
            let mut view = form([editor]).on_submit(move || {
                submits.set(submits.get_untracked() + 1);
                result
            });
            for (index, key) in [key_event(KeyCode::Enter), ctrl_enter_key_event()]
                .into_iter()
                .enumerate()
            {
                assert_eq!(view.handle_event(Event::Key(key))?, result);
                assert_eq!(submits.get_untracked(), index + 1);
                assert_eq!(value.get_untracked(), "Ada");
                assert_eq!(edits.get_untracked(), 0);
                assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Normal);
                assert_eq!(editable_state(form_child(&view, 0)).cursor(), cursor);
            }
        }
        Ok(())
    })
}

/// Verifies visual-mode Enter keeps its existing selection behavior.
///
/// # Example Under Test
///
/// ```text
/// Focused TextArea in Visual or VisualLine mode inside a Form.
/// Enter, Ctrl+Enter.
/// ```
///
/// # Assertions
///
/// - Plain Enter does not submit or edit the selected text.
/// - Plain Enter preserves the visual mode and selection anchor.
/// - Ctrl+Enter still submits once in either visual mode.
#[test]
fn form_text_area_visual_enter_does_not_submit() -> leptatui::Result<()> {
    Owner::new().with(|| {
        for mode in [VimMode::Visual, VimMode::VisualLine] {
            let value = RwSignal::new(String::from("Ada"));
            let submits = RwSignal::new(0);
            let mut editor = text_area(value).with_focus(true);
            editor.editable_state_mut().set_mode(mode);
            editor.editable_state_mut().set_selection_anchor(Some(0));
            let mut view = form([editor]).on_submit(move || {
                submits.set(submits.get_untracked() + 1);
            });
            view.handle_key_event(key_event(KeyCode::Enter))?;
            assert_eq!(submits.get_untracked(), 0);
            assert_eq!(value.get_untracked(), "Ada");
            assert_eq!(editable_state(form_child(&view, 0)).mode(), mode);
            assert_eq!(
                editable_state(form_child(&view, 0)).selection_anchor(),
                Some(0)
            );
            view.handle_key_event(ctrl_enter_key_event())?;
            assert_eq!(submits.get_untracked(), 1);
        }
        Ok(())
    })
}

/// Verifies a pending insert-mode escape sequence does not enable submission.
///
/// # Example Under Test
///
/// ```text
/// Focused insert-mode TextArea bound to "Ada" inside a Form.
/// j, Enter.
/// ```
///
/// # Assertions
///
/// - The pending j reports a block cursor without reporting normal mode.
/// - Enter inserts the pending j and a newline, without submitting.
/// - The input notification runs once and the editor stays in insert mode.
#[test]
fn form_text_area_pending_j_enter_inserts_newline() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let value = RwSignal::new(String::from("Ada"));
        let submits = RwSignal::new(0);
        let edits = RwSignal::new(0);
        let mut editor = text_area(value).with_focus(true).on_input(move || {
            edits.set(edits.get_untracked() + 1);
        });
        editor.editable_state_mut().set_mode(VimMode::Insert);
        let mut view = form([editor]).on_submit(move || {
            submits.set(submits.get_untracked() + 1);
        });
        view.handle_key_event(key_event(KeyCode::Char('j')))?;
        assert_eq!(
            view.__focused_control(),
            Some(FocusedControl::TextArea {
                normal_mode: false,
                insert_mode: false,
                visual_mode: false,
            })
        );
        view.handle_key_event(key_event(KeyCode::Enter))?;
        assert_eq!(value.get_untracked(), "Adaj\n");
        assert_eq!(submits.get_untracked(), 0);
        assert_eq!(edits.get_untracked(), 1);
        assert_eq!(editable_state(form_child(&view, 0)).mode(), VimMode::Insert);
        Ok(())
    })
}
