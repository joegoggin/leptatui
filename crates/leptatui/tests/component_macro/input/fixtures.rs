/// View with an interactive button used by macro runtime tests.
#[component]
fn MacroButtonRoot() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('s') {
            MACRO_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    button("Save").on_press(|| {
        MACRO_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}

/// View with no matching hook for default button key tests.
#[component]
fn MacroDefaultButtonRoot() -> impl leptatui::IntoView {
    button("Default").on_press(|| {
        MACRO_DEFAULT_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}

/// View that wraps one built-in button.
#[component]
fn MacroWrappedButton(
    #[prop(into)] label: String,
    on_press: fn() -> AppControl,
) -> impl leptatui::IntoView {
    button(label).on_press(on_press)
}

/// Root with sibling custom button components.
#[component]
fn MacroWrappedButtonSiblings() -> impl leptatui::IntoView {
    view! {
        <Row>
            <MacroWrappedButton label="First" on_press=macro_first_wrapped_button_press />
            <MacroWrappedButton label="Second" on_press=macro_second_wrapped_button_press />
        </Row>
    }
}

/// Root with a built-in button and a custom button component.
#[component]
fn MacroMixedButtonSiblings() -> impl leptatui::IntoView {
    view! {
        <Row>
            <Button on_press={macro_mixed_builtin_button_press}>"Built in"</Button>
            <MacroWrappedButton label="Wrapped" on_press=macro_mixed_wrapped_button_press />
        </Row>
    }
}

/// View whose key map handles Tab before focus can move.
#[component]
fn MacroTabOverrideButtonRoot() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Tab {
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    button("Default").on_press(|| {
        MACRO_DEFAULT_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}

/// View whose key map handles Enter before a focused button activates.
#[component]
fn MacroEnterOverrideButtonRoot() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Enter {
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    button("Default").on_press(|| {
        MACRO_DEFAULT_BUTTON_PRESSES.fetch_add(1, Ordering::SeqCst);
        AppControl::Continue
    })
}
