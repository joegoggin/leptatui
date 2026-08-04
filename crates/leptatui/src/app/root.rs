//! App root adapter contract.
//!
//! This module defines the root-level rendering interface and adapts [`View`]
//! values into app roots.

use crossterm::event::Event;
use ratatui::Frame;

use crate::{
    AnyView, StyleMetadata, View,
    component::{FocusedControl, RenderCtx},
    context,
    view::core::{events::handle_view_event_with_layout, layout::render_with_layout},
};

use super::{AppControl, Result};

/// Layout work required before the next frame can be painted.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum LayoutMode {
    /// Rebuild and measure the complete layout tree before painting.
    #[default]
    Recompute,
    /// Reuse retained geometry and repaint at updated scroll offsets.
    Reuse,
}

impl LayoutMode {
    /// Combines layout requirements from one batch of terminal events.
    ///
    /// # Arguments
    ///
    /// * `other` — Next event's layout requirement.
    ///
    /// # Returns
    ///
    /// A [`LayoutMode`] that recomputes when either input requires it.
    pub(crate) const fn merge(self, other: Self) -> Self {
        if matches!(self, Self::Recompute) || matches!(other, Self::Recompute) {
            Self::Recompute
        } else {
            Self::Reuse
        }
    }
}

/// Application control paired with the next frame's layout requirement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EventOutcome {
    /// Whether the app loop should continue or exit.
    pub(crate) control: AppControl,
    /// Whether the next frame may reuse retained layout geometry.
    pub(crate) layout: LayoutMode,
}

impl EventOutcome {
    /// Creates an event outcome that conservatively recomputes layout.
    ///
    /// # Arguments
    ///
    /// * `control` — Application control emitted by the event.
    ///
    /// # Returns
    ///
    /// An [`EventOutcome`] requiring complete layout recomputation.
    pub(crate) const fn recompute(control: AppControl) -> Self {
        Self {
            control,
            layout: LayoutMode::Recompute,
        }
    }

    /// Creates an event outcome that may reuse retained layout geometry.
    ///
    /// # Arguments
    ///
    /// * `control` — Application control emitted by the event.
    ///
    /// # Returns
    ///
    /// An [`EventOutcome`] permitting retained-layout reuse.
    pub(crate) const fn reuse(control: AppControl) -> Self {
        Self {
            control,
            layout: LayoutMode::Reuse,
        }
    }
}

/// Runtime adapter consumed by `App`.
pub trait AppRoot {
    /// Renders the current root state into the Ratatui frame.
    ///
    /// # Arguments
    ///
    /// * `frame` — Ratatui frame for the current draw pass.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if rendering through the terminal
    /// backend fails.
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()>;

    /// Renders with an internal retained-layout policy.
    ///
    /// # Arguments
    ///
    /// * `frame` — Ratatui frame receiving the root.
    /// * `_reuse_layout` — Whether standard view roots may reuse retained geometry.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] after rendering.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if rendering fails.
    #[doc(hidden)]
    fn __render(&mut self, frame: &mut Frame<'_>, _reuse_layout: bool) -> Result<()> {
        self.render(frame)
    }

    /// Handles a terminal event.
    ///
    /// # Arguments
    ///
    /// * `_event` — Crossterm event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether the app loop should continue.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails. Returns [`crate::app::Error::LinkOpen`] if an activated
    /// link cannot be opened.
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        Ok(AppControl::Continue)
    }

    /// Handles an event and reports whether the next frame may reuse layout.
    ///
    /// # Arguments
    ///
    /// * `event` — Crossterm event dispatched to the root.
    ///
    /// # Returns
    ///
    /// A tuple containing application control and retained-layout eligibility.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling fails.
    #[doc(hidden)]
    fn __handle_event(&mut self, event: Event) -> Result<(AppControl, bool)> {
        self.handle_event(event).map(|control| (control, false))
    }

    /// Emits any expired pending input inside this root.
    #[doc(hidden)]
    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        None
    }

    /// Returns metadata for the focused built-in control inside this root.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<FocusedControl> {
        None
    }
}

impl AppRoot for AnyView {
    /// Renders a view root inside a fresh Leptatui context scope.
    ///
    /// # Arguments
    ///
    /// * `frame` — Ratatui frame for the current draw pass.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if view rendering performs terminal
    /// I/O that fails.
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()> {
        self.__render(frame, false)
    }

