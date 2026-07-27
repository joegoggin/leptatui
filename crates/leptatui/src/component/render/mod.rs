//! Frame rendering context model.
//!
//! This module wraps a Ratatui frame with the currently assigned render area,
//! scoped stylesheets, inherited style values, selector ancestor metadata, and
//! helper methods for drawing widgets or child views.
//!
//! # Modules
//!
//! - [`clip`] — Offscreen clipped view rendering and cursor remapping.
//! - [`hit`] — Hit-test coordinate clipping and translation.
//! - [`image`] — Image rendering and fallback methods on [`RenderCtx`].
//! - [`layout`] — Transient computed-layout state carried by [`RenderCtx`].
//! - [`target`] — Frame and buffer render targets.

mod clip;
mod hit;
mod image;
mod layout;
mod target;

use std::{cell::Cell, rc::Rc};

use leptos::prelude::{GetUntracked, ReadSignal};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    widgets::{StatefulWidget, Widget},
};

use crate::{
    LayoutGeometry, StyleMetadata, ThemeVariables,
    app::Result,
    context,
    style::{Stylesheet, TuiStyle, ViewportSize},
    terminal_image::TerminalImageSupport,
    view::AnyView,
};

use self::{hit::HitMapper, layout::LayoutState, target::RenderTarget};

pub(crate) use self::layout::LayoutPhase;

/// Rendering context for a single frame and target area.
pub struct RenderCtx<'frame, 'buffer> {
    /// Destination receiving rendered widgets.
    target: RenderTarget<'frame, 'buffer>,
    /// Area inside the frame currently targeted by rendering calls.
    area: Rect,
    /// Active rounded layout snapshot for the rendered view.
    geometry: LayoutGeometry,
    /// Metadata identity that owns the active retained geometry.
    geometry_owner: Option<*const StyleMetadata>,
    /// Root terminal viewport size for responsive style resolution.
    viewport_size: ViewportSize,
    /// Scoped stylesheets used to resolve view styles during rendering.
    stylesheets: Vec<Stylesheet>,
    /// Inherited style declarations available to the current view.
    inherited_style: TuiStyle,
    /// Ancestor metadata used by descendant selector resolution.
    selector_ancestors: Vec<StyleMetadata>,
    /// Terminal image support detected for this render pass.
    terminal_images: TerminalImageSupport,
    /// Mapping from local render coordinates to terminal hit-test coordinates.
    hit_mapper: HitMapper,
    /// Shared counter assigning global back-to-front paint ordinals.
    paint_sequence: Rc<Cell<u64>>,
    /// Transient computed-layout state inherited by child contexts.
    layout_state: LayoutState,
    /// Nearest scrollport constraining sticky descendants.
    sticky_scrollport: Option<StickyScrollport>,
    /// Whether nested positioned boxes are promoted to an ancestor paint context.
    defer_positioned_descendants: bool,
    /// Remaining child indexes leading to one promoted positioned box.
    stacking_path: Option<Vec<usize>>,
    /// Whether the promoted endpoint must continue deferring positioned descendants.
    stacking_endpoint_defers: bool,
}

/// Signed scrollport geometry inherited by sticky descendants.
#[derive(Clone, Copy)]
pub(crate) struct StickyScrollport {
    /// Horizontal start coordinate in the current render target.
    pub(crate) x: i32,
    /// Vertical start coordinate in the current render target.
    pub(crate) y: i32,
    /// Horizontal scrollport size.
    pub(crate) width: u16,
    /// Vertical scrollport size.
    pub(crate) height: u16,
}

impl StickyScrollport {
    /// Returns this scrollport translated into a child target.
    ///
    /// # Arguments
    ///
    /// * `offset` — Signed horizontal and vertical target translation.
    ///
    /// # Returns
    ///
    /// A [`StickyScrollport`] with its origin translated by `offset`.
    pub(crate) fn translated(self, offset: (i32, i32)) -> Self {
        Self {
            x: self.x.saturating_add(offset.0),
            y: self.y.saturating_add(offset.1),
            ..self
        }
    }
}

impl From<Rect> for StickyScrollport {
    /// Creates signed sticky constraint geometry from a terminal rectangle.
    ///
    /// # Arguments
    ///
    /// * `value` — Terminal rectangle supplying the origin and dimensions.
    ///
    /// # Returns
    ///
    /// A [`StickyScrollport`] containing the rectangle geometry.
    fn from(value: Rect) -> Self {
        Self {
            x: i32::from(value.x),
            y: i32::from(value.y),
            width: value.width,
            height: value.height,
        }
    }
}

