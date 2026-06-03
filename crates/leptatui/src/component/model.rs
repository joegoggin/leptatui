//! Frame rendering context model.
//!
//! This module wraps a Ratatui frame with the currently assigned render area and
//! helper methods for drawing widgets or child nodes.

use ratatui::{Frame, layout::Rect, widgets::Widget};

use crate::{
    app::Result,
    node::Node,
    style::{Stylesheet, TuiStyle},
};

static EMPTY_STYLESHEET: Stylesheet = Stylesheet::empty();

/// Rendering context for a single frame and target area.
pub struct RenderCtx<'frame, 'buffer> {
    /// Ratatui frame being rendered during the current draw pass.
    frame: &'frame mut Frame<'buffer>,
    /// Area inside the frame currently targeted by rendering calls.
    area: Rect,
    /// Stylesheet used to resolve node styles during rendering.
    stylesheet: &'frame Stylesheet,
    /// Inherited style declarations available to the current node.
    inherited_style: TuiStyle,
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
        Self::with_stylesheet(frame, &EMPTY_STYLESHEET)
    }

    /// Creates a render context with an application stylesheet.
    ///
    /// # Arguments
    ///
    /// * `frame` — Ratatui frame for the current draw pass.
    /// * `stylesheet` — Stylesheet used when resolving node styles.
    ///
    /// # Returns
    ///
    /// A [`RenderCtx`] covering the full frame.
    pub fn with_stylesheet(
        frame: &'frame mut Frame<'buffer>,
        stylesheet: &'frame Stylesheet,
    ) -> Self {
        let area = frame.area();
        Self {
            frame,
            area,
            stylesheet,
            inherited_style: TuiStyle::new(),
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

    /// Returns the stylesheet used by this render context.
    pub(crate) fn stylesheet(&self) -> &Stylesheet {
        self.stylesheet
    }

    /// Returns the style declarations inherited by the current node.
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
        self.frame.render_widget(widget, self.area);
    }

    /// Renders a Leptatui node into the current target area.
    ///
    /// # Arguments
    ///
    /// * `node` — Node tree to render.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if node rendering performs terminal
    /// I/O that fails.
    pub fn render_node(&mut self, node: &Node) -> Result<()> {
        node.render(self)
    }

    /// Renders into a temporary child area with explicit inherited style.
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
    pub(crate) fn with_area_and_inherited_style<R>(
        &mut self,
        area: Rect,
        inherited_style: TuiStyle,
        render: impl FnOnce(&mut RenderCtx<'_, 'buffer>) -> R,
    ) -> R {
        let mut child = RenderCtx {
            frame: &mut *self.frame,
            area,
            stylesheet: self.stylesheet,
            inherited_style,
        };

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
