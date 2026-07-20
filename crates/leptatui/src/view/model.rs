//! Trait-based render-tree view data structures.
//!
//! This module defines the object-safe [`View`] contract, the type-erased
//! [`AnyView`] tree container, conversion traits, and concrete built-in nodes.

use std::{
    any::Any,
    fmt,
    ops::{Deref, DerefMut},
    path::PathBuf,
    rc::Rc,
};

use crossterm::event::{Event, KeyEvent};
use ratatui::text::Text as RichText;

use super::{
    code_block::{SyntaxTheme, highlighted_source_lines},
    component_view::ComponentView,
    dynamic::DynamicView,
    metadata::{EditableState, StyleMetadata, ViewType},
};
use crate::{
    app::{AppControl, Result},
    component::{FocusedControl, KeyControl, RenderCtx},
    style::{LayoutDirection, TuiStyle},
};

/// Horizontal alignment applied to wrapped table-cell content.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CellAlignment {
    /// Aligns cell content to the left edge.
    #[default]
    Left,
    /// Centers cell content within the allocated column width.
    Center,
    /// Aligns cell content to the right edge.
    Right,
}

/// One-based semantic heading level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeadingLevel {
    /// First-level heading.
    H1,
    /// Second-level heading.
    H2,
    /// Third-level heading.
    H3,
    /// Fourth-level heading.
    H4,
    /// Fifth-level heading.
    H5,
    /// Sixth-level heading.
    H6,
}

impl HeadingLevel {
    /// Returns the numeric heading level.
    ///
    /// # Returns
    ///
    /// A [`u16`] from one through six.
    pub const fn number(self) -> u16 {
        match self {
            Self::H1 => 1,
            Self::H2 => 2,
            Self::H3 => 3,
            Self::H4 => 4,
            Self::H5 => 5,
            Self::H6 => 6,
        }
    }

    /// Returns the semantic selector identity for this heading level.
    ///
    /// # Returns
    ///
    /// A built-in [`ViewType`] heading identity.
    pub const fn view_type(self) -> ViewType {
        match self {
            Self::H1 => ViewType::H1,
            Self::H2 => ViewType::H2,
            Self::H3 => ViewType::H3,
            Self::H4 => ViewType::H4,
            Self::H5 => ViewType::H5,
            Self::H6 => ViewType::H6,
        }
    }
}

/// Marker style used by a semantic list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListKind {
    /// Decimal markers beginning at a configured value.
    Ordered,
    /// Hyphen markers rendered in source order.
    Unordered,
}

/// Semantic role of a table section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableSectionKind {
    /// Header rows with the built-in header style.
    Head,
    /// Body rows.
    Body,
}

/// Editing geometry used by a controlled text editor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditableKind {
    /// Single-line horizontally scrolling input.
    Input,
    /// Multiline vertically scrolling text area.
    TextArea,
}

/// Shared callback invoked when a button is activated.
pub type ButtonAction = Rc<dyn Fn() -> AppControl>;

/// Shared callback invoked when a form is submitted or canceled.
pub type FormAction = Rc<dyn Fn() -> AppControl>;

/// Shared callback invoked when an input proposes a new value.
pub type InputAction = Rc<dyn Fn(String) -> AppControl>;

/// Source data used by an image view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImageSource {
    /// Image loaded from a filesystem path.
    Path(PathBuf),
}

impl From<PathBuf> for ImageSource {
    /// Converts a path buffer into a path-backed image source.
    ///
    /// # Arguments
    ///
    /// * `value` — Path to load when rendering the image.
    ///
    /// # Returns
    ///
    /// An [`ImageSource::Path`] containing `value`.
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<&str> for ImageSource {
    /// Converts a borrowed path into a path-backed image source.
    ///
    /// # Arguments
    ///
    /// * `value` — Path to copy into the image source.
    ///
    /// # Returns
    ///
    /// An [`ImageSource::Path`] containing `value`.
    fn from(value: &str) -> Self {
        Self::Path(PathBuf::from(value))
    }
}

impl From<String> for ImageSource {
    /// Converts an owned path string into a path-backed image source.
    ///
    /// # Arguments
    ///
    /// * `value` — Path string to move into the image source.
    ///
    /// # Returns
    ///
    /// An [`ImageSource::Path`] containing `value`.
    fn from(value: String) -> Self {
        Self::Path(PathBuf::from(value))
    }
}

/// Returns a clamped progress value safe for Ratatui gauge rendering.
pub(crate) fn clamped_progress_value(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Runtime behavior implemented by every terminal view node.
///
/// Only [`render`](Self::render), [`as_any`](Self::as_any), and
/// [`as_any_mut`](Self::as_any_mut) are required for a render-only leaf. The
/// remaining methods provide defaults so custom nodes opt into styling,
/// containers, interaction, and reconciliation independently.
pub trait View: Any {
    /// Renders this node into its current area.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context containing the target area.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if rendering performs terminal I/O that fails.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()>;

    /// Returns the minimum useful height for this node.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Rendering context containing available geometry and styles.
    ///
    /// # Returns
    ///
    /// A [`u16`] row count.
    fn min_height(&self, _ctx: &mut RenderCtx<'_, '_>) -> u16 {
        1
    }

    /// Handles a terminal event using custom and built-in view behavior.
    ///
    /// # Arguments
    ///
    /// * `event` — Crossterm event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether the app loop should continue.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if custom event handling performs terminal
    /// I/O that fails.
    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        super::render::handle_view_event(self, event)
    }

    /// Handles a key using custom handlers followed by built-in view behavior.
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
    /// Returns [`crate::Error::Io`] if custom key handling performs terminal
    /// I/O that fails.
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        super::render::handle_view_key_event(self, key)
    }

