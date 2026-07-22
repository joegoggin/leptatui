/// Returns an unmodified key event for a test key code.
///
/// # Arguments
///
/// * `code` — Key code to wrap in a [`KeyEvent`].
///
/// # Returns
///
/// A [`KeyEvent`] value without modifiers.
fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Returns a control-modified character key event for tests.
///
/// # Arguments
///
/// * `character` — Character to wrap in a control-modified [`KeyEvent`].
///
/// # Returns
///
/// A [`KeyEvent`] value with the control modifier set.
fn ctrl_key_event(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

/// Returns a control-modified enter key event for tests.
///
/// # Returns
///
/// A [`KeyEvent`] value with enter and the control modifier set.
fn ctrl_enter_key_event() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL)
}
