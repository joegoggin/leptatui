//! Semantic table-row container view.

use crate::view::core::capabilities::{impl_container_view, impl_styled_view};
use crate::view::core::measurement::{AvailableSpace, measure_fixed};
use crate::view::{AnyView, IntoViews, StyleMetadata, View, ViewType};
use crate::{LayoutSize, app::Result, component::RenderCtx};

/// Row containing semantic table cells.
#[derive(Debug, PartialEq)]
pub struct TableRowView {
    /// Table cell children.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a semantic table row.
///
/// # Arguments
///
/// * `cells` — Table cells in column order.
///
/// # Returns
///
/// A [`TableRowView`] containing the converted cells.
pub fn table_row(cells: impl IntoViews) -> TableRowView {
    TableRowView {
        children: cells.into_views(),
        metadata: StyleMetadata::new(ViewType::TableRow),
    }
}

impl View for TableRowView {
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }
    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        _available_space: LayoutSize<AvailableSpace>,
        _ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        measure_fixed(LayoutSize::all(0.0), known_dimensions)
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
}

impl_styled_view!(TableRowView);
impl_container_view!(TableRowView);
