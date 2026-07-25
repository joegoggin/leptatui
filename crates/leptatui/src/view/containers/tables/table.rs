//! Semantic table container view.

use super::{
    render::{
        focused_link_span_for_table_view, intrinsic_height_for_table_view, render_table_view,
    },
    table_cell::TableCellView,
    table_row::TableRowView,
    table_section::TableSectionView,
};
use crate::view::core::{
    capabilities::{impl_container_view, impl_styled_view},
    measurement::{
        AvailableSpace, cells_to_u16, resolve_intrinsic_axis, rich_text_intrinsic_widths,
        sanitize_cells,
    },
};
use crate::view::{AnyView, IntoViews, StyleMetadata, View, ViewType};
use crate::{LayoutSize, app::Result, component::RenderCtx};
use ratatui::layout::Rect;

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

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let (min_width, max_width) = table_intrinsic_widths(&self.children);
        let width = resolve_intrinsic_axis(
            known_dimensions.width,
            available_space.width,
            f32::from(min_width),
            f32::from(max_width),
        )
        .max(0.0);
        let layout_width = known_dimensions
            .width
            .or_else(|| available_space.width.definite())
            .map_or(width, sanitize_cells);
        let area = Rect {
            width: cells_to_u16(layout_width),
            ..ctx.area()
        };
        let natural_height = ctx.with_area(area, |ctx| {
            intrinsic_height_for_table_view(&self.children, &self.metadata, ctx)
        });
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

    fn __visit_layout_children(
        &self,
        _ctx: &mut RenderCtx<'_, '_>,
        _visitor: &mut dyn FnMut(&AnyView, &mut RenderCtx<'_, '_>),
    ) {
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        focused_link_span_for_table_view(&self.children, &self.metadata, ctx)
            .map(|span| span.into_tuple())
    }
}

/// Returns min-content and max-content widths for a semantic table.
///
/// Table borders consume one column at each edge and between adjacent columns.
///
/// # Arguments
///
/// * `sections` — Semantic table sections to inspect.
///
/// # Returns
///
/// A tuple containing min-content and max-content table widths.
fn table_intrinsic_widths(sections: &[AnyView]) -> (u16, u16) {
    let mut min_columns = Vec::<u16>::new();
    let mut max_columns = Vec::<u16>::new();

    for section in sections {
        let Some(section) = section.downcast_ref::<TableSectionView>() else {
            continue;
        };
        for row in section.children() {
            let Some(row) = row.downcast_ref::<TableRowView>() else {
                continue;
            };
            for (index, cell) in row.children().iter().enumerate() {
                let Some(cell) = cell.downcast_ref::<TableCellView>() else {
                    continue;
                };
                if min_columns.len() <= index {
                    min_columns.resize(index.saturating_add(1), 0);
                    max_columns.resize(index.saturating_add(1), 0);
                }
                let (min_width, max_width) = rich_text_intrinsic_widths(cell.content());
                min_columns[index] = min_columns[index].max(min_width);
                max_columns[index] = max_columns[index].max(max_width);
            }
        }
    }

    let border_columns = u16::try_from(min_columns.len().saturating_add(1)).unwrap_or(u16::MAX);
    (
        min_columns
            .into_iter()
            .fold(border_columns, u16::saturating_add),
        max_columns
            .into_iter()
            .fold(border_columns, u16::saturating_add),
    )
}

impl_styled_view!(TableView);
impl_container_view!(TableView);
