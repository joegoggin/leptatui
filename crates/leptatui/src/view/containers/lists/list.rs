//! Ordered and unordered semantic list view.

use super::render::{
    focused_control_span_for_list_view, intrinsic_height_for_list_view, render_list_view,
};
use crate::view::core::{
    capabilities::{impl_container_view, impl_styled_view},
    measurement::{AvailableSpace, cells_to_u16, resolve_intrinsic_axis, sanitize_cells},
    render::VerticalSpan,
};
use crate::view::{AnyView, IntoViews, StyleMetadata, View, ViewType};
use crate::{LayoutSize, app::Result, component::RenderCtx};
use ratatui::layout::Rect;

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

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let start = (self.kind == ListKind::Ordered).then_some(self.start);
        let marker_width = list_marker_width(self.children.len(), start);
        let min_width = list_intrinsic_width(
            &self.children,
            marker_width,
            AvailableSpace::MinContent,
            ctx,
        );
        let max_width = list_intrinsic_width(
            &self.children,
            marker_width,
            AvailableSpace::MaxContent,
            ctx,
        );
        let width = resolve_intrinsic_axis(
            known_dimensions.width,
            available_space.width,
            min_width,
            max_width,
        )
        .max(0.0);
        let layout_width = known_dimensions
            .width
            .or_else(|| available_space.width.definite())
            .map_or(width, sanitize_cells);
        let area = Rect {
            width: cells_to_u16(layout_width),
            ..ctx.area()
        };
        let natural_height = ctx.with_area(area, |ctx| {
            intrinsic_height_for_list_view(&self.children, start, &self.metadata, ctx)
        });
        let height = known_dimensions
            .height
            .map_or(f32::from(natural_height), sanitize_cells);
        LayoutSize::new(width, height)
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

    fn __visit_layout_children(
        &self,
        _ctx: &mut RenderCtx<'_, '_>,
        _visitor: &mut dyn FnMut(&AnyView, &mut RenderCtx<'_, '_>),
    ) {
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn can_reconcile_from(&self, previous: &dyn View) -> bool {
        previous
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|previous| self.kind == previous.kind)
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        let start = (self.kind == ListKind::Ordered).then_some(self.start);
        focused_control_span_for_list_view(&self.children, start, &self.metadata, ctx)
            .map(VerticalSpan::into_tuple)
    }
}

/// Returns the shared marker-column width for a semantic list.
///
/// # Arguments
///
/// * `item_count` — Number of list items.
/// * `ordered_start` — First decimal marker, or [`None`] for hyphen markers.
///
/// # Returns
///
/// A `u16` width containing the widest marker.
fn list_marker_width(item_count: usize, ordered_start: Option<usize>) -> u16 {
    (0..item_count)
        .map(|index| {
            ordered_start.map_or(1, |start| {
                start
                    .saturating_add(index)
                    .to_string()
                    .len()
                    .saturating_add(1)
            })
        })
        .max()
        .and_then(|width| u16::try_from(width).ok())
        .unwrap_or(0)
}

/// Returns one intrinsic width for semantic-list items.
///
/// # Arguments
///
/// * `items` — Marked list items to inspect.
/// * `marker_width` — Shared marker-column width.
/// * `constraint` — Min-content or max-content constraint to apply.
/// * `ctx` — Rendering context containing styles and inherited state.
///
/// # Returns
///
/// A measured `f32` terminal-cell width.
fn list_intrinsic_width(
    items: &[AnyView],
    marker_width: u16,
    constraint: AvailableSpace,
    ctx: &mut RenderCtx<'_, '_>,
) -> f32 {
    items
        .iter()
        .map(|item| {
            let children = item
                .downcast_ref::<crate::ListItemView>()
                .map_or_else(|| std::slice::from_ref(item), |item| item.children());
            children
                .iter()
                .map(|child| {
                    let indent = if child.is::<ListView>() {
                        2
                    } else {
                        marker_width.saturating_add(1)
                    };
                    intrinsic_child_width(child, constraint, ctx) + f32::from(indent)
                })
                .fold(f32::from(marker_width), f32::max)
        })
        .fold(0.0, f32::max)
}

/// Returns an intrinsic child width, traversing compatibility containers.
///
/// # Arguments
///
/// * `child` — Child view to measure.
/// * `constraint` — Min-content or max-content constraint to apply.
/// * `ctx` — Rendering context containing styles and inherited state.
///
/// # Returns
///
/// A measured `f32` terminal-cell width.
fn intrinsic_child_width(
    child: &AnyView,
    constraint: AvailableSpace,
    ctx: &mut RenderCtx<'_, '_>,
) -> f32 {
    let measured = child.measure(LayoutSize::all(None), LayoutSize::all(constraint), ctx);
    if measured.width > 0.0 || child.children().is_empty() {
        measured.width
    } else {
        child
            .children()
            .iter()
            .map(|child| intrinsic_child_width(child, constraint, ctx))
            .fold(0.0, f32::max)
    }
}

impl_styled_view!(ListView);
impl_container_view!(ListView);
