//! Node rendering tests.
//!
//! These tests render node trees against Ratatui's test backend and inspect the
//! resulting terminal buffer.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use leptatui::{
    AppControl, Color, Component, Node, NodeType, RenderCtx, Result, StyleMetadata, StyleSelector,
    Stylesheet, TuiStyle, block, button, column, component, dynamic, row, text,
};
use ratatui::{Terminal, backend::TestBackend};

/// Creates a key-press event for a key code.
///
/// # Arguments
///
/// * `code` — Key code to place in the generated event.
///
/// # Returns
///
/// An [`Event`] containing the key press.
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Returns flattened focus states for all buttons in a node tree.
///
/// # Arguments
///
/// * `node` — Node tree to inspect.
///
/// # Returns
///
/// A [`Vec<bool>`] containing focus state for each button.
fn button_focuses(node: &Node) -> Vec<bool> {
    match node {
        Node::Button { metadata, .. } => vec![metadata.is_focused()],
        Node::Block { child, .. } => button_focuses(child),
        Node::Row { children, .. } | Node::Column { children, .. } => {
            children.iter().flat_map(button_focuses).collect()
        }
        Node::Text { .. } | Node::Dynamic(_) | Node::Component(_) => Vec::new(),
    }
}

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

/// Verifies text nodes render with stylesheet-resolved colors.
///
/// # Example Under Test
///
/// ```text
/// text("Hi").with_classes("accent")
/// Stylesheet::new().rule(StyleSelector::class("accent"), yellow on blue)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The node render call succeeds.
/// - The rendered `H` cell exists.
/// - The rendered `H` cell has a yellow foreground.
/// - The rendered `H` cell has a blue background.
#[test]
fn renders_text_with_resolved_stylesheet_style() -> Result<()> {
    let backend = TestBackend::new(12, 3);
    let mut terminal = Terminal::new(backend)?;
    let node = text("Hi").with_classes("accent");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("accent"),
        TuiStyle::new()
            .foreground(Color::Yellow)
            .background(Color::Blue),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| node.render(ctx));
    })?;
    render_result?;

    let cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "H")
        .expect("rendered H cell");

    assert_eq!(cell.fg, Color::Yellow);
    assert_eq!(cell.bg, Color::Blue);

    Ok(())
}

/// Verifies node builders store default selector metadata.
///
/// # Example Under Test
///
/// ```text
/// block(text("child"))
/// ```
///
/// # Assertions
///
/// - Block metadata is available.
/// - The node type is `Block`.
/// - The metadata has no id.
/// - The metadata has no classes.
/// - The metadata has no inline style.
/// - The metadata is not focused.
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

/// Verifies node metadata setters store selector fields.
///
/// # Example Under Test
///
/// ```text
/// button("Save")
///     .with_id("save")
///     .with_classes("primary toolbar")
///     .with_inline_style(yellow)
///     .with_focus(true)
/// ```
///
/// # Assertions
///
/// - Button metadata is available.
/// - The node type is `Button`.
/// - The metadata id is `save`.
/// - The metadata classes are `primary` and `toolbar`.
/// - The metadata inline style is yellow.
/// - The metadata is focused.
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

/// Verifies tab navigation moves between static buttons.
///
/// # Example Under Test
///
/// ```text
/// column([button("One"), text("Gap"), button("Two")])
/// Tab, Tab, BackTab
/// ```
///
/// # Assertions
///
/// - The first tab event succeeds and focuses the first button.
/// - The second tab event succeeds and focuses the second button.
/// - The back-tab event succeeds and returns focus to the first button.
///
/// # Why
///
/// Non-focusable text nodes should be skipped during keyboard focus movement.
#[test]
fn tab_focus_moves_between_static_buttons() -> Result<()> {
    let mut node = column([button("One"), text("Gap"), button("Two")]);

    node.handle_event(key(KeyCode::Tab))?;
    assert_eq!(button_focuses(&node), vec![true, false]);

    node.handle_event(key(KeyCode::Tab))?;
    assert_eq!(button_focuses(&node), vec![false, true]);

    node.handle_event(key(KeyCode::BackTab))?;
    assert_eq!(button_focuses(&node), vec![true, false]);

    Ok(())
}

