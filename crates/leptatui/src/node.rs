//! Basic renderable terminal nodes.
//!
//! This module provides a small node tree for simple hand-written interfaces
//! and builder functions for creating common node variants.

use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

use crate::{app::Result, component::RenderCtx};

/// Minimal renderable node tree for hand-written terminal UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Node {
    /// Bordered container around a child node.
    Block {
        /// Child node rendered inside the block's inner area.
        child: Box<Node>,
    },
    /// Plain text content.
    Text(String),
    /// Horizontally arranged children.
    Row(Vec<Node>),
    /// Vertically arranged children.
    Column(Vec<Node>),
    /// Basic bordered button label.
    Button(String),
}

/// Creates a bordered block around a child node.
///
/// # Arguments
///
/// * `child` — Node-compatible value rendered inside the block.
///
/// # Returns
///
/// A [`Node::Block`] containing the provided child.
pub fn block(child: impl Into<Node>) -> Node {
    Node::Block {
        child: Box::new(child.into()),
    }
}

/// Creates a text node.
///
/// # Arguments
///
/// * `content` — Text content to render.
///
/// # Returns
///
/// A [`Node::Text`] containing the provided content.
pub fn text(content: impl Into<String>) -> Node {
    Node::Text(content.into())
}

/// Creates a horizontal row.
///
/// # Arguments
///
/// * `children` — Child nodes to divide across the row.
///
/// # Returns
///
/// A [`Node::Row`] containing the provided children.
pub fn row(children: impl IntoIterator<Item = Node>) -> Node {
    Node::Row(children.into_iter().collect())
}

/// Creates a vertical column.
///
/// # Arguments
///
/// * `children` — Child nodes to divide down the column.
///
/// # Returns
///
/// A [`Node::Column`] containing the provided children.
pub fn column(children: impl IntoIterator<Item = Node>) -> Node {
    Node::Column(children.into_iter().collect())
}

/// Creates a basic button.
///
/// # Arguments
///
/// * `label` — Button text to center inside a bordered area.
///
/// # Returns
///
/// A [`Node::Button`] containing the provided label.
pub fn button(label: impl Into<String>) -> Node {
    Node::Button(label.into())
}

impl From<String> for Node {
    /// Converts owned text into a text node.
    ///
    /// # Arguments
    ///
    /// * `value` — Text content to render.
    ///
    /// # Returns
    ///
    /// A [`Node::Text`] containing `value`.
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for Node {
    /// Converts borrowed text into a text node.
    ///
    /// # Arguments
    ///
    /// * `value` — Text content to copy into the node.
    ///
    /// # Returns
    ///
    /// A [`Node::Text`] containing `value`.
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

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
        }
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
