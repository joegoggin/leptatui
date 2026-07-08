//! Rendering and event traversal for Leptatui views.
//!
//! This module maps [`View`] variants to Ratatui widgets, layout splits, and
//! component event propagation.

use std::{
    ops::Range,
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use leptos::prelude::{GetUntracked, ReadSignal};
use ratatui::{
    layout::{Constraint, Layout, Position, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use crate::{
    ThemeVariables,
    app::{AppControl, Result},
    component::{Component, FocusedControl, KeyControl, RenderCtx},
    context,
    style::{Borders, LayoutDirection, Modifier, TuiStyle},
};

use super::{
    metadata::{EditableState, PendingInsertKey, StyleMetadata, VimMode},
    model::{FormAction, InputAction, View},
};

/// Maximum time allowed between insert-mode `j` and `k` escape keys.
const INSERT_ESCAPE_TIMEOUT: Duration = Duration::from_millis(1000);

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
fn input_paragraph<'a>(
    value: &'a str,
    style: TuiStyle,
    horizontal_scroll: u16,
    selection: Option<Range<usize>>,
) -> Paragraph<'a> {
    let paragraph = selection.map_or_else(
        || Paragraph::new(value),
        |selection| Paragraph::new(selected_text_lines(value, selection, style)),
    );

    paragraph
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
    selection: Option<Range<usize>>,
) -> Paragraph<'a> {
    let paragraph = selection.map_or_else(
        || Paragraph::new(value),
        |selection| Paragraph::new(selected_text_lines(value, selection, style)),
    );

    paragraph
        .style(style.to_ratatui_style())
        .wrap(Wrap { trim: false })
        .scroll((vertical_scroll, horizontal_scroll))
}