    /// Returns the minimum useful height for compatibility with generated code.
    #[doc(hidden)]
    fn __min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        self.min_height(ctx)
    }

    /// Returns selector metadata when this node participates in styling.
    ///
    /// # Returns
    ///
    /// An optional shared [`StyleMetadata`] reference.
    fn style_metadata(&self) -> Option<&StyleMetadata> {
        None
    }

    /// Returns mutable selector metadata when this node participates in styling.
    ///
    /// # Returns
    ///
    /// An optional mutable [`StyleMetadata`] reference.
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        None
    }

    /// Returns direct children in render order.
    ///
    /// # Returns
    ///
    /// A slice containing direct type-erased children.
    fn children(&self) -> &[AnyView] {
        &[]
    }

    /// Returns mutable direct children in render order.
    ///
    /// # Returns
    ///
    /// A mutable slice containing direct type-erased children.
    fn children_mut(&mut self) -> &mut [AnyView] {
        &mut []
    }

    /// Returns this node as [`Any`] for concrete-type inspection.
    ///
    /// # Returns
    ///
    /// A shared [`Any`] trait object.
    fn as_any(&self) -> &dyn Any;

    /// Returns this node as mutable [`Any`] for concrete-type inspection.
    ///
    /// # Returns
    ///
    /// A mutable [`Any`] trait object.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Reconciles node-specific retained state from a compatible previous node.
    ///
    /// # Arguments
    ///
    /// * `_previous` — Previous node with the same concrete Rust type.
    fn reconcile(&mut self, _previous: &dyn View) {}

    /// Returns whether retained state may be copied from a previous node.
    ///
    /// The default requires identical concrete Rust types. Views that combine
    /// multiple semantic variants in one struct or represent deferred
    /// boundaries should additionally compare their variant or identity.
    ///
    /// # Arguments
    ///
    /// * `previous` — Previous node considered as reconciliation input.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether reconciliation may proceed.
    fn can_reconcile_from(&self, previous: &dyn View) -> bool {
        self.as_any().type_id() == previous.as_any().type_id()
    }

    /// Handles an application-defined non-key event for this node.
    ///
    /// The default subtree dispatcher visits children before invoking this
    /// hook. Override it when a custom view needs mouse, focus, paste, resize,
    /// or other non-key event behavior.
    ///
    /// # Arguments
    ///
    /// * `_event` — Event dispatched to this node.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether the application should exit.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if custom event handling performs terminal
    /// I/O that fails.
    fn on_event(&mut self, _event: &Event) -> Result<AppControl> {
        Ok(AppControl::Continue)
    }

    /// Handles an application-defined key event for this node.
    ///
    /// The default subtree dispatcher visits children before invoking this
    /// hook. Built-in focus, editing, activation, and scrolling behavior runs
    /// afterward when this hook returns [`KeyControl::Pass`].
    ///
    /// # Arguments
    ///
    /// * `_key` — Key event dispatched to this node.
    ///
    /// # Returns
    ///
    /// A [`KeyControl`] value indicating whether propagation should continue.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if custom event handling performs terminal
    /// I/O that fails.
    fn on_key_event(&mut self, _key: KeyEvent) -> Result<KeyControl> {
        Ok(KeyControl::Pass)
    }

    /// Dispatches a non-default event through this subtree.
    #[doc(hidden)]
    fn __dispatch_event(&mut self, event: &Event) -> Result<AppControl> {
        for child in self.children_mut() {
            let control = child.__dispatch_event(event)?;
            if control == AppControl::Exit {
                return Ok(control);
            }
        }
        self.on_event(event)
    }

    /// Dispatches a custom key event through this subtree.
    #[doc(hidden)]
    fn __dispatch_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        for child in self.children_mut() {
            let control = child.__dispatch_key_event(key)?;
            if control != KeyControl::Pass {
                return Ok(control);
            }
        }
        self.on_key_event(key)
    }

    /// Emits an expired pending input sequence in this subtree.
    #[doc(hidden)]
    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        self.children_mut()
            .iter_mut()
            .find_map(AnyView::__flush_pending_input)
    }

    /// Returns the number of focusable controls in this subtree.
    #[doc(hidden)]
    fn __focusable_count(&self) -> usize {
        self.children().iter().map(AnyView::__focusable_count).sum()
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.children()
            .iter()
            .find_map(|child| child.__focused_index_inner(index))
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        for child in self.children_mut() {
            child.__set_focus_by_index_inner(target, index);
        }
    }

    /// Returns the focused control span inside this node.
    #[doc(hidden)]
    fn __focused_control_span(&self, _ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        None
    }

    /// Activates the focused button in this subtree.
    #[doc(hidden)]
    fn __activate_focused_button(&self) -> Option<AppControl> {
        self.children()
            .iter()
            .find_map(AnyView::__activate_focused_button)
    }

    /// Handles a key on the focused editor in this subtree.
    #[doc(hidden)]
    fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.children_mut()
            .iter_mut()
            .find_map(|child| child.__handle_focused_input_key(key))
    }

    /// Returns the focused control kind in this subtree.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<FocusedControl> {
        self.children().iter().find_map(AnyView::__focused_control)
    }

    /// Handles form-owned keys in this subtree.
    #[doc(hidden)]
    fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.children_mut()
            .iter_mut()
            .find_map(|child| child.__handle_form_key(key))
    }

    /// Scrolls the first overflowing container in this subtree.
    #[doc(hidden)]
    fn __scroll_first_overflowing(&mut self, delta: i16) -> bool {
        self.children_mut()
            .iter_mut()
            .any(|child| child.__scroll_first_overflowing(delta))
    }

    /// Scrolls the first overflowing container to the top.
    #[doc(hidden)]
    fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        self.children_mut()
            .iter_mut()
            .any(AnyView::__scroll_first_overflowing_to_top)
    }

    /// Scrolls the first overflowing container to the bottom.
    #[doc(hidden)]
    fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        self.children_mut()
            .iter_mut()
            .any(AnyView::__scroll_first_overflowing_to_bottom)
    }

    /// Returns whether this subtree contains an overflowing container.
    #[doc(hidden)]
    fn __has_overflowing_scroll_target(&self) -> bool {
        self.children()
            .iter()
            .any(AnyView::__has_overflowing_scroll_target)
    }

    /// Stores the pending first key of the `gg` sequence in this subtree.
    #[doc(hidden)]
    fn __set_scroll_to_top_key_pending(&self, pending: bool) -> bool {
        if let Some(metadata) = self.style_metadata() {
            metadata.set_scroll_to_top_key_pending(pending);
            return true;
        }

        self.children()
            .iter()
            .any(|child| child.__set_scroll_to_top_key_pending(pending))
    }

    /// Clears and returns the pending first key of the `gg` sequence.
    #[doc(hidden)]
    fn __take_scroll_to_top_key_pending(&self) -> bool {
        if let Some(metadata) = self.style_metadata() {
            return metadata.take_scroll_to_top_key_pending();
        }

        self.children()
            .iter()
            .any(AnyView::__take_scroll_to_top_key_pending)
    }
}

