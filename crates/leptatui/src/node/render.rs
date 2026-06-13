//! Rendering and event traversal for Leptatui nodes.
//!
//! This module maps [`Node`] variants to Ratatui widgets, layout splits, and
//! component event propagation.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use leptos::prelude::{GetUntracked, ReadSignal};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

use crate::{
    ThemeVariables,
    app::{AppControl, Result},
    component::{Component, KeyControl, RenderCtx},
    context,
    style::{Borders, StyleDeclarations, TuiStyle},
};

use super::{metadata::StyleMetadata, model::Node};

/// Resolves a node style from context stylesheets, ancestors, and inherited style values.
///
/// # Arguments
///
/// * `metadata` — Node selector metadata used by stylesheet resolution.
/// * `ctx` — Rendering context containing stylesheets, ancestor metadata,
///   and inherited style.
///
/// # Returns
///
/// A [`TuiStyle`] containing the resolved node style.
fn resolve_style(metadata: &StyleMetadata, ctx: &RenderCtx<'_, '_>) -> TuiStyle {
    let theme = context::use_context::<ThemeVariables>()
        .or_else(|| {
            context::use_context::<ReadSignal<ThemeVariables>>().map(|theme| theme.get_untracked())
        })
        .unwrap_or_default();

    let mut resolved = StyleDeclarations::from(ctx.inherited_style());
    for stylesheet in ctx.stylesheets() {
        stylesheet.apply_matching_rules(&mut resolved, metadata, ctx.selector_ancestors());
    }

    if let Some(inline_style) = metadata.inline_style() {
        resolved.overlay(&StyleDeclarations::from(inline_style));
    }

    resolved.resolve(&theme)
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
        if let Event::Key(key) = event {
            return Ok(self.handle_key_event(key)?.into());
        }

        self.dispatch_event_ref(&event)
    }

    /// Dispatches a key event through this node tree.
    ///
    /// # Arguments
    ///
    /// * `key` — Crossterm key event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// A [`KeyControl`] value indicating whether the key was handled.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        let control = self.__dispatch_key_event(key.clone())?;
        if control == KeyControl::Pass {
            return self.__handle_default_key_event(key);
        }

        Ok(control)
    }

    /// Dispatches a key event through descendant component boundaries only.
    #[doc(hidden)]
    pub fn __dispatch_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        self.dispatch_key_event_ref(&key)
    }

    /// Handles built-in key behavior for this node tree.
    #[doc(hidden)]
    pub fn __handle_default_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        Ok(self.handle_default_key_event_ref(&key))
    }

    /// Dispatches a key event by reference through this node tree.
    ///
    /// # Arguments
    ///
    /// * `key` — Crossterm key event to dispatch without cloning at every
    ///   branch.
    ///
    /// # Returns
    ///
    /// A [`KeyControl`] value indicating whether the key was handled.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails.
    fn dispatch_key_event_ref(&mut self, key: &KeyEvent) -> Result<KeyControl> {
        match self {
            Self::Block { child, .. } => child.dispatch_key_event_ref(key),
            Self::Row { children, .. } | Self::Column { children, .. } => {
                handle_child_key_events(children, key)
            }
            Self::Dynamic(child) => {
                let mut child = child();
                child.dispatch_key_event_ref(key)
            }
            Self::Component(component) => component.handle_key_event(key.clone()),
            Self::Text { .. } | Self::Button { .. } => Ok(KeyControl::Pass),
        }
    }

    /// Handles the built-in key behavior for focus movement and button activation.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to match against built-in button behavior.
    ///
    /// # Returns
    ///
    /// A [`KeyControl`] value indicating whether the key was handled.
    fn handle_default_key_event_ref(&mut self, key: &KeyEvent) -> KeyControl {
        if key.kind != KeyEventKind::Press {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                let count = self.focusable_count();
                if count == 0 {
                    return KeyControl::Pass;
                }

                let direction = match key.code {
                    KeyCode::Tab => FocusDirection::Forward,
                    KeyCode::BackTab => FocusDirection::Backward,
                    _ => unreachable!("only tab keys are matched"),
                };
                self.move_focus(direction, count);
                KeyControl::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => self
                .activate_focused_button()
                .map_or(KeyControl::Pass, KeyControl::from),
            _ => KeyControl::Pass,
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
            Self::Block { child, .. } => child.dispatch_event_ref(event),
            Self::Row { children, .. } | Self::Column { children, .. } => {
                handle_child_events(children, event)
            }
            Self::Dynamic(child) => child().dispatch_event_ref(event),
            Self::Component(component) => component.handle_event(event.clone()),
            Self::Text { .. } | Self::Button { .. } => Ok(AppControl::Continue),
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

    /// Dispatches a key event when the node is used as a component.
    ///
    /// # Arguments
    ///
    /// * `key` — Crossterm key event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// A [`KeyControl`] value indicating whether the key was handled.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails.
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        Node::handle_key_event(self, key)
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
        if child.dispatch_event_ref(event)? == AppControl::Exit {
            return Ok(AppControl::Exit);
        }
    }

    Ok(AppControl::Continue)
}

/// Dispatches a key event through child nodes until one handles it.
///
/// # Arguments
///
/// * `children` — Child nodes to visit in order.
/// * `key` — Key event to dispatch to each child.
///
/// # Returns
///
/// A [`KeyControl`] value from the first child that handles the key, otherwise
/// [`KeyControl::Pass`].
///
/// # Errors
///
/// Returns [`crate::app::Error::Io`] if child event handling performs terminal
/// I/O that fails.
fn handle_child_key_events(children: &mut [Node], key: &KeyEvent) -> Result<KeyControl> {
    for child in children {
        let control = child.dispatch_key_event_ref(key)?;
        if control != KeyControl::Pass {
            return Ok(control);
        }
    }

    Ok(KeyControl::Pass)
}