/// Returns logical text lines with the selected bytes rendered in reverse video.
fn selected_text_lines<'a>(
    value: &'a str,
    selection: Range<usize>,
    style: TuiStyle,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut line_start = 0usize;
    let selection_style = style.to_ratatui_style().add_modifier(Modifier::REVERSED);

    loop {
        let line_end = value[line_start..]
            .find('\n')
            .map_or(value.len(), |index| line_start + index);
        lines.push(Line::from(selected_line_spans(
            value,
            line_start..line_end,
            selection.clone(),
            selection_style,
        )));

        if line_end == value.len() {
            break;
        }

        line_start = line_end + 1;
        if line_start == value.len() {
            lines.push(Line::from(Vec::<Span<'a>>::new()));
            break;
        }
    }

    lines
}

/// Returns spans for one logical line with the selection split out.
fn selected_line_spans<'a>(
    value: &'a str,
    line: Range<usize>,
    selection: Range<usize>,
    selection_style: ratatui::style::Style,
) -> Vec<Span<'a>> {
    if selection.start == selection.end
        || selection.end <= line.start
        || selection.start >= line.end
    {
        return vec![Span::raw(&value[line])];
    }

    let selected_start = selection.start.max(line.start);
    let selected_end = selection.end.min(line.end);
    let mut spans = Vec::new();

    if line.start < selected_start {
        spans.push(Span::raw(&value[line.start..selected_start]));
    }
    spans.push(Span::styled(
        &value[selected_start..selected_end],
        selection_style,
    ));
    if selected_end < line.end {
        spans.push(Span::raw(&value[selected_end..line.end]));
    }

    spans
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
            Self::Image {
                source,
                alt,
                metadata,
            } => {
                let style = resolve_style(metadata, ctx);
                let path = match source {
                    super::model::ImageSource::Path(path) => path.as_path(),
                };
                ctx.render_terminal_image_path(path, alt.as_deref(), style.to_ratatui_style());
                Ok(())
            }
            Self::Row { children, metadata } => {
                render_layout_view(children, metadata, LayoutDirection::Row, ctx)
            }
            Self::Column { children, metadata } => {
                render_layout_view(children, metadata, LayoutDirection::Column, ctx)
            }
            Self::Form {
                children, metadata, ..
            } => render_layout_view(children, metadata, LayoutDirection::Column, ctx),
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
                let block = style.to_block_with_default_borders(Borders::ALL);
                let inner = block.inner(ctx.area());
                let pending = pending_insert_render(value.as_str(), editable_state);
                let display_value = if let Some(pending) = pending.as_ref() {
                    pending.value.as_str()
                } else if value.is_empty() {
                    placeholder.as_deref().unwrap_or("")
                } else {
                    value.as_str()
                };
                let horizontal_scroll = if let Some(pending) = pending.as_ref() {
                    input_horizontal_scroll(
                        pending.value.as_str(),
                        pending.scroll_cursor,
                        editable_state.horizontal_scroll(),
                        inner.width,
                    )
                } else if value.is_empty() {
                    0
                } else {
                    input_horizontal_scroll(
                        value.as_str(),
                        editable_state.cursor(),
                        editable_state.horizontal_scroll(),
                        inner.width,
                    )
                };
                ctx.render_widget(block);
                ctx.with_area(inner, |ctx| {
                    ctx.render_widget(input_paragraph(
                        display_value,
                        style,
                        horizontal_scroll,
                        pending
                            .as_ref()
                            .and_then(|pending| pending.selection.clone())
                            .or_else(|| {
                                visual_selection_range(
                                    value.as_str(),
                                    editable_state,
                                    EditableControlKind::Input,
                                )
                            }),
                    ));
                    if metadata.is_focused() {
                        if let Some(pending) = pending.as_ref() {
                            set_input_cursor(
                                pending.value.as_str(),
                                pending.cursor,
                                horizontal_scroll,
                                ctx,
                            );
                        } else {
                            set_input_cursor(
                                value.as_str(),
                                editable_state.cursor(),
                                horizontal_scroll,
                                ctx,
                            );
                        }
                    }
                });
                metadata.clear_scroll_into_view_request();
                Ok(())
            }
            Self::TextArea {
                value,
                placeholder,
                metadata,
                editable_state,
                ..
            } => {
                let style = resolve_style(metadata, ctx);
                let block = style.to_block_with_default_borders(Borders::ALL);
                let inner = block.inner(ctx.area());
                let pending = pending_insert_render(value.as_str(), editable_state);
                let display_value = if let Some(pending) = pending.as_ref() {
                    pending.value.as_str()
                } else if value.is_empty() {
                    placeholder.as_deref().unwrap_or("")
                } else {
                    value.as_str()
                };
                let vertical_scroll = if let Some(pending) = pending.as_ref() {
                    text_area_vertical_scroll(
                        pending.value.as_str(),
                        pending.scroll_cursor,
                        editable_state.vertical_scroll(),
                        inner.height,
                        inner.width,
                    )
                } else if value.is_empty() {
                    0
                } else {
                    text_area_vertical_scroll(
                        value.as_str(),
                        editable_state.cursor(),
                        editable_state.vertical_scroll(),
                        inner.height,
                        inner.width,
                    )
                };
                ctx.render_widget(block);
                ctx.with_area(inner, |ctx| {
                    ctx.render_widget(text_area_paragraph(
                        display_value,
                        style,
                        vertical_scroll,
                        editable_state.horizontal_scroll(),
                        pending
                            .as_ref()
                            .and_then(|pending| pending.selection.clone())
                            .or_else(|| {
                                visual_selection_range(
                                    value.as_str(),
                                    editable_state,
                                    EditableControlKind::TextArea,
                                )
                            }),
                    ));
                    if metadata.is_focused() {
                        if let Some(pending) = pending.as_ref() {
                            set_text_area_pending_insert_cursor(
                                pending.value.as_str(),
                                pending.cursor,
                                vertical_scroll,
                                editable_state.horizontal_scroll(),
                                ctx,
                            );
                        } else {
                            set_text_area_cursor(
                                value.as_str(),
                                editable_state.cursor(),
                                vertical_scroll,
                                editable_state.horizontal_scroll(),
                                ctx,
                            );
                        }
                    }
                });
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

    /// Emits any expired pending insert-mode key in this view tree.
    #[doc(hidden)]
    pub fn __flush_pending_input(&mut self) -> Option<AppControl> {
        self.flush_pending_input_at(Instant::now())
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

    /// Emits an expired pending insert-mode key at the provided time.
    fn flush_pending_input_at(&mut self, now: Instant) -> Option<AppControl> {
        match self {
            Self::Block { child, .. } => child.flush_pending_input_at(now),
            Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Form { children, .. } => flush_child_pending_input(children, now),
            Self::Dynamic(child) => child.with_view_mut(|child| child.flush_pending_input_at(now)),
            Self::Component(component) => component.flush_pending_input(),
            Self::Input {
                value,
                on_input,
                editable_state,
                ..
            }
            | Self::TextArea {
                value,
                on_input,
                editable_state,
                ..
            } => flush_expired_insert_key(value, on_input, editable_state, now),
            Self::Text { .. } | Self::Button { .. } => None,
            Self::Image { .. } => None,
        }
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
        focused_control_span_for_view(self, ctx).map(VerticalSpan::into_tuple)
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

    /// Returns the focused built-in control in this view tree.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing focused control metadata when a supported
    /// built-in control is focused.
    #[doc(hidden)]
    pub fn __focused_control(&self) -> Option<FocusedControl> {
        self.focused_control()
    }

    /// Handles form-owned submit or cancel keys in this view tree.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to evaluate for form behavior.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing key traversal control when a form handles the
    /// key.
    #[doc(hidden)]
    pub fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.handle_form_key_ref(&key)
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
            Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Form { children, .. } => handle_child_key_events(children, key),
            Self::Dynamic(child) => child.with_view_mut(|child| child.dispatch_key_event_ref(key)),
            Self::Component(component) => component.dispatch_key_event(*key),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. }
            | Self::Image { .. } => Ok(KeyControl::Pass),
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

        if let Some(control) = self.handle_form_key_ref(key) {
            self.clear_scroll_to_top_key_pending();
            return control;
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
            Self::Row { children, metadata }
            | Self::Column { children, metadata }
            | Self::Form {
                children, metadata, ..
            } => {
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
            | Self::TextArea { .. }
            | Self::Image { .. } => false,
        }
    }

    /// Scrolls the first overflowing vertical layout to an absolute boundary.
    fn scroll_first_overflowing_to(&mut self, boundary: ScrollBoundary) -> bool {
        match self {
            Self::Block { child, .. } => child.scroll_first_overflowing_to(boundary),
            Self::Row { children, metadata }
            | Self::Column { children, metadata }
            | Self::Form {
                children, metadata, ..
            } => {
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
            | Self::TextArea { .. }
            | Self::Image { .. } => false,
        }
    }

    /// Returns whether this tree contains a layout with scrollable overflow.
    fn has_overflowing_scroll_target(&self) -> bool {
        match self {
            Self::Block { child, .. } => child.has_overflowing_scroll_target(),
            Self::Row { children, metadata }
            | Self::Column { children, metadata }
            | Self::Form {
                children, metadata, ..
            } => {
                metadata.max_scroll_offset() > 0
                    || children.iter().any(Self::has_overflowing_scroll_target)
            }
            Self::Dynamic(child) => child.with_view(Self::has_overflowing_scroll_target),
            Self::Component(component) => component.has_overflowing_scroll_target(),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. }
            | Self::Image { .. } => false,
        }
    }

    /// Returns metadata used to store default key-sequence state.
    fn key_sequence_metadata(&self) -> Option<&StyleMetadata> {
        match self {
            Self::Block { metadata, .. }
            | Self::Text { metadata, .. }
            | Self::Row { metadata, .. }
            | Self::Column { metadata, .. }
            | Self::Form { metadata, .. }
            | Self::Button { metadata, .. }
            | Self::Input { metadata, .. }
            | Self::TextArea { metadata, .. }
            | Self::Image { metadata, .. } => Some(metadata),
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
            Self::TextArea {
                value,
                metadata,
                on_input,
                editable_state,
                ..
            } if metadata.is_focused() => {
                let control = handle_text_area_key(value.as_str(), on_input, editable_state, key);
                if matches!(control, Some(KeyControl::Handled | KeyControl::Exit)) {
                    metadata.request_scroll_into_view();
                }

                control
            }
            Self::Block { child, .. } => child.handle_focused_input_key_ref(key),
            Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Form { children, .. } => children
                .iter_mut()
                .find_map(|child| child.handle_focused_input_key_ref(key)),
            Self::Dynamic(child) => {
                child.with_view_mut(|child| child.handle_focused_input_key_ref(key))
            }
            Self::Component(component) => component.handle_focused_input_key(*key),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. }
            | Self::Image { .. } => None,
        }
    }

    /// Returns the focused built-in control in this view tree.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing focused control metadata when a supported
    /// built-in control is focused.
    fn focused_control(&self) -> Option<FocusedControl> {
        match self {
            Self::Button { metadata, .. } if metadata.is_focused() => Some(FocusedControl::Button),
            Self::Input {
                metadata,
                editable_state,
                ..
            } if metadata.is_focused() => Some(FocusedControl::Input {
                insert_mode: editable_state.mode() == VimMode::Insert
                    && !has_active_insert_key_pending(editable_state, Instant::now()),
                visual_mode: matches!(editable_state.mode(), VimMode::Visual | VimMode::VisualLine),
            }),
            Self::TextArea {
                metadata,
                editable_state,
                ..
            } if metadata.is_focused() => Some(FocusedControl::TextArea {
                insert_mode: editable_state.mode() == VimMode::Insert
                    && !has_active_insert_key_pending(editable_state, Instant::now()),
                visual_mode: matches!(editable_state.mode(), VimMode::Visual | VimMode::VisualLine),
            }),
            Self::Block { child, .. } => child.focused_control(),
            Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Form { children, .. } => children.iter().find_map(Self::focused_control),
            Self::Dynamic(child) => child.with_view(Self::focused_control),
            Self::Component(component) => component.focused_control(),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. }
            | Self::Image { .. } => None,
        }
    }

    /// Handles form-owned submit and cancel keys.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to evaluate for form behavior.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing key traversal control when a form handles the
    /// key.
    fn handle_form_key_ref(&mut self, key: &KeyEvent) -> Option<KeyControl> {
        match self {
            Self::Form {
                children,
                on_submit,
                on_cancel,
                ..
            } => {
                if let Some(control) = children
                    .iter_mut()
                    .find_map(|child| child.handle_form_key_ref(key))
                {
                    return Some(control);
                }

                let focused = children.iter().find_map(Self::focused_control)?;
                handle_form_focused_key(focused, key, on_submit, on_cancel)
            }
            Self::Block { child, .. } => child.handle_form_key_ref(key),
            Self::Row { children, .. } | Self::Column { children, .. } => children
                .iter_mut()
                .find_map(|child| child.handle_form_key_ref(key)),
            Self::Dynamic(child) => child.with_view_mut(|child| child.handle_form_key_ref(key)),
            Self::Component(component) => component.handle_form_key(*key),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. }
            | Self::Image { .. } => None,
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
            Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Form { children, .. } => children.iter().map(Self::focusable_count).sum(),
            Self::Dynamic(child) => child.with_view(Self::focusable_count),
            Self::Component(component) => component.focusable_count(),
            Self::Text { .. } | Self::Image { .. } => 0,
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
            Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Form { children, .. } => children
                .iter()
                .find_map(|child| child.focused_index_inner(index)),
            Self::Dynamic(child) => child.with_view(|child| child.focused_index_inner(index)),
            Self::Component(component) => component.focused_index_inner(index),
            Self::Text { .. } | Self::Image { .. } => None,
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
            Self::Button { metadata, .. } => {
                let focused = *index == target;
                metadata.set_focused(focused);
                if focused {
                    metadata.request_scroll_into_view();
                } else {
                    metadata.clear_scroll_into_view_request();
                }
                *index += 1;
            }
            Self::Input {
                metadata,
                editable_state,
                ..
            }
            | Self::TextArea {
                metadata,
                editable_state,
                ..
            } => {
                let focused = *index == target;
                metadata.set_focused(focused);
                if focused {
                    editable_state.set_mode(VimMode::Normal);
                    metadata.request_scroll_into_view();
                } else {
                    metadata.clear_scroll_into_view_request();
                }
                *index += 1;
            }
            Self::Block { child, .. } => child.set_focus_by_index_inner(target, index),
            Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Form { children, .. } => {
                for child in children {
                    child.set_focus_by_index_inner(target, index);
                }
            }
            Self::Dynamic(child) => {
                child.with_view_mut(|child| child.set_focus_by_index_inner(target, index));
            }
            Self::Component(component) => component.set_focus_by_index_inner(target, index),
            Self::Text { .. } | Self::Image { .. } => {}
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
            Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Form { children, .. } => {
                children.iter().find_map(Self::activate_focused_button)
            }
            Self::Dynamic(child) => child.with_view(Self::activate_focused_button),
            Self::Component(component) => component.activate_focused_button(),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. }
            | Self::Image { .. } => None,
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
            Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Form { children, .. } => handle_child_events(children, event),
            Self::Dynamic(child) => child.with_view_mut(|child| child.dispatch_event_ref(event)),
            Self::Component(component) => component.handle_event(event.clone()),
            Self::Text { .. }
            | Self::Button { .. }
            | Self::Input { .. }
            | Self::TextArea { .. }
            | Self::Image { .. } => Ok(AppControl::Continue),
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

    /// Emits any expired pending insert-mode key in the view tree.
    #[doc(hidden)]
    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        View::__flush_pending_input(self)
    }

    /// Returns the focused built-in control in the view tree, if any.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing focused control metadata when a supported
    /// built-in control is focused.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<FocusedControl> {
        View::__focused_control(self)
    }

    /// Handles form-owned submit or cancel keys in the view tree, if any.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to evaluate for form behavior.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing key traversal control when a form handles the
    /// key.
    #[doc(hidden)]
    fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        View::__handle_form_key(self, key)
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

/// Transient render state for an uncommitted pending insert-mode key.
struct PendingInsertRender {
    /// Display value with the pending key inserted.
    value: String,
    /// Byte range highlighted while the pending key is still active.
    selection: Option<Range<usize>>,
    /// Cursor byte index used to place the terminal cursor.
    cursor: usize,
    /// Cursor byte index used to scroll the pending key into view.
    scroll_cursor: usize,
}

/// Converts a handled flag into the matching key traversal control.
fn key_control_from_bool(handled: bool) -> KeyControl {
    if handled {
        KeyControl::Handled
    } else {
        KeyControl::Pass
    }
}

/// Handles a key against the form owning the focused control.
///
/// # Arguments
///
/// * `focused` — Focused descendant control metadata.
/// * `key` — Key event being evaluated for form behavior.
/// * `on_submit` — Optional submit callback stored by the form.
/// * `on_cancel` — Optional cancel callback stored by the form.
///
/// # Returns
///
/// An [`Option`] containing key traversal control when the form handles the
/// key.
fn handle_form_focused_key(
    focused: FocusedControl,
    key: &KeyEvent,
    on_submit: &Option<FormAction>,
    on_cancel: &Option<FormAction>,
) -> Option<KeyControl> {
    match (focused, key.code) {
        (FocusedControl::Input { .. }, KeyCode::Enter) => Some(form_action_control(on_submit)),
        (FocusedControl::TextArea { .. }, KeyCode::Enter)
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Some(form_action_control(on_submit))
        }
        (
            FocusedControl::Input {
                insert_mode: true, ..
            },
            KeyCode::Esc,
        )
        | (
            FocusedControl::Input {
                visual_mode: true, ..
            },
            KeyCode::Esc,
        )
        | (
            FocusedControl::TextArea {
                insert_mode: true, ..
            },
            KeyCode::Esc,
        )
        | (
            FocusedControl::TextArea {
                visual_mode: true, ..
            },
            KeyCode::Esc,
        ) => None,
        (_, KeyCode::Esc) => Some(form_action_control(on_cancel)),
        _ => None,
    }
}

