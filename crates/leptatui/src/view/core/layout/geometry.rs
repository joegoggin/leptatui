//! Computed-layout conversion and retained terminal geometry.

use std::collections::HashMap;

use ratatui::layout::Rect as TerminalRect;
use taffy::tree::{NodeId, TaffyTree};

use crate::{Borders, LayoutGeometry, RenderCtx, View, view::core::measurement::sanitize_cells};

use super::{LayoutPath, measure::visit_children_with_style, style::default_borders};

/// Copies rounded Taffy layouts into Leptatui metadata.
///
/// # Arguments
///
/// * `tree` — Computed transient Taffy tree.
/// * `node` — Current Taffy node being copied.
/// * `parent_origin` — Absolute origin of the parent border box.
/// * `paths` — Mapping from Taffy nodes to logical view paths.
/// * `root` — Rendered root used to resolve each path.
/// * `ctx` — Render context used to reproduce traversal scopes.
/// * `clamp_to_area` — Whether this node is the sole visible root box.
pub(super) fn retain_geometry(
    tree: &TaffyTree<LayoutPath>,
    node: NodeId,
    parent_origin: (f32, f32),
    paths: &HashMap<NodeId, LayoutPath>,
    root: &dyn View,
    ctx: &mut RenderCtx<'_, '_>,
    clamp_to_area: bool,
) {
    let layout = tree
        .layout(node)
        .expect("computed layout node should remain in the transient tree");
    let origin = (
        parent_origin.0 + layout.location.x,
        parent_origin.1 + layout.location.y,
    );

    if let Some(path) = paths.get(&node) {
        let mut border_box = terminal_rect(origin, layout.size.width, layout.size.height);
        if clamp_to_area {
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
        set_geometry_at_path(
            root,
            &path.0,
            LayoutGeometry {
                border_box,
                padding_box,
                content_box,
            },
            ctx,
        );
    }

    for child in tree
        .children(node)
        .expect("computed layout children should remain valid")
    {
        retain_geometry(tree, child, origin, paths, root, ctx, false);
    }
}

/// Stores geometry on the metadata addressed by a logical path.
///
/// # Arguments
///
/// * `view` — Current traversal root.
/// * `path` — Remaining child indexes leading to the target metadata.
/// * `geometry` — Rounded geometry to retain.
/// * `ctx` — Render context used to reproduce traversal scopes.
fn set_geometry_at_path(
    view: &dyn View,
    path: &[usize],
    geometry: LayoutGeometry,
    ctx: &mut RenderCtx<'_, '_>,
) {
    if path.is_empty() {
        if view.style_metadata().is_none() {
            ctx.set_unstyled_layout_geometry(view, geometry);
        }
        if let Some(metadata) = view.style_metadata() {
            let style = ctx.resolve_style(metadata);
            let borders = style.borders.unwrap_or_else(|| default_borders(view));
            let padding_box = inset_rect(
                geometry.border_box,
                (
                    u16::from(borders.contains(Borders::LEFT)),
                    u16::from(borders.contains(Borders::RIGHT)),
                    u16::from(borders.contains(Borders::TOP)),
                    u16::from(borders.contains(Borders::BOTTOM)),
                ),
            );
            let padding = style.padding.unwrap_or_default();
            let content_box = inset_rect(
                padding_box,
                (padding.left, padding.right, padding.top, padding.bottom),
            );
            metadata.set_layout_geometry(LayoutGeometry {
                border_box: geometry.border_box,
                padding_box,
                content_box,
            });
        }
        return;
    }

    let target = path[0];
    let mut index = 0usize;
    visit_children_with_style(view, ctx, &mut |child, child_ctx| {
        if index == target {
            set_geometry_at_path(child.as_view(), &path[1..], geometry, child_ctx);
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
