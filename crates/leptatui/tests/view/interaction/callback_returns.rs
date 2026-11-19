/// Verifies control builders normalize unit and explicit app-control returns.
///
/// # Example Under Test
///
/// ```text
/// Button, dynamic Button, Input, TextArea, and Form callbacks
/// return (), AppControl::Continue, or AppControl::Exit.
/// ```
///
/// # Assertions
///
/// - Every callback executes exactly once per dispatched action.
/// - Unit and explicit Continue returns keep the app running.
/// - Explicit Exit returns propagate from every control.
#[test]
fn control_builders_accept_unit_and_explicit_control() -> leptatui::Result<()> {
    check_control_callback_return((), AppControl::Continue)?;
    check_control_callback_return(AppControl::Continue, AppControl::Continue)?;
    check_control_callback_return(AppControl::Exit, AppControl::Exit)?;
    Ok(())
}

/// Checks every control callback using a shared return value.
///
/// # Arguments
///
/// * `result` — Value returned by each callback.
/// * `expected` — Expected app-loop control after dispatch.
///
/// # Returns
///
/// An empty result when all event dispatches succeed.
///
/// # Errors
///
/// Returns an event-dispatch error if a control fails to handle its event.
fn check_control_callback_return<R: Copy + Into<AppControl> + 'static>(
    result: R,
    expected: AppControl,
) -> leptatui::Result<()> {
    Owner::new().with(|| {
        let calls = RwSignal::new(0);
        let next_value = RwSignal::new(String::new());
        let action = move || {
            calls.set(calls.get() + 1);
            result
        };
        let edit = move || {
            calls.set(calls.get() + 1);

            result
        };
        let mut controls = [
            button("Run").with_focus(true).on_press(action).into_view(),
            dynamic(|| button("Run").with_focus(true)).on_press(action).into_view(),
            input(next_value).with_focus(true).on_input(edit).into_view(),
            text_area(next_value).with_focus(true).on_input(edit).into_view(),
            // Exercise the trait method independently of the inherent wrapper.
            leptatui::EditableView::on_input(input(next_value).with_focus(true), edit).into_view(),
            form([input(RwSignal::new(String::from(""))).with_focus(true)]).on_submit(action).into_view(),
            form([input(RwSignal::new(String::from(""))).with_focus(true)]).on_cancel(action).into_view(),
        ];
        for (index, control) in controls.iter_mut().enumerate() {
            next_value.set(String::new());
            let code = match index {
                2..=4 => {
                    assert_eq!(control.handle_event(key(KeyCode::Char('i')))?, AppControl::Continue);
                    KeyCode::Char('x')
                }
                6 => KeyCode::Esc,
                _ => KeyCode::Enter,
            };
            assert_eq!(control.handle_event(key(code))?, expected, "control {index}");
            assert_eq!(calls.get(), index + 1, "control {index}");
            if (2..=4).contains(&index) {
                assert_eq!(next_value.get(), "x");
            }
        }
        Ok(())
    })
}

/// Verifies view attributes accept unit callbacks and explicit exit callbacks.
///
/// # Example Under Test
///
/// ```text
/// Form and Save button use unit callbacks; a Quit button returns Exit.
/// Tab, Enter, Esc, Tab, Enter, Tab, Enter
/// ```
///
/// # Assertions
///
/// - Submission and cancellation run their callbacks and continue.
/// - The save callback updates state and continues.
/// - The quit callback exits.
#[test]
fn view_attributes_accept_unit_and_exit_callbacks() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let status = RwSignal::new("");
        let mut view = view! {
            <Div>
                <Form
                    on_submit=move || status.set("submitted")
                    on_cancel=move || status.set("canceled")
                >
                    <Input value={RwSignal::new(String::new())} on_input=|| {} />
                </Form>
                <Button on_press=move || status.set("saved")>"Save"</Button>
                <Button on_press=|| AppControl::Exit>"Quit"</Button>
            </Div>
        };
        assert_eq!(view.handle_event(key(KeyCode::Tab))?, AppControl::Continue);
        assert_eq!(view.handle_event(key(KeyCode::Enter))?, AppControl::Continue);
        assert_eq!(status.get(), "submitted");
        assert_eq!(view.handle_event(key(KeyCode::Esc))?, AppControl::Continue);
        assert_eq!(status.get(), "canceled");
        assert_eq!(view.handle_event(key(KeyCode::Tab))?, AppControl::Continue);
        assert_eq!(view.handle_event(key(KeyCode::Enter))?, AppControl::Continue);
        assert_eq!(status.get(), "saved");
        assert_eq!(view.handle_event(key(KeyCode::Tab))?, AppControl::Continue);
        assert_eq!(view.handle_event(key(KeyCode::Enter))?, AppControl::Exit);
        Ok(())
    })
}