/// Converts an optional form action into key traversal control.
///
/// # Arguments
///
/// * `action` — Optional form callback to invoke.
///
/// # Returns
///
/// A [`KeyControl`] value that stops key propagation and reflects the callback
/// result.
fn form_action_control(action: &Option<FormAction>) -> KeyControl {
    action
        .as_ref()
        .map_or(KeyControl::Handled, |action| action().into())
}

/// Returns render-only display state for a pending insert-mode key.
fn pending_insert_render(
    value: &str,
    editable_state: &EditableState,
) -> Option<PendingInsertRender> {
    let pending = editable_state.insert_key_pending()?;
    if editable_state.mode() != VimMode::Insert {
        return None;
    }

    let now = Instant::now();
    let active = !insert_key_pending_expired(pending, now);
    let cursor = clamp_cursor(value, editable_state.cursor());
    let pending_key = pending.key();
    let mut display_value =
        String::with_capacity(value.len().saturating_add(pending_key.len_utf8()));
    display_value.push_str(&value[..cursor]);
    display_value.push(pending_key);
    display_value.push_str(&value[cursor..]);

    let pending_end = cursor.saturating_add(pending_key.len_utf8());
    Some(PendingInsertRender {
        value: display_value,
        selection: active.then_some(cursor..pending_end),
        cursor: if active { cursor } else { pending_end },
        scroll_cursor: pending_end,
    })
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

/// Sets the terminal cursor for a focused single-line input.
fn set_input_cursor(
    value: &str,
    cursor: usize,
    horizontal_scroll: u16,
    ctx: &mut RenderCtx<'_, '_>,
) {
    let area = ctx.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let column = char_column(value, cursor).saturating_sub(usize::from(horizontal_scroll));
    ctx.set_cursor_position(cursor_position_in_area(area, column, 0));
}

/// Sets the terminal cursor for a focused multiline text area.
fn set_text_area_cursor(
    value: &str,
    cursor: usize,
    vertical_scroll: u16,
    horizontal_scroll: u16,
    ctx: &mut RenderCtx<'_, '_>,
) {
    let area = ctx.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let (row, column) = text_area_cursor_position(value, cursor, area.width);
    let row = row.saturating_sub(usize::from(vertical_scroll));
    let column = column.saturating_sub(usize::from(horizontal_scroll));
    ctx.set_cursor_position(cursor_position_in_area(area, column, row));
}

/// Sets the terminal cursor on a pending inserted text-area character.
fn set_text_area_pending_insert_cursor(
    value: &str,
    cursor: usize,
    vertical_scroll: u16,
    horizontal_scroll: u16,
    ctx: &mut RenderCtx<'_, '_>,
) {
    let area = ctx.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let (row, column) = text_area_character_position(value, cursor, area.width);
    let row = row.saturating_sub(usize::from(vertical_scroll));
    let column = column.saturating_sub(usize::from(horizontal_scroll));
    ctx.set_cursor_position(cursor_position_in_area(area, column, row));
}

/// Returns an absolute cursor position clamped inside a render area.
fn cursor_position_in_area(area: Rect, column: usize, row: usize) -> Position {
    Position {
        x: area.x.saturating_add(
            u16::try_from(column.min(usize::from(area.width.saturating_sub(1))))
                .unwrap_or(u16::MAX),
        ),
        y: area.y.saturating_add(
            u16::try_from(row.min(usize::from(area.height.saturating_sub(1)))).unwrap_or(u16::MAX),
        ),
    }
}

/// Returns the vertical scroll offset needed to keep a text-area cursor visible.
///
/// # Arguments
///
/// * `value` — Text-area value used to map the cursor byte index to a rendered
///   row.
/// * `cursor` — Cursor byte index to keep visible.
/// * `current_scroll` — Current vertical scroll offset.
/// * `height` — Available text-area render height.
/// * `width` — Available text-area render width.
///
/// # Returns
///
/// A [`u16`] scroll offset that keeps the cursor within the render height.
fn text_area_vertical_scroll(
    value: &str,
    cursor: usize,
    current_scroll: u16,
    height: u16,
    width: u16,
) -> u16 {
    if height == 0 || width == 0 {
        return 0;
    }

    let viewport_height = usize::from(height);
    let total_rows = text_area_rendered_rows(value, width);
    let max_scroll = total_rows.saturating_sub(viewport_height);
    let current_scroll = usize::from(current_scroll).min(max_scroll);
    let cursor_row = text_area_cursor_row(value, cursor, width);
    let viewport_bottom = current_scroll.saturating_add(viewport_height);
    let next_scroll = if cursor_row < current_scroll {
        cursor_row
    } else if cursor_row >= viewport_bottom {
        cursor_row.saturating_sub(viewport_height.saturating_sub(1))
    } else {
        current_scroll
    }
    .min(max_scroll);

    u16::try_from(next_scroll).unwrap_or(u16::MAX)
}

/// Returns the number of wrapped rows needed to render a text-area value.
///
/// # Arguments
///
/// * `value` — Text-area value to measure.
/// * `width` — Available text-area render width.
///
/// # Returns
///
/// A [`usize`] row count for the wrapped text-area value.
fn text_area_rendered_rows(value: &str, width: u16) -> usize {
    if width == 0 {
        return 1;
    }

    let width = usize::from(width);
    let mut rows = 1usize;
    let mut column = 0usize;

    for character in value.chars() {
        if character == '\n' {
            rows = rows.saturating_add(1);
            column = 0;
            continue;
        }

        if column == width {
            rows = rows.saturating_add(1);
            column = 0;
        }
        column = column.saturating_add(1);
    }

    rows
}

/// Returns the wrapped render row represented by a text-area cursor.
///
/// # Arguments
///
/// * `value` — Text-area value used to map the cursor byte index.
/// * `cursor` — Cursor byte index to locate.
/// * `width` — Available text-area render width.
///
/// # Returns
///
/// A [`usize`] row index containing the cursor.
fn text_area_cursor_row(value: &str, cursor: usize, width: u16) -> usize {
    text_area_cursor_position(value, cursor, width).0
}

/// Returns the wrapped render row and column represented by a text-area cursor.
fn text_area_cursor_position(value: &str, cursor: usize, width: u16) -> (usize, usize) {
    if width == 0 {
        return (0, 0);
    }

    let cursor = clamp_cursor(value, cursor);
    let width = usize::from(width);
    let mut row = 0usize;
    let mut column = 0usize;

    for (index, character) in value.char_indices() {
        if index >= cursor {
            break;
        }

        if character == '\n' {
            row = row.saturating_add(1);
            column = 0;
            continue;
        }

        if column == width {
            row = row.saturating_add(1);
            column = 0;
        }
        column = column.saturating_add(1);
    }

    (row, column)
}

/// Returns the wrapped render row and column for the character at a byte index.
fn text_area_character_position(value: &str, cursor: usize, width: u16) -> (usize, usize) {
    if width == 0 {
        return (0, 0);
    }

    let cursor = clamp_cursor(value, cursor);
    let width = usize::from(width);
    let mut row = 0usize;
    let mut column = 0usize;

    for (index, character) in value.char_indices() {
        if index >= cursor {
            if index == cursor && character != '\n' && column == width {
                row = row.saturating_add(1);
                column = 0;
            }
            break;
        }

        if character == '\n' {
            row = row.saturating_add(1);
            column = 0;
            continue;
        }

        if column == width {
            row = row.saturating_add(1);
            column = 0;
        }
        column = column.saturating_add(1);
    }

    (row, column)
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
    handle_editable_key(
        value,
        on_input,
        editable_state,
        key,
        EditableControlKind::Input,
    )
}

/// Handles a focused text-area key and returns whether default propagation stops.
///
/// # Arguments
///
/// * `value` — Current controlled text-area value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor and scroll state for the text area.
/// * `key` — Key event to apply to the text area.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when the key is handled by
/// text-area editing behavior.
fn handle_text_area_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
) -> Option<KeyControl> {
    handle_editable_key(
        value,
        on_input,
        editable_state,
        key,
        EditableControlKind::TextArea,
    )
}

/// Editable control variant used by shared key handling helpers.
#[derive(Clone, Copy, Eq, PartialEq)]
enum EditableControlKind {
    /// Single-line input behavior.
    Input,
    /// Multiline text-area behavior.
    TextArea,
}

/// Handles a focused editable-control key according to its Vim mode.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
/// * `key` — Key event to apply to the control.
/// * `kind` — Editable control variant receiving the key.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when editable behavior
/// handles the key.
fn handle_editable_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
) -> Option<KeyControl> {
    match editable_state.mode() {
        VimMode::Insert => handle_insert_mode_key(value, on_input, editable_state, key, kind),
        VimMode::Normal => handle_normal_mode_key(value, on_input, editable_state, key, kind),
        VimMode::Visual | VimMode::VisualLine => {
            handle_visual_mode_key(value, on_input, editable_state, key, kind)
        }
    }
}

