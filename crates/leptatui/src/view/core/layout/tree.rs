//! Transient Taffy tree construction and layout orchestration.

use std::collections::HashMap;

use taffy::{
    geometry::Size as TaffySize,
    style::{
        AvailableSpace as TaffyAvailableSpace, Dimension as TaffyDimension,
        Overflow as TaffyOverflow,
    },
    tree::{NodeId, TaffyTree},
};

use crate::{
    Axes, Display, LayoutSize, Overflow, RenderCtx, TuiStyle, View, ViewportSize,
    component::LayoutPhase, view::core::measurement::sanitize_cells,
};

use super::{
    LayoutPath,
    geometry::retain_geometry,
    measure::{
        from_taffy_available, measure_at_path, overflow_at_path, uses_computed_child_layout_at_path,
    },
    style::{synthetic_root_style, to_taffy_style},
};

/// Taffy node associated with one styleable Leptatui view.
#[derive(Clone, Debug)]
struct LayoutNode {
    /// Taffy node receiving computed geometry.
    node: NodeId,
    /// Logical path back to the corresponding Leptatui view.
    path: LayoutPath,
    /// Authored overflow axes used for conditional scrollbar promotion.
    overflow: Axes<Overflow>,
}

/// Builds, computes, rounds, and stores one root layout snapshot.
///
/// # Arguments
///
/// * `root` — Root view whose visible boxes are mirrored into Taffy.
/// * `ctx` — Render context supplying styles, viewport size, and component scopes.
pub(crate) fn prepare_layout(root: &dyn View, ctx: &mut RenderCtx<'_, '_>) {
    ctx.set_layout_phase(LayoutPhase::Build);

    let mut tree = TaffyTree::<LayoutPath>::new();
    let mut nodes = Vec::new();
    let roots = build_view(root, &LayoutPath(Vec::new()), ctx, &mut tree, &mut nodes);

    if roots.is_empty() {
        ctx.set_layout_phase(LayoutPhase::Paint);
        return;
    }

    let available = ViewportSize::from(ctx.area());
    let clamp_root_to_area = roots.len() == 1;
    let root_node = if clamp_root_to_area {
        roots[0]
    } else {
        tree.new_with_children(synthetic_root_style(available), &roots)
            .expect("transient layout roots should form a valid Taffy tree")
    };

    ctx.set_layout_phase(LayoutPhase::Measure);
    compute_layout(&mut tree, root_node, available, root, ctx);
    while promote_overflowing_auto_nodes(&mut tree, &nodes) {
        compute_layout(&mut tree, root_node, available, root, ctx);
    }

    let root_path = nodes
        .iter()
        .find(|layout_node| layout_node.node == root_node)
        .map(|layout_node| layout_node.path.clone());
    let root_overflow = root_path
        .as_ref()
        .and_then(|path| overflow_at_path(root, &path.0, ctx))
        .unwrap_or_else(|| Axes::new(Overflow::Visible, Overflow::Auto));
    let root_uses_computed_layout = root_path
        .as_ref()
        .is_some_and(|path| uses_computed_child_layout_at_path(root, &path.0, ctx));
    let root_layout = tree.layout(root_node).copied().unwrap_or_default();
    let constrain_x = root_layout.size.width > f32::from(available.width)
        && !matches!(root_overflow.x, Overflow::Clip | Overflow::Visible);
    let constrain_y = root_layout.size.height > f32::from(available.height)
        && !matches!(root_overflow.y, Overflow::Clip | Overflow::Visible);
    if root_uses_computed_layout && (constrain_x || constrain_y) {
        let mut style = tree
            .style(root_node)
            .expect("computed root style should remain available")
            .clone();
        if constrain_x {
            style.size.width = TaffyDimension::length(f32::from(available.width));
        }
        if constrain_y {
            style.size.height = TaffyDimension::length(f32::from(available.height));
        }
        if constrain_x && root_overflow.x != Overflow::Hidden {
            style.overflow.x = TaffyOverflow::Scroll;
            style.scrollbar_width = 1.0;
        }
        if constrain_y && root_overflow.y != Overflow::Hidden {
            style.overflow.y = TaffyOverflow::Scroll;
            style.scrollbar_width = 1.0;
        }
        tree.set_style(root_node, style)
            .expect("computed root style should remain mutable");
        compute_layout(&mut tree, root_node, available, root, ctx);
        while promote_overflowing_auto_nodes(&mut tree, &nodes) {
            compute_layout(&mut tree, root_node, available, root, ctx);
        }
    }

    let paths = nodes
        .into_iter()
        .map(|node| (node.node, node.path))
        .collect::<HashMap<_, _>>();
    let area = ctx.area();
    retain_geometry(
        &tree,
        root_node,
        (f32::from(area.x), f32::from(area.y)),
        &paths,
        root,
        ctx,
        clamp_root_to_area,
    );

    ctx.set_layout_phase(LayoutPhase::Paint);
}

