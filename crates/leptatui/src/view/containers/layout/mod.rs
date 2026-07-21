//! Row and column layout view.
//!
//! # Modules
//!
//! - [`render`] — Layout rendering, measurement, and focus geometry.

pub(crate) mod render;

use self::render::{
    focused_control_span_for_layout_view, min_height_for_layout_view, render_layout_view,
};
use crate::view::core::{
    capabilities::{impl_container_view, impl_styled_view},
    render::VerticalSpan,
};
use crate::view::{AnyView, IntoViews, StyleMetadata, View, ViewType};
use crate::{app::Result, component::RenderCtx, style::LayoutDirection};

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

///
/// # Arguments
///
/// * `children` — Homogeneous collection or heterogeneous tuple of child views.
///
/// # Returns
///
/// A row-oriented [`LayoutView`].
pub fn row(children: impl IntoViews) -> LayoutView {
    LayoutView {
        children: children.into_views(),
        default_direction: LayoutDirection::Row,
        metadata: StyleMetadata::new(ViewType::Row),
    }
}

/// Creates a vertical layout.
///
/// # Arguments
///
/// * `children` — Homogeneous collection or heterogeneous tuple of child views.
///
/// # Returns
///
/// A column-oriented [`LayoutView`].
pub fn column(children: impl IntoViews) -> LayoutView {
    LayoutView {
        children: children.into_views(),
        default_direction: LayoutDirection::Column,
        metadata: StyleMetadata::new(ViewType::Column),
    }
}

impl LayoutView {
    /// Returns the direction used when no stylesheet overrides the layout.
    pub const fn default_direction(&self) -> LayoutDirection {
        self.default_direction
    }
}

impl View for LayoutView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        render_layout_view(&self.children, &self.metadata, self.default_direction, ctx)
    }

    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        min_height_for_layout_view(&self.children, &self.metadata, self.default_direction, ctx)
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

    fn __scroll_first_overflowing(&mut self, delta: i16) -> bool {
        if self.metadata.max_scroll_offset() > 0 && self.metadata.scroll_by(delta) {
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
        focused_control_span_for_layout_view(
            &self.children,
            &self.metadata,
            self.default_direction,
            ctx,
        )
        .map(VerticalSpan::into_tuple)
    }
}

impl_styled_view!(LayoutView);
impl_container_view!(LayoutView);
