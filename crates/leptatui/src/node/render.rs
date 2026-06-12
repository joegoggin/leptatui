//! Rendering and event traversal for Leptatui nodes.
//!
//! This module maps [`Node`] variants to Ratatui widgets, layout splits, and
//! component event propagation.

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

use crate::{
    ThemeVariables,
    app::{AppControl, Result},
    component::{Component, RenderCtx},
    context,
    style::{Borders, TuiStyle},
};

use super::{metadata::StyleMetadata, model::Node};

/// Resolves a node style from context stylesheets, ancestors, and inherited style values.
///
/// # Arguments
///
/// * `metadata` — Node selector metadata used by stylesheet resolution.
/// * `ctx` — Rendering context containing the stylesheet, ancestor metadata,
///   and inherited style.
///
/// # Returns
///
/// A [`TuiStyle`] containing the resolved node style.
fn resolve_style(metadata: &StyleMetadata, ctx: &RenderCtx<'_, '_>) -> TuiStyle {
    let theme = context::use_context::<ThemeVariables>().unwrap_or_default();

    ctx.stylesheet().resolve(
        metadata,
        ctx.selector_ancestors(),
        ctx.inherited_style(),
        &theme,
    )
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
            Self::Block { child, metadata } => {
                let style = resolve_style(metadata, ctx);
                let block = style.to_block_with_default_borders(Borders::ALL);
                let inner = block.inner(ctx.area());
                ctx.render_widget(block);
                ctx.with_area_inherited_style_and_selector_ancestor(
                    inner,
                    style.inherited_values(),
                    metadata.clone(),
                    |ctx| child.render(ctx),
                )
            }
            Self::Text { content, metadata } => {
                let style = resolve_style(metadata, ctx);
                ctx.render_widget(Paragraph::new(content.as_str()).style(style.to_ratatui_style()));
                Ok(())
            }
            Self::Row { children, metadata } => {
                let style = resolve_style(metadata, ctx);
                ctx.render_widget(Block::new().style(style.to_ratatui_style()));
                render_children(
                    children,
                    Direction::Row,
                    style.inherited_values(),
                    metadata,
                    ctx,
                )
            }
            Self::Column { children, metadata } => {
                let style = resolve_style(metadata, ctx);
                ctx.render_widget(Block::new().style(style.to_ratatui_style()));
                render_children(
                    children,
                    Direction::Column,
                    style.inherited_values(),
                    metadata,
                    ctx,
                )
            }
            Self::Button {
                label, metadata, ..
            } => {
                let style = resolve_style(metadata, ctx);
                ctx.render_widget(
                    Paragraph::new(label.as_str())
                        .centered()
                        .style(style.to_ratatui_style())
                        .block(style.to_block_with_default_borders(Borders::ALL)),
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
    pub fn handle_event(&mut self, event: Event) -> Result<AppControl> {
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
    fn handle_event_ref(&mut self, event: &Event) -> Result<AppControl> {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Tab | KeyCode::BackTab => {
                    let count = self.focusable_count();
                    if count > 0 {
                        let direction = match key.code {
                            KeyCode::Tab => FocusDirection::Forward,
                            KeyCode::BackTab => FocusDirection::Backward,
                            _ => unreachable!("only tab keys are matched"),
                        };
                        self.move_focus(direction, count);
                        return Ok(AppControl::Continue);
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(control) = self.activate_focused_button() {
                        return Ok(control);
                    }
                }
                _ => {}
            }
        }

        self.dispatch_event_ref(event)
    }

    /// Dispatches an event to child nodes and component boundaries.
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
    fn dispatch_event_ref(&mut self, event: &Event) -> Result<AppControl> {
        match self {
            Self::Block { child, .. } => child.handle_event_ref(event),
            Self::Row { children, .. } | Self::Column { children, .. } => {
                handle_child_events(children, event)
            }
            Self::Dynamic(child) => {
                let mut child = child();
                child.handle_event_ref(event)
            }
            Self::Component(component) => component.handle_event(event.clone()),
            Self::Text { .. } | Self::Button { .. } => Ok(AppControl::Continue),
        }
    }

    /// Returns the number of focusable buttons in this node tree.
    ///
    /// # Returns
    ///
    /// A [`usize`] count of focusable button nodes.
    fn focusable_count(&self) -> usize {
        match self {
            Self::Button { .. } => 1,
            Self::Block { child, .. } => child.focusable_count(),
            Self::Row { children, .. } | Self::Column { children, .. } => {
                children.iter().map(Self::focusable_count).sum()
            }
            Self::Text { .. } | Self::Dynamic(_) | Self::Component(_) => 0,
        }
    }

    /// Moves focus to the next or previous focusable button.
    ///
    /// # Arguments
    ///
    /// * `direction` — Direction to move through focusable buttons.
    /// * `count` — Number of focusable buttons in the node tree.
    fn move_focus(&mut self, direction: FocusDirection, count: usize) {
        if count == 0 {
            return;
        }

        let target = match (self.focused_index(), direction) {
            (Some(index), FocusDirection::Forward) => (index + 1) % count,
            (Some(0), FocusDirection::Backward) => count - 1,
            (Some(index), FocusDirection::Backward) => index - 1,
            (None, FocusDirection::Forward) => 0,
            (None, FocusDirection::Backward) => count - 1,
        };

        self.set_focus_by_index(target);
    }

    /// Returns the flattened index of the currently focused button.
    ///
    /// # Returns
    ///
    /// An [`Option<usize>`] containing the focused button index.
    fn focused_index(&self) -> Option<usize> {
        let mut index = 0;
        self.focused_index_inner(&mut index)
    }

    /// Returns the focused button index while tracking traversal position.
    ///
    /// # Arguments
    ///
    /// * `index` — Current flattened button index during traversal.
    ///
    /// # Returns
    ///
    /// An [`Option<usize>`] containing the focused button index.
    fn focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        match self {
            Self::Button { metadata, .. } => {
                let current = *index;
                *index += 1;
                metadata.is_focused().then_some(current)
            }
            Self::Block { child, .. } => child.focused_index_inner(index),
            Self::Row { children, .. } | Self::Column { children, .. } => children
                .iter()
                .find_map(|child| child.focused_index_inner(index)),
            Self::Text { .. } | Self::Dynamic(_) | Self::Component(_) => None,
        }
    }

    /// Sets focus by flattened button index.
    ///
    /// # Arguments
    ///
    /// * `target` — Flattened button index that should receive focus.
    fn set_focus_by_index(&mut self, target: usize) {
        let mut index = 0;
        self.set_focus_by_index_inner(target, &mut index);
    }

    /// Sets focus by flattened button index while tracking traversal position.
    ///
    /// # Arguments
    ///
    /// * `target` — Flattened button index that should receive focus.
    /// * `index` — Current flattened button index during traversal.
    fn set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        match self {
            Self::Button { metadata, .. } => {
                metadata.set_focused(*index == target);
                *index += 1;
            }
            Self::Block { child, .. } => child.set_focus_by_index_inner(target, index),
            Self::Row { children, .. } | Self::Column { children, .. } => {
                for child in children {
                    child.set_focus_by_index_inner(target, index);
                }
            }
            Self::Text { .. } | Self::Dynamic(_) | Self::Component(_) => {}
        }
    }

    /// Activates the focused button if this node tree contains one.
    ///
    /// # Returns
    ///
    /// An [`Option<AppControl>`] containing the focused button action result.
    fn activate_focused_button(&self) -> Option<AppControl> {
        match self {
            Self::Button {
                metadata, on_press, ..
            } if metadata.is_focused() => Some(
                on_press
                    .as_ref()
                    .map_or(AppControl::Continue, |action| action()),
            ),
            Self::Block { child, .. } => child.activate_focused_button(),
            Self::Row { children, .. } | Self::Column { children, .. } => {
                children.iter().find_map(Self::activate_focused_button)
            }
            Self::Text { .. } | Self::Button { .. } | Self::Dynamic(_) | Self::Component(_) => None,
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

/// Direction used to move focus through focusable nodes.
#[derive(Clone, Copy)]
enum FocusDirection {
    /// Move focus to the next focusable node.
    Forward,
    /// Move focus to the previous focusable node.
    Backward,
}

/// Renders child nodes into equally sized row or column areas.
///
/// # Arguments
///
/// * `children` — Nodes to render into equal areas.
/// * `direction` — Axis used to split the current context area.
/// * `inherited_style` — Style values inherited by child nodes.
/// * `parent_metadata` — Metadata to append to each child's selector ancestor
///   path.
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
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
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
        ctx.with_area_inherited_style_and_selector_ancestor(
            *area,
            inherited_style,
            parent_metadata.clone(),
            |ctx| child.render(ctx),
        )?;
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
fn handle_child_events(children: &mut [Node], event: &Event) -> Result<AppControl> {
    for child in children {
        if child.handle_event_ref(event)? == AppControl::Exit {
            return Ok(AppControl::Exit);
        }
    }

    Ok(AppControl::Continue)
}
