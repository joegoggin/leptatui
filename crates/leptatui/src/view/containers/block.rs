//! Bordered single-child container view.

use crate::view::containers::layout::render::{
    focused_control_span_for_container, render_container_with_default_borders,
};
use crate::view::core::{
    capabilities::{impl_container_view, impl_styled_view},
    events::scroll_overflowing_at_position_in_paint_order,
    measurement::{AvailableSpace, measure_view_height, sanitize_cells},
    render::{VerticalSpan, resolve_style, vertical_border_rows, vertical_padding_rows},
};
use crate::view::{AnyView, IntoView, StyleMetadata, View, ViewType};
use crate::{
    Borders, LayoutSize,
    app::Result,
    component::{LayoutPhase, RenderCtx},
    view::core::layout::prepare_layout,
};

/// Bordered container around one child.
#[derive(Debug, PartialEq)]
pub struct BlockView {
    /// Sole child rendered inside the block.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a bordered block around one child view.
///
/// # Arguments
///
/// * `child` — View-compatible value rendered inside the block.
///
/// # Returns
///
/// A [`BlockView`] containing `child`.
pub fn block(child: impl IntoView) -> BlockView {
    BlockView {
        children: vec![child.into_view()],
        metadata: StyleMetadata::new(ViewType::Block),
    }
}

impl View for BlockView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        if ctx.layout_phase() == LayoutPhase::Inactive || self.metadata.layout_geometry().is_none()
        {
            prepare_layout(self, ctx);
        }
        render_container_with_default_borders(&self.children, &self.metadata, Borders::ALL, ctx)
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let style = resolve_style(&self.metadata, ctx);
        let width = known_dimensions
            .width
            .or_else(|| available_space.width.definite())
            .map_or(0.0, sanitize_cells);
        let child_height = self.children.first().map_or(0, |child| {
            ctx.with_area_inherited_style_and_selector_ancestor(
                ctx.area(),
                style.inherited_values(),
                self.metadata.clone(),
                |ctx| measure_view_height(child.as_view(), ctx),
            )
        });
        let natural_height = child_height
            .saturating_add(vertical_border_rows(style.borders.unwrap_or(Borders::ALL)))
            .saturating_add(vertical_padding_rows(style.padding));
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
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn __scroll_first_overflowing(&mut self, delta: crate::Axes<i16>) -> bool {
        if self.metadata.scroll_by(delta) {
            return true;
        }
        self.children
            .iter_mut()
            .any(|child| child.__scroll_first_overflowing(delta))
    }

    fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        if self.metadata.max_scroll_offset() > 0 && self.metadata.scroll_offset() > 0 {
            self.metadata.set_scroll_offset(0);
            return true;
        }
        self.children
            .iter_mut()
            .any(AnyView::__scroll_first_overflowing_to_top)
    }

    fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        let maximum = self.metadata.max_scroll_offset();
        if maximum > 0 && self.metadata.scroll_offset() < maximum {
            self.metadata.set_scroll_offset(maximum);
            return true;
        }
        self.children
            .iter_mut()
            .any(AnyView::__scroll_first_overflowing_to_bottom)
    }

    fn __has_overflowing_scroll_target(&self) -> bool {
        self.metadata.max_scroll_offset() > 0
            || self
                .children
                .iter()
                .any(AnyView::__has_overflowing_scroll_target)
    }

    fn __scroll_overflowing_at_position(
        &mut self,
        column: u16,
        row: u16,
        delta: crate::Axes<i16>,
    ) -> bool {
        let paint_order = self.metadata.child_paint_order();
        if scroll_overflowing_at_position_in_paint_order(
            &mut self.children,
            &paint_order,
            column,
            row,
            delta,
        ) {
            return true;
        }
        if self.metadata.contains_hit_position(column, row) {
            return self.metadata.scroll_by(delta);
        }
        false
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        focused_control_span_for_container(&self.children, &self.metadata, ctx)
            .map(VerticalSpan::into_tuple)
    }
}

impl_styled_view!(BlockView);
impl_container_view!(BlockView);
