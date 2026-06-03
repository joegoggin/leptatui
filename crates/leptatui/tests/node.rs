//! Node rendering tests.
//!
//! These tests render node trees against Ratatui's test backend and inspect the
//! resulting terminal buffer.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crossterm::event::Event;
use leptatui::{
    AppControl, Color, Component, NodeType, RenderCtx, Result, StyleMetadata, TuiStyle, block,
    button, column, component, dynamic, text,
};
use ratatui::{Terminal, backend::TestBackend};

/// Verifies a block node renders its child text.
///
/// # Example Under Test
///
/// ```text
/// block(text("Hello"))
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The node render call succeeds.
/// - The rendered buffer contains `Hello`.
#[test]
fn renders_block_and_text_nodes() -> Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = block(text("Hello")).render(&mut ctx);
    })?;
    render_result?;

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Hello"));

    Ok(())
}

#[test]
fn node_builders_store_default_selector_metadata() {
    let block_node = block(text("child"));
    let metadata = block_node.style_metadata().expect("block metadata");

    assert_eq!(metadata.node_type(), NodeType::Block);
    assert_eq!(metadata.id(), None);
    assert!(metadata.classes().is_empty());
    assert_eq!(metadata.inline_style(), None);
    assert!(!metadata.is_focused());
}

#[test]
fn node_metadata_setters_store_selector_fields() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let node = button("Save")
        .with_id("save")
        .with_classes("primary toolbar")
        .with_inline_style(style)
        .with_focus(true);
    let metadata = node.style_metadata().expect("button metadata");

    assert_eq!(metadata.node_type(), NodeType::Button);
    assert_eq!(metadata.id(), Some("save"));
    assert_eq!(
        metadata.classes(),
        &[String::from("primary"), String::from("toolbar")]
    );
    assert_eq!(metadata.inline_style(), Some(style));
    assert!(metadata.is_focused());
}

/// Component that renders text and exits on any event.
struct EventExit;

impl Component for EventExit {
    /// Renders the component's child text.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the node boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        ctx.render_node(&text("Child"))
    }

    /// Handles an event by requesting app exit.
    ///
    /// # Arguments
    ///
    /// * `_event` — Event dispatched through the node tree.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value requesting exit.
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        Ok(AppControl::Exit)
    }
}

/// Component that counts how many events it receives.
struct EventCounter {
    /// Shared event count updated by event handling.
    count: Rc<Cell<usize>>,
}

impl Component for EventCounter {
    /// Renders nothing for event-only tests.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Rendering context supplied by the node boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }

    /// Handles an event by incrementing the shared count.
    ///
    /// # Arguments
    ///
    /// * `_event` — Event dispatched through the node tree.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value requesting continued traversal.
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        self.count.set(self.count.get() + 1);
        Ok(AppControl::Continue)
    }
}

struct MetadataRecorder {
    seen: Rc<RefCell<Option<StyleMetadata>>>,
}

impl Component for MetadataRecorder {
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let node = text("Child").with_id("inside").with_classes("component-child");
        *self.seen.borrow_mut() = node.style_metadata().cloned();
        ctx.render_node(&node)
    }
}

#[test]
fn selector_metadata_remains_available_inside_component_boundaries() -> Result<()> {
    let seen = Rc::new(RefCell::new(None));
    let node = component(MetadataRecorder {
        seen: Rc::clone(&seen),
    });
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = node.render(&mut ctx);
    })?;
    render_result?;

    let metadata = seen.borrow().clone().expect("recorded metadata");
    assert_eq!(metadata.id(), Some("inside"));
    assert_eq!(metadata.classes(), &[String::from("component-child")]);

    Ok(())
}

/// Verifies dynamic and component node boundaries render through the node tree.
///
/// # Example Under Test
///
/// ```text
/// column([dynamic(|| text("Dynamic")), component(EventExit)])
/// ```
///
/// # Assertions
///
/// - The dynamic closure is evaluated during rendering.
/// - The component boundary renders through its `Component::render` method.
#[test]
fn renders_dynamic_and_component_child_nodes() -> Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let node = column([dynamic(|| text("Dynamic")), component(EventExit)]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = node.render(&mut ctx);
    })?;
    render_result?;

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Dynamic"));
    assert!(rendered.contains("Child"));

    Ok(())
}

/// Verifies node roots dispatch events through component child boundaries.
///
/// # Example Under Test
///
/// ```text
/// column([text("Static"), component(EventExit)])
///     .handle_event(Event::Resize(24, 5))
/// ```
///
/// # Assertions
///
/// - Static leaf nodes are skipped.
/// - Event traversal reaches the component boundary.
/// - `AppControl::Exit` short-circuits child traversal.
#[test]
fn dispatches_events_through_component_child_nodes() -> Result<()> {
    let node = column([text("Static"), component(EventExit)]);

    assert_eq!(node.handle_event(Event::Resize(24, 5))?, AppControl::Exit);

    Ok(())
}

/// Verifies dynamic children are also traversed during event dispatch.
///
/// # Example Under Test
///
/// ```text
/// column([dynamic(|| component(EventCounter))])
///     .handle_event(Event::Resize(24, 5))
/// ```
///
/// # Assertions
///
/// - The dynamic closure is evaluated during event dispatch.
/// - Events reach the node produced by the dynamic closure.
#[test]
fn dispatches_events_through_dynamic_child_nodes() -> Result<()> {
    let count = Rc::new(Cell::new(0));
    let child_count = Rc::clone(&count);
    let node = column([dynamic(move || {
        component(EventCounter {
            count: Rc::clone(&child_count),
        })
    })]);

    assert_eq!(
        node.handle_event(Event::Resize(24, 5))?,
        AppControl::Continue
    );
    assert_eq!(count.get(), 1);

    Ok(())
}

/// Verifies deferred node equality stays identity-based.
///
/// # Example Under Test
///
/// ```text
/// let first = dynamic(|| text("same"));
/// let first_clone = first.clone();
/// let second = dynamic(|| text("same"));
/// ```
///
/// # Assertions
///
/// - A cloned dynamic node compares equal to its source.
/// - Separate dynamic nodes with identical closures do not compare equal.
#[test]
fn compares_dynamic_nodes_by_identity() {
    let first = dynamic(|| text("same"));
    let first_clone = first.clone();
    let second = dynamic(|| text("same"));

    assert_eq!(first, first_clone);
    assert_ne!(first, second);
}
