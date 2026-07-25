//! Bordered single-child container view.

use crate::view::core::{
    capabilities::{impl_container_view, impl_styled_view},
    measurement::{AvailableSpace, measure_legacy_height},
    render::{VerticalSpan, resolve_style, vertical_border_rows, vertical_padding_rows},
};
use crate::view::{AnyView, IntoView, StyleMetadata, View, ViewType};
use crate::{Borders, LayoutSize, app::Result, component::RenderCtx};

/// Bordered container around one child.
#[derive(Debug, PartialEq)]
pub struct BlockView {
    /// Sole child rendered inside the block.
    pub(crate) children: Vec<AnyView>,
    /// Selector and runtime metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a bordered block around one child view.
///
/// # Arguments
///
/// * `child` — View-compatible value rendered inside the block.
///
/// # Returns
///
/// A [`BlockView`] containing `child`.
pub fn block(child: impl IntoView) -> BlockView {
    BlockView {
        children: vec![child.into_view()],
        metadata: StyleMetadata::new(ViewType::Block),
    }
}

/// Returns the focused control's vertical span within a child view.
fn focused_control_span_for_view(
    view: &AnyView,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    view.__focused_button_span(ctx)
        .map(|(top, bottom)| VerticalSpan { top, bottom })
}

fn focused_control_span_for_block(
    view: &BlockView,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    let child = view.children.first()?;
    let style = resolve_style(&view.metadata, ctx);
    let area = ctx.area();
    let inner = style
        .to_block_with_default_borders(Borders::ALL)
        .inner(area);
    let top_offset = u32::from(inner.y.saturating_sub(area.y));
    ctx.with_area_inherited_style_and_selector_ancestor(
        inner,
        style.inherited_values(),
        view.metadata.clone(),
        |ctx| focused_control_span_for_view(child, ctx),
    )
    .map(|span| span.offset_by(top_offset))
}

impl View for BlockView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let style = resolve_style(&self.metadata, ctx);
        let block = style.to_block_with_default_borders(Borders::ALL);
        let inner = block.inner(ctx.area());
        ctx.render_widget(block);
        let Some(child) = self.children.first() else {
            return Ok(());
        };
        ctx.with_area_inherited_style_and_selector_ancestor(
            inner,
            style.inherited_values(),
            self.metadata.clone(),
            |ctx| child.render(ctx),
        )
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let style = resolve_style(&self.metadata, ctx);
        let child_height = self.children.first().map_or(0, |child| {
            ctx.with_area_inherited_style_and_selector_ancestor(
                ctx.area(),
                style.inherited_values(),
                self.metadata.clone(),
                |ctx| child.__min_height(ctx),
            )
        });
        measure_legacy_height(
            child_height
                .saturating_add(vertical_border_rows(style.borders.unwrap_or(Borders::ALL)))
                .saturating_add(vertical_padding_rows(style.padding)),
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

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        focused_control_span_for_block(self, ctx).map(VerticalSpan::into_tuple)
    }
}

impl_styled_view!(BlockView);
impl_container_view!(BlockView);
