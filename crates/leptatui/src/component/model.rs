//! Frame rendering context model.
//!
//! This module wraps a Ratatui frame with the currently assigned render area and
//! helper methods for drawing widgets or child nodes.

use ratatui::{Frame, layout::Rect, widgets::Widget};

use crate::{app::Result, node::Node};

/// Rendering context for a single frame and target area.
pub struct RenderCtx<'frame, 'buffer> {
    /// Ratatui frame being rendered during the current draw pass.
    frame: &'frame mut Frame<'buffer>,
    /// Area inside the frame currently targeted by rendering calls.
    area: Rect,
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
        Self { frame, area }
    }

    /// Returns the target area for this render context.
    ///
    /// # Returns
    ///
    /// A [`Rect`] describing the current rendering area.
    pub const fn area(&self) -> Rect {
        self.area
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
        let mut child = RenderCtx {
            frame: &mut *self.frame,
            area,
        };

        render(&mut child)
    }
}
