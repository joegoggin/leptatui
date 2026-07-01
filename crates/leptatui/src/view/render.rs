//! Rendering and event traversal for Leptatui views.
//!
//! This module maps [`View`] variants to Ratatui widgets, layout splits, and
//! component event propagation.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use leptos::prelude::{GetUntracked, ReadSignal};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use crate::{
    ThemeVariables,
    app::{AppControl, Result},
    component::{Component, KeyControl, RenderCtx},
    context,
    style::{Borders, LayoutDirection, TuiStyle},
};

use super::{
    metadata::{EditableState, StyleMetadata},
    model::{InputAction, View},
};

/// Resolves a view style from context stylesheets, ancestors, and inherited style values.
///
/// # Arguments
///
/// * `metadata` — View selector metadata used by stylesheet resolution.
/// * `ctx` — Rendering context containing stylesheets, ancestor metadata,
///   and inherited style.
///
/// # Returns
///
/// A [`TuiStyle`] containing the resolved view style.
fn resolve_style(metadata: &StyleMetadata, ctx: &RenderCtx<'_, '_>) -> TuiStyle {
    let theme = context::use_context::<ThemeVariables>()
        .or_else(|| {
            context::use_context::<ReadSignal<ThemeVariables>>().map(|theme| theme.get_untracked())
        })
        .unwrap_or_default();

    crate::Stylesheet::resolve_stylesheets(
        ctx.stylesheets(),
        metadata,
        ctx.selector_ancestors(),
        ctx.inherited_style(),
        metadata.inline_style(),
        Some(ctx.viewport_size()),
        &theme,
    )
}

/// Returns a paragraph configured for Leptatui text rendering.
fn text_paragraph<'a>(content: &'a str, style: TuiStyle) -> Paragraph<'a> {
    Paragraph::new(content)
        .style(style.to_ratatui_style())
        .wrap(Wrap { trim: false })
}

/// Returns a paragraph configured for single-line editable control rendering.
///
/// # Arguments
///
/// * `value` — Text value to render inside the input.
/// * `style` — Resolved view style applied to the paragraph.
/// * `horizontal_scroll` — Horizontal viewport offset applied to the paragraph.
///
/// # Returns
///
/// A [`Paragraph`] configured for input rendering.
fn input_paragraph<'a>(value: &'a str, style: TuiStyle, horizontal_scroll: u16) -> Paragraph<'a> {
    Paragraph::new(value)
        .style(style.to_ratatui_style())
        .scroll((0, horizontal_scroll))
}

/// Returns a paragraph configured for multiline editable control rendering.
///
/// # Arguments
///
/// * `value` — Text value to render inside the text area.
/// * `style` — Resolved view style applied to the paragraph.
/// * `vertical_scroll` — Vertical viewport offset applied to the paragraph.
/// * `horizontal_scroll` — Horizontal viewport offset applied to the paragraph.
///
/// # Returns
///
/// A [`Paragraph`] configured for text-area rendering.
fn text_area_paragraph<'a>(
    value: &'a str,
    style: TuiStyle,
    vertical_scroll: u16,
    horizontal_scroll: u16,
) -> Paragraph<'a> {
    Paragraph::new(value)
        .style(style.to_ratatui_style())
        .wrap(Wrap { trim: false })
        .scroll((vertical_scroll, horizontal_scroll))
}

/// Converts a rendered line count into a saturated terminal height.
fn line_count_height(line_count: usize) -> u16 {
    u16::try_from(line_count).unwrap_or(u16::MAX)
}