/// Owning type-erased view used inside heterogeneous render trees.
pub struct AnyView {
    /// Concrete view node behind the type-erasure boundary.
    inner: Box<dyn View>,
}

impl AnyView {
    /// Erases a concrete view node.
    ///
    /// # Arguments
    ///
    /// * `view` — Concrete view to store.
    ///
    /// # Returns
    ///
    /// An [`AnyView`] owning `view`.
    pub fn new(view: impl View) -> Self {
        Self {
            inner: Box::new(view),
        }
    }

    /// Returns whether the stored node has concrete type `V`.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether `V` is stored.
    pub fn is<V: View>(&self) -> bool {
        self.inner.as_any().is::<V>()
    }

    /// Downcasts the stored node to `V`.
    ///
    /// # Returns
    ///
    /// An optional shared reference to `V`.
    pub fn downcast_ref<V: View>(&self) -> Option<&V> {
        self.inner.as_any().downcast_ref()
    }

    /// Mutably downcasts the stored node to `V`.
    ///
    /// # Returns
    ///
    /// An optional mutable reference to `V`.
    pub fn downcast_mut<V: View>(&mut self) -> Option<&mut V> {
        self.inner.as_any_mut().downcast_mut()
    }

    /// Returns the underlying node contract.
    ///
    /// # Returns
    ///
    /// A shared [`View`] trait object.
    pub fn as_view(&self) -> &dyn View {
        self.inner.as_ref()
    }

    /// Returns the mutable underlying node contract.
    ///
    /// # Returns
    ///
    /// A mutable [`View`] trait object.
    pub fn as_view_mut(&mut self) -> &mut dyn View {
        self.inner.as_mut()
    }

    /// Returns selector metadata for the stored node.
    ///
    /// # Returns
    ///
    /// An optional shared [`StyleMetadata`] reference.
    pub fn style_metadata(&self) -> Option<&StyleMetadata> {
        self.inner.style_metadata()
    }