/// Handles insert-mode editing and cursor movement for a focused control.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
/// * `key` — Key event to apply while the control is in insert mode.
/// * `kind` — Editable control variant receiving the key.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when insert-mode behavior
/// handles the key.
fn handle_insert_mode_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
) -> Option<KeyControl> {
    editable_state.set_normal_key_pending(None);

    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
    let now = Instant::now();

    if let Some(pending) = editable_state.take_insert_key_pending() {
        return Some(handle_pending_insert_mode_key(
            value,
            on_input,
            editable_state,
            key,
            kind,
            pending,
            now,
        ));
    }

    match key.code {
        KeyCode::Esc => {
            exit_insert_mode(value, editable_state);
            Some(KeyControl::Handled)
        }
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
            let cursor = match kind {
                EditableControlKind::Input => 0,
                EditableControlKind::TextArea => {
                    text_area_line_start(value, editable_state.cursor())
                }
            };
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::End => {
            let cursor = insert_line_end(value, editable_state.cursor(), kind);
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Up if kind == EditableControlKind::TextArea => {
            let cursor = text_area_previous_line_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Down if kind == EditableControlKind::TextArea => {
            let cursor = text_area_next_line_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Enter if kind == EditableControlKind::TextArea => Some(handle_insert_input_key(
            value,
            on_input,
            editable_state,
            '\n',
        )),
        KeyCode::Enter => Some(KeyControl::Handled),
        KeyCode::Backspace => Some(handle_backspace_input_key(
            value,
            on_input,
            editable_state,
            kind,
        )),
        KeyCode::Delete => Some(handle_delete_input_key(value, on_input, editable_state)),
        KeyCode::Char('j') if plain_key => {
            editable_state.set_insert_key_pending('j', now);
            Some(KeyControl::Handled)
        }
        KeyCode::Char(character) if plain_key => Some(handle_insert_input_key(
            value,
            on_input,
            editable_state,
            character,
        )),
        _ => None,
    }
}

/// Handles the second key in an insert-mode key sequence.
fn handle_pending_insert_mode_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
    pending: PendingInsertKey,
    now: Instant,
) -> KeyControl {
    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    if insert_key_pending_expired(pending, now) {
        return handle_expired_pending_insert_mode_key(
            value,
            on_input,
            editable_state,
            key,
            kind,
            pending.key(),
            plain_key,
        );
    }

    let pending_key = pending.key();

    match (pending_key, key.code) {
        ('j', KeyCode::Char('k')) if plain_key => {
            exit_insert_mode(value, editable_state);
            KeyControl::Handled
        }
        ('j', KeyCode::Char(character)) if plain_key => {
            let mut inserted = String::with_capacity(pending_key.len_utf8() + character.len_utf8());
            inserted.push(pending_key);
            inserted.push(character);
            handle_insert_text_key(value, on_input, editable_state, &inserted)
        }
        ('j', KeyCode::Esc) => {
            exit_insert_mode(value, editable_state);
            KeyControl::Handled
        }
        ('j', KeyCode::Backspace) => {
            editable_state.set_cursor(clamp_cursor(value, editable_state.cursor()));
            KeyControl::Handled
        }
        ('j', KeyCode::Enter) if plain_key && kind == EditableControlKind::TextArea => {
            let mut text = String::with_capacity(pending_key.len_utf8() + 1);
            text.push(pending_key);
            text.push('\n');
            handle_insert_text_key(value, on_input, editable_state, &text)
        }
        ('j', KeyCode::Enter) if plain_key => {
            handle_insert_input_key(value, on_input, editable_state, pending_key)
        }
        ('j', _) => handle_insert_input_key(value, on_input, editable_state, pending_key),
        _ => KeyControl::Handled,
    }
}

/// Handles a key received after an insert-mode sequence times out.
fn handle_expired_pending_insert_mode_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
    pending: char,
    plain_key: bool,
) -> KeyControl {
    match key.code {
        KeyCode::Char(character) if plain_key => {
            let mut inserted = String::with_capacity(pending.len_utf8() + character.len_utf8());
            inserted.push(pending);
            inserted.push(character);
            handle_insert_text_key(value, on_input, editable_state, &inserted)
        }
        KeyCode::Enter if plain_key && kind == EditableControlKind::TextArea => {
            let mut text = String::with_capacity(pending.len_utf8() + 1);
            text.push(pending);
            text.push('\n');
            handle_insert_text_key(value, on_input, editable_state, &text)
        }
        _ => handle_insert_input_key(value, on_input, editable_state, pending),
    }
}

/// Returns whether a pending insert-mode key sequence has timed out.
fn insert_key_pending_expired(pending: PendingInsertKey, now: Instant) -> bool {
    now.saturating_duration_since(pending.started_at()) >= INSERT_ESCAPE_TIMEOUT
}

/// Returns pending insert-mode key state that is still within the timeout.
fn active_insert_key_pending(
    editable_state: &EditableState,
    now: Instant,
) -> Option<PendingInsertKey> {
    let pending = editable_state.insert_key_pending()?;
    (!insert_key_pending_expired(pending, now)).then_some(pending)
}

/// Returns whether the editable control has an unexpired pending insert key.
fn has_active_insert_key_pending(editable_state: &EditableState, now: Instant) -> bool {
    active_insert_key_pending(editable_state, now).is_some()
}

/// Emits an expired pending insert-mode key, if one exists.
fn flush_expired_insert_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    now: Instant,
) -> Option<AppControl> {
    let pending = editable_state.insert_key_pending()?;
    if !insert_key_pending_expired(pending, now) {
        return None;
    }

    let pending = editable_state.take_insert_key_pending()?;
    Some(handle_insert_input_key(value, on_input, editable_state, pending.key()).into())
}

/// Leaves insert mode using the same cursor placement as Esc.
fn exit_insert_mode(value: &str, editable_state: &mut EditableState) {
    editable_state.set_mode(VimMode::Normal);
    editable_state.set_cursor(normal_cursor_from_insert(value, editable_state.cursor()));
}

