//! Ordered child painting, clipping, and scrollbar rendering.

use ratatui::{
    layout::Rect,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::view::{AnyView, StyleMetadata};
use crate::{Axes, Position, TuiStyle, app::Result, component::RenderCtx};

use super::{
    geometry::{child_geometry, local_geometry, scroll_geometry},
    positioning::{child_paint_style, positioned_child_origin, translated_geometry},
};

/// Geometry and clipping settings used while painting container children.
#[derive(Clone, Copy)]
pub(super) struct ChildPaintOptions {
    /// Parent content box used for retained child positioning.
    pub(super) content_area: Rect,
    /// Accumulated parent clip applied to each direct child.
    pub(super) clip: Rect,
    /// Signed translation from retained to assigned geometry.
    pub(super) layout_offset: (i32, i32),
}

/// One box participating in the current explicit stacking context.
struct PaintEntry {
    /// Child indexes leading from the current container to the painted box.
    path: Vec<usize>,
    /// Back-to-front stacking category resolved for the endpoint.
    stacking_level: crate::view::core::layout::stacking::StackingLevel,
    /// Depth-first logical order used to break equal-level ties.
    source_order: usize,
    /// Whether the endpoint promotes its own positioned descendants.
    endpoint_defers: bool,
}

/// Renders visible children into computed areas with two-axis scroll offsets.
///
/// # Arguments
///
/// * `children` — Child views rendered in source order.
/// * `offsets` — Horizontal and vertical offsets applied to child geometry.
/// * `inherited_style` — Cascaded style inherited by each child.
/// * `parent_metadata` — Parent metadata supplying selector ancestry.
/// * `options` — Retained-geometry translation and axis clipping settings.
/// * `ctx` — Render context targeting the container.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if child rendering performs terminal I/O that fails.
pub(super) fn render_children(
    children: &[AnyView],
    offsets: Axes<u16>,
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    options: ChildPaintOptions,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    if let Some((target, remaining, endpoint_defers)) = ctx.next_stacking_target() {
        let Some(child) = children.get(target) else {
            return Ok(());
        };
        return render_path_child(
            child,
            remaining,
            endpoint_defers,
            offsets,
            &inherited_style,
            parent_metadata,
            options,
            ctx,
        );
    }

    if ctx.defers_positioned_descendants() {
        for child in children {
            let (position, _, insets, _) = scoped_child_paint_style(
                child,
                &inherited_style,
                parent_metadata,
                options.content_area,
                ctx,
            );
            if position != Position::Static {
                continue;
            }
            ctx.with_stacking_state(true, None, false, |ctx| {
                render_child_box(
                    child,
                    position,
                    insets,
                    offsets,
                    &inherited_style,
                    parent_metadata,
                    options,
                    ctx,
                )
            })?;
        }
        return Ok(());
    }

    let mut paint_order = Vec::new();
    let mut source_order = 0usize;
    for (source_index, child) in children.iter().enumerate() {
        let (position, stacking_level, _, establishes_context) = scoped_child_paint_style(
            child,
            &inherited_style,
            parent_metadata,
            options.content_area,
            ctx,
        );
        paint_order.push(PaintEntry {
            path: vec![source_index],
            stacking_level,
            source_order,
            endpoint_defers: !establishes_context,
        });
        source_order = source_order.saturating_add(1);

        if position != Position::Fixed && !establishes_context {
            ctx.with_area_inherited_style_and_selector_ancestor(
                options.content_area,
                inherited_style.clone(),
                parent_metadata.clone(),
                |child_ctx| {
                    collect_positioned_descendants(
                        child.as_view(),
                        &[source_index],
                        child_ctx,
                        &mut source_order,
                        &mut paint_order,
                    );
                },
            );
        }
    }
    paint_order.sort_by_key(|entry| (entry.stacking_level, entry.source_order));

    for entry in paint_order {
        let Some((&target, remaining)) = entry.path.split_first() else {
            continue;
        };
        let Some(child) = children.get(target) else {
            continue;
        };
        render_path_child(
            child,
            remaining.to_vec(),
            entry.endpoint_defers,
            offsets,
            &inherited_style,
            parent_metadata,
            options,
            ctx,
        )?;
    }
    Ok(())
}

/// Resolves one child paint style inside its parent style scope.
///
/// # Arguments
///
/// * `child` — Direct child whose paint style is resolved.
/// * `inherited_style` — Style values inherited from the parent.
/// * `parent_metadata` — Parent metadata supplying selector ancestry.
/// * `content_area` — Parent content box used while resolving the child.
/// * `ctx` — Render context supplying stylesheets and viewport state.
///
/// # Returns
///
/// A [`tuple`](prim@tuple) containing positioning, stacking category, insets,
/// and explicit-context behavior.
fn scoped_child_paint_style(
    child: &AnyView,
    inherited_style: &TuiStyle,
    parent_metadata: &StyleMetadata,
    content_area: Rect,
    ctx: &mut RenderCtx<'_, '_>,
) -> (
    Position,
    crate::view::core::layout::stacking::StackingLevel,
    crate::Edges<crate::LengthAuto>,
    bool,
) {
    ctx.with_area_inherited_style_and_selector_ancestor(
        content_area,
        inherited_style.clone(),
        parent_metadata.clone(),
        |child_ctx| child_paint_style(child, child_ctx),
    )
}

/// Collects positioned descendants promoted through a non-context box.
///
/// # Arguments
///
/// * `view` — Non-context view whose retained descendants are inspected.
/// * `path` — Child indexes leading from the active context to `view`.
/// * `ctx` — Render context carrying the view's inherited style scope.
/// * `source_order` — Monotonic depth-first order for stable ties.
/// * `paint_order` — Output entries receiving promoted positioned boxes.
fn collect_positioned_descendants(
    view: &dyn crate::View,
    path: &[usize],
    ctx: &mut RenderCtx<'_, '_>,
    source_order: &mut usize,
    paint_order: &mut Vec<PaintEntry>,
) {
    visit_retained_children_with_style(view, ctx, &mut |child, child_ctx, child_index| {
        let (position, stacking_level, _, establishes_context) =
            child_paint_style(child, child_ctx);
        let mut child_path = path.to_vec();
        child_path.push(child_index);
        let current_order = *source_order;
        *source_order = source_order.saturating_add(1);

        if position != Position::Static && position != Position::Fixed {
            paint_order.push(PaintEntry {
                path: child_path.clone(),
                stacking_level,
                source_order: current_order,
                endpoint_defers: !establishes_context,
            });
        }
        if position != Position::Fixed && !establishes_context {
            collect_positioned_descendants(
                child.as_view(),
                &child_path,
                child_ctx,
                source_order,
                paint_order,
            );
        }
    });
}

/// Visits retained logical children under the current view's inherited style.
///
/// Structural boundaries do not consume a paint-path index; styled boxes
/// assign one index per retained direct child.
///
/// # Arguments
///
/// * `view` — Parent whose retained logical children are visited.
/// * `ctx` — Render context active at the parent.
/// * `visitor` — Callback receiving each child, scoped context, and paint index.
fn visit_retained_children_with_style(
    view: &dyn crate::View,
    ctx: &mut RenderCtx<'_, '_>,
    visitor: &mut dyn FnMut(&AnyView, &mut RenderCtx<'_, '_>, usize),
) {
    let Some(metadata) = view.style_metadata() else {
        view.__visit_retained_children(ctx, &mut |child, child_ctx| {
            collect_structural_children(child.as_view(), child_ctx, visitor);
        });
        return;
    };
    let style = ctx.resolve_style(metadata);
    ctx.with_area_inherited_style_and_selector_ancestor(
        ctx.area(),
        style.inherited_values(),
        metadata.clone(),
        |child_ctx| {
            let mut child_index = 0usize;
            view.__visit_retained_children(child_ctx, &mut |child, nested_ctx| {
                visitor(child, nested_ctx, child_index);
                child_index = child_index.saturating_add(1);
            });
        },
    );
}

/// Traverses layout-transparent nodes without consuming a paint-path index.
///
/// # Arguments
///
/// * `view` — Structural or styled view reached through a transparent boundary.
/// * `ctx` — Render context carrying retained component and stylesheet scopes.
/// * `visitor` — Callback used once a styled box exposes paint children.
fn collect_structural_children(
    view: &dyn crate::View,
    ctx: &mut RenderCtx<'_, '_>,
    visitor: &mut dyn FnMut(&AnyView, &mut RenderCtx<'_, '_>, usize),
) {
    if view.style_metadata().is_some() {
        visit_retained_children_with_style(view, ctx, visitor);
        return;
    }
    view.__visit_retained_children(ctx, &mut |child, child_ctx| {
        collect_structural_children(child.as_view(), child_ctx, visitor);
    });
}

/// Paints one direct child while traversing toward a promoted endpoint.
///
/// # Arguments
///
/// * `child` — Direct child beginning or continuing the promoted path.
/// * `remaining` — Child indexes below `child` leading to the endpoint.
/// * `endpoint_defers` — Whether the endpoint promotes positioned descendants.
/// * `offsets` — Parent horizontal and vertical scroll offsets.
/// * `inherited_style` — Style values inherited from the parent.
/// * `parent_metadata` — Parent metadata supplying selector ancestry.
/// * `options` — Retained-geometry translation and clipping settings.
/// * `ctx` — Render context targeting the parent.
///
/// # Returns
///
/// An empty [`Result`] after the path endpoint paints.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if path rendering performs terminal I/O that fails.
#[allow(clippy::too_many_arguments)]
fn render_path_child(
    child: &AnyView,
    remaining: Vec<usize>,
    endpoint_defers: bool,
    offsets: Axes<u16>,
    inherited_style: &TuiStyle,
    parent_metadata: &StyleMetadata,
    options: ChildPaintOptions,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let (position, _, insets, _) = scoped_child_paint_style(
        child,
        inherited_style,
        parent_metadata,
        options.content_area,
        ctx,
    );
    let at_endpoint = remaining.is_empty();
    ctx.with_stacking_state(
        at_endpoint && endpoint_defers,
        (!at_endpoint).then_some(remaining),
        endpoint_defers,
        |ctx| {
            render_child_box(
                child,
                position,
                insets,
                offsets,
                inherited_style,
                parent_metadata,
                options,
                ctx,
            )
        },
    )
}

/// Paints one child with resolved positioning, clipping, and scroll translation.
///
/// # Arguments
///
/// * `child` — Direct child to paint.
/// * `position` — Resolved child positioning behavior.
/// * `insets` — Resolved child inset edges.
/// * `offsets` — Parent horizontal and vertical scroll offsets.
/// * `inherited_style` — Style values inherited from the parent.
/// * `parent_metadata` — Parent metadata supplying selector ancestry.
/// * `options` — Retained-geometry translation and clipping settings.
/// * `ctx` — Render context targeting the parent.
///
/// # Returns
///
/// An empty [`Result`] after the visible child region paints.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if child rendering performs terminal I/O that fails.
#[allow(clippy::too_many_arguments)]
fn render_child_box(
    child: &AnyView,
    position: Position,
    insets: crate::Edges<crate::LengthAuto>,
    offsets: Axes<u16>,
    inherited_style: &TuiStyle,
    parent_metadata: &StyleMetadata,
    options: ChildPaintOptions,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    if child
        .style_metadata()
        .is_some_and(StyleMetadata::is_layout_hidden)
        || position == Position::Fixed
    {
        return Ok(());
    }

    let geometry = child_geometry(
        child,
        options.content_area,
        options.clip,
        options.layout_offset,
        ctx,
    );
    let full_area = geometry.border_box;
    let (shifted_left, shifted_top) = positioned_child_origin(
        full_area,
        offsets,
        position,
        ctx.sticky_scrollport(),
        insets,
        ctx.viewport_size(),
    );
    let shifted_right = shifted_left.saturating_add(i32::from(full_area.width));
    let shifted_bottom = shifted_top.saturating_add(i32::from(full_area.height));
    let visible_top = shifted_top.max(i32::from(options.clip.y));
    let visible_bottom = shifted_bottom.min(i32::from(options.clip.bottom()));
    let visible_left = shifted_left.max(i32::from(options.clip.x));
    let visible_right = shifted_right.min(i32::from(options.clip.right()));
    if visible_bottom <= visible_top || visible_right <= visible_left {
        return Ok(());
    }
    let source_x = u16::try_from(visible_left.saturating_sub(shifted_left)).unwrap_or(u16::MAX);
    let source_y = u16::try_from(visible_top.saturating_sub(shifted_top)).unwrap_or(u16::MAX);
    let shifted_area = Rect {
        x: u16::try_from(shifted_left.max(0)).unwrap_or(u16::MAX),
        y: u16::try_from(shifted_top.max(0)).unwrap_or(u16::MAX),
        ..full_area
    };
    let visible_area = Rect {
        x: u16::try_from(visible_left).unwrap_or(u16::MAX),
        y: u16::try_from(visible_top).unwrap_or(u16::MAX),
        width: u16::try_from(visible_right.saturating_sub(visible_left)).unwrap_or(u16::MAX),
        height: u16::try_from(visible_bottom.saturating_sub(visible_top)).unwrap_or(u16::MAX),
    };
    if source_x == 0 && source_y == 0 && visible_area == shifted_area {
        let geometry = if position == Position::Sticky {
            translated_geometry(geometry, shifted_left, shifted_top)
        } else {
            scroll_geometry(geometry, offsets)
        };
        ctx.with_assigned_layout_geometry_and_selector_ancestor(
            geometry,
            child.style_metadata(),
            inherited_style.clone(),
            parent_metadata.clone(),
            |ctx| child.as_view().render(ctx),
        )
    } else {
        let geometry = local_geometry(
            geometry,
            Rect::new(source_x, source_y, visible_area.width, visible_area.height),
        );
        ctx.render_view_clipped(
            child,
            geometry,
            ratatui::layout::Position::new(source_x, source_y),
            visible_area,
            inherited_style.clone(),
            parent_metadata.clone(),
        )
    }
}

/// Renders visible horizontal and vertical scrollbars.
///
/// # Arguments
///
/// * `offsets` — Current horizontal and vertical scroll offsets.
/// * `maximum` — Largest permitted offsets on both axes.
/// * `content_area` — Container content box that receives the scrollbar.
/// * `viewport` — Final content viewport excluding scrollbar gutters.
/// * `gutters` — Whether horizontal and vertical gutters are visible.
/// * `ctx` — Render context targeting the container.
pub(super) fn render_scrollbars(
    offsets: Axes<u16>,
    maximum: Axes<u16>,
    content_area: Rect,
    viewport: Rect,
    gutters: Axes<bool>,
    ctx: &mut RenderCtx<'_, '_>,
) {
    if content_area.width == 0 || content_area.height == 0 {
        return;
    }

    if gutters.y {
        let area = Rect {
            height: viewport.height,
            ..content_area
        };
        let mut state = ScrollbarState::new(usize::from(maximum.y).saturating_add(1))
            .position(usize::from(offsets.y))
            .viewport_content_length(usize::from(area.height));
        ctx.with_area(area, |ctx| {
            ctx.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None),
                &mut state,
            );
        });
    }
    if gutters.x {
        let area = Rect {
            width: viewport.width,
            ..content_area
        };
        let mut state = ScrollbarState::new(usize::from(maximum.x).saturating_add(1))
            .position(usize::from(offsets.x))
            .viewport_content_length(usize::from(area.width));
        ctx.with_area(area, |ctx| {
            ctx.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                    .begin_symbol(None)
                    .end_symbol(None),
                &mut state,
            );
        });
    }
}