    /// Returns mutable selector metadata for the stored node.
    ///
    /// # Returns
    ///
    /// An optional mutable [`StyleMetadata`] reference.
    pub fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        self.inner.style_metadata_mut()
    }

    /// Sets an id selector value when the stored node is styleable.
    ///
    /// # Arguments
    ///
    /// * `id` — Id selector value to store.
    ///
    /// # Returns
    ///
    /// This type-erased view after applying `id` when metadata is available.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_id(id);
        }
        self
    }

    /// Sets whitespace-separated class selectors when the stored node is styleable.
    ///
    /// # Arguments
    ///
    /// * `classes` — Whitespace-separated class selector values to store.
    ///
    /// # Returns
    ///
    /// This type-erased view after applying `classes` when metadata is available.
    pub fn with_classes(mut self, classes: impl Into<String>) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_classes(classes);
        }
        self
    }

    /// Sets an inline style override when the stored node is styleable.
    ///
    /// # Arguments
    ///
    /// * `style` — Inline style override to store.
    ///
    /// # Returns
    ///
    /// This type-erased view after applying `style` when metadata is available.
    pub fn with_inline_style(mut self, style: TuiStyle) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_inline_style(style);
        }
        self
    }

    /// Sets focus state when the stored node is styleable.
    ///
    /// # Arguments
    ///
    /// * `focused` — Whether this view should match the focus selector.
    ///
    /// # Returns
    ///
    /// This type-erased view after applying the focus state when possible.
    pub fn with_focus(mut self, focused: bool) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_focused(focused);
        }
        self
    }

    /// Returns direct children of the stored node.
    ///
    /// # Returns
    ///
    /// A slice containing direct children.
    pub fn children(&self) -> &[AnyView] {
        self.inner.children()
    }

    /// Returns mutable direct children of the stored node.
    ///
    /// # Returns
    ///
    /// A mutable slice containing direct children.
    pub fn children_mut(&mut self) -> &mut [AnyView] {
        self.inner.children_mut()
    }

    /// Reconciles compatible retained state from a previous tree.
    ///
    /// # Arguments
    ///
    /// * `previous` — Previously rendered type-erased view.
    pub fn reconcile_from(&mut self, previous: &Self) {
        reconcile_views(self.inner.as_mut(), previous.inner.as_ref());
    }

    /// Dispatches a non-default event through the stored subtree.
    #[doc(hidden)]
    pub fn __dispatch_event(&mut self, event: &Event) -> Result<AppControl> {
        self.inner.__dispatch_event(event)
    }

    /// Dispatches a custom key event through the stored subtree.
    #[doc(hidden)]
    pub fn __dispatch_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        self.inner.__dispatch_key_event(key)
    }

    /// Emits expired pending input from the stored subtree.
    #[doc(hidden)]
    pub fn __flush_pending_input(&mut self) -> Option<AppControl> {
        self.inner.__flush_pending_input()
    }

    /// Returns the number of focusable controls in the stored subtree.
    #[doc(hidden)]
    pub fn __focusable_count(&self) -> usize {
        self.inner.__focusable_count()
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    pub fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.inner.__focused_index_inner(index)
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    pub fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        self.inner.__set_focus_by_index_inner(target, index);
    }

    /// Returns the focused control span in the stored subtree.
    #[doc(hidden)]
    pub fn __focused_button_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        self.inner.__focused_control_span(ctx)
    }

    /// Activates the focused button in the stored subtree.
    #[doc(hidden)]
    pub fn __activate_focused_button(&self) -> Option<AppControl> {
        self.inner.__activate_focused_button()
    }

    /// Handles a key on the focused editor in the stored subtree.
    #[doc(hidden)]
    pub fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.inner.__handle_focused_input_key(key)
    }

    /// Returns the focused control in the stored subtree.
    #[doc(hidden)]
    pub fn __focused_control(&self) -> Option<FocusedControl> {
        self.inner.__focused_control()
    }

    /// Handles a form-owned key in the stored subtree.
    #[doc(hidden)]
    pub fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.inner.__handle_form_key(key)
    }

    /// Scrolls the first overflowing container in the stored subtree.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing(&mut self, delta: i16) -> bool {
        self.inner.__scroll_first_overflowing(delta)
    }

    /// Scrolls the first overflowing container to the top.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        self.inner.__scroll_first_overflowing_to_top()
    }

    /// Scrolls the first overflowing container to the bottom.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        self.inner.__scroll_first_overflowing_to_bottom()
    }

    /// Returns whether the stored subtree contains an overflowing container.
    #[doc(hidden)]
    pub fn __has_overflowing_scroll_target(&self) -> bool {
        self.inner.__has_overflowing_scroll_target()
    }

    /// Stores the pending first key of the `gg` sequence in the stored subtree.
    #[doc(hidden)]
    pub fn __set_scroll_to_top_key_pending(&self, pending: bool) -> bool {
        self.inner.__set_scroll_to_top_key_pending(pending)
    }

    /// Clears and returns the pending first key of the `gg` sequence.
    #[doc(hidden)]
    pub fn __take_scroll_to_top_key_pending(&self) -> bool {
        self.inner.__take_scroll_to_top_key_pending()
    }
}

/// Reconciles compatible retained state between two view nodes.
pub(crate) fn reconcile_views(next: &mut dyn View, previous: &dyn View) {
    if !next.can_reconcile_from(previous) {
        return;
    }

    if let (Some(next), Some(previous)) = (next.style_metadata_mut(), previous.style_metadata()) {
        next.reconcile_runtime_state(previous);
    }

    next.reconcile(previous);
    for (next, previous) in next.children_mut().iter_mut().zip(previous.children()) {
        next.reconcile_from(previous);
    }
}

impl fmt::Debug for AnyView {
    /// Formats a type-erased view using its concrete type name.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnyView")
            .field("type_id", &self.inner.as_any().type_id())
            .field(
                "view_type",
                &self.inner.style_metadata().map(StyleMetadata::view_type),
            )
            .finish()
    }
}

impl Deref for AnyView {
    type Target = dyn View;

    /// Borrows the stored view contract.
    fn deref(&self) -> &Self::Target {
        self.as_view()
    }
}

impl DerefMut for AnyView {
    /// Mutably borrows the stored view contract.
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_view_mut()
    }
}

impl PartialEq for AnyView {
    /// Compares built-in type-erased nodes by their concrete values.
    fn eq(&self, other: &Self) -> bool {
        macro_rules! compare_type {
            ($type:ty) => {
                if let Some(left) = self.downcast_ref::<$type>() {
                    return other
                        .downcast_ref::<$type>()
                        .is_some_and(|right| left == right);
                }
            };
        }

        compare_type!(BlockView);
        compare_type!(TextView);
        compare_type!(HeadingView);
        compare_type!(ParagraphView);
        compare_type!(CodeBlockView);
        compare_type!(ListView);
        compare_type!(ListItemView);
        compare_type!(TableView);
        compare_type!(TableSectionView);
        compare_type!(TableRowView);
        compare_type!(TableCellView);
        compare_type!(LayoutView);
        compare_type!(FormView);
        compare_type!(ButtonView);
        compare_type!(EditableTextView);
        compare_type!(ImageView);
        compare_type!(ProgressBarView);
        compare_type!(DynamicView);
        compare_type!(ComponentView);

        false
    }
}

impl<V> PartialEq<V> for AnyView
where
    V: View + PartialEq,
{
    /// Compares a type-erased node with a concrete view of the same type.
    fn eq(&self, other: &V) -> bool {
        self.downcast_ref::<V>().is_some_and(|view| view == other)
    }
}

/// Converts a concrete value into a type-erased terminal view.
pub trait IntoView {
    /// Converts this value into an [`AnyView`].
    ///
    /// # Returns
    ///
    /// An [`AnyView`] owning the converted node.
    fn into_view(self) -> AnyView;
}

impl<V> IntoView for V
where
    V: View,
{
    fn into_view(self) -> AnyView {
        AnyView::new(self)
    }
}

impl IntoView for AnyView {
    fn into_view(self) -> AnyView {
        self
    }
}

impl IntoView for String {
    fn into_view(self) -> AnyView {
        super::builders::text(self).into_view()
    }
}

impl IntoView for &str {
    fn into_view(self) -> AnyView {
        super::builders::text(self).into_view()
    }
}