impl View {
    /// Renders this view into a context.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context for the view's target area.
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
                ctx.render_widget(text_paragraph(content.as_str(), style));
                Ok(())
            }
            Self::Row { children, metadata } => {
                let style = resolve_style(metadata, ctx);
                ctx.render_widget(Block::new().style(style.to_ratatui_style()));
                render_children(
                    children,
                    style.direction.unwrap_or(LayoutDirection::Row),
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
                    style.direction.unwrap_or(LayoutDirection::Column),
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
                metadata.clear_scroll_into_view_request();
                Ok(())
            }
            Self::Input {
                value,
                placeholder,
                metadata,
                editable_state,
                ..
            } => {
                let style = resolve_style(metadata, ctx);
                let display_value = if value.is_empty() {
                    placeholder.as_deref().unwrap_or("")
                } else {
                    value.as_str()
                };
                let horizontal_scroll = if value.is_empty() {
                    0
                } else {
                    input_horizontal_scroll(
                        value.as_str(),
                        editable_state.cursor(),
                        editable_state.horizontal_scroll(),
                        ctx.area().width,
                    )
                };
                ctx.render_widget(input_paragraph(display_value, style, horizontal_scroll));
                metadata.clear_scroll_into_view_request();
                Ok(())
            }
            Self::TextArea {
                value,
                metadata,
                editable_state,
            } => {
                let style = resolve_style(metadata, ctx);
                ctx.render_widget(text_area_paragraph(
                    value.as_str(),
                    style,
                    editable_state.vertical_scroll(),
                    editable_state.horizontal_scroll(),
                ));
                metadata.clear_scroll_into_view_request();
                Ok(())
            }
            Self::Dynamic(child) => child.with_view(|child| child.render(ctx)),
            Self::Component(component) => component.render(ctx),
        }
    }

    /// Dispatches an event through this view tree.
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

    /// Dispatches a key event through this view tree.
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
        let control = self.__dispatch_key_event(key)?;
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

    /// Handles built-in key behavior for this view tree.
    #[doc(hidden)]
    pub fn __handle_default_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        Ok(self.handle_default_key_event_ref(&key))
    }

    /// Returns the number of focusable controls in this view tree.
    #[doc(hidden)]
    pub fn __focusable_count(&self) -> usize {
        self.focusable_count()
    }

    /// Returns the minimum useful render height for this view tree.
    #[doc(hidden)]
    pub fn __min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        min_height_for_view(self, ctx)
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    pub fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.focused_index_inner(index)
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    pub fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        self.set_focus_by_index_inner(target, index);
    }

    /// Returns the focused control's vertical span inside this view area.
    #[doc(hidden)]
    pub fn __focused_button_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        focused_button_span_for_view(self, ctx).map(VerticalSpan::into_tuple)
    }

    /// Activates the focused button if this view tree contains one.
    #[doc(hidden)]
    pub fn __activate_focused_button(&self) -> Option<AppControl> {
        self.activate_focused_button()
    }

    /// Handles a key on the focused input, if this tree contains one.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to apply to the focused input.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the key control result when an input handles
    /// the key.
    #[doc(hidden)]
    pub fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.handle_focused_input_key_ref(&key)
    }

    /// Scrolls the first overflowing vertical layout in this view tree.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing(&mut self, delta: i16) -> bool {
        self.scroll_first_overflowing(delta)
    }

    /// Scrolls the first overflowing vertical layout in this view tree to the top.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        self.scroll_first_overflowing_to(ScrollBoundary::Top)
    }

    /// Scrolls the first overflowing vertical layout in this view tree to the bottom.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        self.scroll_first_overflowing_to(ScrollBoundary::Bottom)
    }

    /// Returns whether this view tree contains an overflowing scroll target.
    #[doc(hidden)]
    pub fn __has_overflowing_scroll_target(&self) -> bool {
        self.has_overflowing_scroll_target()
    }

    /// Dispatches a key event by reference through this view tree.
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
            Self::Dynamic(child) => child.with_view_mut(|child| child.dispatch_key_event_ref(key)),
            Self::Component(component) => component.dispatch_key_event(*key),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. } => Ok(KeyControl::Pass),
        }
    }

    /// Handles built-in key behavior for scrolling, focus movement, and button activation.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to match against built-in view behavior.
    ///
    /// # Returns
    ///
    /// A [`KeyControl`] value indicating whether the key was handled.
    fn handle_default_key_event_ref(&mut self, key: &KeyEvent) -> KeyControl {
        if key.kind != KeyEventKind::Press {
            return KeyControl::Pass;
        }

        if let Some(control) = self.handle_focused_input_key_ref(key) {
            self.clear_scroll_to_top_key_pending();
            return control;
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.handle_scroll_key(|view| view.scroll_first_overflowing(1))
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.handle_scroll_key(|view| view.scroll_first_overflowing(-1))
            }
            KeyCode::PageDown => self.handle_scroll_key(|view| view.scroll_first_overflowing(5)),
            KeyCode::PageUp => self.handle_scroll_key(|view| view.scroll_first_overflowing(-5)),
            KeyCode::Char('g') => self.handle_scroll_to_top_key(),
            KeyCode::Char('G') => self
                .handle_scroll_key(|view| view.scroll_first_overflowing_to(ScrollBoundary::Bottom)),
            KeyCode::Tab | KeyCode::BackTab => {
                self.clear_scroll_to_top_key_pending();
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
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.clear_scroll_to_top_key_pending();
                self.activate_focused_button()
                    .map_or(KeyControl::Pass, KeyControl::from)
            }
            _ => {
                self.clear_scroll_to_top_key_pending();
                KeyControl::Pass
            }
        }
    }

    /// Handles a single-key scroll command and clears pending multi-key scroll state.
    fn handle_scroll_key(&mut self, scroll: impl FnOnce(&mut Self) -> bool) -> KeyControl {
        self.clear_scroll_to_top_key_pending();
        key_control_from_bool(scroll(self))
    }

    /// Handles the two-key `gg` scroll-to-top sequence.
    fn handle_scroll_to_top_key(&mut self) -> KeyControl {
        if self.take_scroll_to_top_key_pending() {
            key_control_from_bool(self.scroll_first_overflowing_to(ScrollBoundary::Top))
        } else if self.has_overflowing_scroll_target() {
            self.set_scroll_to_top_key_pending(true);
            KeyControl::Handled
        } else {
            KeyControl::Pass
        }
    }

    /// Scrolls the first overflowing vertical layout found in render order.
    ///
    /// # Arguments
    ///
    /// * `delta` — Signed number of terminal rows to move through the overflow.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether any scroll offset changed.
    fn scroll_first_overflowing(&mut self, delta: i16) -> bool {
        match self {
            Self::Block { child, .. } => child.scroll_first_overflowing(delta),
            Self::Row { children, metadata } | Self::Column { children, metadata } => {
                if metadata.max_scroll_offset() > 0 && metadata.scroll_by(delta) {
                    return true;
                }

                children
                    .iter_mut()
                    .any(|child| child.scroll_first_overflowing(delta))
            }
            Self::Dynamic(child) => {
                child.with_view_mut(|child| child.scroll_first_overflowing(delta))
            }
            Self::Component(component) => component.scroll_first_overflowing(delta),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. } => false,
        }
    }

    /// Scrolls the first overflowing vertical layout to an absolute boundary.
    fn scroll_first_overflowing_to(&mut self, boundary: ScrollBoundary) -> bool {
        match self {
            Self::Block { child, .. } => child.scroll_first_overflowing_to(boundary),
            Self::Row { children, metadata } | Self::Column { children, metadata } => {
                if metadata.max_scroll_offset() > 0 {
                    let target = match boundary {
                        ScrollBoundary::Top => 0,
                        ScrollBoundary::Bottom => metadata.max_scroll_offset(),
                    };

                    if metadata.scroll_offset() != target {
                        metadata.set_scroll_offset(target);
                        return true;
                    }
                }

                children
                    .iter_mut()
                    .any(|child| child.scroll_first_overflowing_to(boundary))
            }
            Self::Component(component) => match boundary {
                ScrollBoundary::Top => component.scroll_first_overflowing_to_top(),
                ScrollBoundary::Bottom => component.scroll_first_overflowing_to_bottom(),
            },
            Self::Dynamic(child) => {
                child.with_view_mut(|child| child.scroll_first_overflowing_to(boundary))
            }
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. } => false,
        }
    }

    /// Returns whether this tree contains a layout with scrollable overflow.
    fn has_overflowing_scroll_target(&self) -> bool {
        match self {
            Self::Block { child, .. } => child.has_overflowing_scroll_target(),
            Self::Row { children, metadata } | Self::Column { children, metadata } => {
                metadata.max_scroll_offset() > 0
                    || children.iter().any(Self::has_overflowing_scroll_target)
            }
            Self::Dynamic(child) => child.with_view(Self::has_overflowing_scroll_target),
            Self::Component(component) => component.has_overflowing_scroll_target(),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. } => false,
        }
    }

    /// Returns metadata used to store default key-sequence state.
    fn key_sequence_metadata(&self) -> Option<&StyleMetadata> {
        match self {
            Self::Block { metadata, .. }
            | Self::Text { metadata, .. }
            | Self::Row { metadata, .. }
            | Self::Column { metadata, .. }
            | Self::Button { metadata, .. }
            | Self::Input { metadata, .. }
            | Self::TextArea { metadata, .. } => Some(metadata),
            Self::Dynamic(_) | Self::Component(_) => None,
        }
    }

    /// Stores whether the first `g` in `gg` has been pressed.
    fn set_scroll_to_top_key_pending(&self, pending: bool) {
        if let Some(metadata) = self.key_sequence_metadata() {
            metadata.set_scroll_to_top_key_pending(pending);
        }
    }

    /// Clears and returns whether the first `g` in `gg` was pressed.
    fn take_scroll_to_top_key_pending(&self) -> bool {
        self.key_sequence_metadata()
            .is_some_and(StyleMetadata::take_scroll_to_top_key_pending)
    }

    /// Clears any pending first `g` key.
    fn clear_scroll_to_top_key_pending(&self) {
        self.set_scroll_to_top_key_pending(false);
    }

    /// Handles an editing key for the currently focused input.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to apply to the focused input.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the key control result when an input handles
    /// the key.
    fn handle_focused_input_key_ref(&mut self, key: &KeyEvent) -> Option<KeyControl> {
        match self {
            Self::Input {
                value,
                metadata,
                on_input,
                editable_state,
                ..
            } if metadata.is_focused() => {
                handle_input_key(value.as_str(), on_input, editable_state, key)
            }
            Self::Block { child, .. } => child.handle_focused_input_key_ref(key),
            Self::Row { children, .. } | Self::Column { children, .. } => children
                .iter_mut()
                .find_map(|child| child.handle_focused_input_key_ref(key)),
            Self::Dynamic(child) => {
                child.with_view_mut(|child| child.handle_focused_input_key_ref(key))
            }
            Self::Component(component) => component.handle_focused_input_key(*key),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. } => None,
        }
    }

    /// Returns the number of focusable controls in this view tree.
    ///
    /// # Returns
    ///
    /// A [`usize`] count of focusable control views.
    fn focusable_count(&self) -> usize {
        match self {
            Self::Button { .. } | Self::Input { .. } | Self::TextArea { .. } => 1,
            Self::Block { child, .. } => child.focusable_count(),
            Self::Row { children, .. } | Self::Column { children, .. } => {
                children.iter().map(Self::focusable_count).sum()
            }
            Self::Dynamic(child) => child.with_view(Self::focusable_count),
            Self::Component(component) => component.focusable_count(),
            Self::Text { .. } => 0,
        }
    }

    /// Moves focus to the next or previous focusable control.
    ///
    /// # Arguments
    ///
    /// * `direction` — Direction to move through focusable controls.
    /// * `count` — Number of focusable controls in the view tree.
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

    /// Returns the flattened index of the currently focused control.
    ///
    /// # Returns
    ///
    /// An [`Option<usize>`] containing the focused control index.
    fn focused_index(&self) -> Option<usize> {
        let mut index = 0;
        self.focused_index_inner(&mut index)
    }

    /// Returns the focused control index while tracking traversal position.
    ///
    /// # Arguments
    ///
    /// * `index` — Current flattened control index during traversal.
    ///
    /// # Returns
    ///
    /// An [`Option<usize>`] containing the focused control index.
    fn focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        match self {
            Self::Button { metadata, .. }
            | Self::Input { metadata, .. }
            | Self::TextArea { metadata, .. } => {
                let current = *index;
                *index += 1;
                metadata.is_focused().then_some(current)
            }
            Self::Block { child, .. } => child.focused_index_inner(index),
            Self::Row { children, .. } | Self::Column { children, .. } => children
                .iter()
                .find_map(|child| child.focused_index_inner(index)),
            Self::Dynamic(child) => child.with_view(|child| child.focused_index_inner(index)),
            Self::Component(component) => component.focused_index_inner(index),
            Self::Text { .. } => None,
        }
    }

    /// Sets focus by flattened control index.
    ///
    /// # Arguments
    ///
    /// * `target` — Flattened control index that should receive focus.
    fn set_focus_by_index(&mut self, target: usize) {
        let mut index = 0;
        self.set_focus_by_index_inner(target, &mut index);
    }

    /// Sets focus by flattened control index while tracking traversal position.
    ///
    /// # Arguments
    ///
    /// * `target` — Flattened control index that should receive focus.
    /// * `index` — Current flattened control index during traversal.
    fn set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        match self {
            Self::Button { metadata, .. }
            | Self::Input { metadata, .. }
            | Self::TextArea { metadata, .. } => {
                let focused = *index == target;
                metadata.set_focused(focused);
                if focused {
                    metadata.request_scroll_into_view();
                } else {
                    metadata.clear_scroll_into_view_request();
                }
                *index += 1;
            }
            Self::Block { child, .. } => child.set_focus_by_index_inner(target, index),
            Self::Row { children, .. } | Self::Column { children, .. } => {
                for child in children {
                    child.set_focus_by_index_inner(target, index);
                }
            }
            Self::Dynamic(child) => {
                child.with_view_mut(|child| child.set_focus_by_index_inner(target, index));
            }
            Self::Component(component) => component.set_focus_by_index_inner(target, index),
            Self::Text { .. } => {}
        }
    }

    /// Activates the focused button if this view tree contains one.
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
            Self::Dynamic(child) => child.with_view(Self::activate_focused_button),
            Self::Component(component) => component.activate_focused_button(),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. } => None,
        }
    }

    /// Dispatches an event to child views and component boundaries.
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
            Self::Dynamic(child) => child.with_view_mut(|child| child.dispatch_event_ref(event)),
            Self::Component(component) => component.handle_event(event.clone()),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. } => Ok(AppControl::Continue),
        }
    }
}

