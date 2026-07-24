//! Semantic list-item container view.

use crate::view::containers::layout::render::{
    focused_control_span_for_layout_view, min_height_for_layout_view, render_layout_view,
};
use crate::view::core::{
    capabilities::{impl_container_view, impl_styled_view},
    measurement::{AvailableSpace, measure_legacy_height},
    render::VerticalSpan,
};
use crate::view::{AnyView, IntoViews, StyleMetadata, View, ViewType};
use crate::{LayoutSize, app::Result, component::RenderCtx, style::LayoutDirection};

/// Vertically stacked blocks belonging to one list marker.
#[derive(Debug, PartialEq)]
pub struct ListItemView {
    /// Document block children.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a semantic list item containing vertically stacked blocks.
///
/// # Arguments
///
/// * `children` — Homogeneous collection or heterogeneous tuple of blocks.
///
/// # Returns
///
/// A [`ListItemView`] containing the converted children.
pub fn list_item(children: impl IntoViews) -> ListItemView {
    ListItemView {
        children: children.into_views(),
        metadata: StyleMetadata::new(ViewType::ListItem),
    }
}

impl View for ListItemView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        render_layout_view(&self.children, &self.metadata, LayoutDirection::Column, ctx)
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        measure_legacy_height(
            min_height_for_layout_view(
                &self.children,
                &self.metadata,
                LayoutDirection::Column,
                ctx,
            ),
            known_dimensions,
            available_space,
        )
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
        focused_control_span_for_layout_view(
            &self.children,
            &self.metadata,
            LayoutDirection::Column,
            ctx,
        )
        .map(VerticalSpan::into_tuple)
    }
}

impl_styled_view!(ListItemView);
impl_container_view!(ListItemView);
