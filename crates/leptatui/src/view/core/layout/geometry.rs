//! Computed-layout conversion and retained terminal geometry.

use std::collections::HashMap;

use ratatui::layout::Rect as TerminalRect;
use taffy::tree::{NodeId, TaffyTree};

use crate::{
    Axes, LayoutGeometry, LayoutSize, Overflow, RenderCtx, View,
    view::core::measurement::sanitize_cells,
};

use super::{LayoutPath, measure::visit_children_with_style};

/// Visibility bounds applied while retaining one layout node.
#[derive(Clone, Copy)]
pub(super) struct RetentionBounds {
    /// Whether the node is the sole root box constrained by the target area.
    pub(super) clamp_to_area: bool,
    /// Accumulated clip inherited from layout ancestors.
    pub(super) inherited_clip: TerminalRect,
}

/// Copies rounded Taffy layouts into Leptatui metadata.
///
/// # Arguments
///
/// * `tree` — Computed transient Taffy tree.
/// * `node` — Current Taffy node being copied.
/// * `parent_origin` — Absolute origin of the parent border box.
/// * `nodes` — Mapping from Taffy nodes to logical paths and overflow behavior.
/// * `root` — Rendered root used to resolve each path.
/// * `ctx` — Render context used to reproduce traversal scopes.
/// * `bounds` — Root clamping and accumulated ancestor clipping bounds.
pub(super) fn retain_geometry(
    tree: &TaffyTree<LayoutPath>,
    node: NodeId,
    parent_origin: (f32, f32),
    nodes: &HashMap<NodeId, (LayoutPath, Axes<Overflow>)>,
    root: &dyn View,
    ctx: &mut RenderCtx<'_, '_>,
    bounds: RetentionBounds,
) {
    let layout = tree
        .layout(node)
        .expect("computed layout node should remain in the transient tree");
    let origin = (
        parent_origin.0 + layout.location.x,
        parent_origin.1 + layout.location.y,
    );

    let mut descendant_clip = bounds.inherited_clip;
    if let Some((path, overflow)) = nodes.get(&node) {
        let mut border_box = terminal_rect(origin, layout.size.width, layout.size.height);
        if bounds.clamp_to_area {
            border_box = border_box.intersection(ctx.area());
        }
        let padding_box = inset_rect(
            border_box,
            edges_to_u16(
                layout.border.left,
                layout.border.right,
                layout.border.top,
                layout.border.bottom,
            ),
        );
        let content_box = inset_rect(
            padding_box,
            edges_to_u16(
                layout.padding.left,
                layout.padding.right,
                layout.padding.top,
                layout.padding.bottom,
            ),
        );
        let content_extent = direct_content_extent(tree, node, origin, content_box);
        let gutters = scrollbar_gutters(content_box, content_extent, *overflow);
        let viewport = inset_rect(
            content_box,
            (0, u16::from(gutters.y), 0, u16::from(gutters.x)),
        );
        let maximum = Axes::new(
            scroll_maximum(
                content_extent.width.saturating_sub(viewport.width),
                overflow.x,
            ),
            scroll_maximum(
                content_extent.height.saturating_sub(viewport.height),
                overflow.y,
            ),
        );
        let clip = bounds.inherited_clip;
        descendant_clip = descendant_clip_rect(bounds.inherited_clip, viewport, *overflow, maximum);
        set_geometry_at_path(
            root,
            &path.0,
            LayoutGeometry {
                border_box,
                padding_box,
                content_box,
                viewport,
                clip,
            },
            content_extent,
            maximum,
            ctx,
        );
    }

    for child in tree
        .children(node)
        .expect("computed layout children should remain valid")
    {
        retain_geometry(
            tree,
            child,
            origin,
            nodes,
            root,
            ctx,
            RetentionBounds {
                clamp_to_area: false,
                inherited_clip: descendant_clip,
            },
        );
    }
}