impl Component for View {
    /// Renders the view when it is used as a component.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context for the view's target area.
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
        View::render(self, ctx)
    }

    /// Returns the minimum useful render height for the view tree.
    #[doc(hidden)]
    fn __min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        View::__min_height(self, ctx)
    }

    /// Dispatches an event when the view is used as a component.
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
        View::handle_event(self, event)
    }

    /// Dispatches a key event when the view is used as a component.
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
        View::handle_key_event(self, key)
    }

    /// Dispatches custom key behavior through the view tree.
    #[doc(hidden)]
    fn __dispatch_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        View::__dispatch_key_event(self, key)
    }

    /// Returns the number of focusable controls in the view tree.
    #[doc(hidden)]
    fn __focusable_count(&self) -> usize {
        View::__focusable_count(self)
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        View::__focused_index_inner(self, index)
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        View::__set_focus_by_index_inner(self, target, index);
    }

    /// Returns the focused control's vertical span inside this component area.
    #[doc(hidden)]
    fn __focused_button_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        View::__focused_button_span(self, ctx)
    }

    /// Activates the focused button in the view tree, if any.
    #[doc(hidden)]
    fn __activate_focused_button(&self) -> Option<AppControl> {
        View::__activate_focused_button(self)
    }

    /// Handles a key on the focused input in the view tree, if any.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to apply to the focused input.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the key control result when an input handles
    /// the key.
    #[doc(hidden)]
    fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        View::__handle_focused_input_key(self, key)
    }

    /// Scrolls the first overflowing vertical layout in the view tree.
    #[doc(hidden)]
    fn __scroll_first_overflowing(&mut self, delta: i16) -> bool {
        View::__scroll_first_overflowing(self, delta)
    }

    /// Scrolls the first overflowing vertical layout in the view tree to the top.
    #[doc(hidden)]
    fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        View::__scroll_first_overflowing_to_top(self)
    }

    /// Scrolls the first overflowing vertical layout in the view tree to the bottom.
    #[doc(hidden)]
    fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        View::__scroll_first_overflowing_to_bottom(self)
    }

    /// Returns whether the view tree contains an overflowing scroll target.
    #[doc(hidden)]
    fn __has_overflowing_scroll_target(&self) -> bool {
        View::__has_overflowing_scroll_target(self)
    }
}

