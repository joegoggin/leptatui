//! Render destinations used by [`RenderCtx`](super::RenderCtx).

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Position, Rect},
    widgets::{StatefulWidget, Widget},
};

/// Destination for render operations.
pub(super) enum RenderTarget<'frame, 'buffer> {
    /// Active Ratatui frame.
    Frame(&'frame mut Frame<'buffer>),
    /// Offscreen buffer used for clipping.
    Buffer {
        /// Offscreen buffer receiving rendered widgets.
        buffer: &'frame mut Buffer,
        /// Cursor position requested while rendering into the buffer.
        cursor_position: &'frame mut Option<Position>,
    },
}

impl<'frame, 'buffer> RenderTarget<'frame, 'buffer> {
    /// Returns a shorter mutable borrow of this render target.
    pub(super) fn reborrow(&mut self) -> RenderTarget<'_, 'buffer> {
        match self {
            Self::Frame(frame) => RenderTarget::Frame(frame),
            Self::Buffer {
                buffer,
                cursor_position,
            } => RenderTarget::Buffer {
                buffer,
                cursor_position,
            },
        }
    }

    /// Renders a widget into the target area.
    pub(super) fn render_widget<W>(&mut self, widget: W, area: Rect)
    where
        W: Widget,
    {
        match self {
            Self::Frame(frame) => frame.render_widget(widget, area),
            Self::Buffer { buffer, .. } => widget.render(area, buffer),
        }
    }

    /// Renders a stateful widget into the target area.
    pub(super) fn render_stateful_widget<W>(&mut self, widget: W, area: Rect, state: &mut W::State)
    where
        W: StatefulWidget,
    {
        match self {
            Self::Frame(frame) => frame.render_stateful_widget(widget, area, state),
            Self::Buffer { buffer, .. } => widget.render(area, buffer, state),
        }
    }

    /// Sets the requested cursor position.
    pub(super) fn set_cursor_position(&mut self, position: Position) {
        match self {
            Self::Frame(frame) => frame.set_cursor_position(position),
            Self::Buffer {
                cursor_position, ..
            } => **cursor_position = Some(position),
        }
    }

    /// Returns whether this target can receive terminal image protocol data.
    pub(super) fn supports_terminal_images(&self) -> bool {
        matches!(self, Self::Frame(_))
    }

    /// Returns the underlying buffer.
    pub(super) fn buffer_mut(&mut self) -> &mut Buffer {
        match self {
            Self::Frame(frame) => frame.buffer_mut(),
            Self::Buffer { buffer, .. } => buffer,
        }
    }
}
