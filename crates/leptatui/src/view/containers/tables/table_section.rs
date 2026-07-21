//! Header or body section of a semantic table.

use crate::view::core::capabilities::{impl_container_view, impl_styled_view};
use crate::view::{AnyView, IntoViews, StyleMetadata, View, ViewType};
use crate::{app::Result, component::RenderCtx};

/// Semantic role of a table section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableSectionKind {
    /// Header rows with the built-in header style.
    Head,
    /// Body rows.
    Body,
}

/// Header or body section of a semantic table.
#[derive(Debug, PartialEq)]
pub struct TableSectionView {
    /// Table row children.
    pub(crate) children: Vec<AnyView>,
    /// Semantic section role.
    pub(crate) kind: TableSectionKind,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a semantic table header.
///
/// # Arguments
///
/// * `rows` — Table rows rendered with header semantics.
///
/// # Returns
///
/// A header [`TableSectionView`].
pub fn table_head(rows: impl IntoViews) -> TableSectionView {
    TableSectionView {
        children: rows.into_views(),
        kind: TableSectionKind::Head,
        metadata: StyleMetadata::new(ViewType::TableHead),
    }
}

/// Creates a semantic table body.
///
/// # Arguments
///
/// * `rows` — Table rows rendered as body content.
///
/// # Returns
///
/// A body [`TableSectionView`].
pub fn table_body(rows: impl IntoViews) -> TableSectionView {
    TableSectionView {
        children: rows.into_views(),
        kind: TableSectionKind::Body,
        metadata: StyleMetadata::new(ViewType::TableBody),
    }
}

impl TableSectionView {
    /// Returns this section's semantic role.
    pub const fn kind(&self) -> TableSectionKind {
        self.kind
    }
}

impl View for TableSectionView {
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }
    fn min_height(&self, _ctx: &mut RenderCtx<'_, '_>) -> u16 {
        0
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
    fn can_reconcile_from(&self, previous: &dyn View) -> bool {
        previous
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|previous| self.kind == previous.kind)
    }
}

impl_styled_view!(TableSectionView);
impl_container_view!(TableSectionView);