/// Direction used to move focus through focusable controls.
#[derive(Clone, Copy)]
enum FocusDirection {
    /// Move focus to the next focusable control.
    Forward,
    /// Move focus to the previous focusable control.
    Backward,
}

/// Absolute scroll boundary for overflowing layouts.
#[derive(Clone, Copy)]
enum ScrollBoundary {
    /// First row of scrollable content.
    Top,
    /// Last valid scroll offset for the content.
    Bottom,
}

/// Converts a handled flag into the matching key traversal control.
fn key_control_from_bool(handled: bool) -> KeyControl {
    if handled {
        KeyControl::Handled
    } else {
        KeyControl::Pass
    }
}

/// Returns the horizontal scroll offset needed to keep an input cursor visible.
///
/// # Arguments
///
/// * `value` — Input value used to map the cursor byte index to a character
///   column.
/// * `cursor` — Cursor byte index to keep visible.
/// * `current_scroll` — Current horizontal scroll offset.
/// * `width` — Available input render width.
///
/// # Returns
///
/// A [`u16`] scroll offset that keeps the cursor within the render width.
fn input_horizontal_scroll(value: &str, cursor: usize, current_scroll: u16, width: u16) -> u16 {
    if width == 0 {
        return 0;
    }

    let cursor = clamp_cursor(value, cursor);
    let cursor_column = char_column(value, cursor);
    let width = usize::from(width);
    let current_scroll = usize::from(current_scroll);
    let next_scroll = if cursor_column < current_scroll {
        cursor_column
    } else if cursor_column > current_scroll.saturating_add(width) {
        cursor_column.saturating_sub(width)
    } else {
        current_scroll
    };

    u16::try_from(next_scroll).unwrap_or(u16::MAX)
}