    fn __render(&mut self, frame: &mut Frame<'_>, reuse_layout: bool) -> Result<()> {
        context::hooks::__with_context_scope(|| {
            let mut ctx = RenderCtx::new(frame);
            ctx.set_layout_reuse_requested(reuse_layout);
            AnyView::render(self, &mut ctx)
        })
    }

    /// Forwards a terminal event to the view root.
    ///
    /// # Arguments
    ///
    /// * `event` — Crossterm event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether the app loop should continue.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails. Returns [`crate::app::Error::LinkOpen`] if an activated
    /// link cannot be opened.
    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        AnyView::handle_event(self, event)
    }

    fn __handle_event(&mut self, event: Event) -> Result<(AppControl, bool)> {
        if self
            .as_view()
            .style_metadata()
            .is_some_and(StyleMetadata::is_layout_hidden)
        {
            return Ok((AppControl::Continue, false));
        }
        handle_view_event_with_layout(self.as_view_mut(), event)
            .map(|outcome| (outcome.control, outcome.layout == LayoutMode::Reuse))
    }

    /// Forwards pending input flushing into the view root.
    #[doc(hidden)]
    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        AnyView::__flush_pending_input(self)
    }

    /// Forwards focused-control metadata from the view root.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<FocusedControl> {
        AnyView::__focused_control(self)
    }
}

