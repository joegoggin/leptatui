//! Semantic rich-text paragraph view.

use crate::view::core::{
    capabilities::{impl_styled_view, impl_textual_view},
    measurement::{AvailableSpace, measure_rich_text},
    render::{resolve_style, semantic_paragraph},
};
use crate::view::{
    CellAlignment, StyleMetadata, View, ViewType,
    link::{RichTextWrapMode, impl_rich_text_view, resolved_rich_text},
};
use crate::{LayoutSize, RichText, app::Result, component::RenderCtx};

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
        let area = if let Some(geometry) = ctx.active_layout_geometry(&self.metadata) {
            ctx.with_area(geometry.border_box, |ctx| {
                ctx.render_widget(style.to_block());
            });
            ctx.with_area(geometry.content_box, |ctx| {
                ctx.render_widget(semantic_paragraph(&rendered, style));
            });
            geometry.content_box
        } else {
            let area = ctx.area();
            ctx.render_widget(semantic_paragraph(&rendered, style));
            area
        };
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

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let style = resolve_style(&self.metadata, ctx);
        let mut measured = measure_rich_text(
            self.content.text(),
            style,
            known_dimensions,
            available_space,
        );
        if known_dimensions.height.is_none() {
            measured.height = measured.height.max(1.0);
        }
        measured
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
            .focused_link_span(
                ctx.active_layout_geometry(&self.metadata)
                    .map_or_else(|| ctx.area().width, |geometry| geometry.content_box.width),
            )
            .map(|span| span.into_tuple())
    }
}

impl_styled_view!(ParagraphView);
impl_textual_view!(ParagraphView);