/// Handles a focused input key and returns whether default propagation stops.
///
/// # Arguments
///
/// * `value` — Current controlled input value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor and scroll state for the input.
/// * `key` — Key event to apply to the input.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when the key is handled by
/// input editing behavior.
fn handle_input_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
) -> Option<KeyControl> {
    match key.code {
        KeyCode::Left => {
            let cursor = previous_char_boundary(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Right => {
            let cursor = next_char_boundary(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Home => {
            editable_state.set_cursor(0);
            Some(KeyControl::Handled)
        }
        KeyCode::End => {
            editable_state.set_cursor(value.len());
            Some(KeyControl::Handled)
        }
        KeyCode::Enter => Some(KeyControl::Handled),
        KeyCode::Backspace => Some(handle_backspace_input_key(value, on_input, editable_state)),
        KeyCode::Delete => Some(handle_delete_input_key(value, on_input, editable_state)),
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(handle_insert_input_key(
                value,
                on_input,
                editable_state,
                character,
            ))
        }
        _ => None,
    }
}

/// Handles insertion for a focused input.
///
/// # Arguments
///
/// * `value` — Current controlled input value.
/// * `on_input` — Optional callback that receives the inserted value.
/// * `editable_state` — Retained cursor and scroll state for the input.
/// * `character` — Character to insert at the cursor.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that insertion was handled.
fn handle_insert_input_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    character: char,
) -> KeyControl {
    let cursor = clamp_cursor(value, editable_state.cursor());
    let mut next = String::with_capacity(value.len().saturating_add(character.len_utf8()));
    next.push_str(&value[..cursor]);
    next.push(character);
    next.push_str(&value[cursor..]);

    commit_input_value(
        on_input,
        editable_state,
        next,
        cursor.saturating_add(character.len_utf8()),
    )
}

/// Handles backspace for a focused input.
///
/// # Arguments
///
/// * `value` — Current controlled input value.
/// * `on_input` — Optional callback that receives the shortened value.
/// * `editable_state` — Retained cursor and scroll state for the input.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that backspace was handled.
fn handle_backspace_input_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
) -> KeyControl {
    let cursor = clamp_cursor(value, editable_state.cursor());
    if cursor == 0 {
        editable_state.set_cursor(0);
        return KeyControl::Handled;
    }

    let previous = previous_char_boundary(value, cursor);
    let mut next = String::with_capacity(value.len().saturating_sub(cursor - previous));
    next.push_str(&value[..previous]);
    next.push_str(&value[cursor..]);

    commit_input_value(on_input, editable_state, next, previous)
}

/// Handles delete for a focused input.
///
/// # Arguments
///
/// * `value` — Current controlled input value.
/// * `on_input` — Optional callback that receives the shortened value.
/// * `editable_state` — Retained cursor and scroll state for the input.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that delete was handled.
fn handle_delete_input_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
) -> KeyControl {
    let cursor = clamp_cursor(value, editable_state.cursor());
    if cursor == value.len() {
        editable_state.set_cursor(cursor);
        return KeyControl::Handled;
    }

    let next_boundary = next_char_boundary(value, cursor);
    let mut next = String::with_capacity(value.len().saturating_sub(next_boundary - cursor));
    next.push_str(&value[..cursor]);
    next.push_str(&value[next_boundary..]);

    commit_input_value(on_input, editable_state, next, cursor)
}

/// Emits a controlled input update when a callback exists.
///
/// # Arguments
///
/// * `on_input` — Optional callback that receives the proposed value.
/// * `editable_state` — Retained cursor and scroll state for the input.
/// * `next` — Proposed next controlled value.
/// * `next_cursor` — Cursor byte index to retain after emitting the value.
///
/// # Returns
///
/// A [`KeyControl`] value produced by the callback or handled by default when
/// no callback exists.
fn commit_input_value(
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    next: String,
    next_cursor: usize,
) -> KeyControl {
    let Some(on_input) = on_input.as_ref() else {
        return KeyControl::Handled;
    };

    editable_state.set_cursor(next_cursor);
    on_input(next).into()
}

/// Clamps a cursor to a valid byte index and UTF-8 character boundary.
///
/// # Arguments
///
/// * `value` — Input value that defines valid byte boundaries.
/// * `cursor` — Candidate cursor byte index.
///
/// # Returns
///
/// A [`usize`] cursor byte index within `value` and on a UTF-8 boundary.
fn clamp_cursor(value: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(value.len());
    while !value.is_char_boundary(cursor) {
        cursor = cursor.saturating_sub(1);
    }

    cursor
}