impl<V> AppRoot for V
where
    V: View,
{
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()> {
        self.__render(frame, false)
    }

    fn __render(&mut self, frame: &mut Frame<'_>, reuse_layout: bool) -> Result<()> {
        context::hooks::__with_context_scope(|| {
            let mut ctx = RenderCtx::new(frame);
            ctx.set_layout_reuse_requested(reuse_layout);
            render_with_layout(self, &mut ctx, |ctx| View::render(self, ctx))
        })
    }

    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        if View::style_metadata(self).is_some_and(StyleMetadata::is_layout_hidden) {
            return Ok(AppControl::Continue);
        }
        View::handle_event(self, event)
    }

    fn __handle_event(&mut self, event: Event) -> Result<(AppControl, bool)> {
        if View::style_metadata(self).is_some_and(StyleMetadata::is_layout_hidden) {
            return Ok((AppControl::Continue, false));
        }
        handle_view_event_with_layout(self, event)
            .map(|outcome| (outcome.control, outcome.layout == LayoutMode::Reuse))
    }

    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        if View::style_metadata(self).is_some_and(StyleMetadata::is_layout_hidden) {
            return None;
        }
        View::__flush_pending_input(self)
    }

    fn __focused_control(&self) -> Option<FocusedControl> {
        if View::style_metadata(self).is_some_and(StyleMetadata::is_layout_hidden) {
            return None;
        }
        View::__focused_control(self)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
    use ratatui::{Terminal, backend::TestBackend};

    use crate::{
        AnyView, IntoView, LayoutSize, StyleMetadata, View, ViewType, component::RenderCtx, div,
        view::core::measurement::AvailableSpace,
    };

    use super::AppRoot;

    /// Styleable intrinsic leaf that records layout measurements.
    struct MeasuringLeaf {
        /// Retained selector and layout metadata.
        metadata: StyleMetadata,
        /// Shared number of intrinsic measurements.
        measurements: Rc<Cell<usize>>,
    }

    impl View for MeasuringLeaf {
        /// Paints an empty measurement probe.
        fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> crate::Result<()> {
            Ok(())
        }

        /// Records one intrinsic measurement and returns overflowing dimensions.
        fn measure(
            &self,
            _known_dimensions: LayoutSize<Option<f32>>,
            _available_space: LayoutSize<AvailableSpace>,
            _ctx: &mut RenderCtx<'_, '_>,
        ) -> LayoutSize<f32> {
            self.measurements
                .set(self.measurements.get().saturating_add(1));
            LayoutSize::new(8.0, 20.0)
        }

        /// Returns the probe's retained selector metadata.
        fn style_metadata(&self) -> Option<&StyleMetadata> {
            Some(&self.metadata)
        }

        /// Returns mutable retained selector metadata.
        fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
            Some(&mut self.metadata)
        }

        /// Returns the probe for shared concrete-type inspection.
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        /// Returns the probe for mutable concrete-type inspection.
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    /// Custom unstyled leaf whose geometry exists only within one frame.
    struct UnstyledMeasuringLeaf {
        /// Shared number of intrinsic measurements.
        measurements: Rc<Cell<usize>>,
    }

    impl View for UnstyledMeasuringLeaf {
        /// Paints an empty measurement probe.
        fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> crate::Result<()> {
            Ok(())
        }

        /// Records one intrinsic measurement and returns overflowing dimensions.
        fn measure(
            &self,
            _known_dimensions: LayoutSize<Option<f32>>,
            _available_space: LayoutSize<AvailableSpace>,
            _ctx: &mut RenderCtx<'_, '_>,
        ) -> LayoutSize<f32> {
            self.measurements
                .set(self.measurements.get().saturating_add(1));
            LayoutSize::new(8.0, 20.0)
        }

        /// Returns the probe for shared concrete-type inspection.
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        /// Returns the probe for mutable concrete-type inspection.
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    /// Draws one erased root with the requested retained-layout policy.
    ///
    /// # Arguments
    ///
    /// * `terminal` — Test terminal receiving the frame.
    /// * `root` — Erased view root under test.
    /// * `reuse_layout` — Whether the frame requests retained geometry.
    ///
    /// # Returns
    ///
    /// An empty [`crate::Result`] after drawing.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if test rendering fails.
    fn draw(
        terminal: &mut Terminal<TestBackend>,
        root: &mut AnyView,
        reuse_layout: bool,
    ) -> crate::Result<()> {
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            render_result = root.__render(frame, reuse_layout);
        })?;
        render_result
    }

    /// Verifies built-in scrolling reuses retained styleable geometry.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// 10x4 root with one 8x20 styleable leaf
    /// Down, mouse wheel down, resize, unrelated key
    /// ```
    ///
    /// # Assertions
    ///
    /// - Initial rendering measures the overflowing leaf.
    /// - Keyboard and mouse scrolling request reuse without remeasurement.
    /// - Resize and unrelated key events request layout recomputation.
    #[test]
    fn scroll_redraw_reuses_retained_layout_without_remeasuring() -> crate::Result<()> {
        let measurements = Rc::new(Cell::new(0));
        let mut root = div((MeasuringLeaf {
            metadata: StyleMetadata::new(ViewType::Text),
            measurements: measurements.clone(),
        },))
        .into_view();
        let mut terminal = Terminal::new(TestBackend::new(10, 4))?;

        draw(&mut terminal, &mut root, false)?;
        let initial_measurements = measurements.get();
        assert!(initial_measurements > 0);

        let (control, reuse_layout) =
            root.__handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))?;
        assert_eq!(control, crate::AppControl::Continue);
        assert!(reuse_layout);

        draw(&mut terminal, &mut root, reuse_layout)?;
        assert_eq!(measurements.get(), initial_measurements);

        let (_, reuse_layout) = root.__handle_event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }))?;
        assert!(reuse_layout);
        draw(&mut terminal, &mut root, reuse_layout)?;
        assert_eq!(measurements.get(), initial_measurements);

        let (_, reuse_layout) = root.__handle_event(Event::Resize(20, 8))?;
        assert!(!reuse_layout);
        let (_, reuse_layout) = root.__handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
        )))?;
        assert!(!reuse_layout);
        Ok(())
    }

    /// Verifies frame-local custom geometry falls back to full layout.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// 10x4 root with one unstyled 8x20 custom leaf
    /// Down, retained-layout draw
    /// ```
    ///
    /// # Assertions
    ///
    /// - Initial rendering measures the overflowing custom leaf.
    /// - Scrolling remains eligible for retained-layout rendering.
    /// - Snapshot validation rejects the frame-local custom geometry.
    /// - Fallback layout measures the custom leaf again.
    #[test]
    fn scroll_redraw_remeasures_unstyled_custom_layout_leaves() -> crate::Result<()> {
        let measurements = Rc::new(Cell::new(0));
        let mut root = div((UnstyledMeasuringLeaf {
            measurements: measurements.clone(),
        },))
        .into_view();
        let mut terminal = Terminal::new(TestBackend::new(10, 4))?;

        draw(&mut terminal, &mut root, false)?;
        let initial_measurements = measurements.get();
        assert!(initial_measurements > 0);

        let (_, reuse_layout) =
            root.__handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))?;
        assert!(reuse_layout);
        draw(&mut terminal, &mut root, reuse_layout)?;

        assert!(measurements.get() > initial_measurements);
        Ok(())
    }
}
