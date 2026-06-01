//! Component rendering contract.

use crossterm::event::Event;
use ratatui::{Frame, layout::Rect, widgets::Widget};

use crate::{
    app::{AppControl, Result},
    node::Node,
};

/// Root or child component that can render into a terminal frame.
pub trait Component {
    /// Render the current component state into the provided context.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()>;

    /// Handle a terminal event.
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        Ok(AppControl::Continue)
    }
}

/// Rendering context for a single frame and target area.
pub struct RenderCtx<'frame, 'buffer> {
    frame: &'frame mut Frame<'buffer>,
    area: Rect,
}

impl<'frame, 'buffer> RenderCtx<'frame, 'buffer> {
    /// Create a render context that targets the full frame area.
    pub fn new(frame: &'frame mut Frame<'buffer>) -> Self {
        let area = frame.area();
        Self { frame, area }
    }

    /// Return the target area for this render context.
    pub const fn area(&self) -> Rect {
        self.area
    }

    /// Render a Ratatui widget into the current target area.
    pub fn render_widget<W>(&mut self, widget: W)
    where
        W: Widget,
    {
        self.frame.render_widget(widget, self.area);
    }

    /// Render a Leptatui node into the current target area.
    pub fn render_node(&mut self, node: &Node) -> Result<()> {
        node.render(self)
    }

    /// Render into a temporary child area.
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