/// Returns the previous character boundary before or at a cursor.
///
/// # Arguments
///
/// * `value` — Input value that defines valid byte boundaries.
/// * `cursor` — Candidate cursor byte index.
///
/// # Returns
///
/// A [`usize`] cursor byte index for the previous character boundary.
fn previous_char_boundary(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    value[..cursor]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

/// Returns the next character boundary after or at a cursor.
///
/// # Arguments
///
/// * `value` — Input value that defines valid byte boundaries.
/// * `cursor` — Candidate cursor byte index.
///
/// # Returns
///
/// A [`usize`] cursor byte index for the next character boundary.
fn next_char_boundary(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    if cursor == value.len() {
        return cursor;
    }

    value[cursor..]
        .char_indices()
        .nth(1)
        .map_or(value.len(), |(index, _)| cursor + index)
}

/// Returns the character column represented by a byte cursor.
///
/// # Arguments
///
/// * `value` — Input value that defines character columns.
/// * `cursor` — Candidate cursor byte index.
///
/// # Returns
///
/// A [`usize`] character column represented by the clamped cursor.
fn char_column(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    value[..cursor].chars().count()
}

/// Vertical content span, with an exclusive bottom row.
#[derive(Clone, Copy)]
struct VerticalSpan {
    /// First row occupied by the span.
    top: u32,
    /// Row after the span.
    bottom: u32,
}

impl VerticalSpan {
    /// Creates a span starting at row zero with the provided height.
    fn from_height(height: u16) -> Self {
        Self {
            top: 0,
            bottom: u32::from(height),
        }
    }

    /// Returns this span offset by a parent content row.
    fn offset_by(self, offset: u32) -> Self {
        Self {
            top: self.top.saturating_add(offset),
            bottom: self.bottom.saturating_add(offset),
        }
    }

    /// Returns the span height.
    fn height(self) -> u32 {
        self.bottom.saturating_sub(self.top)
    }

    /// Converts the span to its tuple representation for hidden component APIs.
    fn into_tuple(self) -> (u32, u32) {
        (self.top, self.bottom)
    }
}

/// Moves a scroll offset just enough to make a span visible.
fn scroll_span_into_view(
    metadata: &StyleMetadata,
    span: VerticalSpan,
    viewport_height: u16,
    max_scroll_offset: u16,
) {
    if viewport_height == 0 {
        return;
    }

    let viewport_height = u32::from(viewport_height);
    let current = u32::from(metadata.scroll_offset().min(max_scroll_offset));
    let viewport_bottom = current.saturating_add(viewport_height);
    let next = if span.top < current {
        span.top
    } else if span.bottom > viewport_bottom {
        if span.height() > viewport_height {
            span.top
        } else {
            span.bottom.saturating_sub(viewport_height)
        }
    } else {
        current
    }
    .min(u32::from(max_scroll_offset));

    metadata.set_scroll_offset(u16::try_from(next).unwrap_or(u16::MAX));
}

/// Returns the focused control's vertical span within a view's render area.
fn focused_button_span_for_view(view: &View, ctx: &mut RenderCtx<'_, '_>) -> Option<VerticalSpan> {
    match view {
        View::Button { metadata, .. }
        | View::Input { metadata, .. }
        | View::TextArea { metadata, .. }
            if metadata.is_focused() && metadata.scroll_into_view_requested() =>
        {
            Some(VerticalSpan::from_height(ctx.area().height))
        }
        View::Block { child, metadata } => {
            let style = resolve_style(metadata, ctx);
            let area = ctx.area();
            let block = style.to_block_with_default_borders(Borders::ALL);
            let inner = block.inner(area);
            let top_offset = u32::from(inner.y.saturating_sub(area.y));

            ctx.with_area_inherited_style_and_selector_ancestor(
                inner,
                style.inherited_values(),
                metadata.clone(),
                |ctx| focused_button_span_for_view(child, ctx),
            )
            .map(|span| span.offset_by(top_offset))
        }
        View::Row { children, metadata } => {
            focused_button_span_for_layout_view(children, metadata, LayoutDirection::Row, ctx)
        }
        View::Column { children, metadata } => {
            focused_button_span_for_layout_view(children, metadata, LayoutDirection::Column, ctx)
        }
        View::Dynamic(child) => child.with_view(|child| focused_button_span_for_view(child, ctx)),
        View::Component(component) => component
            .focused_button_span(ctx)
            .map(|(top, bottom)| VerticalSpan { top, bottom }),
        View::Text { .. } | View::Button { .. } | View::Input { .. } | View::TextArea { .. } => {
            None
        }
    }
}

/// Returns the focused control's vertical span inside a layout view.
fn focused_button_span_for_layout_view(
    children: &[View],
    metadata: &StyleMetadata,
    default_direction: LayoutDirection,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    if children.is_empty() {
        return None;
    }

    let style = resolve_style(metadata, ctx);
    let direction = style.direction.unwrap_or(default_direction);

    match direction {
        LayoutDirection::Row => {
            focused_button_span_in_row_children(children, style.inherited_values(), metadata, ctx)
        }
        LayoutDirection::Column => {
            let min_heights = child_min_heights(children, style.inherited_values(), metadata, ctx);
            focused_button_span_in_column_children(
                children,
                &min_heights,
                style.inherited_values(),
                metadata,
                ctx,
            )
        }
    }
}

/// Returns the focused control's vertical span inside row children.
fn focused_button_span_in_row_children(
    children: &[View],
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    let area = ctx.area();
    let constraints = vec![Constraint::Fill(1); children.len()];
    let areas = Layout::horizontal(constraints).split(area);

    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        inherited_style,
        parent_metadata.clone(),
        |ctx| {
            children.iter().zip(areas.iter()).find_map(|(child, area)| {
                ctx.with_area(*area, |ctx| focused_button_span_for_view(child, ctx))
            })
        },
    )
}

