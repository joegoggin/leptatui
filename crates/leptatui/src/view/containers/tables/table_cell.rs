//! Rich-text semantic table-cell view.

use crate::view::core::{
    capabilities::{impl_styled_view, impl_textual_view},
    measurement::{AvailableSpace, measure_rich_text},
    render::resolve_style,
};
use crate::view::{StyleMetadata, View, ViewType, link::impl_rich_text_view};
use crate::{LayoutSize, RichText, app::Result, component::RenderCtx};

/// Horizontal alignment applied to wrapped table-cell content.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CellAlignment {
    /// Aligns cell content to the left edge.
    #[default]
    Left,
    /// Centers cell content within the allocated column width.
    Center,
    /// Aligns cell content to the right edge.
    Right,
}

/// Inline rich-text table cell.
#[derive(Debug, PartialEq)]
pub struct TableCellView {
    /// Rich cell content.
    pub(crate) content: RichText,
    /// Horizontal content alignment.
    pub(crate) alignment: CellAlignment,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

impl TableCellView {
    /// Sets horizontal alignment for wrapped content lines.
    ///
    /// # Arguments
    ///
    /// * `alignment` — Alignment applied to every wrapped content line.
    ///
    /// # Returns
    ///
    /// This table cell with the requested alignment.
    pub fn alignment(mut self, alignment: CellAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

/// Creates a semantic table cell.
///
/// # Arguments
///
/// * `content` — Rich text content to render in the cell.
///
/// # Returns
///
/// A left-aligned [`TableCellView`].
pub fn table_cell(content: impl Into<RichText>) -> TableCellView {
    TableCellView {
        content: content.into(),
        alignment: CellAlignment::Left,
        metadata: StyleMetadata::new(ViewType::TableCell),
    }
}

impl TableCellView {
    /// Returns this cell's horizontal alignment.
    pub const fn cell_alignment(&self) -> CellAlignment {
        self.alignment
    }
}

impl View for TableCellView {
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }
    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        measure_rich_text(
            self.content.text(),
            resolve_style(&self.metadata, ctx),
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
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    impl_rich_text_view!();

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        self.content
            .focused_link_span(ctx.area().width)
            .map(|span| span.into_tuple())
    }
}

impl_styled_view!(TableCellView);
impl_textual_view!(TableCellView);
