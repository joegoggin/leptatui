//! Type erasure for heterogeneous terminal view trees.
//!
//! # Modules
//!
//! - [`interaction`] — Event, focus, scrolling, activation, and anchor forwarding.
//! - [`render`] — Computed-layout rendering, measurement, and clipped images.

mod interaction;
mod render;

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use crate::{
    MarkdownView,
    style::TuiStyle,
    view::{
        BlockView, ButtonView, CodeBlockView, ComponentView, DivView, DynamicView, FormView,
        HeadingView, ImageView, InputView, LinkView, ListItemView, ListView, ParagraphView,
        ProgressBarView, TableCellView, TableRowView, TableSectionView, TableView, TextAreaView,
        TextView, reconciliation::reconcile_views,
    },
};

use super::{contract::View, metadata::StyleMetadata};

/// Owning type-erased view used inside heterogeneous render trees.
pub struct AnyView {
    /// Concrete view node behind the type-erasure boundary.
    inner: Box<dyn View>,
}

impl AnyView {
    /// Returns whether the latest layout pass excluded this subtree.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether this view is in a `display: none` subtree.
    fn is_layout_hidden(&self) -> bool {
        self.inner
            .style_metadata()
            .is_some_and(StyleMetadata::is_layout_hidden)
    }

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
        compare_type!(DivView);
        compare_type!(FormView);
        compare_type!(ButtonView);
        compare_type!(LinkView);
        compare_type!(InputView);
        compare_type!(TextAreaView);
        compare_type!(ImageView);
        compare_type!(ProgressBarView);
        compare_type!(MarkdownView);
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