impl<'frame, 'buffer> RenderCtx<'frame, 'buffer> {
    /// Creates a render context that targets the full frame area.
    ///
    /// # Arguments
    ///
    /// * `frame` — Ratatui frame for the current draw pass.
    ///
    /// # Returns
    ///
    /// A [`RenderCtx`] covering the full frame.
    pub fn new(frame: &'frame mut Frame<'buffer>) -> Self {
        let area = frame.area();
        Self {
            target: RenderTarget::Frame(frame),
            area,
            geometry: geometry_for_area(area),
            geometry_owner: None,
            viewport_size: area.into(),
            stylesheets: Vec::new(),
            inherited_style: TuiStyle::new(),
            selector_ancestors: Vec::new(),
            terminal_images: context::use_context::<TerminalImageSupport>().unwrap_or_default(),
            hit_mapper: HitMapper::identity(),
            paint_sequence: Rc::new(Cell::new(0)),
            layout_state: LayoutState::default(),
            sticky_scrollport: Some(area.into()),
            defer_positioned_descendants: false,
            stacking_path: None,
            stacking_endpoint_defers: false,
        }
    }

    /// Returns the target area for this render context.
    ///
    /// # Returns
    ///
    /// A [`Rect`] describing the current rendering area.
    pub const fn area(&self) -> Rect {
        self.area
    }

    /// Returns the active rounded geometry for the rendered view.
    ///
    /// # Returns
    ///
    /// A [`LayoutGeometry`] containing border, padding, content, viewport, and
    /// accumulated clip rectangles in current target coordinates.
    pub const fn layout_geometry(&self) -> LayoutGeometry {
        self.geometry
    }

    /// Returns active geometry when the view participated in retained layout.
    ///
    /// Composite widgets can assign internal child areas that intentionally
    /// derive local chrome from [`area`](Self::area).
    ///
    /// # Arguments
    ///
    /// * `metadata` — Metadata for the view querying its active geometry.
    ///
    /// # Returns
    ///
    /// An optional [`LayoutGeometry`] in current target coordinates.
    pub(crate) fn active_layout_geometry(
        &self,
        metadata: &StyleMetadata,
    ) -> Option<LayoutGeometry> {
        self.geometry_owner
            .is_some_and(|owner| std::ptr::eq(owner, std::ptr::from_ref(metadata)))
            .then_some(self.geometry)
    }

    /// Returns the root terminal viewport size for this render pass.
    ///
    /// # Returns
    ///
    /// A [`ViewportSize`] measured in terminal cells.
    pub const fn viewport_size(&self) -> ViewportSize {
        self.viewport_size
    }

    /// Resolves authored and inherited styles for view metadata.
    ///
    /// Custom [`crate::View`] implementations can use this method before
    /// rendering a Ratatui widget so application stylesheets, inline styles,
    /// selector ancestry, viewport queries, and theme variables behave like
    /// they do for built-in views.
    ///
    /// # Arguments
    ///
    /// * `metadata` — Selector and inline-style metadata for the view.
    ///
    /// # Returns
    ///
    /// A resolved [`TuiStyle`] for the current rendering context.
    pub fn resolve_style(&self, metadata: &StyleMetadata) -> TuiStyle {
        let theme = context::use_context::<ThemeVariables>()
            .or_else(|| {
                context::use_context::<ReadSignal<ThemeVariables>>()
                    .map(|theme| theme.get_untracked())
            })
            .unwrap_or_default();

        Stylesheet::resolve_stylesheets(
            &self.stylesheets,
            metadata,
            &self.selector_ancestors,
            self.inherited_style.clone(),
            metadata.inline_style(),
            Some(self.viewport_size),
            &theme,
        )
    }

    /// Renders with an additional component-scoped stylesheet.
    #[doc(hidden)]
    pub fn __with_stylesheet<R>(
        &mut self,
        stylesheet: &Stylesheet,
        render: impl FnOnce(&mut RenderCtx<'_, 'buffer>) -> R,
    ) -> R {
        let mut stylesheets = self.stylesheets.clone();
        stylesheets.push(stylesheet.clone());
        let area = self.area;
        let inherited_style = self.inherited_style.clone();
        let selector_ancestors = self.selector_ancestors.clone();

        let mut child = self.child_context(area, inherited_style, stylesheets, selector_ancestors);

        render(&mut child)
    }

