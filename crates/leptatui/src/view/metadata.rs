//! Selector metadata attached to render-tree views.
//!
//! This module stores the type, id, class, inline-style, focus, and scroll
//! metadata used during style resolution and rendering.

use std::{cell::Cell, time::Instant};

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
    /// Grouping container for form controls.
    Form,
    /// Basic button view.
    Button,
    /// Single-line editable text control.
    Input,
    /// Multiline editable text control.
    TextArea,
    /// Path-backed terminal image with text fallback.
    Image,
}

/// Vim editing mode retained for editable text controls.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VimMode {
    /// Text entry mode.
    Insert,
    /// Command mode.
    #[default]
    Normal,
    /// Character-wise visual selection mode.
    Visual,
    /// Line-wise visual selection mode.
    VisualLine,
}

/// Pending insert-mode key sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PendingInsertKey {
    /// First key in the sequence.
    key: char,
    /// Time when the first key was received.
    started_at: Instant,
}

impl PendingInsertKey {
    /// Creates pending insert-mode key state.
    pub(crate) fn new(key: char, started_at: Instant) -> Self {
        Self { key, started_at }
    }

    /// Returns the pending first key.
    pub(crate) fn key(self) -> char {
        self.key
    }

    /// Returns when the pending key was received.
    pub(crate) fn started_at(self) -> Instant {
        self.started_at
    }
}

/// Runtime state shared by editable text controls.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EditableState {
    /// Text cursor offset retained across reconciled redraws.
    cursor: usize,
    /// Horizontal viewport offset retained across reconciled redraws.
    horizontal_scroll: u16,
    /// Vertical viewport offset retained across reconciled redraws.
    vertical_scroll: u16,
    /// Vim mode retained across reconciled redraws.
    mode: VimMode,
    /// Yank buffer retained across reconciled redraws.
    yank_buffer: String,
    /// Whether the yank buffer should paste as whole logical lines.
    yank_linewise: bool,
    /// Undo snapshots retained across reconciled redraws.
    undo_stack: Vec<String>,
    /// Redo snapshots retained across reconciled redraws.
    redo_stack: Vec<String>,
    /// First key in a pending insert-mode key sequence.
    insert_key_pending: Option<PendingInsertKey>,
    /// First key in a pending normal-mode multi-key command.
    normal_key_pending: Option<char>,
    /// Fixed selection endpoint used by visual modes.
    selection_anchor: Option<usize>,
}

impl EditableState {
    /// Creates empty editable-control state.
    ///
    /// # Returns
    ///
    /// An [`EditableState`] value with zeroed cursor and scroll offsets, normal
    /// mode, and empty history buffers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the retained text cursor offset.
    ///
    /// # Returns
    ///
    /// A [`usize`] cursor offset.
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the retained horizontal viewport offset.
    ///
    /// # Returns
    ///
    /// A [`u16`] column offset.
    pub const fn horizontal_scroll(&self) -> u16 {
        self.horizontal_scroll
    }

    /// Returns the retained vertical viewport offset.
    ///
    /// # Returns
    ///
    /// A [`u16`] row offset.
    pub const fn vertical_scroll(&self) -> u16 {
        self.vertical_scroll
    }

    /// Returns the retained Vim mode.
    ///
    /// # Returns
    ///
    /// A [`VimMode`] value.
    pub const fn mode(&self) -> VimMode {
        self.mode
    }

    /// Returns the retained visual-mode selection anchor.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the fixed selection endpoint byte offset.
    pub const fn selection_anchor(&self) -> Option<usize> {
        self.selection_anchor
    }

    /// Returns the retained selection-free yank buffer.
    ///
    /// # Returns
    ///
    /// A string slice containing the yank buffer.
    pub fn yank_buffer(&self) -> &str {
        &self.yank_buffer
    }

    /// Returns retained undo history.
    ///
    /// # Returns
    ///
    /// A slice containing retained undo snapshots.
    pub fn undo_stack(&self) -> &[String] {
        &self.undo_stack
    }

    /// Returns retained redo history.
    ///
    /// # Returns
    ///
    /// A slice containing retained redo snapshots.
    pub fn redo_stack(&self) -> &[String] {
        &self.redo_stack
    }

