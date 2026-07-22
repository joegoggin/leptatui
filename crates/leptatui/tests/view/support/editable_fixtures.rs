/// Creates a focused input view that records emitted values.
///
/// # Arguments
///
/// * `value` — Initial controlled input value.
/// * `emitted` — Shared vector that receives callback values.
///
/// # Returns
///
/// A focused insert-mode [`View`] configured as an input.
fn emitting_input(value: impl Into<String>, emitted: &Rc<RefCell<Vec<String>>>) -> InputView {
    let emitted_for_input = Rc::clone(emitted);
    let mut view = input(value).with_focus(true).on_input(move |next| {
        emitted_for_input.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut view).set_mode(VimMode::Insert);
    view
}

/// Creates a focused text-area view that records emitted values.
///
/// # Arguments
///
/// * `value` — Initial controlled text-area value.
/// * `emitted` — Shared vector that receives callback values.
///
/// # Returns
///
/// A focused insert-mode [`View`] configured as a text area.
fn emitting_text_area(
    value: impl Into<String>,
    emitted: &Rc<RefCell<Vec<String>>>,
) -> TextAreaView {
    let emitted_for_text_area = Rc::clone(emitted);
    let mut view = text_area(value).with_focus(true).on_input(move |next| {
        emitted_for_text_area.borrow_mut().push(next);
        AppControl::Continue
    });
    editable_state_mut(&mut view).set_mode(VimMode::Insert);
    view
}

/// Returns a reconciled input view with a new controlled value.
///
/// # Arguments
///
/// * `previous` — Previous input view whose retained metadata should be reused.
/// * `value` — Next controlled input value.
/// * `emitted` — Shared vector that receives callback values.
///
/// # Returns
///
/// A [`View`] containing the reconciled input.
fn reconcile_input_value(
    previous: &InputView,
    value: impl Into<String>,
    emitted: &Rc<RefCell<Vec<String>>>,
) -> InputView {
    let mut next = emitting_input(value, emitted);
    leptatui::__private::__reconcile_view(&mut next, previous);
    next
}

/// Returns a reconciled text-area view with a new controlled value.
///
/// # Arguments
///
/// * `previous` — Previous text-area view whose retained metadata should be
///   reused.
/// * `value` — Next controlled text-area value.
/// * `emitted` — Shared vector that receives callback values.
///
/// # Returns
///
/// A [`View`] containing the reconciled text area.
fn reconcile_text_area_value(
    previous: &TextAreaView,
    value: impl Into<String>,
    emitted: &Rc<RefCell<Vec<String>>>,
) -> TextAreaView {
    let mut next = emitting_text_area(value, emitted);
    leptatui::__private::__reconcile_view(&mut next, previous);
    next
}

/// Creates a controlled form test view backed by shared caller-owned state.
///
/// # Arguments
///
/// * `name` — Shared controlled input value.
/// * `notes` — Shared controlled text-area value.
/// * `submits` — Shared form submit count.
/// * `cancels` — Shared form cancel count.
///
/// # Returns
///
/// A [`View`] containing a form with an input, text area, and submit button.
fn controlled_form_view(
    name: &Rc<RefCell<String>>,
    notes: &Rc<RefCell<String>>,
    submits: &Rc<Cell<usize>>,
    cancels: &Rc<Cell<usize>>,
) -> FormView {
    let name_value = name.borrow().clone();
    let notes_value = notes.borrow().clone();
    let name_for_input = Rc::clone(name);
    let notes_for_text_area = Rc::clone(notes);
    let submits_for_form = Rc::clone(submits);
    let cancels_for_form = Rc::clone(cancels);

    form((
        input(name_value).placeholder("Name").on_input(move |next| {
            *name_for_input.borrow_mut() = next;
            AppControl::Continue
        }),
        text_area(notes_value)
            .placeholder("Notes")
            .on_input(move |next| {
                *notes_for_text_area.borrow_mut() = next;
                AppControl::Continue
            }),
        button("Submit"),
    ))
    .on_submit(move || {
        submits_for_form.set(submits_for_form.get() + 1);
        AppControl::Continue
    })
    .on_cancel(move || {
        cancels_for_form.set(cancels_for_form.get() + 1);
        AppControl::Continue
    })
}

/// Returns a reconciled controlled form from the latest shared state.
///
/// # Arguments
///
/// * `previous` — Previous controlled form view.
/// * `name` — Shared controlled input value.
/// * `notes` — Shared controlled text-area value.
/// * `submits` — Shared form submit count.
/// * `cancels` — Shared form cancel count.
///
/// # Returns
///
/// A [`View`] containing the next controlled form with retained editable state.
fn reconcile_controlled_form(
    previous: &FormView,
    name: &Rc<RefCell<String>>,
    notes: &Rc<RefCell<String>>,
    submits: &Rc<Cell<usize>>,
    cancels: &Rc<Cell<usize>>,
) -> FormView {
    let mut next = controlled_form_view(name, notes, submits, cancels);
    leptatui::__private::__reconcile_view(&mut next, previous);
    next
}

/// Returns a child from a controlled form by index.
///
/// # Arguments
///
/// * `view` — Form view to inspect.
/// * `index` — Child index to return.
///
/// # Returns
///
/// A [`View`] reference for the requested form child.
fn form_child(view: &FormView, index: usize) -> &dyn View {
    view.children()[index].as_view()
}

/// Returns the controlled value from an input view.
///
/// # Arguments
///
/// * `view` — Input view to inspect.
///
/// # Returns
///
/// A string slice containing the input's controlled value.
fn input_value(view: &dyn View) -> &str {
    let editor = view
        .as_any()
        .downcast_ref::<InputView>()
        .expect("expected input view");
    editor.value()
}

/// Returns the controlled value from a text-area view.
///
/// # Arguments
///
/// * `view` — Text-area view to inspect.
///
/// # Returns
///
/// A string slice containing the text area's controlled value.
fn text_area_value(view: &dyn View) -> &str {
    let editor = view
        .as_any()
        .downcast_ref::<TextAreaView>()
        .expect("expected text-area view");
    editor.value()
}
