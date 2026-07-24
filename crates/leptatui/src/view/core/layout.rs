//! Transient Taffy layout-tree construction and rounded terminal geometry.
//!
//! Each root render mirrors visible styleable views into one short-lived
//! engine tree, delegates leaf sizing to [`View::measure`], and stores rounded
//! engine-independent rectangles back on view metadata. Component and dynamic
//! boundaries contribute scopes and children without generating layout boxes.

use std::collections::HashMap;

use ratatui::layout::Rect as TerminalRect;
use taffy::{
    geometry::{Line as TaffyLine, Point as TaffyPoint, Rect as TaffyRect, Size as TaffySize},
    style::{
        AlignContent as TaffyAlignContent, AlignItems as TaffyAlignItems,
        AlignSelf as TaffyAlignSelf, AvailableSpace as TaffyAvailableSpace,
        BoxSizing as TaffyBoxSizing, Dimension as TaffyDimension, Display as TaffyDisplay,
        FlexDirection as TaffyFlexDirection, FlexWrap as TaffyFlexWrap,
        GridAutoFlow as TaffyGridAutoFlow, GridPlacement as TaffyGridPlacement,
        JustifyContent as TaffyJustifyContent, LengthPercentage, LengthPercentageAuto,
        Overflow as TaffyOverflow, Position as TaffyPosition, Style as TaffyStyle,
    },
    tree::{NodeId, TaffyTree},
};

use crate::{
    AlignContent, AlignItems, AlignSelf, AnyView, AvailableSpace, Borders, BoxSizing, Dimension,
    Display, FlexDirection, FlexWrap, GridAutoFlow, GridLine, GridPlacement, JustifyContent,
    JustifyItems, JustifySelf, LayoutGeometry, LayoutSize, Length, LengthAuto, Overflow, Position,
    RenderCtx, TuiStyle, View, ViewportSize,
    component::LayoutPhase,
    view::{
        BlockView, ButtonView, CodeBlockView, InputView, TextAreaView,
        core::measurement::sanitize_cells,
    },
};

/// Logical child indexes from the rendered root to one layout box.
#[derive(Clone, Debug)]
struct LayoutPath(
    /// Ordered logical child indexes.
    Vec<usize>,
);

/// Taffy node associated with one styleable Leptatui view.
#[derive(Clone, Debug)]
struct LayoutNode {
    /// Taffy node receiving computed geometry.
    node: NodeId,
    /// Logical path back to the corresponding Leptatui view.
    path: LayoutPath,
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

    let viewport = ctx.viewport_size();
    let clamp_root_to_viewport = roots.len() == 1;
    let root_node = if clamp_root_to_viewport {
        roots[0]
    } else {
        tree.new_with_children(synthetic_root_style(viewport), &roots)
            .expect("transient layout roots should form a valid Taffy tree")
    };

