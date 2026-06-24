//! Shared helpers for Leptatui integration tests.
#![allow(dead_code)]

use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use leptatui::{Component, RenderCtx, Result};
use ratatui::{Terminal, backend::TestBackend};
use tokio::{task::yield_now, time::timeout};

/// Creates a key-press event for a key code.
///
/// # Arguments
///
/// * `code` — Key code to place in the generated event.
///
/// # Returns
///
/// An [`Event`] containing the key press.
pub(crate) fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Renders a component into a test backend.
///
/// # Arguments
///
/// * `component` — Component to render into the test backend.
/// * `width` — Width of the test backend in terminal cells.
/// * `height` — Height of the test backend in terminal cells.
///
/// # Returns
///
/// A [`Terminal`] containing the rendered component output.
///
/// # Errors
///
/// Returns [`leptatui::Error::Io`] if terminal drawing or component rendering fails.
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

    draw_component(&mut terminal, component)?;

    Ok(terminal)
}

/// Draws a component into an existing test terminal.
///
/// # Arguments
///
/// * `terminal` — Test terminal used as the render target.
/// * `component` — Component to render into the terminal.
///
/// # Returns
///
/// An empty [`Result`] when terminal drawing and component rendering succeed.
///
/// # Errors
///
/// Returns [`leptatui::Error::Io`] if terminal drawing or component rendering fails.
pub(crate) fn draw_component<C>(
    terminal: &mut Terminal<TestBackend>,
    component: &mut C,
) -> Result<()>
where
    C: Component,
{
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = Component::render(component, &mut ctx);
    })?;

    render_result
}

/// Returns rendered terminal text as a flat string.
///
/// # Arguments
///
/// * `terminal` — Test terminal whose buffer should be read.
///
/// # Returns
///
/// A [`String`] containing all rendered terminal symbols.
pub(crate) fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

/// Waits until a predicate becomes true.
///
/// # Arguments
///
/// * `predicate` — Condition polled between Tokio task yields.
pub(crate) async fn wait_until(mut predicate: impl FnMut() -> bool) {
    timeout(Duration::from_secs(1), async {
        while !predicate() {
            yield_now().await;
        }
    })
    .await
    .expect("condition should become true");
}

/// Yields repeatedly so spawned tasks can observe completed channels.
pub(crate) async fn settle_tasks() {
    for _ in 0..10 {
        yield_now().await;
    }
}