/// Resolves mutually affecting scrollbar gutters from direct child geometry.
///
/// # Arguments
///
/// * `content_box` — Full rounded content rectangle before gutters.
/// * `extent` — Rounded direct-child content extent.
/// * `overflow` — Authored overflow behavior on both axes.
///
/// # Returns
///
/// An [`Axes`] value indicating horizontal and vertical gutter visibility.
fn scrollbar_gutters(
    content_box: TerminalRect,
    extent: LayoutSize<u16>,
    overflow: Axes<Overflow>,
) -> Axes<bool> {
    let mut gutters = Axes::new(
        overflow.x == Overflow::Scroll,
        overflow.y == Overflow::Scroll,
    );
    loop {
        let viewport_width = content_box.width.saturating_sub(u16::from(gutters.y));
        let viewport_height = content_box.height.saturating_sub(u16::from(gutters.x));
        let next = Axes::new(
            overflow.x == Overflow::Scroll
                || (overflow.x == Overflow::Auto && extent.width > viewport_width),
            overflow.y == Overflow::Scroll
                || (overflow.y == Overflow::Auto && extent.height > viewport_height),
        );
        if next == gutters {
            return gutters;
        }
        gutters = next;
    }
}

/// Returns the descendant clip established by one overflow box.
///
/// # Arguments
///
/// * `inherited` — Clip inherited from layout ancestors.
/// * `viewport` — Rounded content viewport for the current box.
/// * `overflow` — Authored overflow behavior on both axes.
/// * `maximum` — Rounded scroll ranges on both axes.
///
/// # Returns
///
/// A [`TerminalRect`] constraining clipped axes while preserving visible axes.
pub(crate) fn descendant_clip_rect(
    inherited: TerminalRect,
    viewport: TerminalRect,
    overflow: Axes<Overflow>,
    maximum: Axes<u16>,
) -> TerminalRect {
    let clips_x = clips_axis(overflow.x, maximum.x);
    let clips_y = clips_axis(overflow.y, maximum.y);
    let left = if !clips_x {
        inherited.x
    } else {
        inherited.x.max(viewport.x)
    };
    let right = if !clips_x {
        inherited.right()
    } else {
        inherited.right().min(viewport.right())
    };
    let top = if !clips_y {
        inherited.y
    } else {
        inherited.y.max(viewport.y)
    };
    let bottom = if !clips_y {
        inherited.bottom()
    } else {
        inherited.bottom().min(viewport.bottom())
    };

    TerminalRect::new(
        left,
        top,
        right.saturating_sub(left),
        bottom.saturating_sub(top),
    )
}

/// Returns whether one overflow axis establishes a descendant clip.
///
/// Automatic overflow clips only when direct child geometry creates a
/// scrollable range.
///
/// # Arguments
///
/// * `overflow` — Authored overflow behavior for the axis.
/// * `maximum` — Rounded maximum scroll offset for the axis.
///
/// # Returns
///
/// A [`bool`] indicating whether descendants are clipped to the viewport.
fn clips_axis(overflow: Overflow, maximum: u16) -> bool {
    match overflow {
        Overflow::Visible => false,
        Overflow::Auto => maximum > 0,
        Overflow::Hidden | Overflow::Clip | Overflow::Scroll => true,
    }
}

/// Converts one computed scroll range into an authored-axis maximum.
///
/// # Arguments
///
/// * `maximum` — Rounded range derived from direct child geometry.
/// * `overflow` — Authored overflow behavior for the axis.
///
/// # Returns
///
/// A rounded terminal-cell maximum, or zero for non-scrollable overflow modes.
fn scroll_maximum(maximum: u16, overflow: Overflow) -> u16 {
    if matches!(
        overflow,
        Overflow::Hidden | Overflow::Scroll | Overflow::Auto
    ) {
        maximum
    } else {
        0
    }
}

/// Returns the rounded extent of a node's direct layout children.
///
/// Visible overflow inside a child does not enlarge the parent's scrollable
/// range; only boxes directly assigned to the parent contribute.
///
/// # Arguments
///
/// * `tree` — Computed Taffy tree containing child layouts.
/// * `node` — Parent node whose direct children are measured.
/// * `origin` — Absolute floating-point origin of the parent border box.
/// * `content_box` — Rounded parent content rectangle establishing the origin.
///
/// # Returns
///
/// A [`LayoutSize`] containing the direct child border-box extent.
fn direct_content_extent(
    tree: &TaffyTree<LayoutPath>,
    node: NodeId,
    origin: (f32, f32),
    content_box: TerminalRect,
) -> LayoutSize<u16> {
    let mut extent = LayoutSize::all(0);
    for child in tree
        .children(node)
        .expect("computed layout children should remain valid")
    {
        let layout = tree
            .layout(child)
            .expect("computed child layout should remain valid");
        let child_box = terminal_rect(
            (origin.0 + layout.location.x, origin.1 + layout.location.y),
            layout.size.width,
            layout.size.height,
        );
        extent.width = extent
            .width
            .max(child_box.right().saturating_sub(content_box.x));
        extent.height = extent
            .height
            .max(child_box.bottom().saturating_sub(content_box.y));
    }
    extent
}

