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
        <Div style={TuiStyle::new().display(Display::Flex)}>
            <MacroWrappedButton label="First" on_press=macro_first_wrapped_button_press />
            <MacroWrappedButton label="Second" on_press=macro_second_wrapped_button_press />
        </Div>
    }
}

/// Root with a built-in button and a custom button component.
#[component]
fn MacroMixedButtonSiblings() -> impl leptatui::IntoView {
    view! {
        <Div style={TuiStyle::new().display(Display::Flex)}>
            <Button on_press={macro_mixed_builtin_button_press}>"Built in"</Button>
            <MacroWrappedButton label="Wrapped" on_press=macro_mixed_wrapped_button_press />
        </Div>
    }
}

/// Root that rebuilds a prop-bearing button component on every draw.
///
/// # Returns
///
/// A dynamic Leptatui view containing the current wrapped button.
#[component]
fn MacroDynamicWrappedButtonRoot() -> impl leptatui::IntoView {
    let (label, set_label) = signal(String::from("Before"));
    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('u') {
            set_label.update(|label| *label = String::from("After"));
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    dynamic(move || {
        view! {
            <MacroWrappedButton
                label=label.get()
                on_press=macro_dynamic_wrapped_button_press
            />
        }
    })
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
