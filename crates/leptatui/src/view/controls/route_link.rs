//! Focusable internal router link.

use leptos::prelude::GetUntracked;

use crate::{
    AppControl, LayoutSize, Navigate, NavigateOptions, RichText,
    component::{FocusedControl, RenderCtx},
    route::{Location, use_location, use_navigate},
    view::{StyleMetadata, View, ViewType},
};

use crate::view::core::{
    capabilities::impl_styled_view,
    measurement::{AvailableSpace, measure_rich_text},
    render::{resolve_style, semantic_paragraph},
};
use crate::view::link::{mark_route_visited, sync_route_visited};

/// Focusable link that updates the nearest router.
pub struct RouteLinkView {
    /// Rich label displayed by the link.
    label: RichText,
    /// URL-like internal destination.
    href: String,
    /// Whether active matching requires pathname equality.
    exact: bool,
    /// Reactive router location.
    location: Location,
    /// Programmatic navigation callback.
    navigate: Navigate,
    /// Selector, focus, and hit-test metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a focusable internal route link.
///
/// # Arguments
///
/// * `label` — Rich text displayed by the link.
/// * `href` — Absolute, relative, or query-only router destination.
/// * `exact` — Whether active matching requires pathname equality.
///
/// # Returns
///
/// A [`RouteLinkView`] connected to the nearest router.
///
/// # Panics
///
/// Panics if no [`crate::Router`] exists in context.
pub fn route_link(
    label: impl Into<RichText>,
    href: impl Into<String>,
    exact: bool,
) -> RouteLinkView {
    RouteLinkView {
        label: label.into(),
        href: href.into(),
        exact,
        location: use_location(),
        navigate: use_navigate(),
        metadata: StyleMetadata::new(ViewType::A),
    }
}

impl RouteLinkView {
    /// Returns whether the destination matches the current pathname.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether the link is active.
    pub fn is_active(&self) -> bool {
        let current = self.location.pathname().get_untracked();
        let target = normalize_target_path(&self.href, &current);
        current == target
            || (!self.exact
                && target != "/"
                && current
                    .strip_prefix(&target)
                    .is_some_and(|suffix| suffix.starts_with('/')))
    }

    /// Returns metadata with the computed active pseudo-class state.
    ///
    /// # Returns
    ///
    /// A [`StyleMetadata`] clone used for style resolution.
    fn resolved_metadata(&self) -> StyleMetadata {
        sync_route_visited(&self.metadata, &self.normalized_target_location());
        let mut metadata = self.metadata.clone();
        metadata.set_active(self.is_active());
        metadata
    }

    /// Returns the normalized router destination used for visited tracking.
    ///
    /// # Returns
    ///
    /// A [`String`] containing the resolved pathname and optional query.
    fn normalized_target_location(&self) -> String {
        let current = self.location.pathname().get_untracked();
        normalize_target_location(&self.href, &current)
    }
}

impl View for RouteLinkView {
    /// Renders the internal link label.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Render context containing layout and styles.
    ///
    /// # Returns
    ///
    /// An empty [`crate::Result`] on success.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> crate::Result<()> {
        let metadata = self.resolved_metadata();
        let style = resolve_style(&metadata, ctx);
        if let Some(geometry) = ctx.active_layout_geometry(&self.metadata) {
            ctx.with_area(geometry.border_box, |ctx| {
                ctx.render_widget(style.to_block());
            });
            ctx.with_area(geometry.content_box, |ctx| {
                ctx.render_widget(semantic_paragraph(self.label.text(), style));
            });
        } else {
            ctx.render_widget(semantic_paragraph(self.label.text(), style));
        }
        ctx.record_metadata_hit_area(&self.metadata);
        self.metadata.clear_scroll_into_view_request();
        Ok(())
    }