    ctx.set_layout_phase(LayoutPhase::Measure);
    tree.compute_layout_with_measure(
        root_node,
        TaffySize {
            width: TaffyAvailableSpace::Definite(f32::from(viewport.width)),
            height: TaffyAvailableSpace::Definite(f32::from(viewport.height)),
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

    let root_path = nodes
        .iter()
        .find(|layout_node| layout_node.node == root_node)
        .map(|layout_node| layout_node.path.clone());
    let root_overflows_viewport = tree
        .layout(root_node)
        .is_ok_and(|layout| layout.size.height > f32::from(viewport.height));
    if root_overflows_viewport
        && root_path
            .as_ref()
            .is_some_and(|path| uses_computed_child_layout_at_path(root, &path.0, ctx))
    {
        let mut style = tree
            .style(root_node)
            .expect("computed root style should remain available")
            .clone();
        style.size.height = TaffyDimension::length(f32::from(viewport.height));
        style.overflow.y = TaffyOverflow::Scroll;
        style.scrollbar_width = 1.0;
        tree.set_style(root_node, style)
            .expect("computed root style should remain mutable");
        tree.compute_layout_with_measure(
            root_node,
            TaffySize {
                width: TaffyAvailableSpace::Definite(f32::from(viewport.width)),
                height: TaffyAvailableSpace::Definite(f32::from(viewport.height)),
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
        .expect("scrolling root layout should use valid node identifiers");
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
        clamp_root_to_viewport,
    );

    ctx.set_layout_phase(LayoutPhase::Paint);
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

/// Measures the view addressed by one logical traversal path.
///
/// # Arguments
///
/// * `view` — Current traversal root.
/// * `path` — Remaining child indexes leading to the measured leaf.
/// * `known` — Exact dimensions supplied by Taffy.
/// * `available` — Soft available-space constraints supplied by Taffy.
/// * `ctx` — Render context used to reproduce style and component scopes.
///
/// # Returns
///
/// A [`LayoutSize`] containing the measured terminal-cell dimensions.
fn measure_at_path(
    view: &dyn View,
    path: &[usize],
    known: LayoutSize<Option<f32>>,
    available: LayoutSize<AvailableSpace>,
    ctx: &mut RenderCtx<'_, '_>,
) -> LayoutSize<f32> {
    if path.is_empty() {
        return view.measure(known, available, ctx);
    }

    let target = path[0];
    let mut index = 0usize;
    let mut measured = None;
    visit_children_with_style(view, ctx, &mut |child, child_ctx| {
        if index == target {
            measured = Some(measure_at_path(
                child.as_view(),
                &path[1..],
                known,
                available,
                child_ctx,
            ));
        }
        index = index.saturating_add(1);
    });
    measured.unwrap_or_else(|| LayoutSize::all(0.0))
}

/// Returns whether the view at one logical path computes child layout.
///
/// # Arguments
///
/// * `view` — Current traversal root.
/// * `path` — Remaining child indexes leading to the target view.
/// * `ctx` — Render context used to reproduce structural scopes.
///
/// # Returns
///
/// `true` when the addressed view lays out retained children.
fn uses_computed_child_layout_at_path(
    view: &dyn View,
    path: &[usize],
    ctx: &mut RenderCtx<'_, '_>,
) -> bool {
    if path.is_empty() {
        return view.__uses_computed_child_layout();
    }

    let target = path[0];
    let mut index = 0usize;
    let mut uses_computed_layout = false;
    visit_children_with_style(view, ctx, &mut |child, child_ctx| {
        if index == target {
            uses_computed_layout =
                uses_computed_child_layout_at_path(child.as_view(), &path[1..], child_ctx);
        }
        index = index.saturating_add(1);
    });
    uses_computed_layout
}

/// Visits logical children with the same inherited style used during building.
///
/// # Arguments
///
/// * `view` — Parent whose logical children are visited.
/// * `ctx` — Render context active at the parent.
/// * `visitor` — Callback invoked for each logical child and its scoped context.
fn visit_children_with_style(
    view: &dyn View,
    ctx: &mut RenderCtx<'_, '_>,
    visitor: &mut dyn FnMut(&AnyView, &mut RenderCtx<'_, '_>),
) {
    let Some(metadata) = view.style_metadata() else {
        view.__visit_layout_children(ctx, visitor);
        return;
    };
    let style = ctx.resolve_style(metadata);
    let area = ctx.area();
    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        style.inherited_values(),
        metadata.clone(),
        |child_ctx| view.__visit_layout_children(child_ctx, visitor),
    );
}

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
/// * `clamp_to_viewport` — Whether this node is the sole visible root box.
fn retain_geometry(
    tree: &TaffyTree<LayoutPath>,
    node: NodeId,
    parent_origin: (f32, f32),
    paths: &HashMap<NodeId, LayoutPath>,
    root: &dyn View,
    ctx: &mut RenderCtx<'_, '_>,
    clamp_to_viewport: bool,
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
        if clamp_to_viewport {
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

/// Creates the viewport-sized node used only for multiple transparent roots.
///
/// # Arguments
///
/// * `viewport` — Terminal viewport constraining the synthetic root.
///
/// # Returns
///
/// A [`TaffyStyle`] containing definite viewport dimensions.
fn synthetic_root_style(viewport: ViewportSize) -> TaffyStyle {
    TaffyStyle {
        display: TaffyDisplay::Block,
        size: TaffySize {
            width: TaffyDimension::length(f32::from(viewport.width)),
            height: TaffyDimension::length(f32::from(viewport.height)),
        },
        ..TaffyStyle::default()
    }
}

/// Converts resolved Leptatui values into one Taffy style.
///
/// # Arguments
///
/// * `view` — View supplying widget-specific border defaults.
/// * `style` — Fully resolved Leptatui style.
/// * `viewport` — Terminal viewport used to resolve viewport-relative lengths.
///
/// # Returns
///
/// A [`TaffyStyle`] containing equivalent engine-owned layout values.
fn to_taffy_style(view: &dyn View, style: TuiStyle, viewport: ViewportSize) -> TaffyStyle {
    let display = style.display.unwrap_or(Display::Block);
    let flex_direction = style.flex_direction.unwrap_or_default();
    let borders = style.borders.unwrap_or_else(|| default_borders(view));
    let padding = style.padding.unwrap_or_default();
    let measures_own_box = view.as_any().is::<BlockView>()
        || view.as_any().is::<ButtonView>()
        || view.as_any().is::<CodeBlockView>()
        || view.as_any().is::<InputView>()
        || view.as_any().is::<TextAreaView>();
    let layout_borders = if measures_own_box {
        Borders::NONE
    } else {
        borders
    };
    let layout_padding = if measures_own_box {
        crate::TuiSpacing::ZERO
    } else {
        padding
    };
    let gap = style
        .gap
        .unwrap_or_else(|| crate::Axes::all(Length::Cells(0.0)));

    TaffyStyle {
        display: map_display(display),
        box_sizing: map_box_sizing(style.box_sizing.unwrap_or_default()),
        overflow: TaffyPoint {
            x: map_overflow(style.overflow.unwrap_or_default().x),
            y: map_overflow(style.overflow.unwrap_or_default().y),
        },
        position: map_position(style.position.unwrap_or_default()),
        inset: map_auto_edges(style.inset.unwrap_or_default(), viewport),
        size: map_dimensions(style.size.unwrap_or_default(), viewport),
        min_size: map_dimensions(style.min_size.unwrap_or_default(), viewport),
        max_size: map_dimensions(style.max_size.unwrap_or_default(), viewport),
        margin: map_auto_edges(
            style
                .margin
                .unwrap_or_else(|| crate::Edges::all(LengthAuto::Length(Length::Cells(0.0)))),
            viewport,
        ),
        padding: TaffyRect {
            left: LengthPercentage::length(f32::from(layout_padding.left)),
            right: LengthPercentage::length(f32::from(layout_padding.right)),
            top: LengthPercentage::length(f32::from(layout_padding.top)),
            bottom: LengthPercentage::length(f32::from(layout_padding.bottom)),
        },
        border: border_edges(layout_borders),
        gap: TaffySize {
            width: map_length(gap.x, viewport),
            height: map_length(gap.y, viewport),
        },
        flex_direction: map_flex_direction(flex_direction),
        flex_wrap: map_flex_wrap(style.flex_wrap.unwrap_or_default()),
        flex_basis: map_dimension(style.flex_basis.unwrap_or_default(), viewport),
        flex_grow: sanitize_factor(style.flex_grow.unwrap_or(0.0)),
        flex_shrink: sanitize_factor(style.flex_shrink.unwrap_or(1.0)),
        align_items: style.align_items.map(map_align_items),
        align_self: style.align_self.and_then(map_align_self),
        align_content: style.align_content.map(map_align_content),
        justify_items: style.justify_items.map(map_justify_items),
        justify_self: style.justify_self.and_then(map_justify_self),
        justify_content: style.justify_content.map(map_justify_content),
        grid_auto_flow: map_grid_auto_flow(style.grid_auto_flow.unwrap_or_default()),
        grid_row: map_grid_line(style.grid_row.unwrap_or_default()),
        grid_column: map_grid_line(style.grid_column.unwrap_or_default()),
        ..TaffyStyle::default()
    }
}

/// Converts a Leptatui display value into Taffy's equivalent.
///
/// # Arguments
///
/// * `value` — Public display value to convert.
///
/// # Returns
///
/// A [`TaffyDisplay`] with matching box-generation behavior.
fn map_display(value: Display) -> TaffyDisplay {
    match value {
        Display::Block => TaffyDisplay::Block,
        Display::Flex => TaffyDisplay::Flex,
        Display::Grid => TaffyDisplay::Grid,
        Display::None => TaffyDisplay::None,
    }
}

/// Converts a Leptatui box-sizing value into Taffy's equivalent.
///
/// # Arguments
///
/// * `value` — Public box-sizing value to convert.
///
/// # Returns
///
/// A [`TaffyBoxSizing`] with matching authored-size semantics.
fn map_box_sizing(value: BoxSizing) -> TaffyBoxSizing {
    match value {
        BoxSizing::ContentBox => TaffyBoxSizing::ContentBox,
        BoxSizing::BorderBox => TaffyBoxSizing::BorderBox,
    }
}

/// Converts one overflow axis into Taffy's layout-affecting equivalent.
///
/// # Arguments
///
/// * `value` — Public overflow behavior to convert.
///
/// # Returns
///
/// A [`TaffyOverflow`] containing the currently supported layout behavior.
fn map_overflow(value: Overflow) -> TaffyOverflow {
    match value {
        Overflow::Visible => TaffyOverflow::Visible,
        Overflow::Hidden | Overflow::Auto => TaffyOverflow::Hidden,
        Overflow::Clip => TaffyOverflow::Clip,
        Overflow::Scroll => TaffyOverflow::Scroll,
    }
}

/// Converts positioning into the subset currently represented by Taffy.
///
/// # Arguments
///
/// * `value` — Public positioning mode to convert.
///
/// # Returns
///
/// A [`TaffyPosition`] containing relative or absolute layout behavior.
fn map_position(value: Position) -> TaffyPosition {
    match value {
        Position::Absolute | Position::Fixed => TaffyPosition::Absolute,
        Position::Static | Position::Relative | Position::Sticky => TaffyPosition::Relative,
    }
}

/// Converts width and height dimensions for the current viewport.
///
/// # Arguments
///
/// * `value` — Authored width and height dimensions.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`TaffySize`] containing converted dimensions.
fn map_dimensions(
    value: LayoutSize<Dimension>,
    viewport: ViewportSize,
) -> TaffySize<TaffyDimension> {
    TaffySize {
        width: map_dimension(value.width, viewport),
        height: map_dimension(value.height, viewport),
    }
}

/// Converts one authored dimension for the current viewport.
///
/// # Arguments
///
/// * `value` — Authored dimension to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`TaffyDimension`] containing the supported size behavior.
fn map_dimension(value: Dimension, viewport: ViewportSize) -> TaffyDimension {
    match value {
        Dimension::Auto | Dimension::MinContent | Dimension::MaxContent => TaffyDimension::auto(),
        Dimension::Length(length) | Dimension::FitContent(length) => match length {
            Length::Percent(percent) => TaffyDimension::percent(sanitize_percent(percent)),
            length => TaffyDimension::length(resolve_viewport_length(length, viewport)),
        },
    }
}

/// Converts one definite length for the current viewport.
///
/// # Arguments
///
/// * `value` — Definite length to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`LengthPercentage`] containing cells or a containing-block ratio.
fn map_length(value: Length, viewport: ViewportSize) -> LengthPercentage {
    match value {
        Length::Percent(percent) => LengthPercentage::percent(sanitize_percent(percent)),
        value => LengthPercentage::length(resolve_viewport_length(value, viewport)),
    }
}

/// Converts one automatic or definite length for the current viewport.
///
/// # Arguments
///
/// * `value` — Automatic or definite length to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`LengthPercentageAuto`] containing the converted value.
fn map_auto_length(value: LengthAuto, viewport: ViewportSize) -> LengthPercentageAuto {
    match value {
        LengthAuto::Auto => LengthPercentageAuto::auto(),
        LengthAuto::Length(Length::Percent(percent)) => {
            LengthPercentageAuto::percent(sanitize_percent(percent))
        }
        LengthAuto::Length(length) => {
            LengthPercentageAuto::length(resolve_viewport_length(length, viewport))
        }
    }
}

/// Converts four automatic or definite physical edges.
///
/// # Arguments
///
/// * `value` — Public physical edges to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`TaffyRect`] containing converted inset or margin edges.
fn map_auto_edges(
    value: crate::Edges<LengthAuto>,
    viewport: ViewportSize,
) -> TaffyRect<LengthPercentageAuto> {
    TaffyRect {
        left: map_auto_length(value.left, viewport),
        right: map_auto_length(value.right, viewport),
        top: map_auto_length(value.top, viewport),
        bottom: map_auto_length(value.bottom, viewport),
    }
}

/// Resolves cell and viewport-relative lengths into finite terminal cells.
///
/// # Arguments
///
/// * `value` — Public length to resolve.
/// * `viewport` — Terminal viewport supplying relative axis sizes.
///
/// # Returns
///
/// A finite `f32` terminal-cell length.
fn resolve_viewport_length(value: Length, viewport: ViewportSize) -> f32 {
    let width = f32::from(viewport.width);
    let height = f32::from(viewport.height);
    let resolved = match value {
        Length::Cells(cells) => cells,
        Length::Percent(percent) => percent,
        Length::ViewportWidth(percent) => width * percent / 100.0,
        Length::ViewportHeight(percent) => height * percent / 100.0,
        Length::ViewportMin(percent) => width.min(height) * percent / 100.0,
        Length::ViewportMax(percent) => width.max(height) * percent / 100.0,
    };
    sanitize_cells(resolved)
}

/// Converts a public `0..100` percentage into Taffy's finite ratio.
///
/// # Arguments
///
/// * `value` — Percentage where `100.0` represents the full containing axis.
///
/// # Returns
///
/// A finite non-negative `f32` ratio.
fn sanitize_percent(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0) / 100.0
    } else {
        0.0
    }
}

/// Returns a finite non-negative flex factor.
///
/// # Arguments
///
/// * `value` — Authored growth or shrink factor.
///
/// # Returns
///
/// A finite non-negative `f32` factor.
fn sanitize_factor(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

/// Converts a flex main-axis direction.
///
/// # Arguments
///
/// * `value` — Public flex direction to convert.
///
/// # Returns
///
/// A [`TaffyFlexDirection`] with matching axis and ordering.
fn map_flex_direction(value: FlexDirection) -> TaffyFlexDirection {
    match value {
        FlexDirection::Row => TaffyFlexDirection::Row,
        FlexDirection::RowReverse => TaffyFlexDirection::RowReverse,
        FlexDirection::Column => TaffyFlexDirection::Column,
        FlexDirection::ColumnReverse => TaffyFlexDirection::ColumnReverse,
    }
}

/// Converts a flex wrapping mode.
///
/// # Arguments
///
/// * `value` — Public flex wrapping mode to convert.
///
/// # Returns
///
/// A [`TaffyFlexWrap`] with matching line behavior.
fn map_flex_wrap(value: FlexWrap) -> TaffyFlexWrap {
    match value {
        FlexWrap::NoWrap => TaffyFlexWrap::NoWrap,
        FlexWrap::Wrap => TaffyFlexWrap::Wrap,
        FlexWrap::WrapReverse => TaffyFlexWrap::WrapReverse,
    }
}

/// Converts container cross-axis item alignment.
///
/// # Arguments
///
/// * `value` — Public item alignment to convert.
///
/// # Returns
///
/// A [`TaffyAlignItems`] with matching alignment behavior.
fn map_align_items(value: AlignItems) -> TaffyAlignItems {
    match value {
        AlignItems::Start => TaffyAlignItems::START,
        AlignItems::End => TaffyAlignItems::END,
        AlignItems::FlexStart => TaffyAlignItems::FLEX_START,
        AlignItems::FlexEnd => TaffyAlignItems::FLEX_END,
        AlignItems::Center => TaffyAlignItems::CENTER,
        AlignItems::Baseline => TaffyAlignItems::BASELINE,
        AlignItems::Stretch => TaffyAlignItems::STRETCH,
    }
}

/// Converts item cross-axis alignment, preserving automatic inheritance.
///
/// # Arguments
///
/// * `value` — Public self-alignment to convert.
///
/// # Returns
///
/// An optional [`TaffyAlignSelf`] omitted for automatic inheritance.
fn map_align_self(value: AlignSelf) -> Option<TaffyAlignSelf> {
    match value {
        AlignSelf::Auto => None,
        AlignSelf::Start => Some(TaffyAlignSelf::START),
        AlignSelf::End => Some(TaffyAlignSelf::END),
        AlignSelf::FlexStart => Some(TaffyAlignSelf::FLEX_START),
        AlignSelf::FlexEnd => Some(TaffyAlignSelf::FLEX_END),
        AlignSelf::Center => Some(TaffyAlignSelf::CENTER),
        AlignSelf::Baseline => Some(TaffyAlignSelf::BASELINE),
        AlignSelf::Stretch => Some(TaffyAlignSelf::STRETCH),
    }
}

/// Converts grid inline-axis item alignment.
///
/// # Arguments
///
/// * `value` — Public grid item alignment to convert.
///
/// # Returns
///
/// A [`TaffyAlignItems`] with matching inline-axis behavior.
fn map_justify_items(value: JustifyItems) -> TaffyAlignItems {
    match value {
        JustifyItems::Start => TaffyAlignItems::START,
        JustifyItems::End => TaffyAlignItems::END,
        JustifyItems::Center => TaffyAlignItems::CENTER,
        JustifyItems::Baseline => TaffyAlignItems::BASELINE,
        JustifyItems::Stretch => TaffyAlignItems::STRETCH,
    }
}

/// Converts grid inline-axis self alignment, preserving automatic inheritance.
///
/// # Arguments
///
/// * `value` — Public grid self-alignment to convert.
///
/// # Returns
///
/// An optional [`TaffyAlignSelf`] omitted for automatic inheritance.
fn map_justify_self(value: JustifySelf) -> Option<TaffyAlignSelf> {
    match value {
        JustifySelf::Auto => None,
        JustifySelf::Start => Some(TaffyAlignSelf::START),
        JustifySelf::End => Some(TaffyAlignSelf::END),
        JustifySelf::Center => Some(TaffyAlignSelf::CENTER),
        JustifySelf::Baseline => Some(TaffyAlignSelf::BASELINE),
        JustifySelf::Stretch => Some(TaffyAlignSelf::STRETCH),
    }
}

/// Converts cross-axis content distribution.
///
/// # Arguments
///
/// * `value` — Public content alignment to convert.
///
/// # Returns
///
/// A [`TaffyAlignContent`] with matching distribution behavior.
fn map_align_content(value: AlignContent) -> TaffyAlignContent {
    match value {
        AlignContent::Start => TaffyAlignContent::START,
        AlignContent::End => TaffyAlignContent::END,
        AlignContent::FlexStart => TaffyAlignContent::FLEX_START,
        AlignContent::FlexEnd => TaffyAlignContent::FLEX_END,
        AlignContent::Center => TaffyAlignContent::CENTER,
        AlignContent::Stretch => TaffyAlignContent::STRETCH,
        AlignContent::SpaceBetween => TaffyAlignContent::SPACE_BETWEEN,
        AlignContent::SpaceAround => TaffyAlignContent::SPACE_AROUND,
        AlignContent::SpaceEvenly => TaffyAlignContent::SPACE_EVENLY,
    }
}

/// Converts main-axis or inline-axis content distribution.
///
/// # Arguments
///
/// * `value` — Public content justification to convert.
///
/// # Returns
///
/// A [`TaffyJustifyContent`] with matching distribution behavior.
fn map_justify_content(value: JustifyContent) -> TaffyJustifyContent {
    match value {
        JustifyContent::Start => TaffyJustifyContent::START,
        JustifyContent::End => TaffyJustifyContent::END,
        JustifyContent::FlexStart => TaffyJustifyContent::FLEX_START,
        JustifyContent::FlexEnd => TaffyJustifyContent::FLEX_END,
        JustifyContent::Center => TaffyJustifyContent::CENTER,
        JustifyContent::Stretch => TaffyJustifyContent::STRETCH,
        JustifyContent::SpaceBetween => TaffyJustifyContent::SPACE_BETWEEN,
        JustifyContent::SpaceAround => TaffyJustifyContent::SPACE_AROUND,
        JustifyContent::SpaceEvenly => TaffyJustifyContent::SPACE_EVENLY,
    }
}

/// Converts automatic grid placement flow.
///
/// # Arguments
///
/// * `value` — Public automatic-flow mode to convert.
///
/// # Returns
///
/// A [`TaffyGridAutoFlow`] with matching axis and density.
fn map_grid_auto_flow(value: GridAutoFlow) -> TaffyGridAutoFlow {
    match value {
        GridAutoFlow::Row => TaffyGridAutoFlow::Row,
        GridAutoFlow::Column => TaffyGridAutoFlow::Column,
        GridAutoFlow::RowDense => TaffyGridAutoFlow::RowDense,
        GridAutoFlow::ColumnDense => TaffyGridAutoFlow::ColumnDense,
    }
}

/// Converts both placements for one grid axis.
///
/// # Arguments
///
/// * `value` — Public start and end placements to convert.
///
/// # Returns
///
/// A [`TaffyLine`] containing both converted placements.
fn map_grid_line(value: GridLine) -> TaffyLine<TaffyGridPlacement> {
    TaffyLine {
        start: map_grid_placement(value.start),
        end: map_grid_placement(value.end),
    }
}

/// Converts one grid edge placement.
///
/// # Arguments
///
/// * `value` — Public grid placement to convert.
///
/// # Returns
///
/// A [`TaffyGridPlacement`] containing automatic, line, or span placement.
fn map_grid_placement(value: GridPlacement) -> TaffyGridPlacement {
    match value {
        GridPlacement::Auto => TaffyGridPlacement::Auto,
        GridPlacement::Line(line) => TaffyGridPlacement::Line(line.into()),
        GridPlacement::Span(span) => TaffyGridPlacement::Span(span),
    }
}

/// Returns the widget borders used when no authored value overrides them.
///
/// # Arguments
///
/// * `view` — View whose built-in border behavior is inspected.
///
/// # Returns
///
/// A [`Borders`] bitset containing the widget defaults.
fn default_borders(view: &dyn View) -> Borders {
    if view.as_any().is::<BlockView>()
        || view.as_any().is::<ButtonView>()
        || view.as_any().is::<CodeBlockView>()
        || view.as_any().is::<InputView>()
        || view.as_any().is::<TextAreaView>()
    {
        Borders::ALL
    } else {
        Borders::NONE
    }
}

/// Converts enabled terminal border sides into one-cell Taffy edges.
///
/// # Arguments
///
/// * `borders` — Enabled terminal border sides.
///
/// # Returns
///
/// A [`TaffyRect`] containing zero- or one-cell physical edges.
fn border_edges(borders: Borders) -> TaffyRect<LengthPercentage> {
    TaffyRect {
        left: LengthPercentage::length(f32::from(borders.contains(Borders::LEFT))),
        right: LengthPercentage::length(f32::from(borders.contains(Borders::RIGHT))),
        top: LengthPercentage::length(f32::from(borders.contains(Borders::TOP))),
        bottom: LengthPercentage::length(f32::from(borders.contains(Borders::BOTTOM))),
    }
}

/// Converts Taffy's measurement constraint into the public view contract.
///
/// # Arguments
///
/// * `value` — Taffy available-space constraint to convert.
///
/// # Returns
///
/// An [`AvailableSpace`] with matching definite or intrinsic behavior.
fn from_taffy_available(value: TaffyAvailableSpace) -> AvailableSpace {
    match value {
        TaffyAvailableSpace::Definite(value) => AvailableSpace::Definite(value),
        TaffyAvailableSpace::MinContent => AvailableSpace::MinContent,
        TaffyAvailableSpace::MaxContent => AvailableSpace::MaxContent,
    }
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