/// Computes one Taffy layout pass with Leptatui intrinsic measurement.
///
/// # Arguments
///
/// * `tree` — Transient Taffy tree to compute.
/// * `root_node` — Root node for the computation.
/// * `available` — Definite terminal area available to the root.
/// * `root` — Rendered Leptatui root used to resolve measurement paths.
/// * `ctx` — Render context used to reproduce traversal scopes.
fn compute_layout(
    tree: &mut TaffyTree<LayoutPath>,
    root_node: NodeId,
    available: ViewportSize,
    root: &dyn View,
    ctx: &mut RenderCtx<'_, '_>,
) {
    tree.compute_layout_with_measure(
        root_node,
        TaffySize {
            width: TaffyAvailableSpace::Definite(f32::from(available.width)),
            height: TaffyAvailableSpace::Definite(f32::from(available.height)),
        },
        |known, available, _node, path, _style| {
            let Some(path) = path else {
                return TaffySize::ZERO;
            };
            let measured = measure_at_path(
                root,
                &path.0,
                LayoutSize::new(known.width, known.height),
                LayoutSize::new(
                    from_taffy_available(available.width),
                    from_taffy_available(available.height),
                ),
                ctx,
            );
            TaffySize {
                width: sanitize_cells(measured.width),
                height: sanitize_cells(measured.height),
            }
        },
    )
    .expect("transient layout computation should use valid node identifiers");
}

/// Promotes overflowing automatic containers to scrollbar-reserving layout.
///
/// # Arguments
///
/// * `tree` — Computed Taffy tree whose styles may be promoted.
/// * `nodes` — Leptatui node records containing authored overflow behavior.
///
/// # Returns
///
/// A [`bool`] indicating whether another layout pass is required.
fn promote_overflowing_auto_nodes(tree: &mut TaffyTree<LayoutPath>, nodes: &[LayoutNode]) -> bool {
    let mut changed = false;
    for node in nodes {
        let Ok(layout) = tree.layout(node.node).copied() else {
            continue;
        };
        let mut style = tree
            .style(node.node)
            .expect("computed auto-overflow style should remain available")
            .clone();
        let promote_x = node.overflow.x == Overflow::Auto
            && layout.scroll_width() > 0.0
            && style.overflow.x != TaffyOverflow::Scroll;
        let promote_y = node.overflow.y == Overflow::Auto
            && layout.scroll_height() > 0.0
            && style.overflow.y != TaffyOverflow::Scroll;
        if promote_x || promote_y {
            if promote_x {
                style.overflow.x = TaffyOverflow::Scroll;
            }
            if promote_y {
                style.overflow.y = TaffyOverflow::Scroll;
            }
            style.scrollbar_width = 1.0;
            tree.set_style(node.node, style)
                .expect("auto-overflow style should remain mutable");
            changed = true;
        }
    }
    changed
}

