/// View that exits when `q` is pressed.
#[component]
fn MacroKeyExitRoot() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    text("Press q")
}

/// Parent key map used to prove child handlers get priority.
#[component]
fn MacroParentKeyRoot() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('x') {
            MACRO_PARENT_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    component(MacroChildKeyHandler::new())
}

/// Child key map that handles the same key as its parent.
#[component]
fn MacroChildKeyHandler() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('x') {
            MACRO_CHILD_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    text("Child")
}

/// Parent key map used to prove child pass-through reaches ancestors.
#[component]
fn MacroParentAfterPassRoot() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('p') {
            MACRO_PASS_PARENT_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    component(MacroPassingChildKeyHandler::new())
}

/// Child key map that observes a key but passes it to its parent.
#[component]
fn MacroPassingChildKeyHandler() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('p') {
            MACRO_PASS_CHILD_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
        }

        KeyControl::Pass
    });

    text("Child")
}

/// View with several handlers used to prove source-order short-circuiting.
#[component]
fn MacroMultipleKeyHandlers() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('m') {
            MACRO_FIRST_KEY_HANDLER.fetch_add(1, Ordering::SeqCst);
        }

        KeyControl::Pass
    });
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('m') {
            MACRO_SECOND_KEY_HANDLER.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('m') {
            MACRO_THIRD_KEY_HANDLER.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    text("Handlers")
}

/// View with explicit repeat and release key handlers.
#[component]
fn MacroKindSpecificKeyHandlers() -> impl leptatui::IntoView {
    use_key_event(KeyEventKind::Repeat, |key| {
        if key.code == KeyCode::Char('k') {
            MACRO_REPEAT_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });
    use_key_event(KeyEventKind::Release, |key| {
        if key.code == KeyCode::Char('k') {
            MACRO_RELEASE_KEY_PRESSES.fetch_add(1, Ordering::SeqCst);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    text("Kinds")
}
