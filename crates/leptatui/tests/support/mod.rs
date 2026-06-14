//! Shared helpers for Leptatui integration tests.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use leptatui::{Component, RenderCtx, Result};
use ratatui::{Terminal, backend::TestBackend};

/// Creates a key-press event for a key code.
pub(crate) fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Renders a component into a test backend.
pub(crate) fn render_component<C>(
    component: &mut C,
    width: u16,
    height: u16,
) -> Result<Terminal<TestBackend>>
where
    C: Component,
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = Component::render(component, &mut ctx);
    })?;
    render_result?;

    Ok(terminal)
}

/// Returns rendered terminal text as a flat string.
pub(crate) fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}