/// Handles normal-mode movement, command sequences, and mutations.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
/// * `key` — Key event to apply while the control is in normal mode.
/// * `kind` — Editable control variant receiving the key.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when normal-mode behavior
/// handles the key.
fn handle_normal_mode_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
) -> Option<KeyControl> {
    if key.code == KeyCode::Char('r') && key.modifiers == KeyModifiers::CONTROL {
        editable_state.set_normal_key_pending(None);
        return Some(handle_redo_input_key(value, on_input, editable_state));
    }

    if let Some(pending) = editable_state.take_normal_key_pending() {
        return Some(handle_pending_normal_mode_key(
            value,
            on_input,
            editable_state,
            key,
            kind,
            pending,
        ));
    }

    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    match key.code {
        KeyCode::Esc => Some(KeyControl::Handled),
        KeyCode::Left | KeyCode::Char('h') if plain_key => {
            let cursor = normal_previous_char_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Right | KeyCode::Char('l') if plain_key => {
            let cursor = normal_next_char_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Up | KeyCode::Char('k') if plain_key => {
            handle_normal_vertical_key(value, editable_state, kind, text_area_previous_line_cursor)
        }
        KeyCode::Down | KeyCode::Char('j') if plain_key => {
            handle_normal_vertical_key(value, editable_state, kind, text_area_next_line_cursor)
        }
        KeyCode::Home | KeyCode::Char('0') if plain_key => {
            let cursor = line_start(value, editable_state.cursor(), kind);
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::End | KeyCode::Char('$') if plain_key => {
            let cursor = normal_line_end(value, editable_state.cursor(), kind);
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('w') if plain_key => {
            let cursor = next_word_start_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('b') if plain_key => {
            let cursor = previous_word_start_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('e') if plain_key => {
            let cursor = word_end_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('g') if plain_key => {
            editable_state.set_normal_key_pending(Some('g'));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('G') if plain_key => {
            editable_state.set_cursor(normal_last_char_cursor(value));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('d') if plain_key => {
            editable_state.set_normal_key_pending(Some('d'));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('y') if plain_key => {
            editable_state.set_normal_key_pending(Some('y'));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('v') if plain_key => {
            enter_visual_mode(value, editable_state, VimMode::Visual, kind);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('V') if plain_key => {
            enter_visual_mode(value, editable_state, VimMode::VisualLine, kind);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('x') if plain_key => Some(handle_delete_normal_char_key(
            value,
            on_input,
            editable_state,
        )),
        KeyCode::Char('p') if plain_key => Some(handle_paste_input_key(
            value,
            on_input,
            editable_state,
            kind,
        )),
        KeyCode::Char('u') if plain_key => {
            Some(handle_undo_input_key(value, on_input, editable_state))
        }
        KeyCode::Char('o') if plain_key => Some(handle_open_line_key(
            value,
            on_input,
            editable_state,
            kind,
            OpenLinePosition::Below,
        )),
        KeyCode::Char('O') if plain_key => Some(handle_open_line_key(
            value,
            on_input,
            editable_state,
            kind,
            OpenLinePosition::Above,
        )),
        KeyCode::Char('i') if plain_key => {
            editable_state.set_mode(VimMode::Insert);
            editable_state.set_cursor(clamp_cursor(value, editable_state.cursor()));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('a') if plain_key => {
            editable_state.set_mode(VimMode::Insert);
            editable_state.set_cursor(insert_after_normal_cursor(value, editable_state.cursor()));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('I') if plain_key => {
            editable_state.set_mode(VimMode::Insert);
            editable_state.set_cursor(line_start(value, editable_state.cursor(), kind));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('A') if plain_key => {
            editable_state.set_mode(VimMode::Insert);
            editable_state.set_cursor(insert_line_end(value, editable_state.cursor(), kind));
            Some(KeyControl::Handled)
        }
        KeyCode::Enter | KeyCode::Backspace | KeyCode::Delete => Some(KeyControl::Handled),
        KeyCode::Char(_) if plain_key => Some(KeyControl::Handled),
        _ => None,
    }
}

/// Handles vertical normal-mode movement inside editable controls.
///
/// Returns [`None`] when the cursor cannot move within the editable control so
/// parent containers can handle boundary scrolling.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `editable_state` — Retained cursor, mode, selection, and history state.
/// * `kind` — Editable control variant receiving the key.
/// * `move_text_area_cursor` — Movement function used for text-area cursor rows.
///
/// # Returns
///
/// An [`Option<KeyControl>`] indicating whether the editable control handled
/// the movement.
fn handle_normal_vertical_key(
    value: &str,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
    move_text_area_cursor: fn(&str, usize) -> usize,
) -> Option<KeyControl> {
    let cursor = normal_cursor(value, editable_state.cursor());
    let next_cursor = match kind {
        EditableControlKind::Input => cursor,
        EditableControlKind::TextArea => normal_cursor(value, move_text_area_cursor(value, cursor)),
    };

    if next_cursor == cursor {
        return None;
    }

    editable_state.set_cursor(next_cursor);
    Some(KeyControl::Handled)
}

/// Handles visual-mode movement and selection mutations.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor, mode, selection, and history state.
/// * `key` — Key event to apply while the control is in visual mode.
/// * `kind` — Editable control variant receiving the key.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when visual-mode behavior
/// handles the key.
fn handle_visual_mode_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
) -> Option<KeyControl> {
    if let Some(pending) = editable_state.take_normal_key_pending() {
        return Some(handle_pending_visual_mode_key(
            value,
            editable_state,
            key,
            pending,
        ));
    }

    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    match key.code {
        KeyCode::Esc => {
            exit_visual_mode(value, editable_state);
            Some(KeyControl::Handled)
        }
        KeyCode::Left | KeyCode::Char('h') if plain_key => {
            let cursor = normal_previous_char_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Right | KeyCode::Char('l') if plain_key => {
            let cursor = normal_next_char_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Up | KeyCode::Char('k') if plain_key => {
            let cursor = match kind {
                EditableControlKind::Input => normal_cursor(value, editable_state.cursor()),
                EditableControlKind::TextArea => normal_cursor(
                    value,
                    text_area_previous_line_cursor(value, editable_state.cursor()),
                ),
            };
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Down | KeyCode::Char('j') if plain_key => {
            let cursor = match kind {
                EditableControlKind::Input => normal_cursor(value, editable_state.cursor()),
                EditableControlKind::TextArea => normal_cursor(
                    value,
                    text_area_next_line_cursor(value, editable_state.cursor()),
                ),
            };
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Home | KeyCode::Char('0') if plain_key => {
            let cursor = line_start(value, editable_state.cursor(), kind);
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::End | KeyCode::Char('$') if plain_key => {
            let cursor = normal_line_end(value, editable_state.cursor(), kind);
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('w') if plain_key => {
            let cursor = next_word_start_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('b') if plain_key => {
            let cursor = previous_word_start_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('e') if plain_key => {
            let cursor = word_end_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('g') if plain_key => {
            editable_state.set_normal_key_pending(Some('g'));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('G') if plain_key => {
            editable_state.set_cursor(normal_last_char_cursor(value));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('v') if plain_key => {
            if editable_state.mode() == VimMode::Visual {
                exit_visual_mode(value, editable_state);
            } else {
                editable_state.set_mode(VimMode::Visual);
                ensure_visual_anchor(value, editable_state);
            }
            Some(KeyControl::Handled)
        }
        KeyCode::Char('V') if plain_key => {
            if editable_state.mode() == VimMode::VisualLine {
                exit_visual_mode(value, editable_state);
            } else {
                editable_state.set_mode(VimMode::VisualLine);
                ensure_visual_anchor(value, editable_state);
            }
            Some(KeyControl::Handled)
        }
        KeyCode::Char('y') if plain_key => Some(handle_yank_visual_selection_key(
            value,
            editable_state,
            kind,
        )),
        KeyCode::Char('d') | KeyCode::Char('x') if plain_key => Some(
            handle_delete_visual_selection_key(value, on_input, editable_state, kind),
        ),
        KeyCode::Enter | KeyCode::Backspace | KeyCode::Delete => Some(KeyControl::Handled),
        KeyCode::Char(_) if plain_key => Some(KeyControl::Handled),
        _ => None,
    }
}

/// Handles the second key in a visual-mode command sequence.
fn handle_pending_visual_mode_key(
    value: &str,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    pending: char,
) -> KeyControl {
    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    match (pending, key.code) {
        ('g', KeyCode::Char('g')) if plain_key => {
            editable_state.set_cursor(0);
            KeyControl::Handled
        }
        _ => {
            ensure_visual_anchor(value, editable_state);
            KeyControl::Handled
        }
    }
}

/// Handles the second key in a normal-mode command sequence.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
/// * `key` — Key event completing or cancelling the sequence.
/// * `kind` — Editable control variant receiving the key.
/// * `pending` — First key already captured for the sequence.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that the pending sequence was handled.
fn handle_pending_normal_mode_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
    pending: char,
) -> KeyControl {
    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    match (pending, key.code) {
        ('g', KeyCode::Char('g')) if plain_key => {
            editable_state.set_cursor(0);
            KeyControl::Handled
        }
        ('d', KeyCode::Char('d')) if plain_key => {
            handle_delete_line_key(value, on_input, editable_state, kind)
        }
        ('y', KeyCode::Char('y')) if plain_key => handle_yank_line_key(value, editable_state, kind),
        _ => KeyControl::Handled,
    }
}

/// Enters a visual mode with the current normal cursor as the fixed anchor.
fn enter_visual_mode(
    value: &str,
    editable_state: &mut EditableState,
    mode: VimMode,
    _kind: EditableControlKind,
) {
    editable_state.set_normal_key_pending(None);
    let cursor = normal_cursor(value, editable_state.cursor());
    editable_state.set_cursor(cursor);
    editable_state.set_mode(mode);
    editable_state.set_selection_anchor(Some(cursor));
}

/// Leaves visual mode and clears selection state.
fn exit_visual_mode(value: &str, editable_state: &mut EditableState) {
    editable_state.set_normal_key_pending(None);
    editable_state.set_cursor(normal_cursor(value, editable_state.cursor()));
    editable_state.set_mode(VimMode::Normal);
}

/// Ensures a visual selection anchor exists after mode changes or stale state.
fn ensure_visual_anchor(value: &str, editable_state: &mut EditableState) {
    if editable_state.selection_anchor().is_none() {
        editable_state.set_selection_anchor(Some(normal_cursor(value, editable_state.cursor())));
    }
}

/// Returns the active visual selection range for rendering or mutation.
fn visual_selection_range(
    value: &str,
    editable_state: &EditableState,
    kind: EditableControlKind,
) -> Option<Range<usize>> {
    let anchor = editable_state.selection_anchor()?;
    let selection = match editable_state.mode() {
        VimMode::Visual => visual_charwise_range(value, anchor, editable_state.cursor()),
        VimMode::VisualLine => {
            visual_linewise_content_range(value, anchor, editable_state.cursor(), kind)
        }
        VimMode::Insert | VimMode::Normal => return None,
    };

    Some(selection)
}

/// Returns the inclusive character-wise visual selection as a byte range.
fn visual_charwise_range(value: &str, anchor: usize, cursor: usize) -> Range<usize> {
    if value.is_empty() {
        return 0..0;
    }

    let anchor = normal_cursor(value, anchor);
    let cursor = normal_cursor(value, cursor);
    if anchor <= cursor {
        anchor..next_char_boundary(value, cursor)
    } else {
        cursor..next_char_boundary(value, anchor)
    }
}

/// Returns the content bytes covered by a line-wise visual selection.
fn visual_linewise_content_range(
    value: &str,
    anchor: usize,
    cursor: usize,
    kind: EditableControlKind,
) -> Range<usize> {
    if value.is_empty() {
        return 0..0;
    }

    match kind {
        EditableControlKind::Input => 0..value.len(),
        EditableControlKind::TextArea => {
            let anchor = clamp_cursor(value, anchor);
            let cursor = clamp_cursor(value, cursor);
            let start =
                text_area_line_start(value, anchor).min(text_area_line_start(value, cursor));
            let end = text_area_line_end(value, anchor).max(text_area_line_end(value, cursor));

            start..end
        }
    }
}

/// Returns the bytes removed by a line-wise visual delete.
fn visual_linewise_delete_range(value: &str, content_range: Range<usize>) -> Range<usize> {
    if value.is_empty() {
        return 0..0;
    }

    if content_range.end < value.len() {
        content_range.start..content_range.end + 1
    } else if content_range.start > 0 {
        content_range.start - 1..content_range.end
    } else {
        content_range
    }
}

/// Handles visual-mode `y`.
fn handle_yank_visual_selection_key(
    value: &str,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    let selection = visual_selection_range(value, editable_state, kind).unwrap_or(0..0);
    if editable_state.mode() == VimMode::VisualLine && kind == EditableControlKind::TextArea {
        editable_state.set_linewise_yank_buffer(value[selection.clone()].to_owned());
    } else {
        editable_state.set_yank_buffer(value[selection.clone()].to_owned());
    }

    editable_state.set_cursor(normal_cursor_after_change(value, selection.start));
    editable_state.set_normal_key_pending(None);
    editable_state.set_mode(VimMode::Normal);
    KeyControl::Handled
}

/// Returns a copy of `value` with `range` replaced by `replacement`.
fn replace_value_range(value: &str, range: Range<usize>, replacement: &str) -> String {
    let mut next = String::with_capacity(
        value
            .len()
            .saturating_sub(range.len())
            .saturating_add(replacement.len()),
    );
    next.push_str(&value[..range.start]);
    next.push_str(replacement);
    next.push_str(&value[range.end..]);
    next
}

/// Handles visual-mode `d` and `x`.
fn handle_delete_visual_selection_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    let selection = visual_selection_range(value, editable_state, kind).unwrap_or(0..0);
    let linewise =
        editable_state.mode() == VimMode::VisualLine && kind == EditableControlKind::TextArea;
    if linewise {
        editable_state.set_linewise_yank_buffer(value[selection.clone()].to_owned());
    } else {
        editable_state.set_yank_buffer(value[selection.clone()].to_owned());
    }

    let delete_range = if linewise {
        visual_linewise_delete_range(value, selection)
    } else {
        selection
    };
    let delete_start = delete_range.start;
    let next = replace_value_range(value, delete_range, "");
    let next_cursor = normal_cursor_after_change(&next, delete_start);

    editable_state.set_normal_key_pending(None);
    editable_state.set_mode(VimMode::Normal);
    commit_input_value(value, on_input, editable_state, next, next_cursor)
}

/// Returns the logical line start for an editable control.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used to select the line.
/// * `kind` — Editable control variant that defines line behavior.
///
/// # Returns
///
/// A [`usize`] byte index for the start of the logical line.
fn line_start(value: &str, cursor: usize, kind: EditableControlKind) -> usize {
    match kind {
        EditableControlKind::Input => 0,
        EditableControlKind::TextArea => text_area_line_start(value, cursor),
    }
}

/// Returns the insert-mode line end for an editable control.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used to select the line.
/// * `kind` — Editable control variant that defines line behavior.
///
/// # Returns
///
/// A [`usize`] byte index for the insert-mode end of the logical line.
fn insert_line_end(value: &str, cursor: usize, kind: EditableControlKind) -> usize {
    match kind {
        EditableControlKind::Input => value.len(),
        EditableControlKind::TextArea => text_area_line_end(value, cursor),
    }
}

/// Returns the normal-mode cursor position for the end of the current line.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used to select the line.
/// * `kind` — Editable control variant that defines line behavior.
///
/// # Returns
///
/// A [`usize`] byte index for the last character in the logical line, or the
/// line start for an empty line.
fn normal_line_end(value: &str, cursor: usize, kind: EditableControlKind) -> usize {
    let start = line_start(value, cursor, kind);
    let end = insert_line_end(value, cursor, kind);
    if end > start {
        previous_char_boundary(value, end)
    } else {
        start
    }
}

/// Converts an insert-mode cursor to the matching normal-mode cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Insert-mode cursor byte index.
///
/// # Returns
///
/// A [`usize`] byte index for the normal-mode cursor.
fn normal_cursor_from_insert(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    if cursor == 0 || is_trailing_empty_line_cursor(value, cursor) {
        cursor
    } else {
        previous_char_boundary(value, cursor)
    }
}

/// Returns a normal-mode cursor clamped to an existing normal-mode position.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Candidate cursor byte index.
///
/// # Returns
///
/// A [`usize`] byte index for an existing character, the trailing empty line, or
/// zero for an empty value.
fn normal_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        return 0;
    }

    let cursor = clamp_cursor(value, cursor);
    if cursor == value.len() && !is_trailing_empty_line_cursor(value, cursor) {
        previous_char_boundary(value, cursor)
    } else {
        cursor
    }
}

/// Returns the final normal-mode cursor position in a value.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
///
/// # Returns
///
/// A [`usize`] byte index for the final character, the trailing empty line, or
/// zero for an empty value.
fn normal_last_char_cursor(value: &str) -> usize {
    normal_cursor(value, value.len())
}

/// Returns whether a cursor addresses the empty logical line after a final newline.
fn is_trailing_empty_line_cursor(value: &str, cursor: usize) -> bool {
    cursor == value.len() && value.ends_with('\n')
}

/// Returns the previous normal-mode character cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used as the movement origin.
///
/// # Returns
///
/// A [`usize`] byte index for the previous normal-mode cursor.
fn normal_previous_char_cursor(value: &str, cursor: usize) -> usize {
    previous_char_boundary(value, normal_cursor(value, cursor))
}

/// Returns the next normal-mode character cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used as the movement origin.
///
/// # Returns
///
/// A [`usize`] byte index for the next normal-mode cursor.
fn normal_next_char_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        return 0;
    }

    let cursor = normal_cursor(value, cursor);
    let next = next_char_boundary(value, cursor);
    if next == value.len() { cursor } else { next }
}

/// Returns the insert position after the current normal-mode cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Normal-mode cursor byte index.
///
/// # Returns
///
/// A [`usize`] byte index where inserted text should begin.
fn insert_after_normal_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        0
    } else {
        next_char_boundary(value, normal_cursor(value, cursor))
    }
}

/// Returns a cursor after replacing the controlled value.
///
/// # Arguments
///
/// * `value` — Replacement controlled editable value.
/// * `cursor` — Cursor byte index retained before replacement.
/// * `mode` — Vim mode that determines cursor clamping behavior.
///
/// # Returns
///
/// A [`usize`] byte index valid for the replacement value.
fn cursor_after_value_replace(value: &str, cursor: usize, mode: VimMode) -> usize {
    match mode {
        VimMode::Insert => clamp_cursor(value, cursor),
        VimMode::Normal | VimMode::Visual | VimMode::VisualLine => {
            normal_cursor_after_change(value, cursor)
        }
    }
}

/// Returns a normal-mode cursor after mutating text near `cursor`.
///
/// # Arguments
///
/// * `value` — Mutated controlled editable value.
/// * `cursor` — Cursor byte index near the mutation.
///
/// # Returns
///
/// A [`usize`] normal-mode cursor byte index valid for the mutated value.
fn normal_cursor_after_change(value: &str, cursor: usize) -> usize {
    normal_cursor(value, cursor)
}

/// Returns the character at a byte cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index to inspect.
///
/// # Returns
///
/// An [`Option`] containing the character starting at `cursor`.
fn char_at(value: &str, cursor: usize) -> Option<char> {
    value.get(cursor..)?.chars().next()
}

/// Returns whether a character participates in Vim word motions.
///
/// # Arguments
///
/// * `character` — Character to classify.
///
/// # Returns
///
/// A [`bool`] value indicating whether `character` belongs to a Vim word.
fn is_word_character(character: char) -> bool {
    !character.is_whitespace()
}

/// Returns the start of the next word for normal-mode `w`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used as the movement origin.
///
/// # Returns
///
/// A [`usize`] byte index for the next word start.
fn next_word_start_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        return 0;
    }

    let mut cursor = normal_cursor(value, cursor);
    if char_at(value, cursor).is_some_and(is_word_character) {
        while cursor < value.len() && char_at(value, cursor).is_some_and(is_word_character) {
            cursor = next_char_boundary(value, cursor);
        }
    }

    while cursor < value.len()
        && char_at(value, cursor).is_some_and(|character| !is_word_character(character))
    {
        cursor = next_char_boundary(value, cursor);
    }

    if cursor == value.len() {
        normal_last_char_cursor(value)
    } else {
        cursor
    }
}

/// Returns the start of the previous word for normal-mode `b`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used as the movement origin.
///
/// # Returns
///
/// A [`usize`] byte index for the previous word start.
fn previous_word_start_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        return 0;
    }

    let cursor = normal_cursor(value, cursor);
    if cursor == 0 {
        return 0;
    }

    let mut cursor = previous_char_boundary(value, cursor);
    while cursor > 0
        && char_at(value, cursor).is_some_and(|character| !is_word_character(character))
    {
        cursor = previous_char_boundary(value, cursor);
    }

    while cursor > 0 {
        let previous = previous_char_boundary(value, cursor);
        if char_at(value, previous).is_some_and(|character| !is_word_character(character)) {
            break;
        }
        cursor = previous;
    }

    cursor
}

/// Returns the end of the current or next word for normal-mode `e`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used as the movement origin.
///
/// # Returns
///
/// A [`usize`] byte index for the current or next word end.
fn word_end_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        return 0;
    }

    let mut cursor = normal_cursor(value, cursor);
    if char_at(value, cursor).is_some_and(is_word_character) {
        let next = next_char_boundary(value, cursor);
        if next < value.len() && char_at(value, next).is_some_and(is_word_character) {
            cursor = next;
            while next_char_boundary(value, cursor) < value.len()
                && char_at(value, next_char_boundary(value, cursor)).is_some_and(is_word_character)
            {
                cursor = next_char_boundary(value, cursor);
            }
            return cursor;
        }
        cursor = next;
    }

    while cursor < value.len()
        && char_at(value, cursor).is_some_and(|character| !is_word_character(character))
    {
        cursor = next_char_boundary(value, cursor);
    }

    if cursor == value.len() {
        return normal_last_char_cursor(value);
    }

    while next_char_boundary(value, cursor) < value.len()
        && char_at(value, next_char_boundary(value, cursor)).is_some_and(is_word_character)
    {
        cursor = next_char_boundary(value, cursor);
    }

    cursor
}

/// Handles normal-mode `x`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the shortened value.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that character deletion was handled.
fn handle_delete_normal_char_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
) -> KeyControl {
    if value.is_empty() {
        editable_state.set_cursor(0);
        return KeyControl::Handled;
    }

    let cursor = normal_cursor(value, editable_state.cursor());
    let next_boundary = next_char_boundary(value, cursor);
    let next = replace_value_range(value, cursor..next_boundary, "");
    let next_cursor = normal_cursor_after_change(&next, cursor);

    commit_input_value(value, on_input, editable_state, next, next_cursor)
}

/// Handles normal-mode `dd`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the shortened value.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
/// * `kind` — Editable control variant receiving the command.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that line deletion was handled.
fn handle_delete_line_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    if value.is_empty() {
        editable_state.set_yank_buffer("");
        editable_state.set_cursor(0);
        return KeyControl::Handled;
    }

    match kind {
        EditableControlKind::Input => {
            editable_state.set_yank_buffer(value);
            commit_input_value(value, on_input, editable_state, String::new(), 0)
        }
        EditableControlKind::TextArea => {
            let content_range = text_area_line_content_range(value, editable_state.cursor());
            let deleted_line = value[content_range].to_owned();
            editable_state.set_linewise_yank_buffer(deleted_line);

            let delete_range = text_area_line_delete_range(value, editable_state.cursor());
            let delete_start = delete_range.start;
            let next = replace_value_range(value, delete_range, "");
            let next_cursor = normal_cursor_after_change(&next, delete_start);

            commit_input_value(value, on_input, editable_state, next, next_cursor)
        }
    }
}

/// Handles normal-mode `yy`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `editable_state` — Retained cursor, mode, and yank-buffer state for the
///   control.
/// * `kind` — Editable control variant receiving the command.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that line yanking was handled.
fn handle_yank_line_key(
    value: &str,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    match kind {
        EditableControlKind::Input => editable_state.set_yank_buffer(value),
        EditableControlKind::TextArea => {
            let range = text_area_line_content_range(value, editable_state.cursor());
            editable_state.set_linewise_yank_buffer(value[range].to_owned());
        }
    }

    KeyControl::Handled
}

/// Normal-mode line placement for Vim open-line commands.
#[derive(Clone, Copy)]
enum OpenLinePosition {
    /// Insert a line above the current logical line.
    Above,
    /// Insert a line below the current logical line.
    Below,
}

/// Handles normal-mode `o` and `O`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the opened-line value.
/// * `editable_state` — Retained cursor, mode, and history state for the
///   control.
/// * `kind` — Editable control variant receiving the command.
/// * `position` — Whether to open the new line above or below the current line.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that the open-line command was handled.
fn handle_open_line_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
    position: OpenLinePosition,
) -> KeyControl {
    if kind == EditableControlKind::Input {
        return KeyControl::Handled;
    }

    editable_state.set_mode(VimMode::Insert);
    editable_state.set_normal_key_pending(None);

    if value.is_empty() {
        editable_state.set_cursor(0);
        return KeyControl::Handled;
    }

    let cursor = normal_cursor(value, editable_state.cursor());
    let insert_at = match position {
        OpenLinePosition::Above => text_area_line_start(value, cursor),
        OpenLinePosition::Below => text_area_line_end(value, cursor),
    };
    let next_cursor = match position {
        OpenLinePosition::Above => insert_at,
        OpenLinePosition::Below => insert_at.saturating_add(1),
    };

    let next = replace_value_range(value, insert_at..insert_at, "\n");

    commit_input_value(value, on_input, editable_state, next, next_cursor)
}

/// Handles normal-mode `p`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the pasted value.
/// * `editable_state` — Retained cursor, mode, and yank-buffer state for the
///   control.
/// * `kind` — Editable control variant receiving the command.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that paste was handled.
fn handle_paste_input_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    let yank_buffer = editable_state.yank_buffer().to_owned();
    if yank_buffer.is_empty() {
        return KeyControl::Handled;
    }

    let (next, next_cursor) =
        if kind == EditableControlKind::TextArea && editable_state.yank_linewise() {
            text_area_linewise_paste(value, editable_state.cursor(), &yank_buffer)
        } else {
            charwise_paste(value, editable_state.cursor(), &yank_buffer)
        };

    commit_input_value(value, on_input, editable_state, next, next_cursor)
}

/// Handles normal-mode `u`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the restored value.
/// * `editable_state` — Retained cursor, mode, and undo-history state for the
///   control.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that undo was handled.
fn handle_undo_input_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
) -> KeyControl {
    let Some(on_input) = on_input.as_ref() else {
        return KeyControl::Handled;
    };
    let Some(previous) = editable_state.pop_undo() else {
        return KeyControl::Handled;
    };

    editable_state.push_redo(value.to_owned());
    let next_cursor =
        cursor_after_value_replace(&previous, editable_state.cursor(), editable_state.mode());
    editable_state.set_cursor(next_cursor);
    on_input(previous).into()
}

/// Handles normal-mode `Ctrl+r`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the redone value.
/// * `editable_state` — Retained cursor, mode, and redo-history state for the
///   control.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that redo was handled.
fn handle_redo_input_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
) -> KeyControl {
    let Some(on_input) = on_input.as_ref() else {
        return KeyControl::Handled;
    };
    let Some(next) = editable_state.pop_redo() else {
        return KeyControl::Handled;
    };

    editable_state.push_undo(value.to_owned());
    let next_cursor =
        cursor_after_value_replace(&next, editable_state.cursor(), editable_state.mode());
    editable_state.set_cursor(next_cursor);
    on_input(next).into()
}

/// Returns a charwise paste result and normal-mode cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Normal-mode cursor byte index used as the paste origin.
/// * `yank_buffer` — Character-wise yank buffer to insert.
///
/// # Returns
///
/// A `(String, usize)` tuple containing the pasted value and next
/// normal-mode cursor.
fn charwise_paste(value: &str, cursor: usize, yank_buffer: &str) -> (String, usize) {
    let insert_at = insert_after_normal_cursor(value, cursor);
    let next = replace_value_range(value, insert_at..insert_at, yank_buffer);
    let next_cursor =
        normal_cursor_after_change(&next, insert_at.saturating_add(yank_buffer.len()));

    (next, next_cursor)
}

/// Returns a linewise text-area paste result and normal-mode cursor.
///
/// # Arguments
///
/// * `value` — Current controlled text-area value.
/// * `cursor` — Normal-mode cursor byte index used to select the current line.
/// * `yank_buffer` — Linewise yank buffer to insert.
///
/// # Returns
///
/// A `(String, usize)` tuple containing the pasted value and next
/// normal-mode cursor.
fn text_area_linewise_paste(value: &str, cursor: usize, yank_buffer: &str) -> (String, usize) {
    if value.is_empty() {
        return (yank_buffer.to_owned(), 0);
    }

    let current_end = text_area_line_end(value, cursor);
    if current_end < value.len() {
        let insert_at = current_end + 1;
        let mut replacement = String::with_capacity(yank_buffer.len().saturating_add(1));
        replacement.push_str(yank_buffer);
        replacement.push('\n');
        let next = replace_value_range(value, insert_at..insert_at, &replacement);
        return (next, insert_at);
    }

    let insert_at = value.len().saturating_add(1);
    let mut next = String::with_capacity(
        value
            .len()
            .saturating_add(yank_buffer.len())
            .saturating_add(1),
    );
    next.push_str(value);
    next.push('\n');
    next.push_str(yank_buffer);
    let next_cursor = normal_cursor_after_change(&next, insert_at);

    (next, next_cursor)
}

/// Returns the current text-area line content range without a trailing newline.
///
/// # Arguments
///
/// * `value` — Current controlled text-area value.
/// * `cursor` — Cursor byte index used to select the line.
///
/// # Returns
///
/// A [`Range`] covering the current line content.
fn text_area_line_content_range(value: &str, cursor: usize) -> Range<usize> {
    let start = text_area_line_start(value, cursor);
    let end = text_area_line_end(value, cursor);
    start..end
}

/// Returns the text-area range removed by a linewise delete.
///
/// # Arguments
///
/// * `value` — Current controlled text-area value.
/// * `cursor` — Cursor byte index used to select the line.
///
/// # Returns
///
/// A [`Range`] covering the bytes removed by a linewise delete.
fn text_area_line_delete_range(value: &str, cursor: usize) -> Range<usize> {
    let start = text_area_line_start(value, cursor);
    let end = text_area_line_end(value, cursor);

    if end < value.len() {
        start..end + 1
    } else if start > 0 {
        start - 1..end
    } else {
        start..end
    }
}

/// Returns the byte index at the start of the cursor's logical line.
///
/// # Arguments
///
/// * `value` — Text-area value containing logical lines.
/// * `cursor` — Cursor byte index used to select the line.
///
/// # Returns
///
/// A [`usize`] byte index for the start of the logical line.
fn text_area_line_start(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    value[..cursor].rfind('\n').map_or(0, |index| index + 1)
}

/// Returns the byte index at the end of the cursor's logical line.
///
/// # Arguments
///
/// * `value` — Text-area value containing logical lines.
/// * `cursor` — Cursor byte index used to select the line.
///
/// # Returns
///
/// A [`usize`] byte index for the end of the logical line.
fn text_area_line_end(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    value[cursor..]
        .find('\n')
        .map_or(value.len(), |index| cursor + index)
}

/// Returns the character column represented by a cursor within its logical line.
///
/// # Arguments
///
/// * `value` — Text-area value containing logical lines.
/// * `cursor` — Cursor byte index used to select the line and column.
///
/// # Returns
///
/// A [`usize`] character column within the logical line.
fn text_area_line_column(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    let start = text_area_line_start(value, cursor);
    value[start..cursor].chars().count()
}

/// Returns the cursor byte index for a character column inside a line range.
///
/// # Arguments
///
/// * `value` — Text-area value containing the target line.
/// * `line_start` — Byte index where the target line starts.
/// * `line_end` — Byte index where the target line ends.
/// * `target_column` — Character column to locate within the line.
///
/// # Returns
///
/// A [`usize`] cursor byte index for the target line and column.
fn text_area_cursor_for_line_column(
    value: &str,
    line_start: usize,
    line_end: usize,
    target_column: usize,
) -> usize {
    let mut column = 0usize;

    for (offset, _) in value[line_start..line_end].char_indices() {
        if column == target_column {
            return line_start + offset;
        }
        column = column.saturating_add(1);
    }

    line_end
}

/// Returns the cursor position on the previous logical line.
///
/// # Arguments
///
/// * `value` — Text-area value containing logical lines.
/// * `cursor` — Cursor byte index used to derive the source column.
///
/// # Returns
///
/// A [`usize`] cursor byte index on the previous line, or the original cursor
/// when no previous line exists.
fn text_area_previous_line_cursor(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    let current_start = text_area_line_start(value, cursor);
    if current_start == 0 {
        return cursor;
    }

    let target_column = text_area_line_column(value, cursor);
    let previous_end = current_start.saturating_sub(1);
    let previous_start = value[..previous_end]
        .rfind('\n')
        .map_or(0, |index| index + 1);

    text_area_cursor_for_line_column(value, previous_start, previous_end, target_column)
}

/// Returns the cursor position on the next logical line.
///
/// # Arguments
///
/// * `value` — Text-area value containing logical lines.
/// * `cursor` — Cursor byte index used to derive the source column.
///
/// # Returns
///
/// A [`usize`] cursor byte index on the next line, or the original cursor when
/// no next line exists.
fn text_area_next_line_cursor(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    let current_end = text_area_line_end(value, cursor);
    if current_end == value.len() {
        return cursor;
    }

    let target_column = text_area_line_column(value, cursor);
    let next_start = current_end + 1;
    let next_end = value[next_start..]
        .find('\n')
        .map_or(value.len(), |index| next_start + index);

    text_area_cursor_for_line_column(value, next_start, next_end, target_column)
}

/// Handles insertion for a focused editable text control.
///
/// # Arguments
///
/// * `value` — Current controlled value.
/// * `on_input` — Optional callback that receives the inserted value.
/// * `editable_state` — Retained cursor and scroll state for the control.
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
    let mut inserted = String::with_capacity(character.len_utf8());
    inserted.push(character);
    handle_insert_text_key(value, on_input, editable_state, &inserted)
}

/// Handles text insertion for a focused editable text control.
///
/// # Arguments
///
/// * `value` — Current controlled value.
/// * `on_input` — Optional callback that receives the inserted value.
/// * `editable_state` — Retained cursor and scroll state for the control.
/// * `inserted` — Text to insert at the cursor.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that insertion was handled.
fn handle_insert_text_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    inserted: &str,
) -> KeyControl {
    let cursor = clamp_cursor(value, editable_state.cursor());
    let next = replace_value_range(value, cursor..cursor, inserted);

    commit_input_value(
        value,
        on_input,
        editable_state,
        next,
        cursor.saturating_add(inserted.len()),
    )
}

/// Handles backspace for a focused editable text control.
///
/// # Arguments
///
/// * `value` — Current controlled value.
/// * `on_input` — Optional callback that receives the shortened value.
/// * `editable_state` — Retained cursor and scroll state for the control.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that backspace was handled.
fn handle_backspace_input_key(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    let cursor = clamp_cursor(value, editable_state.cursor());
    if cursor == 0 {
        if kind == EditableControlKind::TextArea
            && let Some(next) = value.strip_prefix('\n')
        {
            return commit_input_value(value, on_input, editable_state, next.to_owned(), 0);
        }
        editable_state.set_cursor(0);
        return KeyControl::Handled;
    }

    let previous = previous_char_boundary(value, cursor);
    let next = replace_value_range(value, previous..cursor, "");

    commit_input_value(value, on_input, editable_state, next, previous)
}

/// Handles delete for a focused editable text control.
///
/// # Arguments
///
/// * `value` — Current controlled value.
/// * `on_input` — Optional callback that receives the shortened value.
/// * `editable_state` — Retained cursor and scroll state for the control.
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
    let next = replace_value_range(value, cursor..next_boundary, "");

    commit_input_value(value, on_input, editable_state, next, cursor)
}

/// Emits a controlled editable value update when a callback exists.
///
/// # Arguments
///
/// * `value` — Current controlled value before the proposed update.
/// * `on_input` — Optional callback that receives the proposed value.
/// * `editable_state` — Retained cursor and scroll state for the control.
/// * `next` — Proposed next controlled value.
/// * `next_cursor` — Cursor byte index to retain after emitting the value.
///
/// # Returns
///
/// A [`KeyControl`] value produced by the callback or handled by default when
/// no callback exists.
fn commit_input_value(
    value: &str,
    on_input: &Option<InputAction>,
    editable_state: &mut EditableState,
    next: String,
    next_cursor: usize,
) -> KeyControl {
    let Some(on_input) = on_input.as_ref() else {
        return KeyControl::Handled;
    };

    if next != value {
        editable_state.push_undo(value.to_owned());
        editable_state.clear_redo();
    }
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
fn focused_control_span_for_view(view: &View, ctx: &mut RenderCtx<'_, '_>) -> Option<VerticalSpan> {
    match view {
        View::Button { metadata, .. } | View::Input { metadata, .. }
            if metadata.is_focused() && metadata.scroll_into_view_requested() =>
        {
            Some(VerticalSpan::from_height(ctx.area().height))
        }
        View::TextArea {
            value,
            metadata,
            editable_state,
            ..
        } if metadata.is_focused() && metadata.scroll_into_view_requested() => {
            let style = resolve_style(metadata, ctx);
            let area = ctx.area();
            let inner = style
                .to_block_with_default_borders(Borders::ALL)
                .inner(area);
            let top_offset = u32::from(inner.y.saturating_sub(area.y));
            let cursor_row = u32::try_from(text_area_cursor_row(
                value.as_str(),
                editable_state.cursor(),
                inner.width,
            ))
            .unwrap_or(u32::MAX);
            let top = top_offset.saturating_add(cursor_row);

            Some(VerticalSpan {
                top,
                bottom: top.saturating_add(1),
            })
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
                |ctx| focused_control_span_for_view(child, ctx),
            )
            .map(|span| span.offset_by(top_offset))
        }
        View::Row { children, metadata } => {
            focused_control_span_for_layout_view(children, metadata, LayoutDirection::Row, ctx)
        }
        View::Column { children, metadata } => {
            focused_control_span_for_layout_view(children, metadata, LayoutDirection::Column, ctx)
        }
        View::Form {
            children, metadata, ..
        } => focused_control_span_for_layout_view(children, metadata, LayoutDirection::Column, ctx),
        View::Dynamic(child) => child.with_view(|child| focused_control_span_for_view(child, ctx)),
        View::Component(component) => component
            .focused_control_span(ctx)
            .map(|(top, bottom)| VerticalSpan { top, bottom }),
        View::Text { .. }
        | View::Button { .. }
        | View::Input { .. }
        | View::TextArea { .. }
        | View::Image { .. } => None,
    }
}

/// Returns the focused control's vertical span inside a layout view.
fn focused_control_span_for_layout_view(
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
            focused_control_span_in_row_children(children, style.inherited_values(), metadata, ctx)
        }
        LayoutDirection::Column => {
            let min_heights = child_min_heights(children, style.inherited_values(), metadata, ctx);
            focused_control_span_in_column_children(
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
fn focused_control_span_in_row_children(
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
                ctx.with_area(*area, |ctx| focused_control_span_for_view(child, ctx))
            })
        },
    )
}

/// Returns the focused control's vertical span inside column children.
fn focused_control_span_in_column_children(
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
                    ctx.with_area(child_area, |ctx| focused_control_span_for_view(child, ctx))
                {
                    return Some(span.offset_by(row));
                }

                row = row.saturating_add(u32::from(*min_height));
            }

            None
        },
    )
}

