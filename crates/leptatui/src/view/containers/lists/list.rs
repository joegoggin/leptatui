//! Ordered and unordered semantic list view.

use super::render::{
    focused_control_span_for_list_view, min_height_for_list_view, render_list_view,
};
use crate::view::core::{
    capabilities::{impl_container_view, impl_styled_view},
    render::VerticalSpan,
};
use crate::view::{AnyView, IntoViews, StyleMetadata, View, ViewType};
use crate::{app::Result, component::RenderCtx};

/// Marker style used by a semantic list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListKind {
    /// Decimal markers beginning at a configured value.
    Ordered,
    /// Hyphen markers rendered in source order.
    Unordered,
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

/// Creates a semantic ordered list.
///
/// # Arguments
///
/// * `items` — Homogeneous collection or heterogeneous tuple of list items.
///
/// # Returns
///
/// A [`ListView`] numbered from one.
pub fn ordered_list(items: impl IntoViews) -> ListView {
    ListView {
        children: items.into_views(),
        kind: ListKind::Ordered,
        start: 1,
        metadata: StyleMetadata::new(ViewType::OrderedList),
    }
}

/// Creates a semantic unordered list.
///
/// # Arguments
///
/// * `items` — Homogeneous collection or heterogeneous tuple of list items.
///
/// # Returns
///
/// A hyphen-marked [`ListView`].
pub fn unordered_list(items: impl IntoViews) -> ListView {
    ListView {
        children: items.into_views(),
        kind: ListKind::Unordered,
        start: 1,
        metadata: StyleMetadata::new(ViewType::UnorderedList),
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

impl View for ListView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let start = (self.kind == ListKind::Ordered).then_some(self.start);
        render_list_view(&self.children, start, &self.metadata, ctx)
    }

    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        let start = (self.kind == ListKind::Ordered).then_some(self.start);
        min_height_for_list_view(&self.children, start, &self.metadata, ctx)
    }

    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }
    fn children(&self) -> &[AnyView] {
        &self.children
    }
    fn children_mut(&mut self) -> &mut [AnyView] {
        &mut self.children
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        let start = (self.kind == ListKind::Ordered).then_some(self.start);
        focused_control_span_for_list_view(&self.children, start, &self.metadata, ctx)
            .map(VerticalSpan::into_tuple)
    }
}

impl_styled_view!(ListView);
impl_container_view!(ListView);