/// Converts a homogeneous or tuple-shaped child collection into a view list.
pub trait IntoViews {
    /// Converts this value into type-erased children.
    ///
    /// # Returns
    ///
    /// A [`Vec`] of [`AnyView`] values in source order.
    fn into_views(self) -> Vec<AnyView>;
}

impl<V> IntoViews for Vec<V>
where
    V: IntoView,
{
    fn into_views(self) -> Vec<AnyView> {
        self.into_iter().map(IntoView::into_view).collect()
    }
}

impl<V, const N: usize> IntoViews for [V; N]
where
    V: IntoView,
{
    fn into_views(self) -> Vec<AnyView> {
        self.into_iter().map(IntoView::into_view).collect()
    }
}

impl IntoViews for () {
    fn into_views(self) -> Vec<AnyView> {
        Vec::new()
    }
}

macro_rules! impl_into_views_tuple {
    ($($name:ident),+) => {
        impl<$($name),+> IntoViews for ($($name,)+)
        where
            $($name: IntoView),+
        {
            #[allow(non_snake_case)]
            fn into_views(self) -> Vec<AnyView> {
                let ($($name,)+) = self;
                vec![$($name.into_view()),+]
            }
        }
    };
}

impl_into_views_tuple!(A);
impl_into_views_tuple!(A, B);
impl_into_views_tuple!(A, B, C);
impl_into_views_tuple!(A, B, C, D);
impl_into_views_tuple!(A, B, C, D, E);
impl_into_views_tuple!(A, B, C, D, E, F);
impl_into_views_tuple!(A, B, C, D, E, F, G);
impl_into_views_tuple!(A, B, C, D, E, F, G, H);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U
);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V
);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);

/// Fluent styling behavior shared by concrete styleable views.
pub trait StyledView: Sized {
    /// Returns this view's selector metadata.
    ///
    /// # Returns
    ///
    /// A shared [`StyleMetadata`] reference.
    fn metadata(&self) -> &StyleMetadata;

    /// Returns this view's mutable selector metadata.
    ///
    /// # Returns
    ///
    /// A mutable [`StyleMetadata`] reference.
    fn metadata_mut(&mut self) -> &mut StyleMetadata;

    /// Sets an id selector value.
    ///
    /// # Arguments
    ///
    /// * `id` — Id selector value to store.
    ///
    /// # Returns
    ///
    /// This view with the updated metadata.
    fn with_id(mut self, id: impl Into<String>) -> Self {
        self.metadata_mut().set_id(id);
        self
    }

    /// Sets whitespace-separated class selector values.
    ///
    /// # Arguments
    ///
    /// * `classes` — Whitespace-separated class selector values to store.
    ///
    /// # Returns
    ///
    /// This view with the updated metadata.
    fn with_classes(mut self, classes: impl Into<String>) -> Self {
        self.metadata_mut().set_classes(classes);
        self
    }

    /// Sets an inline style override.
    ///
    /// # Arguments
    ///
    /// * `style` — Inline style override to store.
    ///
    /// # Returns
    ///
    /// This view with the updated metadata.
    fn with_inline_style(mut self, style: TuiStyle) -> Self {
        self.metadata_mut().set_inline_style(style);
        self
    }

    /// Sets the current focus pseudo-class state.
    ///
    /// # Arguments
    ///
    /// * `focused` — Whether this view should match the focus selector.
    ///
    /// # Returns
    ///
    /// This view with the updated focus state.
    fn with_focus(mut self, focused: bool) -> Self {
        self.metadata_mut().set_focused(focused);
        self
    }
}

/// Child access shared by concrete container views.
pub trait ContainerView {
    /// Returns direct children in render order.
    ///
    /// # Returns
    ///
    /// A slice of type-erased child views.
    fn child_views(&self) -> &[AnyView];

    /// Returns mutable direct children in render order.
    ///
    /// # Returns
    ///
    /// A mutable slice of type-erased child views.
    fn child_views_mut(&mut self) -> &mut [AnyView];
}

/// Rich text access shared by semantic text nodes.
pub trait TextualView {
    /// Returns the node's rich text content.
    ///
    /// # Returns
    ///
    /// A shared reference to the retained rich text.
    fn content(&self) -> &RichText<'static>;
}

/// Fluent configuration shared by input and text-area views.
pub trait EditableView: Sized {
    /// Returns the mutable placeholder slot.
    #[doc(hidden)]
    fn __placeholder_mut(&mut self) -> &mut Option<String>;

    /// Returns the mutable controlled-value callback slot.
    #[doc(hidden)]
    fn __on_input_mut(&mut self) -> &mut Option<InputAction>;

    /// Stores placeholder text.
    ///
    /// # Arguments
    ///
    /// * `placeholder` — Text displayed when the controlled value is empty.
    ///
    /// # Returns
    ///
    /// This editable view with the placeholder configured.
    fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        *self.__placeholder_mut() = Some(placeholder.into());
        self
    }

    /// Stores a callback invoked with proposed controlled values.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback receiving each proposed next value.
    ///
    /// # Returns
    ///
    /// This editable view with the callback configured.
    fn on_input(mut self, action: impl Fn(String) -> AppControl + 'static) -> Self {
        *self.__on_input_mut() = Some(Rc::new(action));
        self
    }
}

