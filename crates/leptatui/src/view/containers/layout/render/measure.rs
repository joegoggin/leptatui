//! Intrinsic computed-container measurement.

use ratatui::layout::Rect;

use crate::view::core::{
    measurement::{AvailableSpace, cells_to_u16, measure_view, sanitize_cells},
    render::resolve_style,
};
use crate::view::{AnyView, StyleMetadata};
use crate::{LayoutSize, component::RenderCtx};

/// Measures a computed container through the two-axis view contract.
///
/// # Arguments
///
/// * `children` — Child views measured in source order.
/// * `metadata` — Container metadata supplying inherited style.
/// * `known_dimensions` — Exact dimensions supplied by parent layout.
/// * `available_space` — Soft constraints used for unknown dimensions.
/// * `ctx` — Render context used for child measurement.
///
/// # Returns
///
/// A [`LayoutSize`] containing the container's intrinsic dimensions.
pub(crate) fn measure_container(
    children: &[AnyView],
    metadata: &StyleMetadata,
    known_dimensions: LayoutSize<Option<f32>>,
    available_space: LayoutSize<AvailableSpace>,
    ctx: &mut RenderCtx<'_, '_>,
) -> LayoutSize<f32> {
    let style = resolve_style(metadata, ctx);
    let width = known_dimensions
        .width
        .or_else(|| available_space.width.definite())
        .map_or(0.0, sanitize_cells);
    let area = Rect {
        width: cells_to_u16(width),
        ..ctx.area()
    };
    let natural_height = ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            let heights = children
                .iter()
                .map(|child| measure_view(child.as_view(), ctx).height);
            if style.display == Some(crate::Display::Flex)
                && matches!(
                    style.flex_direction.unwrap_or_default(),
                    crate::FlexDirection::Row | crate::FlexDirection::RowReverse
                )
            {
                heights.fold(0.0, f32::max)
            } else {
                heights.fold(0.0, |total, height| sanitize_cells(total + height))
            }
        },
    );
    let height = known_dimensions
        .height
        .map_or_else(|| sanitize_cells(natural_height), sanitize_cells);
    LayoutSize::new(width, height)
}
