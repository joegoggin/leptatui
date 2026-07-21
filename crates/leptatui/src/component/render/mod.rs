//! Frame rendering context model.
//!
//! This module wraps a Ratatui frame with the currently assigned render area,
//! scoped stylesheets, inherited style values, selector ancestor metadata, and
//! helper methods for drawing widgets or child views.
//!
//! # Modules
//!
//! - [`image`] - Image rendering and fallback methods on [`RenderCtx`].
//! - [`target`] - Frame and buffer render targets.

mod image;
mod target;

use leptos::prelude::{GetUntracked, ReadSignal};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Position, Rect},
    widgets::{StatefulWidget, Widget},
};

use crate::{
    StyleMetadata, ThemeVariables,
    app::Result,
    context,
    style::{Stylesheet, TuiStyle, ViewportSize},
    terminal_image::TerminalImageSupport,
    view::AnyView,
};

use self::target::RenderTarget;

/// Rendering context for a single frame and target area.
pub struct RenderCtx<'frame, 'buffer> {
    /// Destination receiving rendered widgets.
    target: RenderTarget<'frame, 'buffer>,
    /// Area inside the frame currently targeted by rendering calls.
    area: Rect,
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
            viewport_size: area.into(),
            stylesheets: Vec::new(),
            inherited_style: TuiStyle::new(),
            selector_ancestors: Vec::new(),
            terminal_images: context::use_context::<TerminalImageSupport>().unwrap_or_default(),
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
            self.inherited_style,
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
        let inherited_style = self.inherited_style;
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
            viewport_size: self.viewport_size,
            stylesheets,
            inherited_style,
            selector_ancestors,
            terminal_images: self.terminal_images.clone(),
        }
    }

    /// Returns the style declarations inherited by the current view.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing inherited style values for the current area.
    pub(crate) fn inherited_style(&self) -> TuiStyle {
        self.inherited_style
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

    /// Sets the terminal cursor position for this render pass.
    pub(crate) fn set_cursor_position(&mut self, position: Position) {
        self.target.set_cursor_position(position);
    }

    /// Renders a Ratatui stateful widget into the current target area.
    pub(crate) fn render_stateful_widget<W>(&mut self, widget: W, state: &mut W::State)
    where
        W: StatefulWidget,
    {
        self.target.render_stateful_widget(widget, self.area, state);
    }

    /// Renders a Leptatui view into the current target area.
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
        view.render(self)
    }

    /// Renders a view into an offscreen buffer and copies a clipped row slice.
    pub(crate) fn render_view_clipped(
        &mut self,
        view: &AnyView,
        full_area: Rect,
        source_y: u16,
        target_area: Rect,
        inherited_style: TuiStyle,
        selector_ancestor: StyleMetadata,
    ) -> Result<()> {
        if target_area.width == 0 || target_area.height == 0 || full_area.height == 0 {
            return Ok(());
        }

        if self.target.supports_terminal_images() && self.terminal_images.supports_protocol() {
            let handled = self.with_area_inherited_style_and_selector_ancestor(
                full_area,
                inherited_style,
                selector_ancestor.clone(),
                |ctx| view.render_terminal_image_clipped(source_y, target_area, ctx),
            )?;
            if handled {
                return Ok(());
            }
        }

        let mut buffer = Buffer::empty(Rect::new(0, 0, full_area.width, full_area.height));
        {
            let target = self.target.buffer_mut();
            for y in 0..target_area.height {
                for x in 0..target_area.width {
                    let target_position = (
                        target_area.x.saturating_add(x),
                        target_area.y.saturating_add(y),
                    );
                    let buffer_position = (x, source_y.saturating_add(y));

                    if let (Some(target_cell), Some(buffer_cell)) = (
                        target.cell(target_position),
                        buffer.cell_mut(buffer_position),
                    ) {
                        *buffer_cell = target_cell.clone();
                    }
                }
            }
        }

        let mut selector_ancestors = self.selector_ancestors.clone();
        selector_ancestors.push(selector_ancestor);

        let mut cursor_position = None;

        {
            let mut buffer_ctx = RenderCtx {
                target: RenderTarget::Buffer {
                    buffer: &mut buffer,
                    cursor_position: &mut cursor_position,
                },
                area: Rect::new(0, 0, full_area.width, full_area.height),
                viewport_size: self.viewport_size,
                stylesheets: self.stylesheets.clone(),
                inherited_style,
                selector_ancestors,
                terminal_images: TerminalImageSupport::default(),
            };
            view.render(&mut buffer_ctx)?;
        }

        let target = self.target.buffer_mut();
        for y in 0..target_area.height {
            for x in 0..target_area.width {
                let source = buffer[(x, source_y.saturating_add(y))].clone();
                let destination_position = (
                    target_area.x.saturating_add(x),
                    target_area.y.saturating_add(y),
                );
                if let Some(destination) = target.cell_mut(destination_position) {
                    *destination = source;
                }
            }
        }

        if let Some(position) = cursor_position
            && position.y >= source_y
            && position.y < source_y.saturating_add(target_area.height)
            && position.x < target_area.width
        {
            self.set_cursor_position(Position {
                x: target_area.x.saturating_add(position.x),
                y: target_area
                    .y
                    .saturating_add(position.y.saturating_sub(source_y)),
            });
        }

        Ok(())
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
        self.with_area_and_inherited_style(area, self.inherited_style, render)
    }
}
