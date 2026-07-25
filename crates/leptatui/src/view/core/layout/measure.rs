//! Taffy-to-view measurement adaptation and logical-path traversal.

use taffy::style::AvailableSpace as TaffyAvailableSpace;

use crate::{
    AnyView, AvailableSpace, Axes, Borders, LayoutSize, Overflow, RenderCtx, TuiStyle, View,
    view::{
        BlockView, ButtonView, CodeBlockView, InputView, TextAreaView,
        core::measurement::sanitize_cells,
    },
};

use super::style::default_borders;

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
pub(super) fn measure_at_path(
    view: &dyn View,
    path: &[usize],
    known: LayoutSize<Option<f32>>,
    available: LayoutSize<AvailableSpace>,
    ctx: &mut RenderCtx<'_, '_>,
) -> LayoutSize<f32> {
    if path.is_empty() {
        let (known, available) = box_inclusive_measurement_constraints(view, known, available, ctx);
        let measured = view.measure(known, available, ctx);
        return measured_content_size(view, measured, ctx);
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

/// Converts Taffy content-box constraints for a chrome-inclusive built-in.
///
/// # Arguments
///
/// * `view` — View that will receive the intrinsic measurement request.
/// * `known` — Exact content-box dimensions supplied by Taffy.
/// * `available` — Soft content-box constraints supplied by Taffy.
/// * `ctx` — Render context used to resolve the view's effective chrome.
///
/// # Returns
///
/// A tuple containing border-box dimensions expected by the built-in view.
fn box_inclusive_measurement_constraints(
    view: &dyn View,
    known: LayoutSize<Option<f32>>,
    available: LayoutSize<AvailableSpace>,
    ctx: &RenderCtx<'_, '_>,
) -> (LayoutSize<Option<f32>>, LayoutSize<AvailableSpace>) {
    let Some(chrome) = measured_box_chrome(view, ctx) else {
        return (known, available);
    };
    let add_available = |constraint, inset| match constraint {
        AvailableSpace::Definite(value) => AvailableSpace::Definite(sanitize_cells(value + inset)),
        intrinsic => intrinsic,
    };
    (
        LayoutSize::new(
            known
                .width
                .map(|width| sanitize_cells(width + chrome.width)),
            known
                .height
                .map(|height| sanitize_cells(height + chrome.height)),
        ),
        LayoutSize::new(
            add_available(available.width, chrome.width),
            add_available(available.height, chrome.height),
        ),
    )
}

/// Converts a chrome-inclusive built-in measurement into content-box size.
///
/// # Arguments
///
/// * `view` — View that produced the intrinsic measurement.
/// * `measured` — Intrinsic border-box size returned by the view.
/// * `ctx` — Render context used to resolve the view's effective chrome.
///
/// # Returns
///
/// A [`LayoutSize`] containing the content size expected by Taffy.
fn measured_content_size(
    view: &dyn View,
    measured: LayoutSize<f32>,
    ctx: &RenderCtx<'_, '_>,
) -> LayoutSize<f32> {
    let Some(chrome) = measured_box_chrome(view, ctx) else {
        return measured;
    };

    LayoutSize::new(
        sanitize_cells(measured.width - chrome.width),
        sanitize_cells(measured.height - chrome.height),
    )
}

/// Returns whether a built-in view includes chrome in intrinsic measurements.
///
/// # Arguments
///
/// * `view` — View whose measurement behavior is inspected.
///
/// # Returns
///
/// `true` when the view reports a border-box intrinsic size.
fn measures_own_box(view: &dyn View) -> bool {
    view.as_any().is::<BlockView>()
        || view.as_any().is::<ButtonView>()
        || view.as_any().is::<CodeBlockView>()
        || view.as_any().is::<InputView>()
        || view.as_any().is::<TextAreaView>()
}

/// Returns the resolved border and padding consumed by one built-in view.
///
/// # Arguments
///
/// * `view` — View whose measurement convention and chrome are inspected.
/// * `ctx` — Render context used to resolve the view's effective chrome.
///
/// # Returns
///
/// An optional [`LayoutSize`] containing horizontal and vertical chrome.
fn measured_box_chrome(view: &dyn View, ctx: &RenderCtx<'_, '_>) -> Option<LayoutSize<f32>> {
    if !measures_own_box(view) {
        return None;
    }
    let style = view
        .style_metadata()
        .map_or_else(TuiStyle::new, |metadata| ctx.resolve_style(metadata));
    let borders = style.borders.unwrap_or_else(|| default_borders(view));
    let padding = style.padding.unwrap_or_default();
    let horizontal = u16::from(borders.contains(Borders::LEFT))
        .saturating_add(u16::from(borders.contains(Borders::RIGHT)))
        .saturating_add(padding.left)
        .saturating_add(padding.right);
    let vertical = u16::from(borders.contains(Borders::TOP))
        .saturating_add(u16::from(borders.contains(Borders::BOTTOM)))
        .saturating_add(padding.top)
        .saturating_add(padding.bottom);

    Some(LayoutSize::new(f32::from(horizontal), f32::from(vertical)))
}

/// Returns whether the view at one logical path exposes layout children.
///
/// # Arguments
///
/// * `view` — Current traversal root.
/// * `path` — Remaining child indexes leading to the target view.
/// * `ctx` — Render context used to reproduce structural scopes.
///
/// # Returns
///
/// `true` when the addressed view exposes at least one computed layout child.
pub(super) fn has_layout_children_at_path(
    view: &dyn View,
    path: &[usize],
    ctx: &mut RenderCtx<'_, '_>,
) -> bool {
    if path.is_empty() {
        let mut has_children = false;
        visit_children_with_style(view, ctx, &mut |_child, _child_ctx| {
            has_children = true;
        });
        return has_children;
    }

    let target = path[0];
    let mut index = 0usize;
    let mut has_layout_children = false;
    visit_children_with_style(view, ctx, &mut |child, child_ctx| {
        if index == target {
            has_layout_children =
                has_layout_children_at_path(child.as_view(), &path[1..], child_ctx);
        }
        index = index.saturating_add(1);
    });
    has_layout_children
}

/// Returns the resolved overflow axes for one logical path.
///
/// # Arguments
///
/// * `view` — Current traversal root.
/// * `path` — Remaining child indexes leading to the target view.
/// * `ctx` — Render context used to reproduce structural scopes.
///
/// # Returns
///
/// An optional [`Axes`] value authored for the addressed view.
pub(super) fn overflow_at_path(
    view: &dyn View,
    path: &[usize],
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<Axes<Overflow>> {
    if path.is_empty() {
        return view.style_metadata().map(|metadata| {
            ctx.resolve_style(metadata)
                .overflow
                .unwrap_or_else(|| Axes::new(Overflow::Visible, Overflow::Auto))
        });
    }

    let target = path[0];
    let mut index = 0usize;
    let mut overflow = None;
    visit_children_with_style(view, ctx, &mut |child, child_ctx| {
        if index == target {
            overflow = overflow_at_path(child.as_view(), &path[1..], child_ctx);
        }
        index = index.saturating_add(1);
    });
    overflow
}

/// Visits logical children with the same inherited style used during building.
///
/// # Arguments
///
/// * `view` — Parent whose logical children are visited.
/// * `ctx` — Render context active at the parent.
/// * `visitor` — Callback invoked for each logical child and its scoped context.
pub(super) fn visit_children_with_style(
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

/// Converts Taffy's measurement constraint into the public view contract.
///
/// # Arguments
///
/// * `value` — Taffy available-space constraint to convert.
///
/// # Returns
///
/// An [`AvailableSpace`] with matching definite or intrinsic behavior.
pub(super) fn from_taffy_available(value: TaffyAvailableSpace) -> AvailableSpace {
    match value {
        TaffyAvailableSpace::Definite(value) => AvailableSpace::Definite(value),
        TaffyAvailableSpace::MinContent => AvailableSpace::MinContent,
        TaffyAvailableSpace::MaxContent => AvailableSpace::MaxContent,
    }
}