/// Verifies enter and space activate focused button actions.
///
/// # Example Under Test
///
/// ```text
/// column([button("Enter").on_press(...), button("Space").on_press(...)])
/// Tab, Enter, Tab, Space
/// ```
///
/// # Assertions
///
/// - The first tab event succeeds.
/// - The enter event succeeds and increments the action count to `1`.
/// - The second tab event succeeds.
/// - The space event succeeds and increments the action count to `2`.
///
/// # Why
///
/// Both common activation keys should trigger only the currently focused
/// button.
#[test]
fn enter_and_space_activate_focused_button() -> Result<()> {
    let count = Rc::new(Cell::new(0));
    let enter_count = Rc::clone(&count);
    let space_count = Rc::clone(&count);

    let mut node = column([
        button("Enter").on_press(move || {
            enter_count.set(enter_count.get() + 1);
            AppControl::Continue
        }),
        button("Space").on_press(move || {
            space_count.set(space_count.get() + 1);
            AppControl::Continue
        }),
    ]);

    node.handle_event(key(KeyCode::Tab))?;
    node.handle_event(key(KeyCode::Enter))?;
    assert_eq!(count.get(), 1);

    node.handle_event(key(KeyCode::Tab))?;
    node.handle_event(key(KeyCode::Char(' ')))?;
    assert_eq!(count.get(), 2);

    Ok(())
}

/// Verifies focused button actions can request app exit.
///
/// # Example Under Test
///
/// ```text
/// button("Quit").on_press(|| AppControl::Exit)
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - The tab event succeeds and focuses the button.
/// - The enter event returns [`AppControl::Exit`].
#[test]
fn focused_button_action_can_exit_app_loop() -> Result<()> {
    let mut node = button("Quit").on_press(|| AppControl::Exit);

    node.handle_event(key(KeyCode::Tab))?;

    assert_eq!(node.handle_event(key(KeyCode::Enter))?, AppControl::Exit);

    Ok(())
}

/// Verifies focused buttons render with focus stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// row([button("One"), button("Two")])
/// Stylesheet::new().rule(StyleSelector::focus(), black on yellow)
/// with_focus(true)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The node render call succeeds.
/// - The rendered focused button label exists.
/// - The focused cell has a black foreground.
/// - The focused cell has a yellow background.
///
/// # Why
///
/// Focus selector state should affect rendered button styling.
#[test]
fn renders_focused_button_with_focus_stylesheet_rule() -> Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let node = row([button("One").with_focus(true), button("Two")]);
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::focus(),
        TuiStyle::new()
            .foreground(Color::Black)
            .background(Color::Yellow),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| node.render(ctx));
    })?;
    render_result?;

    let focused_cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "O")
        .expect("rendered focused button label");

    assert_eq!(focused_cell.fg, Color::Black);
    assert_eq!(focused_cell.bg, Color::Yellow);

    Ok(())
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

/// Component that records selector metadata from a rendered child node.
struct MetadataRecorder {
    /// Shared slot receiving child selector metadata.
    seen: Rc<RefCell<Option<StyleMetadata>>>,
}

impl Component for MetadataRecorder {
    /// Renders a child node and records its selector metadata.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the component boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let node = text("Child")
            .with_id("inside")
            .with_classes("component-child");
        *self.seen.borrow_mut() = node.style_metadata().cloned();
        ctx.render_node(&node)
    }
}

/// Verifies selector metadata remains available inside component boundaries.
///
/// # Example Under Test
///
/// ```text
/// component(MetadataRecorder)
/// text("Child").with_id("inside").with_classes("component-child")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component render call succeeds.
/// - Child metadata is recorded.
/// - The recorded id is `inside`.
/// - The recorded classes contain `component-child`.
///
/// # Why
///
/// Component boundaries should not prevent child nodes from carrying selector
/// metadata used by stylesheets.
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
    let mut node = column([text("Static"), component(EventExit)]);

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
    let mut node = column([dynamic(move || {
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