/// Returns the focused control's vertical span inside column children.
fn focused_button_span_in_column_children(
    children: &[View],
    min_heights: &[u16],
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    let area = ctx.area();

    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        inherited_style,
        parent_metadata.clone(),
        |ctx| {
            let mut row = 0u32;

            for (child, min_height) in children.iter().zip(min_heights.iter()) {
                let child_area = Rect {
                    height: *min_height,
                    ..area
                };

                if let Some(span) =
                    ctx.with_area(child_area, |ctx| focused_button_span_for_view(child, ctx))
                {
                    return Some(span.offset_by(row));
                }

                row = row.saturating_add(u32::from(*min_height));
            }

            None
        },
    )
}

/// Renders child views into row or column areas.
///
/// # Arguments
///
/// * `children` — Views to render into split areas.
/// * `direction` — Axis used to split the current context area.
/// * `inherited_style` — Style values inherited by child views.
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
    children: &[View],
    direction: LayoutDirection,
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    if children.is_empty() {
        parent_metadata.set_max_scroll_offset(0);
        return Ok(());
    }

    if direction == LayoutDirection::Column
        && try_render_overflowing_column_children(children, inherited_style, parent_metadata, ctx)?
    {
        return Ok(());
    }

    parent_metadata.set_max_scroll_offset(0);

    let constraints = child_constraints(children, direction, inherited_style, parent_metadata, ctx);
    let areas = match direction {
        LayoutDirection::Row => Layout::horizontal(constraints).split(ctx.area()),
        LayoutDirection::Column => Layout::vertical(constraints).split(ctx.area()),
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

/// Renders a vertically overflowing column when the children exceed the viewport.
fn try_render_overflowing_column_children(
    children: &[View],
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<bool> {
    let min_heights = child_min_heights(children, inherited_style, parent_metadata, ctx);
    let content_height: u32 = min_heights.iter().map(|height| u32::from(*height)).sum();
    let area = ctx.area();
    let area_height = area.height;

    if content_height <= u32::from(area_height) || area_height == 0 {
        return Ok(false);
    }

    let content_area = scrolled_content_area(area);
    let min_heights = ctx.with_area(content_area, |ctx| {
        child_min_heights(children, inherited_style, parent_metadata, ctx)
    });
    let scrolled_content_height: u32 = min_heights.iter().map(|height| u32::from(*height)).sum();
    let content_height = scrolled_content_height.max(content_height);
    let max_scroll_offset =
        u16::try_from(content_height.saturating_sub(u32::from(area_height))).unwrap_or(u16::MAX);
    parent_metadata.set_max_scroll_offset(max_scroll_offset);

    if let Some(span) = ctx.with_area(content_area, |ctx| {
        focused_button_span_in_column_children(
            children,
            &min_heights,
            inherited_style,
            parent_metadata,
            ctx,
        )
    }) {
        scroll_span_into_view(parent_metadata, span, area_height, max_scroll_offset);
    }

    let row_offset = parent_metadata.scroll_offset().min(max_scroll_offset);
    ctx.with_area(content_area, |ctx| {
        render_scrolled_column_children(
            children,
            &min_heights,
            row_offset,
            inherited_style,
            parent_metadata,
            ctx,
        )
    })?;
    render_column_scrollbar(row_offset, max_scroll_offset, area_height, ctx);

    Ok(true)
}

/// Returns the content area used when a right-side scrollbar is visible.
fn scrolled_content_area(area: Rect) -> Rect {
    Rect {
        width: area.width.saturating_sub(1),
        ..area
    }
}

/// Renders the right-side scrollbar for an overflowing column.
fn render_column_scrollbar(
    row_offset: u16,
    max_scroll_offset: u16,
    viewport_height: u16,
    ctx: &mut RenderCtx<'_, '_>,
) {
    if ctx.area().width == 0 || viewport_height == 0 {
        return;
    }

    let content_length = usize::from(max_scroll_offset).saturating_add(1);
    let mut state = ScrollbarState::new(content_length)
        .position(usize::from(row_offset))
        .viewport_content_length(usize::from(viewport_height));

    ctx.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        &mut state,
    );
}

/// Renders a vertically overflowing column from a child scroll offset.
fn render_scrolled_column_children(
    children: &[View],
    min_heights: &[u16],
    row_offset: u16,
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let area = ctx.area();
    let bottom = area.y.saturating_add(area.height);
    let mut y = area.y;
    let mut skipped_rows = row_offset;

    for (child, min_height) in children.iter().zip(min_heights.iter()) {
        if skipped_rows >= *min_height {
            skipped_rows -= *min_height;
            continue;
        }

        let remaining = bottom.saturating_sub(y);
        if remaining == 0 {
            break;
        }

        let source_y = skipped_rows;
        skipped_rows = 0;
        let height = min_height.saturating_sub(source_y).min(remaining);
        if height == 0 {
            continue;
        }

        let child_area = Rect {
            x: area.x,
            y,
            width: area.width,
            height,
        };

        if source_y == 0 && height == *min_height {
            ctx.with_area_inherited_style_and_selector_ancestor(
                child_area,
                inherited_style,
                parent_metadata.clone(),
                |ctx| child.render(ctx),
            )?;
        } else {
            ctx.render_view_clipped(
                child,
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: *min_height,
                },
                source_y,
                child_area,
                inherited_style,
                parent_metadata.clone(),
            )?;
        }

        y = y.saturating_add(height);
    }

    Ok(())
}

