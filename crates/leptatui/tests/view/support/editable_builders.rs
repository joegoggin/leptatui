/// Creates an editable input test view.
///
/// # Arguments
///
/// * `value` — Caller-owned value to display in the input.
///
/// # Returns
///
/// A [`View`] containing an input with fresh editable state.
fn editable_input(value: impl Into<String>) -> InputView {
    input(value)
}

/// Creates an editable text-area test view.
///
/// # Arguments
///
/// * `value` — Caller-owned value to display in the text area.
///
/// # Returns
///
/// A [`View`] containing a text area with fresh editable state.
fn editable_text_area(value: impl Into<String>) -> TextAreaView {
    text_area(value)
}

/// Creates non-default editable state for reconciliation tests.
///
/// # Returns
///
/// An [`EditableState`] value containing cursor, scroll, mode, selection, yank,
/// undo, and redo state.
fn editable_state_fixture() -> EditableState {
    let mut state = EditableState::new();
    state.set_cursor(6);
    state.set_horizontal_scroll(2);
    state.set_vertical_scroll(3);
    state.set_mode(VimMode::Visual);
    state.set_selection_anchor(Some(2));
    state.set_yank_buffer("copied");
    state.push_undo("before");
    state.push_redo("after");
    state
}

/// Returns editable state stored by an editable test view.
///
/// Panics if `view` is not an editable control.
///
/// # Arguments
///
/// * `view` — Editable view to inspect.
///
/// # Returns
///
/// An [`EditableState`] reference retained by the view.
fn editable_state(view: &dyn View) -> &EditableState {
    if let Some(view) = view.as_any().downcast_ref::<InputView>() {
        return view.editable_state();
    }
    view.as_any()
        .downcast_ref::<TextAreaView>()
        .expect("expected editable view")
        .editable_state()
}

/// Returns mutable editable state stored by an editable test view.
///
/// Panics if `view` is not an editable control.
///
/// # Arguments
///
/// * `view` — Editable view to mutate.
///
/// # Returns
///
/// An [`EditableState`] reference retained by the view.
fn editable_state_mut(view: &mut dyn View) -> &mut EditableState {
    if view.as_any().is::<InputView>() {
        return view
            .as_any_mut()
            .downcast_mut::<InputView>()
            .expect("expected input view")
            .editable_state_mut();
    }
    view.as_any_mut()
        .downcast_mut::<TextAreaView>()
        .expect("expected text-area view")
        .editable_state_mut()
}
