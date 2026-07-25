//! Plain rich-text terminal view.

use ratatui::text::Text;

use crate::view::core::{
    capabilities::{impl_styled_view, impl_textual_view},
    measurement::{AvailableSpace, measure_rich_text},
    render::{resolve_style, semantic_paragraph},
};
use crate::view::{StyleMetadata, View, ViewType};
use crate::{LayoutSize, app::Result, component::RenderCtx};

/// Plain rich-text content.
#[derive(Debug, PartialEq)]
pub struct TextView {
    /// Rich text rendered by this node.
    pub(crate) content: Text<'static>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a plain text view.
///
/// # Arguments
///
/// * `content` — Rich text content to render.
///
/// # Returns
///
/// A [`TextView`] containing `content`.
pub fn text(content: impl Into<String>) -> TextView {
    TextView {
        content: Text::from(content.into()),
        metadata: StyleMetadata::new(ViewType::Text),
    }
}

impl View for TextView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let style = resolve_style(&self.metadata, ctx);
        if let Some(geometry) = ctx.active_layout_geometry(&self.metadata) {
            ctx.with_area(geometry.border_box, |ctx| {
                ctx.render_widget(style.to_block());
            });
            ctx.with_area(geometry.content_box, |ctx| {
                ctx.render_widget(semantic_paragraph(&self.content, style));
            });
        } else {
            ctx.render_widget(semantic_paragraph(&self.content, style));
        }
        Ok(())
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let style = resolve_style(&self.metadata, ctx);
        measure_rich_text(&self.content, style, known_dimensions, available_space)
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
}

impl_styled_view!(TextView);
impl_textual_view!(TextView);