/// Bordered container around one child.
#[derive(Debug, PartialEq)]
pub struct BlockView {
    /// Sole child rendered inside the block.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Plain rich-text content.
#[derive(Debug, PartialEq)]
pub struct TextView {
    /// Rich text rendered by this node.
    pub(crate) content: RichText<'static>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Markdown-style semantic heading.
#[derive(Debug, PartialEq)]
pub struct HeadingView {
    /// Rich heading content.
    pub(crate) content: RichText<'static>,
    /// Heading level controlling markers and selector identity.
    pub(crate) level: HeadingLevel,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Semantic paragraph content.
#[derive(Debug, PartialEq)]
pub struct ParagraphView {
    /// Rich paragraph content.
    pub(crate) content: RichText<'static>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Bordered syntax-highlighted source code.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeBlockView {
    /// Original source used when highlighting configuration changes.
    pub(crate) source: String,
    /// Caller-supplied language token.
    pub(crate) language: Option<String>,
    /// Whether one-based line numbers are displayed.
    pub(crate) line_numbers: bool,
    /// Bundled syntax theme used for recognized source.
    pub(crate) syntax_theme: SyntaxTheme,
    /// Retained highlighted logical source lines.
    pub(crate) highlighted_lines: Vec<ratatui::text::Line<'static>>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

impl CodeBlockView {
    /// Sets the language token used for syntax highlighting.
    ///
    /// # Arguments
    ///
    /// * `language` — Grammar token or alias to select and display.
    ///
    /// # Returns
    ///
    /// This code block with refreshed highlighted lines.
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self.highlighted_lines =
            highlighted_source_lines(&self.source, self.language.as_deref(), self.syntax_theme);
        self
    }

    /// Sets whether one-based line numbers are displayed.
    ///
    /// # Arguments
    ///
    /// * `line_numbers` — Whether to render a one-based line-number gutter.
    ///
    /// # Returns
    ///
    /// This code block with the requested line-number setting.
    pub fn line_numbers(mut self, line_numbers: bool) -> Self {
        self.line_numbers = line_numbers;
        self
    }

