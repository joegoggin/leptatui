//! Rich-text semantic table-cell view.

use crate::view::core::capabilities::{impl_styled_view, impl_textual_view};
use crate::view::{StyleMetadata, View, ViewType};
use crate::{
    RichText,
    app::{AppControl, Result},
    component::{FocusedControl, RenderCtx},
};

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
    fn min_height(&self, _ctx: &mut RenderCtx<'_, '_>) -> u16 {
        0
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

    fn reconcile(&mut self, previous: &dyn View) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.content.reconcile_links(&previous.content);
        }
    }

    fn __focusable_count(&self) -> usize {
        self.content.focusable_count()
    }

    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.content.focused_index_inner(index)
    }

    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        self.content.set_focus_by_index_inner(target, index);
    }

    fn __focusable_index_at_position_inner(
        &self,
        column: u16,
        row: u16,
        index: &mut usize,
    ) -> Option<usize> {
        self.content.focusable_index_at_position(column, row, index)
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        self.content
            .focused_link_span(ctx.area().width)
            .map(|span| span.into_tuple())
    }

    fn __activate_focused_button(&self) -> Result<Option<AppControl>> {
        self.content.activate_focused_link()
    }

    fn __focused_control(&self) -> Option<FocusedControl> {
        self.content.focused_control()
    }

    fn __focused_link_target(&self) -> Option<crate::LinkTarget> {
        self.content.focused_link_target()
    }

    fn __clear_hit_areas(&self) {
        self.metadata.clear_hit_areas();
        self.content.clear_link_hit_areas();
    }
}

impl_styled_view!(TableCellView);
impl_textual_view!(TableCellView);