/// Returns constraints for child layout.
fn child_constraints(
    children: &[View],
    direction: LayoutDirection,
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Vec<Constraint> {
    if direction == LayoutDirection::Column {
        let min_heights = child_min_heights(children, inherited_style, parent_metadata, ctx);
        if min_heights.iter().any(|height| *height > 1) {
            return min_heights.into_iter().map(Constraint::Min).collect();
        }
    }

    vec![Constraint::Fill(1); children.len()]
}

/// Returns minimum render heights for child views in a parent selector scope.
fn child_min_heights(
    children: &[View],
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Vec<u16> {
    let area = ctx.area();
    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        inherited_style,
        parent_metadata.clone(),
        |ctx| {
            children
                .iter()
                .map(|child| min_height_for_view(child, ctx))
                .collect()
        },
    )
}

/// Returns child minimum heights after applying row split widths.
fn row_child_min_heights(
    children: &[View],
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Vec<u16> {
    let area = ctx.area();
    let constraints = vec![Constraint::Fill(1); children.len()];
    let areas = Layout::horizontal(constraints).split(area);

    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        inherited_style,
        parent_metadata.clone(),
        |ctx| {
            children
                .iter()
                .zip(areas.iter())
                .map(|(child, area)| ctx.with_area(*area, |ctx| min_height_for_view(child, ctx)))
                .collect()
        },
    )
}

/// Returns the minimum useful render height for a view.
fn min_height_for_view(view: &View, ctx: &mut RenderCtx<'_, '_>) -> u16 {
    match view {
        View::Text { content, metadata } => {
            let style = resolve_style(metadata, ctx);
            line_count_height(text_paragraph(content.as_str(), style).line_count(ctx.area().width))
        }
        View::Dynamic(child) => child.with_view(|child| min_height_for_view(child, ctx)),
        View::Button { metadata, .. } => {
            let style = resolve_style(metadata, ctx);
            1 + vertical_border_rows(style.borders.unwrap_or(Borders::ALL))
                + vertical_padding_rows(style.padding)
        }
        View::Input { metadata, .. } => {
            let _style = resolve_style(metadata, ctx);
            1
        }
        View::TextArea {
            value,
            metadata,
            editable_state,
        } => {
            let style = resolve_style(metadata, ctx);
            line_count_height(
                text_area_paragraph(
                    value.as_str(),
                    style,
                    editable_state.vertical_scroll(),
                    editable_state.horizontal_scroll(),
                )
                .line_count(ctx.area().width),
            )
            .max(1)
        }
        View::Block { child, metadata } => {
            let style = resolve_style(metadata, ctx);
            let area = ctx.area();
            let child_height = ctx.with_area_inherited_style_and_selector_ancestor(
                area,
                style.inherited_values(),
                metadata.clone(),
                |ctx| min_height_for_view(child, ctx),
            );

            child_height
                + vertical_border_rows(style.borders.unwrap_or(Borders::ALL))
                + vertical_padding_rows(style.padding)
        }
        View::Row { children, metadata } => {
            min_height_for_layout_view(children, metadata, LayoutDirection::Row, ctx)
        }
        View::Column { children, metadata } => {
            min_height_for_layout_view(children, metadata, LayoutDirection::Column, ctx)
        }
        View::Component(component) => component.min_height(ctx),
    }
}

/// Returns minimum height for a layout view after resolving its direction.
fn min_height_for_layout_view(
    children: &[View],
    metadata: &StyleMetadata,
    default_direction: LayoutDirection,
    ctx: &mut RenderCtx<'_, '_>,
) -> u16 {
    if children.is_empty() {
        return 0;
    }

    let style = resolve_style(metadata, ctx);
    let direction = style.direction.unwrap_or(default_direction);
    let min_heights = match direction {
        LayoutDirection::Row => {
            row_child_min_heights(children, style.inherited_values(), metadata, ctx)
        }
        LayoutDirection::Column => {
            child_min_heights(children, style.inherited_values(), metadata, ctx)
        }
    };

    match direction {
        LayoutDirection::Row => min_heights.into_iter().max().unwrap_or(0),
        LayoutDirection::Column => min_heights.into_iter().fold(0, u16::saturating_add),
    }
}

/// Returns how many vertical rows the configured borders consume.
fn vertical_border_rows(borders: Borders) -> u16 {
    u16::from(borders.contains(Borders::TOP)) + u16::from(borders.contains(Borders::BOTTOM))
}

/// Returns how many vertical rows the configured padding consumes.
fn vertical_padding_rows(padding: Option<crate::TuiSpacing>) -> u16 {
    padding.map_or(0, |padding| padding.top.saturating_add(padding.bottom))
}

/// Dispatches an event through child views until one requests exit.
///
/// # Arguments
///
/// * `children` — Child views to visit in order.
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
fn handle_child_events(children: &mut [View], event: &Event) -> Result<AppControl> {
    for child in children {
        if child.dispatch_event_ref(event)? == AppControl::Exit {
            return Ok(AppControl::Exit);
        }
    }

    Ok(AppControl::Continue)
}

/// Dispatches a key event through child views until one handles it.
///
/// # Arguments
///
/// * `children` — Child views to visit in order.
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
fn handle_child_key_events(children: &mut [View], key: &KeyEvent) -> Result<KeyControl> {
    for child in children {
        let control = child.dispatch_key_event_ref(key)?;
        if control != KeyControl::Pass {
            return Ok(control);
        }
    }

    Ok(KeyControl::Pass)
}
