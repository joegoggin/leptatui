//! Selector metadata attached to render-tree views.
//!
//! This module stores the type, id, class, inline-style, focus, and scroll
//! metadata used during style resolution and rendering.

use std::cell::Cell;

use crate::style::TuiStyle;

/// Static terminal element type used by style selectors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewType {
    /// Bordered container view.
    Block,
    /// Plain text view.
    Text,
    /// Horizontal layout view.
    Row,
    /// Vertical layout view.
    Column,
    /// Basic button view.
    Button,
}

/// Selector metadata stored with styleable render-tree views.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleMetadata {
    view_type: ViewType,
    id: Option<String>,
    classes: Vec<String>,
    inline_style: Option<TuiStyle>,
    focused: bool,
    scroll_into_view_requested: Cell<bool>,
    scroll_to_top_key_pending: Cell<bool>,
    scroll_offset: Cell<u16>,
    max_scroll_offset: Cell<u16>,
}

impl StyleMetadata {
    /// Creates empty selector metadata for a view type.
    ///
    /// # Arguments
    ///
    /// * `view_type` — Static view type represented by the metadata.
    ///
    /// # Returns
    ///
    /// A [`StyleMetadata`] value with no id, classes, inline style, or focus.
    pub fn new(view_type: ViewType) -> Self {
        Self {
            view_type,
            id: None,
            classes: Vec::new(),
            inline_style: None,
            focused: false,
            scroll_into_view_requested: Cell::new(false),
            scroll_to_top_key_pending: Cell::new(false),
            scroll_offset: Cell::new(0),
            max_scroll_offset: Cell::new(0),
        }
    }

    /// Returns the style selector view type.
    ///
    /// # Returns
    ///
    /// A [`ViewType`] value used by type selectors.
    pub const fn view_type(&self) -> ViewType {
        self.view_type
    }

    /// Returns the optional id selector value.
    ///
    /// # Returns
    ///
    /// An [`Option<&str>`] containing the view id.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Returns class selector values in deterministic source order.
    ///
    /// # Returns
    ///
    /// A string slice containing class selector values.
    pub fn classes(&self) -> &[String] {
        &self.classes
    }

    /// Returns the inline style override, if present.
    ///
    /// # Returns
    ///
    /// An [`Option<TuiStyle>`] containing the inline style override.
    pub const fn inline_style(&self) -> Option<TuiStyle> {
        self.inline_style
    }

    /// Returns whether this view currently matches `:focus`.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether this view is focused.
    pub const fn is_focused(&self) -> bool {
        self.focused
    }

    /// Returns whether this view requested focus visibility scrolling.
    pub(crate) fn scroll_into_view_requested(&self) -> bool {
        self.scroll_into_view_requested.get()
    }

    /// Returns the current vertical scroll offset.
    ///
    /// The offset is maintained by render traversal for overflowing vertical
    /// layouts and consumed by default scroll key handling.
    pub fn scroll_offset(&self) -> u16 {
        self.scroll_offset.get()
    }

    /// Returns the maximum currently valid vertical scroll offset.
    pub fn max_scroll_offset(&self) -> u16 {
        self.max_scroll_offset.get()
    }

    /// Replaces the id selector value.
    ///
    /// # Arguments
    ///
    /// * `id` — Id selector value to store.
    pub fn set_id(&mut self, id: impl Into<String>) {
        self.id = Some(id.into());
    }

    /// Replaces class selector values by splitting an HTML-like class string.
    ///
    /// # Arguments
    ///
    /// * `classes` — Whitespace-separated class selector values.
    pub fn set_classes(&mut self, classes: impl Into<String>) {
        self.classes = classes
            .into()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
    }

    /// Replaces the inline style override.
    ///
    /// # Arguments
    ///
    /// * `style` — Inline style override to store.
    pub fn set_inline_style(&mut self, style: TuiStyle) {
        self.inline_style = Some(style);
    }

    /// Replaces the current focus pseudo-class state.
    ///
    /// # Arguments
    ///
    /// * `focused` — Whether this view should match `:focus`.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Requests that this view be scrolled into visible overflow bounds.
    pub(crate) fn request_scroll_into_view(&self) {
        self.scroll_into_view_requested.set(true);
    }

    /// Clears a pending focus visibility scroll request.
    pub(crate) fn clear_scroll_into_view_request(&self) {
        self.scroll_into_view_requested.set(false);
    }

    /// Stores whether a `g` key is waiting for a second `g`.
    pub(crate) fn set_scroll_to_top_key_pending(&self, pending: bool) {
        self.scroll_to_top_key_pending.set(pending);
    }

    /// Clears and returns whether a `g` key was waiting for a second `g`.
    pub(crate) fn take_scroll_to_top_key_pending(&self) -> bool {
        self.scroll_to_top_key_pending.replace(false)
    }

    /// Updates the maximum scroll offset and clamps the current offset.
    pub(crate) fn set_max_scroll_offset(&self, max_scroll_offset: u16) {
        self.max_scroll_offset.set(max_scroll_offset);
        self.clamp_scroll_offset();
    }

    /// Adjusts the current scroll offset within the known scroll range.
    pub(crate) fn scroll_by(&self, delta: i16) -> bool {
        let current = i32::from(self.scroll_offset.get());
        let max = i32::from(self.max_scroll_offset.get());
        let next = (current + i32::from(delta)).clamp(0, max) as u16;

        if next == self.scroll_offset.get() {
            return false;
        }

        self.scroll_offset.set(next);
        true
    }

    /// Replaces the current scroll offset within the known scroll range.
    pub(crate) fn set_scroll_offset(&self, scroll_offset: u16) {
        self.scroll_offset.set(scroll_offset);
        self.clamp_scroll_offset();
    }

    fn clamp_scroll_offset(&self) {
        let scroll_offset = self.scroll_offset.get();
        let max_scroll_offset = self.max_scroll_offset.get();

        if scroll_offset > max_scroll_offset {
            self.scroll_offset.set(max_scroll_offset);
        }
    }
}
