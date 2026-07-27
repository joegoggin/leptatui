//! Generic block container view.
//!
//! # Modules
//!
//! - [`render`] — Container rendering, measurement, and focus geometry.

pub(crate) mod render;

use self::render::{focused_control_span_for_container, measure_container, render_container};
use crate::view::core::{
    capabilities::{impl_container_view, impl_styled_view},
    measurement::AvailableSpace,
    render::VerticalSpan,
};
use crate::view::{AnyView, IntoViews, StyleMetadata, View, ViewType};
use crate::{
    LayoutSize,
    app::Result,
    component::{LayoutPhase, RenderCtx},
    view::core::layout::{prepare_layout, render_fixed_descendants},
};

/// Generic block container with shared scrolling behavior.
#[derive(Debug, PartialEq)]
pub struct DivView {
    /// Child views arranged by the computed layout.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a generic block container.
///
/// # Arguments
///
/// * `children` — Homogeneous collection or heterogeneous tuple of child views.
///
/// # Returns
///
/// A [`DivView`] containing the converted children.
pub fn div(children: impl IntoViews) -> DivView {
    DivView {
        children: children.into_views(),
        metadata: StyleMetadata::new(ViewType::Div),
    }
}

impl View for DivView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let is_layout_root = ctx.layout_phase() == LayoutPhase::Inactive;
        if is_layout_root || self.metadata.layout_geometry().is_none() {
            prepare_layout(self, ctx);
        }
        render_container(&self.children, &self.metadata, ctx)?;
        if is_layout_root {
            render_fixed_descendants(self, ctx)?;
        }
        Ok(())
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        measure_container(
            &self.children,
            &self.metadata,
            known_dimensions,
            available_space,
            ctx,
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
        let max = self.metadata.max_scroll_offset();
        if max > 0 && self.metadata.scroll_offset() < max {
            self.metadata.set_scroll_offset(max);
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

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        focused_control_span_for_container(&self.children, &self.metadata, ctx)
            .map(VerticalSpan::into_tuple)
    }
}

impl_styled_view!(DivView);
impl_container_view!(DivView);
