//! Basic renderable terminal nodes.

use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

use crate::{app::Result, component::RenderCtx};

/// Minimal renderable node tree for hand-written terminal UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Node {
    /// Bordered container around a child.
    Block { child: Box<Node> },
    /// Plain text content.
    Text(String),
    /// Horizontally arranged children.
    Row(Vec<Node>),
    /// Vertically arranged children.
    Column(Vec<Node>),
    /// Basic bordered button label.
    Button(String),
}

/// Create a bordered block around a child node.
pub fn block(child: impl Into<Node>) -> Node {
    Node::Block {
        child: Box::new(child.into()),
    }
}

/// Create a text node.
pub fn text(content: impl Into<String>) -> Node {
    Node::Text(content.into())
}

/// Create a horizontal row.
pub fn row(children: impl IntoIterator<Item = Node>) -> Node {
    Node::Row(children.into_iter().collect())
}

/// Create a vertical column.
pub fn column(children: impl IntoIterator<Item = Node>) -> Node {
    Node::Column(children.into_iter().collect())
}

/// Create a basic button.
pub fn button(label: impl Into<String>) -> Node {
    Node::Button(label.into())
}

impl From<String> for Node {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for Node {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl Node {
    /// Render this node into a context.
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

#[derive(Clone, Copy)]
enum Direction {
    Row,
    Column,
}

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