    /// Replaces the retained text cursor offset.
    ///
    /// # Arguments
    ///
    /// * `cursor` — Text cursor offset to retain.
    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
    }

    /// Replaces the retained horizontal viewport offset.
    ///
    /// # Arguments
    ///
    /// * `horizontal_scroll` — Horizontal viewport offset to retain.
    pub fn set_horizontal_scroll(&mut self, horizontal_scroll: u16) {
        self.horizontal_scroll = horizontal_scroll;
    }

    /// Replaces the retained vertical viewport offset.
    ///
    /// # Arguments
    ///
    /// * `vertical_scroll` — Vertical viewport offset to retain.
    pub fn set_vertical_scroll(&mut self, vertical_scroll: u16) {
        self.vertical_scroll = vertical_scroll;
    }

    /// Replaces the retained Vim mode.
    ///
    /// # Arguments
    ///
    /// * `mode` — Vim mode to retain.
    pub fn set_mode(&mut self, mode: VimMode) {
        self.mode = mode;
        self.insert_key_pending = None;
        if !matches!(mode, VimMode::Visual | VimMode::VisualLine) {
            self.selection_anchor = None;
        }
    }

    /// Replaces the retained visual-mode selection anchor.
    ///
    /// # Arguments
    ///
    /// * `selection_anchor` — Fixed selection endpoint byte offset to retain.
    pub fn set_selection_anchor(&mut self, selection_anchor: Option<usize>) {
        self.selection_anchor = selection_anchor;
    }

    /// Replaces the retained selection-free yank buffer.
    ///
    /// Marks the buffer as character-wise so later paste operations insert it
    /// after the current normal-mode cursor.
    ///
    /// # Arguments
    ///
    /// * `yank_buffer` — Yank buffer contents to retain.
    pub fn set_yank_buffer(&mut self, yank_buffer: impl Into<String>) {
        self.yank_buffer = yank_buffer.into();
        self.yank_linewise = false;
    }

    /// Pushes a retained undo-history snapshot.
    ///
    /// # Arguments
    ///
    /// * `value` — Undo snapshot to append.
    pub fn push_undo(&mut self, value: impl Into<String>) {
        self.undo_stack.push(value.into());
    }

    /// Pushes a retained redo-history snapshot.
    ///
    /// # Arguments
    ///
    /// * `value` — Redo snapshot to append.
    pub fn push_redo(&mut self, value: impl Into<String>) {
        self.redo_stack.push(value.into());
    }

    /// Replaces the retained linewise yank buffer.
    ///
    /// Marks the buffer as linewise so later text-area paste operations insert
    /// it as a whole logical line.
    ///
    /// # Arguments
    ///
    /// * `yank_buffer` — Yank buffer contents to retain.
    pub(crate) fn set_linewise_yank_buffer(&mut self, yank_buffer: impl Into<String>) {
        self.yank_buffer = yank_buffer.into();
        self.yank_linewise = true;
    }

    /// Returns whether the yank buffer should paste as whole logical lines.
    ///
    /// # Returns
    ///
    /// A [`bool`] value indicating whether the yank buffer is linewise.
    pub(crate) const fn yank_linewise(&self) -> bool {
        self.yank_linewise
    }

    /// Clears retained redo history.
    pub(crate) fn clear_redo(&mut self) {
        self.redo_stack.clear();
    }

    /// Pops the most recent retained undo snapshot.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the most recent undo snapshot.
    pub(crate) fn pop_undo(&mut self) -> Option<String> {
        self.undo_stack.pop()
    }

    /// Pops the most recent retained redo snapshot.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the most recent redo snapshot.
    pub(crate) fn pop_redo(&mut self) -> Option<String> {
        self.redo_stack.pop()
    }

    /// Replaces the pending insert-mode sequence key.
    ///
    /// # Arguments
    ///
    /// * `key` — Pending first key in an insert-mode key sequence.
    /// * `started_at` — Time when the key was received.
    pub(crate) fn set_insert_key_pending(&mut self, key: char, started_at: Instant) {
        self.insert_key_pending = Some(PendingInsertKey::new(key, started_at));
    }

    /// Returns the pending insert-mode sequence key.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the pending first key.
    pub(crate) fn insert_key_pending(&self) -> Option<PendingInsertKey> {
        self.insert_key_pending
    }

    /// Clears and returns the pending insert-mode sequence key.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the pending first key.
    pub(crate) fn take_insert_key_pending(&mut self) -> Option<PendingInsertKey> {
        self.insert_key_pending.take()
    }

    /// Replaces the pending normal-mode command key.
    ///
    /// # Arguments
    ///
    /// * `key` — Pending first key in a normal-mode command sequence.
    pub(crate) fn set_normal_key_pending(&mut self, key: Option<char>) {
        self.normal_key_pending = key;
    }

    /// Clears and returns the pending normal-mode command key.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the pending first key.
    pub(crate) fn take_normal_key_pending(&mut self) -> Option<char> {
        self.normal_key_pending.take()
    }
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

    /// Clamps the current scroll offset to the known maximum.
    fn clamp_scroll_offset(&self) {
        let scroll_offset = self.scroll_offset.get();
        let max_scroll_offset = self.max_scroll_offset.get();

        if scroll_offset > max_scroll_offset {
            self.scroll_offset.set(max_scroll_offset);
        }
    }
}