    /// Creates a child render context that reborrows the frame target.
    ///
    /// # Arguments
    ///
    /// * `area` — Terminal area assigned to the child context.
    /// * `inherited_style` — Style values inherited by child views.
    /// * `stylesheets` — Active stylesheet stack for child resolution.
    /// * `selector_ancestors` — Selector metadata for ancestor matching.
    ///
    /// # Returns
    ///
    /// A [`RenderCtx`] scoped to the child area and style state.
    fn child_context(
        &mut self,
        area: Rect,
        inherited_style: TuiStyle,
        stylesheets: Vec<Stylesheet>,
        selector_ancestors: Vec<StyleMetadata>,
    ) -> RenderCtx<'_, 'buffer> {
        RenderCtx {
            target: self.target.reborrow(),
            area,
            geometry: self.geometry,
            geometry_owner: self.geometry_owner,
            viewport_size: self.viewport_size,
            stylesheets,
            inherited_style,
            selector_ancestors,
            terminal_images: self.terminal_images.clone(),
            hit_mapper: self.hit_mapper.clone(),
            paint_sequence: Rc::clone(&self.paint_sequence),
            layout_state: self.layout_state.clone(),
            sticky_scrollport: self.sticky_scrollport,
            defer_positioned_descendants: self.defer_positioned_descendants,
            stacking_path: self.stacking_path.clone(),
            stacking_endpoint_defers: self.stacking_endpoint_defers,
        }
    }

    /// Returns the nearest scrollport constraining sticky descendants.
    ///
    /// # Returns
    ///
    /// An optional [`StickyScrollport`] in current target coordinates.
    pub(crate) const fn sticky_scrollport(&self) -> Option<StickyScrollport> {
        self.sticky_scrollport
    }

    /// Renders with a replacement nearest sticky scrollport.
    ///
    /// # Arguments
    ///
    /// * `scrollport` — Scrollport inherited by nested child contexts.
    /// * `render` — Closure rendered with the replacement scrollport.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `render`.
    pub(crate) fn with_sticky_scrollport<R>(
        &mut self,
        scrollport: Option<StickyScrollport>,
        render: impl FnOnce(&mut RenderCtx<'_, 'buffer>) -> R,
    ) -> R {
        let previous = self.sticky_scrollport;
        self.sticky_scrollport = scrollport;
        let result = render(self);
        self.sticky_scrollport = previous;
        result
    }

    /// Returns whether the current box defers nested positioned descendants.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether positioned children belong to an ancestor
    /// paint context.
    pub(crate) const fn defers_positioned_descendants(&self) -> bool {
        self.defer_positioned_descendants
    }

    /// Returns whether rendering is traversing to a promoted positioned box.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether intermediate box chrome should be skipped.
    pub(crate) const fn is_stacking_path_traversal(&self) -> bool {
        self.stacking_path.is_some()
    }

    /// Returns the next child index on the promoted stacking path.
    ///
    /// # Returns
    ///
    /// An optional tuple containing the next child index, remaining path, and
    /// endpoint deferral behavior.
    pub(crate) fn next_stacking_target(&self) -> Option<(usize, Vec<usize>, bool)> {
        let path = self.stacking_path.as_deref()?;
        let (&target, remaining) = path.split_first()?;
        Some((target, remaining.to_vec(), self.stacking_endpoint_defers))
    }

    /// Renders with replacement positioned-descendant and path state.
    ///
    /// # Arguments
    ///
    /// * `defer_positioned_descendants` — Whether nested positioned boxes are
    ///   deferred to an ancestor context.
    /// * `stacking_path` — Remaining child indexes leading to a promoted box.
    /// * `endpoint_defers` — Whether the promoted endpoint remains a
    ///   non-context box.
    /// * `render` — Closure rendered with the replacement paint state.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `render`.
    pub(crate) fn with_stacking_state<R>(
        &mut self,
        defer_positioned_descendants: bool,
        stacking_path: Option<Vec<usize>>,
        endpoint_defers: bool,
        render: impl FnOnce(&mut RenderCtx<'_, 'buffer>) -> R,
    ) -> R {
        let previous_defer = self.defer_positioned_descendants;
        let previous_path = self.stacking_path.clone();
        let previous_endpoint_defers = self.stacking_endpoint_defers;
        self.defer_positioned_descendants = defer_positioned_descendants;
        self.stacking_path = stacking_path;
        self.stacking_endpoint_defers = endpoint_defers;
        let result = render(self);
        self.defer_positioned_descendants = previous_defer;
        self.stacking_path = previous_path;
        self.stacking_endpoint_defers = previous_endpoint_defers;
        result
    }