    /// Measures the route link label.
    ///
    /// # Arguments
    ///
    /// * `known_dimensions` — Exact dimensions supplied by the parent.
    /// * `available_space` — Remaining layout space.
    /// * `ctx` — Render context containing styles.
    ///
    /// # Returns
    ///
    /// A [`LayoutSize`] containing the measured label size.
    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        let metadata = self.resolved_metadata();
        let style = resolve_style(&metadata, ctx);
        measure_rich_text(self.label.text(), style, known_dimensions, available_space)
    }

    /// Returns selector metadata.
    ///
    /// # Returns
    ///
    /// An optional shared [`StyleMetadata`] reference.
    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }

    /// Returns mutable selector metadata.
    ///
    /// # Returns
    ///
    /// An optional mutable [`StyleMetadata`] reference.
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }

    /// Returns this link for type erasure.
    ///
    /// # Returns
    ///
    /// A shared [`std::any::Any`] reference.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Returns this link mutably for type erasure.
    ///
    /// # Returns
    ///
    /// A mutable [`std::any::Any`] reference.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Returns whether focus state can reconcile from a previous link.
    fn can_reconcile_from(&self, previous: &dyn View) -> bool {
        previous
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|previous| self.href == previous.href && self.exact == previous.exact)
    }

    /// Returns one focusable control.
    fn __focusable_count(&self) -> usize {
        1
    }

    /// Returns the focused flattened index.
    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        let current = *index;
        *index = index.saturating_add(1);
        self.metadata.is_focused().then_some(current)
    }

    /// Updates focus using a flattened index.
    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        let focused = *index == target;
        self.metadata.set_focused(focused);
        if focused {
            self.metadata.request_scroll_into_view();
        } else {
            self.metadata.clear_scroll_into_view_request();
        }
        *index = index.saturating_add(1);
    }

    /// Hit-tests this link in flattened focus order.
    fn __focusable_index_at_position_inner(
        &self,
        column: u16,
        row: u16,
        index: &mut usize,
    ) -> Option<(usize, u64)> {
        let current = *index;
        *index = index.saturating_add(1);
        self.metadata
            .contains_hit_position(column, row)
            .then(|| self.metadata.paint_order().map(|order| (current, order)))?
    }

    /// Returns the focused link's scroll span.
    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        (self.metadata.is_focused() && self.metadata.scroll_into_view_requested())
            .then_some((0, u32::from(ctx.area().height)))
    }

    /// Navigates when this focused link is activated.
    fn __activate_focused_button(&self) -> crate::Result<Option<AppControl>> {
        if self.metadata.is_focused() {
            let target = self.normalized_target_location();
            (self.navigate)(&self.href, NavigateOptions::default());
            mark_route_visited(&self.metadata, &target);
            return Ok(Some(AppControl::Continue));
        }
        Ok(None)
    }

    /// Returns focused-control semantics for keyboard handling.
    fn __focused_control(&self) -> Option<FocusedControl> {
        self.metadata.is_focused().then_some(FocusedControl::Link)
    }
}

impl_styled_view!(RouteLinkView);

impl crate::view::TextualView for RouteLinkView {
    /// Returns the visible rich-text label.
    ///
    /// # Returns
    ///
    /// A shared Ratatui text reference.
    fn content(&self) -> &ratatui::text::Text<'static> {
        self.label.text()
    }
}

/// Resolves an anchor target for active matching.
///
/// # Arguments
///
/// * `href` — Anchor destination.
/// * `current` — Current pathname.
///
/// # Returns
///
/// A normalized absolute pathname.
fn normalize_target_path(href: &str, current: &str) -> String {
    let path = href.split(['?', '#']).next().unwrap_or_default();
    if path.is_empty() {
        return current.to_owned();
    }
    let absolute = if path.starts_with('/') {
        path.to_owned()
    } else {
        let base = current.rsplit_once('/').map_or("/", |(base, _)| base);
        format!("{base}/{path}")
    };
    let mut segments = Vec::new();
    for segment in absolute.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            segment => segments.push(segment),
        }
    }
    if segments.is_empty() {
        String::from("/")
    } else {
        format!("/{}", segments.join("/"))
    }
}

/// Resolves an anchor target into its visited-registry identity.
///
/// Fragments are excluded because the router does not retain them. Query text
/// remains part of the identity so anchors for distinct routed states do not
/// share visited status.
///
/// # Arguments
///
/// * `href` — Anchor destination.
/// * `current` — Current pathname used to resolve relative destinations.
///
/// # Returns
///
/// A [`String`] containing a normalized pathname and optional query.
fn normalize_target_location(href: &str, current: &str) -> String {
    let target = href.split('#').next().unwrap_or_default();
    let search = target.split_once('?').map_or("", |(_, search)| search);
    let pathname = normalize_target_path(target, current);
    if search.is_empty() {
        pathname
    } else {
        format!("{pathname}?{search}")
    }
}

#[cfg(test)]
/// Unit tests for route-anchor target normalization.
mod tests {
    use super::normalize_target_location;

    /// Verifies visited route keys match the router's path and query behavior.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// current: /guides/start
    /// targets: ../api?mode=full#methods, ?mode=compact, #details
    /// ```
    ///
    /// # Assertions
    ///
    /// - Relative path segments resolve against the current route.
    /// - Query strings remain part of the visited identity.
    /// - Fragments are excluded from the visited identity.
    /// - Query-only and fragment-only targets resolve against the current path.
    #[test]
    fn visited_target_normalization_matches_router_locations() {
        assert_eq!(
            normalize_target_location("../api?mode=full#methods", "/guides/start"),
            "/api?mode=full"
        );
        assert_eq!(
            normalize_target_location("?mode=compact", "/guides/start"),
            "/guides/start?mode=compact"
        );
        assert_eq!(
            normalize_target_location("#details", "/guides/start"),
            "/guides/start"
        );
    }
}
