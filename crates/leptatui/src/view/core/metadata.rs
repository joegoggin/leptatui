//! Selector and runtime metadata attached to render-tree views.
//!
//! This module stores the type, id, class, inline-style, focus, and scroll
//! metadata used during style resolution and rendering.

use std::cell::{Cell, RefCell};

use ratatui::layout::Rect;

use crate::style::{Modifier, TuiStyle};

/// Rounded terminal rectangles computed for one visible layout box.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutGeometry {
    /// Rectangle including content, padding, and borders.
    pub border_box: Rect,
    /// Rectangle including content and padding but excluding borders.
    pub padding_box: Rect,
    /// Rectangle available to the view's content.
    pub content_box: Rect,
}

/// Transient layout state from the most recent root render.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum LayoutState {
    /// No layout pass has visited this view yet.
    #[default]
    Uncomputed,
    /// The view generated a visible layout box.
    Visible(LayoutGeometry),
    /// The view belongs to a `display: none` subtree.
    Hidden,
}

/// Open terminal element identity used by stylesheet type selectors.
///
/// Built-in identities are available as associated constants. External views
/// can create their own identity with [`new`](Self::new).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ViewType(&'static str);

impl ViewType {
    /// Bordered container view.
    #[allow(non_upper_case_globals)]
    pub const Block: Self = Self::new("Block");
    /// Plain text view.
    #[allow(non_upper_case_globals)]
    pub const Text: Self = Self::new("Text");
    /// First-level semantic heading view.
    #[allow(non_upper_case_globals)]
    pub const H1: Self = Self::new("H1");
    /// Second-level semantic heading view.
    #[allow(non_upper_case_globals)]
    pub const H2: Self = Self::new("H2");
    /// Third-level semantic heading view.
    #[allow(non_upper_case_globals)]
    pub const H3: Self = Self::new("H3");
    /// Fourth-level semantic heading view.
    #[allow(non_upper_case_globals)]
    pub const H4: Self = Self::new("H4");
    /// Fifth-level semantic heading view.
    #[allow(non_upper_case_globals)]
    pub const H5: Self = Self::new("H5");
    /// Sixth-level semantic heading view.
    #[allow(non_upper_case_globals)]
    pub const H6: Self = Self::new("H6");
    /// Semantic paragraph view.
    #[allow(non_upper_case_globals)]
    pub const Paragraph: Self = Self::new("Paragraph");
    /// Syntax-highlighted code-block view.
    #[allow(non_upper_case_globals)]
    pub const CodeBlock: Self = Self::new("CodeBlock");
    /// Semantic ordered-list view.
    #[allow(non_upper_case_globals)]
    pub const OrderedList: Self = Self::new("OrderedList");
    /// Semantic unordered-list view.
    #[allow(non_upper_case_globals)]
    pub const UnorderedList: Self = Self::new("UnorderedList");
    /// Semantic list-item view.
    #[allow(non_upper_case_globals)]
    pub const ListItem: Self = Self::new("ListItem");
    /// Semantic table view.
    #[allow(non_upper_case_globals)]
    pub const Table: Self = Self::new("Table");
    /// Semantic table-head view.
    #[allow(non_upper_case_globals)]
    pub const TableHead: Self = Self::new("TableHead");
    /// Semantic table-body view.
    #[allow(non_upper_case_globals)]
    pub const TableBody: Self = Self::new("TableBody");
    /// Semantic table-row view.
    #[allow(non_upper_case_globals)]
    pub const TableRow: Self = Self::new("TableRow");
    /// Semantic table-cell view.
    #[allow(non_upper_case_globals)]
    pub const TableCell: Self = Self::new("TableCell");
    /// Generic block container view.
    #[allow(non_upper_case_globals)]
    pub const Div: Self = Self::new("Div");
    /// Grouping container for form controls.
    #[allow(non_upper_case_globals)]
    pub const Form: Self = Self::new("Form");
    /// Basic button view.
    #[allow(non_upper_case_globals)]
    pub const Button: Self = Self::new("Button");
    /// Single-line editable text control.
    #[allow(non_upper_case_globals)]
    pub const Input: Self = Self::new("Input");
    /// Multiline editable text control.
    #[allow(non_upper_case_globals)]
    pub const TextArea: Self = Self::new("TextArea");
    /// Path-backed terminal image with text fallback.
    #[allow(non_upper_case_globals)]
    pub const Image: Self = Self::new("Image");
    /// Progress indicator rendered as a gauge.
    #[allow(non_upper_case_globals)]
    pub const ProgressBar: Self = Self::new("ProgressBar");
    /// Focusable standalone or embedded link.
    #[allow(non_upper_case_globals)]
    pub const Link: Self = Self::new("Link");