    /// Returns the style declarations inherited by the current view.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing inherited style values for the current area.
    pub(crate) fn inherited_style(&self) -> TuiStyle {
        self.inherited_style.clone()
    }

    /// Renders a Ratatui widget into the current target area.
    ///
    /// # Arguments
    ///
    /// * `widget` — Ratatui widget to render.
    pub fn render_widget<W>(&mut self, widget: W)
    where
        W: Widget,
    {
        self.target.render_widget(widget, self.area);
    }

    /// Records the current render area for later mouse hit testing.
    ///
    /// # Arguments
    ///
    /// * `metadata` — View metadata that receives the mapped hit area.
    pub(crate) fn record_metadata_hit_area(&self, metadata: &StyleMetadata) {
        let area = self.map_hit_area(self.geometry.border_box);
        metadata.set_hit_area(area);
        if area.is_some() {
            metadata.set_paint_order(self.next_paint_order());
        }
    }

    /// Appends one mapped hit area and records its global paint ordinal.
    ///
    /// # Arguments
    ///
    /// * `metadata` — View metadata receiving the hit area and paint ordinal.
    /// * `area` — Rectangle expressed in current local render coordinates.
    pub(crate) fn push_metadata_hit_area(&self, metadata: &StyleMetadata, area: Rect) {
        if let Some(area) = self.map_hit_area(area) {
            metadata.push_hit_area(area);
            metadata.set_paint_order(self.next_paint_order());
        }
    }

    /// Maps a local render rectangle into terminal hit-test coordinates.
    ///
    /// # Arguments
    ///
    /// * `area` — Rectangle expressed in the current local render coordinates.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing a clipped terminal rectangle, or [`None`] when
    /// the area is empty, outside the clip, or cannot be represented.
    pub(crate) fn map_hit_area(&self, area: Rect) -> Option<Rect> {
        self.hit_mapper.map(area.intersection(self.geometry.clip))
    }

    /// Returns the next global paint ordinal for the current root render.
    ///
    /// # Returns
    ///
    /// A `u64` ordered after every previously recorded visible paint.
    fn next_paint_order(&self) -> u64 {
        let next = self.paint_sequence.get().saturating_add(1);
        self.paint_sequence.set(next);
        next
    }

    /// Sets the terminal cursor position for this render pass.
    pub(crate) fn set_cursor_position(&mut self, position: Position) {
        if self.geometry.clip.contains(position) {
            self.target.set_cursor_position(position);
        }
    }

    /// Renders a Ratatui stateful widget into the current target area.
    pub(crate) fn render_stateful_widget<W>(&mut self, widget: W, state: &mut W::State)
    where
        W: StatefulWidget,
    {
        self.target.render_stateful_widget(widget, self.area, state);
    }

    /// Renders a Leptatui view as composite content in the current target area.
    ///
    /// The child uses the explicitly assigned area instead of adopting retained
    /// outer layout geometry.
    ///
    /// # Arguments
    ///
    /// * `view` — View tree to render.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if view rendering performs terminal
    /// I/O that fails.
    pub fn render_view(&mut self, view: &AnyView) -> Result<()> {
        let area = self.area;
        let stylesheets = self.stylesheets.clone();
        let selector_ancestors = self.selector_ancestors.clone();
        let mut child = self.child_context(
            area,
            self.inherited_style.clone(),
            stylesheets,
            selector_ancestors,
        );
        child.geometry = geometry_for_area(area);
        child.geometry_owner = None;
        child.layout_state.disable_retained_geometry();
        view.render(&mut child)
    }

    /// Renders into a temporary child area with explicit inherited style.
    ///
    /// Preserves the current selector ancestor path for the child context.
    ///
    /// # Arguments
    ///
    /// * `area` — Child area to use while invoking `render`.
    /// * `inherited_style` — Inherited style declarations for the child area.
    /// * `render` — Closure that renders into the child context.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `render`.
    pub fn with_area_and_inherited_style<R>(
        &mut self,
        area: Rect,
        inherited_style: TuiStyle,
        render: impl FnOnce(&mut RenderCtx<'_, 'buffer>) -> R,
    ) -> R {
        let stylesheets = self.stylesheets.clone();
        let selector_ancestors = self.selector_ancestors.clone();
        let mut child = self.child_context(area, inherited_style, stylesheets, selector_ancestors);

        render(&mut child)
    }