/// Stores geometry on the metadata addressed by a logical path.
///
/// # Arguments
///
/// * `view` — Current traversal root.
/// * `path` — Remaining child indexes leading to the target metadata.
/// * `geometry` — Rounded geometry to retain.
/// * `content_extent` — Rounded scrollable content dimensions.
/// * `maximum` — Rounded maximum scroll offsets.
/// * `ctx` — Render context used to reproduce traversal scopes.
fn set_geometry_at_path(
    view: &dyn View,
    path: &[usize],
    geometry: LayoutGeometry,
    content_extent: LayoutSize<u16>,
    maximum: Axes<u16>,
    ctx: &mut RenderCtx<'_, '_>,
) {
    if path.is_empty() {
        if view.style_metadata().is_none() {
            ctx.set_unstyled_layout_geometry(view, geometry);
        }
        if let Some(metadata) = view.style_metadata() {
            metadata.set_layout_geometry(geometry);
            metadata.set_content_extent(content_extent);
            metadata.set_max_scroll_offsets(maximum);
        }
        return;
    }

    let target = path[0];
    let mut index = 0usize;
    visit_children_with_style(view, ctx, &mut |child, child_ctx| {
        if index == target {
            set_geometry_at_path(
                child.as_view(),
                &path[1..],
                geometry,
                content_extent,
                maximum,
                child_ctx,
            );
        }
        index = index.saturating_add(1);
    });
}

/// Converts rounded floating-point layout values into a terminal rectangle.
///
/// # Arguments
///
/// * `origin` — Absolute floating-point x and y coordinates.
/// * `width` — Floating-point border-box width.
/// * `height` — Floating-point border-box height.
///
/// # Returns
///
/// A [`TerminalRect`] containing saturated terminal coordinates and extents.
fn terminal_rect(origin: (f32, f32), width: f32, height: f32) -> TerminalRect {
    TerminalRect::new(
        to_cell(origin.0),
        to_cell(origin.1),
        to_cell(width),
        to_cell(height),
    )
}

/// Converts four computed edges into saturated terminal-cell counts.
///
/// # Arguments
///
/// * `left` — Computed left edge.
/// * `right` — Computed right edge.
/// * `top` — Computed top edge.
/// * `bottom` — Computed bottom edge.
///
/// # Returns
///
/// A tuple containing left, right, top, and bottom cell counts.
fn edges_to_u16(left: f32, right: f32, top: f32, bottom: f32) -> (u16, u16, u16, u16) {
    (to_cell(left), to_cell(right), to_cell(top), to_cell(bottom))
}

/// Insets a terminal rectangle without allowing coordinate underflow.
///
/// # Arguments
///
/// * `rect` — Outer terminal rectangle.
/// * `edges` — Left, right, top, and bottom inset counts.
///
/// # Returns
///
/// A [`TerminalRect`] containing the saturated inner rectangle.
fn inset_rect(rect: TerminalRect, edges: (u16, u16, u16, u16)) -> TerminalRect {
    let (left, right, top, bottom) = edges;
    TerminalRect {
        x: rect.x.saturating_add(left).min(rect.right()),
        y: rect.y.saturating_add(top).min(rect.bottom()),
        width: rect.width.saturating_sub(left.saturating_add(right)),
        height: rect.height.saturating_sub(top.saturating_add(bottom)),
    }
}

/// Rounds one finite layout value into a terminal-cell coordinate or extent.
///
/// # Arguments
///
/// * `value` — Floating-point layout value to convert.
///
/// # Returns
///
/// A saturated `u16` terminal-cell value.
fn to_cell(value: f32) -> u16 {
    sanitize_cells(value).round() as u16
}