    /// Sets the bundled syntax-highlighting theme.
    ///
    /// # Arguments
    ///
    /// * `syntax_theme` — Bundled theme used to highlight recognized source.
    ///
    /// # Returns
    ///
    /// This code block with refreshed highlighted lines.
    pub fn syntax_theme(mut self, syntax_theme: SyntaxTheme) -> Self {
        self.syntax_theme = syntax_theme;
        self.highlighted_lines =
            highlighted_source_lines(&self.source, self.language.as_deref(), self.syntax_theme);
        self
    }
}

/// Ordered or unordered semantic list.
#[derive(Debug, PartialEq)]
pub struct ListView {
    /// List item children.
    pub(crate) children: Vec<AnyView>,
    /// Marker behavior.
    pub(crate) kind: ListKind,
    /// First marker value for ordered lists.
    pub(crate) start: usize,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

impl ListView {
    /// Sets the first marker value when this is an ordered list.
    ///
    /// # Arguments
    ///
    /// * `start` — Decimal value used for the first ordered marker.
    ///
    /// # Returns
    ///
    /// This list, updated when it has ordered-list semantics.
    pub fn start(mut self, start: usize) -> Self {
        if self.kind == ListKind::Ordered {
            self.start = start;
        }
        self
    }
}

/// Vertically stacked blocks belonging to one list marker.
#[derive(Debug, PartialEq)]
pub struct ListItemView {
    /// Document block children.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Semantic table containing head and body sections.
#[derive(Debug, PartialEq)]
pub struct TableView {
    /// Table section children.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Header or body section of a semantic table.
#[derive(Debug, PartialEq)]
pub struct TableSectionView {
    /// Table row children.
    pub(crate) children: Vec<AnyView>,
    /// Semantic section role.
    pub(crate) kind: TableSectionKind,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Row containing semantic table cells.
#[derive(Debug, PartialEq)]
pub struct TableRowView {
    /// Table cell children.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Inline rich-text table cell.
#[derive(Debug, PartialEq)]
pub struct TableCellView {
    /// Rich cell content.
    pub(crate) content: RichText<'static>,
    /// Horizontal content alignment.
    pub(crate) alignment: CellAlignment,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

impl TableCellView {
    /// Sets horizontal alignment for wrapped content lines.
    ///
    /// # Arguments
    ///
    /// * `alignment` — Alignment applied to every wrapped content line.
    ///
    /// # Returns
    ///
    /// This table cell with the requested alignment.
    pub fn alignment(mut self, alignment: CellAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

/// Row or column layout with shared scrolling behavior.
#[derive(Debug, PartialEq)]
pub struct LayoutView {
    /// Child views arranged by the layout.
    pub(crate) children: Vec<AnyView>,
    /// Direction used when no stylesheet overrides it.
    pub(crate) default_direction: LayoutDirection,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Column layout that owns form submit and cancel actions.
pub struct FormView {
    /// Form control children.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
    /// Optional submit callback.
    pub(crate) on_submit: Option<FormAction>,
    /// Optional cancel callback.
    pub(crate) on_cancel: Option<FormAction>,
}

impl FormView {
    /// Stores a submit callback.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback invoked when the form submits.
    ///
    /// # Returns
    ///
    /// This form with the submit callback configured.
    pub fn on_submit(mut self, action: impl Fn() -> AppControl + 'static) -> Self {
        self.on_submit = Some(Rc::new(action));
        self
    }

    /// Stores a cancel callback.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback invoked when the form cancels.
    ///
    /// # Returns
    ///
    /// This form with the cancel callback configured.
    pub fn on_cancel(mut self, action: impl Fn() -> AppControl + 'static) -> Self {
        self.on_cancel = Some(Rc::new(action));
        self
    }

    /// Returns whether a submit callback is configured.
    ///
    /// # Returns
    ///
    /// `true` when this form has a submit callback.
    pub fn has_on_submit(&self) -> bool {
        self.on_submit.is_some()
    }

    /// Returns whether a cancel callback is configured.
    ///
    /// # Returns
    ///
    /// `true` when this form has a cancel callback.
    pub fn has_on_cancel(&self) -> bool {
        self.on_cancel.is_some()
    }
}

/// Focusable bordered button.
pub struct ButtonView {
    /// Centered button label.
    pub(crate) label: String,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
    /// Optional activation callback.
    pub(crate) on_press: Option<ButtonAction>,
}

impl ButtonView {
    /// Stores an activation callback.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback invoked when this button is activated.
    ///
    /// # Returns
    ///
    /// This button with the callback configured.
    pub fn on_press(mut self, action: impl Fn() -> AppControl + 'static) -> Self {
        self.on_press = Some(Rc::new(action));
        self
    }
}

/// Controlled single-line or multiline text editor.
pub struct EditableTextView {
    /// Caller-owned value displayed by the editor.
    pub(crate) value: String,
    /// Placeholder displayed for an empty value.
    pub(crate) placeholder: Option<String>,
    /// Editing geometry.
    pub(crate) kind: EditableKind,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
    /// Optional controlled-value callback.
    pub(crate) on_input: Option<InputAction>,
    /// Retained cursor, selection, scroll, and history state.
    pub(crate) editable_state: EditableState,
}

impl EditableView for EditableTextView {
    fn __placeholder_mut(&mut self) -> &mut Option<String> {
        &mut self.placeholder
    }

    fn __on_input_mut(&mut self) -> &mut Option<InputAction> {
        &mut self.on_input
    }
}

/// Path-backed terminal image with deterministic text fallback.
#[derive(Debug, PartialEq)]
pub struct ImageView {
    /// Image source to render.
    pub(crate) source: ImageSource,
    /// Optional fallback text.
    pub(crate) alt: Option<String>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

impl ImageView {
    /// Stores fallback text for unavailable image rendering.
    ///
    /// # Arguments
    ///
    /// * `alt` — Text displayed when terminal image rendering is unavailable.
    ///
    /// # Returns
    ///
    /// This image view with fallback text configured.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }
}

/// Gauge-style progress indicator.
#[derive(Debug, PartialEq)]
pub struct ProgressBarView {
    /// Clamped completion ratio.
    pub(crate) value: f64,
    /// Optional gauge label.
    pub(crate) label: Option<String>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Returns whether optional callbacks are both absent or share allocation identity.
fn actions_equal<T: ?Sized>(left: &Option<Rc<T>>, right: &Option<Rc<T>>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
        _ => false,
    }
}

impl PartialEq for FormView {
    fn eq(&self, other: &Self) -> bool {
        self.children == other.children
            && self.metadata == other.metadata
            && actions_equal(&self.on_submit, &other.on_submit)
            && actions_equal(&self.on_cancel, &other.on_cancel)
    }
}

impl fmt::Debug for FormView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FormView")
            .field("children", &self.children)
            .field("metadata", &self.metadata)
            .field("has_on_submit", &self.on_submit.is_some())
            .field("has_on_cancel", &self.on_cancel.is_some())
            .finish()
    }
}

impl PartialEq for ButtonView {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label
            && self.metadata == other.metadata
            && actions_equal(&self.on_press, &other.on_press)
    }
}

impl fmt::Debug for ButtonView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ButtonView")
            .field("label", &self.label)
            .field("metadata", &self.metadata)
            .field("has_on_press", &self.on_press.is_some())
            .finish()
    }
}

impl PartialEq for EditableTextView {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.placeholder == other.placeholder
            && self.kind == other.kind
            && self.metadata == other.metadata
            && actions_equal(&self.on_input, &other.on_input)
            && self.editable_state == other.editable_state
    }
}

impl fmt::Debug for EditableTextView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EditableTextView")
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("kind", &self.kind)
            .field("metadata", &self.metadata)
            .field("has_on_input", &self.on_input.is_some())
            .field("editable_state", &self.editable_state)
            .finish()
    }
}

impl ProgressBarView {
    /// Stores label text rendered over the gauge.
    ///
    /// # Arguments
    ///
    /// * `label` — Text rendered over the gauge.
    ///
    /// # Returns
    ///
    /// This progress bar with a label configured.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

macro_rules! impl_styled_view {
    ($($type:ty),+ $(,)?) => {
        $(
            impl StyledView for $type {
                fn metadata(&self) -> &StyleMetadata {
                    &self.metadata
                }

                fn metadata_mut(&mut self) -> &mut StyleMetadata {
                    &mut self.metadata
                }
            }
        )+
    };
}

impl_styled_view!(
    BlockView,
    TextView,
    HeadingView,
    ParagraphView,
    CodeBlockView,
    ListView,
    ListItemView,
    TableView,
    TableSectionView,
    TableRowView,
    TableCellView,
    LayoutView,
    FormView,
    ButtonView,
    EditableTextView,
    ImageView,
    ProgressBarView,
);

macro_rules! impl_inherent_styled_view {
    ($($type:ty),+ $(,)?) => {
        $(
            impl $type {
                /// Returns this view's selector and runtime metadata.
                pub fn metadata(&self) -> &StyleMetadata {
                    StyledView::metadata(self)
                }

                /// Returns mutable selector and runtime metadata.
                pub fn metadata_mut(&mut self) -> &mut StyleMetadata {
                    StyledView::metadata_mut(self)
                }

                /// Returns selector metadata through the core view terminology.
                pub fn style_metadata(&self) -> Option<&StyleMetadata> {
                    Some(StyledView::metadata(self))
                }

                /// Returns mutable selector metadata through the core view terminology.
                pub fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
                    Some(StyledView::metadata_mut(self))
                }

                /// Sets an id selector value.
                pub fn with_id(self, id: impl Into<String>) -> Self {
                    StyledView::with_id(self, id)
                }

                /// Sets whitespace-separated class selector values.
                pub fn with_classes(self, classes: impl Into<String>) -> Self {
                    StyledView::with_classes(self, classes)
                }

                /// Sets an inline style override.
                pub fn with_inline_style(self, style: TuiStyle) -> Self {
                    StyledView::with_inline_style(self, style)
                }

                /// Sets the current focus pseudo-class state.
                pub fn with_focus(self, focused: bool) -> Self {
                    StyledView::with_focus(self, focused)
                }
            }
        )+
    };
}

impl_inherent_styled_view!(
    BlockView,
    TextView,
    HeadingView,
    ParagraphView,
    CodeBlockView,
    ListView,
    ListItemView,
    TableView,
    TableSectionView,
    TableRowView,
    TableCellView,
    LayoutView,
    FormView,
    ButtonView,
    EditableTextView,
    ImageView,
    ProgressBarView,
);

impl EditableTextView {
    /// Stores placeholder text.
    pub fn placeholder(self, placeholder: impl Into<String>) -> Self {
        EditableView::placeholder(self, placeholder)
    }

