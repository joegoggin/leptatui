/// Creates a key event for a key code and kind.
fn key_with_kind(code: KeyCode, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind))
}
