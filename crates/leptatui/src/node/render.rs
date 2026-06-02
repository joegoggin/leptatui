use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

use crate::{app::Result, component::RenderCtx};

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