/// Mirrors one view and its logical children into Taffy.
///
/// # Arguments
///
/// * `view` — Current view being inspected.
/// * `path` — Logical path from the rendered root to `view`.
/// * `ctx` — Render context used for style and boundary resolution.
/// * `tree` — Transient Taffy tree receiving visible boxes.
/// * `nodes` — Mapping records retained for geometry assignment.
///
/// # Returns
///
/// A [`Vec`] containing the Taffy nodes exposed to the layout parent.
fn build_view(
    view: &dyn View,
    path: &LayoutPath,
    ctx: &mut RenderCtx<'_, '_>,
    tree: &mut TaffyTree<LayoutPath>,
    nodes: &mut Vec<LayoutNode>,
) -> Vec<NodeId> {
    let Some(metadata) = view.style_metadata() else {
        if view.__is_layout_transparent() {
            return build_children(view, path, ctx, tree, nodes);
        }

        let node = tree
            .new_leaf_with_context(
                to_taffy_style(view, TuiStyle::new(), ctx.viewport_size()),
                path.clone(),
            )
            .expect("transient custom-view leaf should be valid");
        nodes.push(LayoutNode {
            node,
            path: path.clone(),
            overflow: Axes::new(Overflow::Visible, Overflow::Visible),
        });
        return vec![node];
    };

    metadata.clear_layout_geometry();
    let resolved = ctx.resolve_style(metadata);
    if resolved.display == Some(Display::None) {
        mark_hidden(view, ctx);
        return Vec::new();
    }

    let children = if view.__uses_computed_child_layout() {
        build_children_with_style(view, path, resolved, ctx, tree, nodes)
    } else {
        Vec::new()
    };
    let style = to_taffy_style(view, resolved, ctx.viewport_size());
    let node = if children.is_empty() {
        tree.new_leaf_with_context(style, path.clone())
            .expect("transient layout leaf should be valid")
    } else {
        tree.new_with_children(style, &children)
            .expect("transient layout children should be valid")
    };
    nodes.push(LayoutNode {
        node,
        path: path.clone(),
        overflow: resolved
            .overflow
            .unwrap_or_else(|| Axes::new(Overflow::Visible, Overflow::Auto)),
    });
    vec![node]
}

/// Builds logical children for a structural boundary.
///
/// # Arguments
///
/// * `view` — Structural or styleable parent whose children are visited.
/// * `path` — Logical path to the parent.
/// * `ctx` — Render context active for child traversal.
/// * `tree` — Transient Taffy tree receiving child boxes.
/// * `nodes` — Mapping records retained for geometry assignment.
///
/// # Returns
///
/// A [`Vec`] containing the visible child node identifiers.
fn build_children(
    view: &dyn View,
    path: &LayoutPath,
    ctx: &mut RenderCtx<'_, '_>,
    tree: &mut TaffyTree<LayoutPath>,
    nodes: &mut Vec<LayoutNode>,
) -> Vec<NodeId> {
    let mut children = Vec::new();
    let mut index = 0usize;
    view.__visit_layout_children(ctx, &mut |child, child_ctx| {
        let mut child_path = path.0.clone();
        child_path.push(index);
        children.extend(build_view(
            child.as_view(),
            &LayoutPath(child_path),
            child_ctx,
            tree,
            nodes,
        ));
        index = index.saturating_add(1);
    });
    children
}

/// Builds children under inherited style and selector ancestry.
///
/// # Arguments
///
/// * `view` — Styleable parent whose children are visited.
/// * `path` — Logical path to the parent.
/// * `style` — Resolved parent style inherited by descendants.
/// * `ctx` — Render context used to create the descendant scope.
/// * `tree` — Transient Taffy tree receiving child boxes.
/// * `nodes` — Mapping records retained for geometry assignment.
///
/// # Returns
///
/// A [`Vec`] containing the visible child node identifiers.
fn build_children_with_style(
    view: &dyn View,
    path: &LayoutPath,
    style: TuiStyle,
    ctx: &mut RenderCtx<'_, '_>,
    tree: &mut TaffyTree<LayoutPath>,
    nodes: &mut Vec<LayoutNode>,
) -> Vec<NodeId> {
    let metadata = view
        .style_metadata()
        .expect("styled layout node should retain metadata")
        .clone();
    let mut children = Vec::new();
    let area = ctx.area();
    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        style.inherited_values(),
        metadata,
        |child_ctx| {
            children = build_children(view, path, child_ctx, tree, nodes);
        },
    );
    children
}

/// Marks a hidden subtree without asking any leaf to measure.
///
/// # Arguments
///
/// * `view` — Root of the excluded subtree.
/// * `ctx` — Render context used to traverse structural boundaries.
fn mark_hidden(view: &dyn View, ctx: &mut RenderCtx<'_, '_>) {
    if let Some(metadata) = view.style_metadata() {
        metadata.set_layout_hidden();
    }
    view.__visit_layout_children(ctx, &mut |child, child_ctx| {
        mark_hidden(child.as_view(), child_ctx);
    });
}
