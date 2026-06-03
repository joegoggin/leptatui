//! Rendering and event traversal for Leptatui nodes.
//!
//! This module maps [`Node`] variants to Ratatui widgets, layout splits, and
//! component event propagation.

use crossterm::event::Event;
use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

use crate::{
    app::{AppControl, Result},
    component::{Component, RenderCtx},
};

use super::model::Node;

impl Node {
    /// Renders this node into a context.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context for the node's target area.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if rendering performs terminal I/O
    /// that fails.
    pub fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        match self {
            Self::Block { child } => {
                let block = Block::bordered();
                let inner = block.inner(ctx.area());
                ctx.render_widget(block);
                ctx.with_area(inner, |ctx| child.render(ctx))
            }
            Self::Text(content) => {
                ctx.render_widget(Paragraph::new(content.as_str()));
                Ok(())
            }
            Self::Row(children) => render_children(children, Direction::Row, ctx),
            Self::Column(children) => render_children(children, Direction::Column, ctx),
            Self::Button(label) => {
                ctx.render_widget(
                    Paragraph::new(label.as_str())
                        .centered()
                        .block(Block::bordered()),
                );
                Ok(())
            }
            Self::Dynamic(child) => child().render(ctx),
            Self::Component(component) => component.render(ctx),
        }
    }

    /// Dispatches an event through this node tree.
    ///
    /// # Arguments
    ///
    /// * `event` — Crossterm event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether traversal should continue.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails.
    pub fn handle_event(&self, event: Event) -> Result<AppControl> {
        self.handle_event_ref(&event)
    }

    /// Dispatches an event by reference through this node tree.
    ///
    /// # Arguments
    ///
    /// * `event` — Crossterm event to dispatch without cloning at every branch.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether traversal should continue.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails.
    fn handle_event_ref(&self, event: &Event) -> Result<AppControl> {
        match self {
            Self::Block { child } => child.handle_event_ref(event),
            Self::Row(children) | Self::Column(children) => handle_child_events(children, event),
            Self::Dynamic(child) => child().handle_event_ref(event),
            Self::Component(component) => component.handle_event(event.clone()),
            Self::Text(_) | Self::Button(_) => Ok(AppControl::Continue),
        }
    }
}

impl Component for Node {
    /// Renders the node when it is used as a component.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context for the node's target area.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if rendering performs terminal I/O
    /// that fails.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Node::render(self, ctx)
    }

    /// Dispatches an event when the node is used as a component.
    ///
    /// # Arguments
    ///
    /// * `event` — Crossterm event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether traversal should continue.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails.
    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        Node::handle_event(self, event)
    }
}

/// Axis used to split child node layout areas.
#[derive(Clone, Copy)]
enum Direction {
    /// Split the available area horizontally.
    Row,
    /// Split the available area vertically.
    Column,
}

/// Renders child nodes into equally sized row or column areas.
///
/// # Arguments
///
/// * `children` — Nodes to render into equal areas.
/// * `direction` — Axis used to split the current context area.
/// * `ctx` — Rendering context for the parent area.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::app::Error::Io`] if child rendering performs terminal I/O
/// that fails.
fn render_children(
    children: &[Node],
    direction: Direction,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    if children.is_empty() {
        return Ok(());
    }

    let constraints = vec![Constraint::Fill(1); children.len()];
    let areas = match direction {
        Direction::Row => Layout::horizontal(constraints).split(ctx.area()),
        Direction::Column => Layout::vertical(constraints).split(ctx.area()),
    };

    for (child, area) in children.iter().zip(areas.iter()) {
        ctx.with_area(*area, |ctx| child.render(ctx))?;
    }

    Ok(())
}

/// Dispatches an event through child nodes until one requests exit.
///
/// # Arguments
///
/// * `children` — Child nodes to visit in order.
/// * `event` — Event to dispatch to each child.
///
/// # Returns
///
/// An [`AppControl`] value requesting exit when any child exits, otherwise
/// continue.
///
/// # Errors
///
/// Returns [`crate::app::Error::Io`] if child event handling performs terminal
/// I/O that fails.
fn handle_child_events(children: &[Node], event: &Event) -> Result<AppControl> {
    for child in children {
        if child.handle_event_ref(event)? == AppControl::Exit {
            return Ok(AppControl::Exit);
        }
    }

    Ok(AppControl::Continue)
}
