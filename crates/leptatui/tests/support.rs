//! Shared helpers for Leptatui integration tests.
#![allow(dead_code)]

use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use leptatui::{
    AnyView, BoxSizing, ContainerView, Dimension, DivView, GridTemplateTrack, GridTrackSize,
    LayoutSize, Length, RenderCtx, StyleMetadata, TuiStyle, View,
};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
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

/// Returns a definite border-box size for a conformance fixture.
///
/// # Arguments
///
/// * `width` — Width in terminal cells.
/// * `height` — Height in terminal cells.
///
/// # Returns
///
/// A [`TuiStyle`] containing the requested border-box size.
pub(crate) fn fixture_size(width: f32, height: f32) -> TuiStyle {
    TuiStyle::new()
        .box_sizing(BoxSizing::BorderBox)
        .size(LayoutSize::new(
            Dimension::from(Length::cells(width)),
            Dimension::from(Length::cells(height)),
        ))
}

/// Creates a fixed grid track measured in terminal cells.
///
/// # Arguments
///
/// * `cells` — Track size in terminal cells.
///
/// # Returns
///
/// A [`GridTemplateTrack`] containing the fixed size.
pub(crate) fn fixed_grid_track(cells: f32) -> GridTemplateTrack {
    GridTemplateTrack::from(GridTrackSize::from(Length::cells(cells)))
}

/// Returns retained child rectangles from an erased layout container.
///
/// # Arguments
///
/// * `root` — Erased division view rendered by a conformance fixture.
///
/// # Returns
///
/// A [`Vec`] containing child border boxes in source order.
pub(crate) fn retained_child_rects(root: &AnyView) -> Vec<Rect> {
    root.downcast_ref::<DivView>()
        .expect("fixture root should be a DivView")
        .child_views()
        .iter()
        .map(|child| {
            child
                .style_metadata()
                .and_then(StyleMetadata::layout_geometry)
                .expect("fixture child should retain layout geometry")
                .border_box
        })
        .collect()
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
) -> leptatui::app::Result<Terminal<TestBackend>>
where
    C: View,
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
) -> leptatui::app::Result<()>
where
    C: View,
{
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = View::render(component, &mut ctx);
    })?;

    render_result
}

/// Renders a view into an existing test terminal.
///
/// # Arguments
///
/// * `terminal` — Test terminal used as the render target.
/// * `view` — View tree to render.
///
/// # Returns
///
/// An empty [`Result`] when terminal drawing and view rendering succeed.
///
/// # Errors
///
/// Returns [`leptatui::Error::Io`] if terminal drawing or view rendering fails.
pub(crate) fn draw_view(
    terminal: &mut Terminal<TestBackend>,
    view: &dyn View,
) -> leptatui::app::Result<()> {
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;

    render_result
}

/// Renders a view into a fixed-size test terminal.
///
/// # Arguments
///
/// * `view` — View tree to render.
/// * `width` — Terminal width in cells.
/// * `height` — Terminal height in cells.
///
/// # Returns
///
/// A [`Terminal`] containing the rendered view output.
///
/// # Errors
///
/// Returns [`leptatui::Error::Io`] if terminal drawing or view rendering fails.
pub(crate) fn render_view(
    view: &dyn View,
    width: u16,
    height: u16,
) -> leptatui::app::Result<Terminal<TestBackend>> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    draw_view(&mut terminal, view)?;
    Ok(terminal)
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

/// Returns terminal symbols grouped by rendered row.
///
/// # Arguments
///
/// * `terminal` — Test terminal whose buffer should be inspected.
///
/// # Returns
///
/// A [`Vec`] containing one symbol string for each terminal row.
pub(crate) fn rendered_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let width = usize::from(terminal.backend().buffer().area.width);
    if width == 0 {
        return Vec::new();
    }

    terminal
        .backend()
        .buffer()
        .content()
        .chunks(width)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect())
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