    /// Creates a semantic view identity.
    ///
    /// # Arguments
    ///
    /// * `name` — Static PascalCase name used by stylesheet selectors.
    ///
    /// # Returns
    ///
    /// A [`ViewType`] containing `name`.
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// Returns the semantic selector name.
    ///
    /// # Returns
    ///
    /// A static string slice containing the selector name.
    pub const fn name(self) -> &'static str {
        self.0
    }

    /// Returns the low-precedence style defaults for this view type.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing semantic defaults applied before authored
    /// stylesheet and inline declarations.
    pub(crate) fn default_style(self) -> TuiStyle {
        match self {
            Self::H1 => TuiStyle::new().modifier(Modifier::BOLD),
            Self::H2 | Self::TableHead => TuiStyle::new().modifier(Modifier::BOLD),
            Self::H3 => TuiStyle::new().modifier(Modifier::BOLD | Modifier::ITALIC),
            Self::H4 => TuiStyle::new().modifier(Modifier::ITALIC),
            Self::H5 => TuiStyle::new().modifier(Modifier::DIM | Modifier::ITALIC),
            Self::H6 => TuiStyle::new().modifier(Modifier::DIM),
            Self::Paragraph
            | Self::CodeBlock
            | Self::OrderedList
            | Self::UnorderedList
            | Self::ListItem
            | Self::Table
            | Self::TableBody
            | Self::TableRow
            | Self::TableCell
            | Self::Block
            | Self::Text
            | Self::Div
            | Self::Form
            | Self::Button
            | Self::Input
            | Self::TextArea
            | Self::Image
            | Self::ProgressBar => TuiStyle::new(),
            Self::Link => TuiStyle::new().modifier(Modifier::UNDERLINED),
            _ => TuiStyle::new(),
        }
    }

    /// Returns low-precedence defaults for the current pseudo-class state.
    ///
    /// # Arguments
    ///
    /// * `focused` — Whether the view currently matches the focus pseudo-class.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing defaults contributed by the current state.
    pub(crate) fn default_state_style(self, focused: bool) -> TuiStyle {
        if self == Self::Link && focused {
            TuiStyle::new().modifier(Modifier::UNDERLINED | Modifier::REVERSED)
        } else {
            TuiStyle::new()
        }
    }
}

/// Selector metadata stored with styleable render-tree views.
#[derive(Clone, Debug)]
pub struct StyleMetadata {
    view_type: ViewType,
    id: Option<String>,
    classes: Vec<String>,
    inline_style: Option<TuiStyle>,
    focused: bool,
    scroll_into_view_requested: Cell<bool>,
    /// Pending request to align this view with an overflowing parent's top.
    scroll_to_anchor_requested: Cell<bool>,
    scroll_to_top_key_pending: Cell<bool>,
    scroll_offset: Cell<u16>,
    max_scroll_offset: Cell<u16>,
    /// Terminal-coordinate hit areas recorded during the latest render.
    hit_areas: RefCell<Vec<Rect>>,
    /// Rounded geometry from the latest root layout pass.
    layout_state: Cell<LayoutState>,
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
            scroll_to_anchor_requested: Cell::new(false),
            scroll_to_top_key_pending: Cell::new(false),
            scroll_offset: Cell::new(0),
            max_scroll_offset: Cell::new(0),
            hit_areas: RefCell::new(Vec::new()),
            layout_state: Cell::new(LayoutState::Uncomputed),
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

    /// Returns rounded geometry from the most recent root layout pass.
    ///
    /// Hidden views and views that have not participated in a layout pass
    /// return `None`.
    ///
    /// # Returns
    ///
    /// An optional [`LayoutGeometry`] containing border, padding, and content
    /// rectangles in terminal coordinates.
    pub fn layout_geometry(&self) -> Option<LayoutGeometry> {
        match self.layout_state.get() {
            LayoutState::Visible(geometry) => Some(geometry),
            LayoutState::Uncomputed | LayoutState::Hidden => None,
        }
    }

    /// Returns whether the latest layout pass excluded this view.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether the view is in a `display: none` subtree.
    pub(crate) fn is_layout_hidden(&self) -> bool {
        self.layout_state.get() == LayoutState::Hidden
    }

    /// Clears geometry before rebuilding a root layout snapshot.
    pub(crate) fn clear_layout_geometry(&self) {
        self.layout_state.set(LayoutState::Uncomputed);
    }

    /// Stores visible rounded geometry for this view.
    ///
    /// # Arguments
    ///
    /// * `geometry` — Rounded border, padding, and content rectangles to retain.
    pub(crate) fn set_layout_geometry(&self, geometry: LayoutGeometry) {
        self.layout_state.set(LayoutState::Visible(geometry));
    }