    /// Renders into a child area with inherited style and one added selector ancestor.
    ///
    /// # Arguments
    ///
    /// * `area` — Child area to use while invoking `render`.
    /// * `inherited_style` — Inherited style declarations for the child area.
    /// * `selector_ancestor` — Parent view metadata to append to the selector
    ///   ancestor path.
    /// * `render` — Closure that renders into the child context.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `render`.
    pub fn with_area_inherited_style_and_selector_ancestor<R>(
        &mut self,
        area: Rect,
        inherited_style: TuiStyle,
        selector_ancestor: StyleMetadata,
        render: impl FnOnce(&mut RenderCtx<'_, 'buffer>) -> R,
    ) -> R {
        let mut selector_ancestors = self.selector_ancestors.clone();
        selector_ancestors.push(selector_ancestor);
        let stylesheets = self.stylesheets.clone();

        let mut child = self.child_context(area, inherited_style, stylesheets, selector_ancestors);
        child.geometry = geometry_for_area(area);
        child.geometry_owner = None;
        child.layout_state.disable_retained_geometry();

        render(&mut child)
    }

    /// Renders a view with translated retained geometry in a parent-assigned area.
    ///
    /// # Arguments
    ///
    /// * `geometry` — Active geometry translated into current target coordinates.
    /// * `metadata` — Optional metadata that owns `geometry`.
    /// * `inherited_style` — Inherited style declarations for the child.
    /// * `selector_ancestor` — Parent metadata appended to selector ancestry.
    /// * `render` — Closure that renders into the assigned child context.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `render`.
    pub(crate) fn with_assigned_layout_geometry_and_selector_ancestor<R>(
        &mut self,
        geometry: LayoutGeometry,
        metadata: Option<&StyleMetadata>,
        inherited_style: TuiStyle,
        selector_ancestor: StyleMetadata,
        render: impl FnOnce(&mut RenderCtx<'_, 'buffer>) -> R,
    ) -> R {
        let mut selector_ancestors = self.selector_ancestors.clone();
        selector_ancestors.push(selector_ancestor);
        let stylesheets = self.stylesheets.clone();
        let mut child = self.child_context(
            geometry.border_box,
            inherited_style,
            stylesheets,
            selector_ancestors,
        );
        child.geometry = geometry;
        child.geometry_owner = metadata.map(std::ptr::from_ref);
        child.layout_state.disable_retained_geometry();
        render(&mut child)
    }

    /// Renders a view using its retained layout snapshot.
    ///
    /// # Arguments
    ///
    /// * `geometry` — Rounded geometry assigned to the view.
    /// * `metadata` — Metadata that owns `geometry`.
    /// * `render` — Closure that renders into the geometry-aware context.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `render`.
    pub(crate) fn with_layout_geometry<R>(
        &mut self,
        geometry: LayoutGeometry,
        metadata: &StyleMetadata,
        render: impl FnOnce(&mut RenderCtx<'_, 'buffer>) -> R,
    ) -> R {
        let stylesheets = self.stylesheets.clone();
        let selector_ancestors = self.selector_ancestors.clone();
        let mut child = self.child_context(
            geometry.border_box,
            self.inherited_style.clone(),
            stylesheets,
            selector_ancestors,
        );
        child.geometry = geometry;
        child.geometry_owner = Some(std::ptr::from_ref(metadata));
        render(&mut child)
    }

    /// Renders into a temporary child area.
    ///
    /// # Arguments
    ///
    /// * `area` — Child area to use while invoking `render`.
    /// * `render` — Closure that renders into the child context.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `render`.
    pub fn with_area<R>(
        &mut self,
        area: Rect,
        render: impl FnOnce(&mut RenderCtx<'_, 'buffer>) -> R,
    ) -> R {
        self.with_area_and_inherited_style(area, self.inherited_style.clone(), render)
    }
}

/// Creates identity geometry for a render area without retained box chrome.
///
/// # Arguments
///
/// * `area` — Target area used for every geometry rectangle.
///
/// # Returns
///
/// A [`LayoutGeometry`] whose five rectangles all equal `area`.
const fn geometry_for_area(area: Rect) -> LayoutGeometry {
    LayoutGeometry {
        border_box: area,
        padding_box: area,
        content_box: area,
        viewport: area,
        clip: area,
    }
}
