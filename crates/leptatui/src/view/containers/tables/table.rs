//! Semantic table container view.

use super::render::{min_height_for_table_view, render_table_view};
use crate::view::core::capabilities::{impl_container_view, impl_styled_view};
use crate::view::{AnyView, IntoViews, StyleMetadata, View, ViewType};
use crate::{app::Result, component::RenderCtx};

/// Semantic table containing head and body sections.
#[derive(Debug, PartialEq)]
pub struct TableView {
    /// Table section children.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a semantic table from head and body sections.
///
/// # Arguments
///
/// * `sections` — Table-head and table-body sections in source order.
///
/// # Returns
///
/// A [`TableView`] containing the converted sections.
pub fn table(sections: impl IntoViews) -> TableView {
    TableView {
        children: sections.into_views(),
        metadata: StyleMetadata::new(ViewType::Table),
    }
}

impl View for TableView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        render_table_view(&self.children, &self.metadata, ctx)
    }

    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        min_height_for_table_view(&self.children, &self.metadata, ctx)
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

impl_styled_view!(TableView);
impl_container_view!(TableView);