/// Renders a styled layout view and its children.
///
/// # Arguments
///
/// * `children` — Child views to render into split areas.
/// * `metadata` — Selector metadata for resolving layout styles.
/// * `default_direction` — Layout direction used when style does not override it.
/// * `ctx` — Rendering context for the layout view.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::app::Error::Io`] if child rendering performs terminal I/O
/// that fails.
fn render_layout_view(
    children: &[View],
    metadata: &StyleMetadata,
    default_direction: LayoutDirection,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let style = resolve_style(metadata, ctx);
    ctx.render_widget(Block::new().style(style.to_ratatui_style()));
    render_children(
        children,
        style.direction.unwrap_or(default_direction),
        style.inherited_values(),
        metadata,
        ctx,
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
        focused_control_span_in_column_children(
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

/// Returns extra empty rows dropped by string-backed paragraph conversion.
fn trailing_text_area_empty_line_rows(value: &str) -> usize {
    usize::from(value.ends_with('\n'))
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
            let style = resolve_style(metadata, ctx);
            1 + vertical_border_rows(style.borders.unwrap_or(Borders::ALL))
                + vertical_padding_rows(style.padding)
        }
        View::TextArea {
            value,
            placeholder,
            metadata,
            ..
        } => {
            let style = resolve_style(metadata, ctx);
            let display_value = if value.is_empty() {
                placeholder.as_deref().unwrap_or("")
            } else {
                value.as_str()
            };
            let block = style.to_block_with_default_borders(Borders::ALL);
            let inner = block.inner(ctx.area());
            let content_height = line_count_height(
                text_area_paragraph(display_value, style, 0, 0, None)
                    .line_count(inner.width)
                    .saturating_add(trailing_text_area_empty_line_rows(display_value)),
            )
            .max(1);

            content_height
                + vertical_border_rows(style.borders.unwrap_or(Borders::ALL))
                + vertical_padding_rows(style.padding)
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
        View::Form {
            children, metadata, ..
        } => min_height_for_layout_view(children, metadata, LayoutDirection::Column, ctx),
        View::Image { .. } => 1,
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

/// Emits the first expired pending insert-mode key in child views.
fn flush_child_pending_input(children: &mut [View], now: Instant) -> Option<AppControl> {
    for child in children {
        if let Some(control) = child.flush_pending_input_at(now) {
            return Some(control);
        }
    }

    None
}
