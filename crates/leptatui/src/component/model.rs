//! Frame rendering context model.
//!
//! This module wraps a Ratatui frame with the currently assigned render area,
//! inherited style values, selector ancestor metadata, and helper methods for
//! drawing widgets or child nodes.

use ratatui::{Frame, layout::Rect, widgets::Widget};

use crate::{
    StyleMetadata,
    app::Result,
    node::Node,
    style::{Stylesheet, TuiStyle},
};

/// Shared empty stylesheet used by render contexts created without app styles.
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
    /// Ancestor metadata used by descendant selector resolution.
    selector_ancestors: Vec<StyleMetadata>,
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
            selector_ancestors: Vec::new(),
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
    ///
    /// # Returns
    ///
    /// A [`Stylesheet`] reference used for node style resolution.
    pub(crate) fn stylesheet(&self) -> &Stylesheet {
        self.stylesheet
    }

    /// Returns the style declarations inherited by the current node.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing inherited style values for the current area.
    pub(crate) fn inherited_style(&self) -> TuiStyle {
        self.inherited_style
    }

    /// Returns selector metadata for ancestor nodes in render order.
    ///
    /// # Returns
    ///
    /// A [`StyleMetadata`] slice ordered from outermost ancestor to innermost
    /// ancestor.
    pub(crate) fn selector_ancestors(&self) -> &[StyleMetadata] {
        &self.selector_ancestors
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
            selector_ancestors: self.selector_ancestors.clone(),
        };

        render(&mut child)
    }

    /// Renders into a child area with inherited style and one added selector ancestor.
    ///
    /// # Arguments
    ///
    /// * `area` — Child area to use while invoking `render`.
    /// * `inherited_style` — Inherited style declarations for the child area.
    /// * `selector_ancestor` — Parent node metadata to append to the selector
    ///   ancestor path.
    /// * `render` — Closure that renders into the child context.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `render`.
    pub(crate) fn with_area_inherited_style_and_selector_ancestor<R>(
        &mut self,
        area: Rect,
        inherited_style: TuiStyle,
        selector_ancestor: StyleMetadata,
        render: impl FnOnce(&mut RenderCtx<'_, 'buffer>) -> R,
    ) -> R {
        let mut selector_ancestors = self.selector_ancestors.clone();
        selector_ancestors.push(selector_ancestor);

        let mut child = RenderCtx {
            frame: &mut *self.frame,
            area,
            stylesheet: self.stylesheet,
            inherited_style,
            selector_ancestors,
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
