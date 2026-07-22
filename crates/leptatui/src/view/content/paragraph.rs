//! Semantic rich-text paragraph view.

use crate::view::core::{
    capabilities::{impl_styled_view, impl_textual_view},
    render::{line_count_height, resolve_style, semantic_paragraph},
};
use crate::view::{
    CellAlignment, StyleMetadata, View, ViewType,
    link::{RichTextWrapMode, resolved_rich_text},
};
use crate::{
    RichText,
    app::{AppControl, Result},
    component::{FocusedControl, RenderCtx},
};

/// Semantic paragraph content.
#[derive(Debug, PartialEq)]
pub struct ParagraphView {
    /// Rich paragraph content.
    pub(crate) content: RichText,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a semantic paragraph.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`ParagraphView`] containing `content`.
pub fn paragraph(content: impl Into<RichText>) -> ParagraphView {
    ParagraphView {
        content: content.into(),
        metadata: StyleMetadata::new(ViewType::Paragraph),
    }
}

impl View for ParagraphView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let style = resolve_style(&self.metadata, ctx);
        let rendered = resolved_rich_text(&self.content, &self.metadata, style, ctx);
        let area = ctx.area();
        ctx.render_widget(semantic_paragraph(&rendered, style));
        self.content.record_link_hit_areas(
            area,
            area.width,
            CellAlignment::Left,
            RichTextWrapMode::Word,
            ctx,
        );
        self.content.clear_link_scroll_requests();
        Ok(())
    }

    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        let style = resolve_style(&self.metadata, ctx);
        line_count_height(
            semantic_paragraph(self.content.text(), style).line_count(ctx.area().width),
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

impl_styled_view!(ParagraphView);
impl_textual_view!(ParagraphView);