    /// Marks this view as excluded from layout and interaction.
    pub(crate) fn set_layout_hidden(&self) {
        self.layout_state.set(LayoutState::Hidden);
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

    /// Copies transient interaction state from compatible previous metadata.
    ///
    /// Authored selector values and inline styles remain owned by the newly
    /// built view while focus and scrolling state survive reactive rebuilds.
    ///
    /// # Arguments
    ///
    /// * `previous` — Metadata from the previous compatible view node.
    pub(crate) fn reconcile_runtime_state(&mut self, previous: &Self) {
        self.focused = previous.focused;
        self.scroll_into_view_requested
            .set(previous.scroll_into_view_requested.get());
        self.scroll_to_anchor_requested
            .set(previous.scroll_to_anchor_requested.get());
        self.scroll_to_top_key_pending
            .set(previous.scroll_to_top_key_pending.get());
        self.scroll_offset.set(previous.scroll_offset.get());
        self.max_scroll_offset.set(previous.max_scroll_offset.get());
    }

    /// Requests that this view be scrolled into visible overflow bounds.
    pub(crate) fn request_scroll_into_view(&self) {
        self.scroll_into_view_requested.set(true);
    }

    /// Clears a pending focus visibility scroll request.
    pub(crate) fn clear_scroll_into_view_request(&self) {
        self.scroll_into_view_requested.set(false);
    }

    /// Requests that this view be aligned to the top of an overflowing parent.
    pub(crate) fn request_scroll_to_anchor(&self) {
        self.scroll_to_anchor_requested.set(true);
    }

    /// Returns whether top-aligned anchor scrolling is pending.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether an overflowing parent should align this
    /// view with its top edge.
    pub(crate) fn scroll_to_anchor_requested(&self) -> bool {
        self.scroll_to_anchor_requested.get()
    }

    /// Clears a pending top-aligned anchor request.
    pub(crate) fn clear_scroll_to_anchor_request(&self) {
        self.scroll_to_anchor_requested.set(false);
    }

    /// Returns whether any last-rendered hit area contains a position.
    ///
    /// # Arguments
    ///
    /// * `column` — Zero-based terminal column to test.
    /// * `row` — Zero-based terminal row to test.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether any retained hit area contains the cell.
    pub(crate) fn contains_hit_position(&self, column: u16, row: u16) -> bool {
        self.hit_areas
            .borrow()
            .iter()
            .any(|area| rect_contains(*area, column, row))
    }

    /// Clears all last-rendered hit areas.
    pub(crate) fn clear_hit_areas(&self) {
        self.hit_areas.borrow_mut().clear();
    }

    /// Replaces last-rendered hit areas with one optional area.
    ///
    /// Empty rectangles are discarded after all prior areas are cleared.
    ///
    /// # Arguments
    ///
    /// * `area` — Optional terminal-coordinate rectangle to retain.
    pub(crate) fn set_hit_area(&self, area: Option<Rect>) {
        let mut hit_areas = self.hit_areas.borrow_mut();
        hit_areas.clear();
        if let Some(area) = area
            && area.width > 0
            && area.height > 0
        {
            hit_areas.push(area);
        }
    }

    /// Appends one last-rendered hit area.
    ///
    /// # Arguments
    ///
    /// * `area` — Non-empty terminal-coordinate rectangle to append.
    pub(crate) fn push_hit_area(&self, area: Rect) {
        if area.width > 0 && area.height > 0 {
            self.hit_areas.borrow_mut().push(area);
        }
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

impl PartialEq for StyleMetadata {
    /// Compares retained state while ignoring transient render hit areas.
    fn eq(&self, other: &Self) -> bool {
        self.view_type == other.view_type
            && self.id == other.id
            && self.classes == other.classes
            && self.inline_style == other.inline_style
            && self.focused == other.focused
            && self.scroll_into_view_requested.get() == other.scroll_into_view_requested.get()
            && self.scroll_to_anchor_requested.get() == other.scroll_to_anchor_requested.get()
            && self.scroll_to_top_key_pending.get() == other.scroll_to_top_key_pending.get()
            && self.scroll_offset.get() == other.scroll_offset.get()
            && self.max_scroll_offset.get() == other.max_scroll_offset.get()
    }
}

impl Eq for StyleMetadata {}

/// Returns whether a terminal rectangle contains a cell position.
///
/// Rectangle right and bottom edges are treated as exclusive.
///
/// # Arguments
///
/// * `area` — Terminal-coordinate rectangle to inspect.
/// * `column` — Zero-based terminal column to test.
/// * `row` — Zero-based terminal row to test.
///
/// # Returns
///
/// A [`bool`] indicating whether the cell falls within the half-open bounds.
pub(crate) fn rect_contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}