    /// Stores a callback invoked with proposed controlled values.
    pub fn on_input(self, action: impl Fn(String) -> AppControl + 'static) -> Self {
        EditableView::on_input(self, action)
    }

    /// Returns the controlled text value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the optional placeholder text.
    pub fn placeholder_text(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }

    /// Returns whether this editor is an input or text area.
    pub const fn kind(&self) -> EditableKind {
        self.kind
    }

    /// Returns retained cursor, selection, scrolling, and history state.
    pub fn editable_state(&self) -> &EditableState {
        &self.editable_state
    }

    /// Returns mutable retained cursor, selection, scrolling, and history state.
    pub fn editable_state_mut(&mut self) -> &mut EditableState {
        &mut self.editable_state
    }

    /// Returns whether a controlled-value callback is configured.
    pub fn has_on_input(&self) -> bool {
        self.on_input.is_some()
    }
}

macro_rules! impl_inherent_container_view {
    ($($type:ty),+ $(,)?) => {
        $(
            impl $type {
                /// Returns direct children in render order.
                pub fn children(&self) -> &[AnyView] {
                    ContainerView::child_views(self)
                }

                /// Returns mutable direct children in render order.
                pub fn children_mut(&mut self) -> &mut [AnyView] {
                    ContainerView::child_views_mut(self)
                }
            }
        )+
    };
}

impl_inherent_container_view!(
    BlockView,
    ListView,
    ListItemView,
    TableView,
    TableSectionView,
    TableRowView,
    LayoutView,
    FormView,
);

macro_rules! impl_inherent_textual_view {
    ($($type:ty),+ $(,)?) => {
        $(
            impl $type {
                /// Returns this view's rich text content.
                pub fn content(&self) -> &RichText<'static> {
                    TextualView::content(self)
                }
            }
        )+
    };
}

impl_inherent_textual_view!(TextView, HeadingView, ParagraphView, TableCellView);

impl HeadingView {
    /// Returns this heading's semantic level.
    pub const fn level(&self) -> HeadingLevel {
        self.level
    }
}

impl CodeBlockView {
    /// Returns the original source text.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the optional language token.
    pub fn language_token(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Returns whether line numbers are enabled.
    pub const fn has_line_numbers(&self) -> bool {
        self.line_numbers
    }

    /// Returns the selected syntax theme.
    pub const fn selected_syntax_theme(&self) -> SyntaxTheme {
        self.syntax_theme
    }

    /// Returns retained highlighted logical source lines.
    pub fn highlighted_lines(&self) -> &[ratatui::text::Line<'static>] {
        &self.highlighted_lines
    }
}

impl ListView {
    /// Returns the list marker behavior.
    pub const fn kind(&self) -> ListKind {
        self.kind
    }

    /// Returns the first ordered-list marker value.
    pub const fn start_value(&self) -> usize {
        self.start
    }
}

impl TableSectionView {
    /// Returns this section's semantic role.
    pub const fn kind(&self) -> TableSectionKind {
        self.kind
    }
}

impl TableCellView {
    /// Returns this cell's horizontal alignment.
    pub const fn cell_alignment(&self) -> CellAlignment {
        self.alignment
    }
}

impl LayoutView {
    /// Returns the direction used when no stylesheet overrides the layout.
    pub const fn default_direction(&self) -> LayoutDirection {
        self.default_direction
    }
}

impl ButtonView {
    /// Returns the button label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns whether an activation callback is configured.
    pub fn has_on_press(&self) -> bool {
        self.on_press.is_some()
    }
}

impl ImageView {
    /// Returns the configured image source.
    pub fn source(&self) -> &ImageSource {
        &self.source
    }

    /// Returns optional fallback text.
    pub fn alt_text(&self) -> Option<&str> {
        self.alt.as_deref()
    }
}

impl ProgressBarView {
    /// Returns the clamped completion ratio.
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// Returns the optional gauge label.
    pub fn label_text(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

macro_rules! impl_container_view {
    ($($type:ty),+ $(,)?) => {
        $(
            impl ContainerView for $type {
                fn child_views(&self) -> &[AnyView] {
                    &self.children
                }

                fn child_views_mut(&mut self) -> &mut [AnyView] {
                    &mut self.children
                }
            }
        )+
    };
}

impl_container_view!(
    BlockView,
    ListView,
    ListItemView,
    TableView,
    TableSectionView,
    TableRowView,
    LayoutView,
    FormView,
);

impl TextualView for TextView {
    fn content(&self) -> &RichText<'static> {
        &self.content
    }
}

impl TextualView for HeadingView {
    fn content(&self) -> &RichText<'static> {
        &self.content
    }
}

impl TextualView for ParagraphView {
    fn content(&self) -> &RichText<'static> {
        &self.content
    }
}

impl TextualView for TableCellView {
    fn content(&self) -> &RichText<'static> {
        &self.content
    }
}
