//! Default interactive screen for fallible component errors.
//!
//! Fallible generated components lower their errors into this full-viewport
//! view. The screen exposes router-aware recovery when history is available
//! and always provides a clean application exit.

use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::widgets::Clear;

use crate::{
    AnyView, AppControl, Axes, BorderType, Borders, BoxSizing, Color, Dimension, Display, Edges,
    FlexDirection, IntoView, JustifyContent, KeyControl, LayoutSize, Length, LengthAuto, Modifier,
    Overflow, Position, StyleMetadata, TuiSpacing, TuiStyle, View, ViewError, ViewType, ZIndex,
    app::{Result, StandaloneScreenRegistry},
    block, button,
    component::RenderCtx,
    context, div, h1, paragraph, use_key_event,
};

use super::boundary::component::ComponentView;

/// Creates the default interactive screen for a fallible component error.
///
/// # Arguments
///
/// * `error` — Type-erased error and context chain to display.
/// * `source_file` — Source-relative file containing the fallible component.
/// * `source_line` — One-based declaration line of the fallible component.
///
/// # Returns
///
/// An [`AnyView`] covering the terminal viewport with Back and Quit controls.
#[doc(hidden)]
pub fn __view_error(error: ViewError, source_file: &'static str, source_line: u32) -> AnyView {
    let history = crate::route::try_use_history();
    let error_screens = context::use_context::<StandaloneScreenRegistry>();
    let screen = ComponentView::new_style_isolated(ErrorScreen::with_props(ErrorScreenProps {
        message: format!("{error:#}"),
        source_file: source_file.to_owned(),
        source_line,
        history,
        error_screens: error_screens.clone(),
    }));

    if let Some(error_screens) = error_screens {
        error_screens.register(&screen);
    }

    screen.into_view()
}

/// Creates the interactive content for one standalone error screen.
///
/// # Arguments
///
/// * `message` — Formatted error and source-chain diagnostic.
/// * `source_file` — Source-relative file containing the fallible component.
/// * `source_line` — One-based declaration line of the fallible component.
/// * `history` — Optional router history used by the Back action.
/// * `error_screens` — Optional managed-app registry used to dismiss the screen.
///
/// # Returns
///
/// A full-viewport error screen with router-aware recovery controls.
#[crate::component]
#[allow(non_snake_case)]
fn ErrorScreen(
    message: String,
    source_file: String,
    source_line: u32,
    history: Option<crate::History>,
    error_screens: Option<StandaloneScreenRegistry>,
) -> impl IntoView {
    crate::stylesheet! {
        Paragraph => { fg: Color::Red }
        Button => {
            fg: Color::Red,
            bg: Color::Reset,
            border_type: BorderType::Plain,
            padding: TuiSpacing::ZERO
        }
        Button:focus => {
            fg: Color::White,
            bg: Color::Red
        }
    }

    let can_go_back = history
        .as_ref()
        .is_some_and(crate::route::History::can_go_back_untracked);
    let shortcut_history = history.clone();
    let shortcut_error_screens = error_screens.clone();

    use_key_event(KeyEventKind::Press, move |key| match key.code {
        KeyCode::Char('q') => KeyControl::Exit,
        KeyCode::Esc | KeyCode::Char('b')
            if shortcut_history
                .as_ref()
                .is_some_and(crate::route::History::can_go_back_untracked) =>
        {
            if let Some(error_screens) = &shortcut_error_screens {
                error_screens.dismiss();
            }
            if let Some(history) = &shortcut_history {
                history.back();
            }
            KeyControl::Handled
        }
        _ => KeyControl::Pass,
    });

    let mut actions = Vec::new();
    if can_go_back {
        let back_history = history.expect("available router history should be retained");
        let back_error_screens = error_screens;
        actions.push(
            button("Back")
                .on_press(move || {
                    if let Some(error_screens) = &back_error_screens {
                        error_screens.dismiss();
                    }
                    back_history.back();
                    AppControl::Continue
                })
                .with_focus(true)
                .into_view(),
        );
    }
    actions.push(
        button("Quit")
            .on_press(|| AppControl::Exit)
            .with_focus(!can_go_back)
            .into_view(),
    );

    let title = h1("Error").with_inline_style(
        TuiStyle::new()
            .foreground(Color::Red)
            .modifier(Modifier::BOLD)
            .flex_shrink(0.0),
    );
    let details = div((
        paragraph(format!("File: {source_file}")),
        paragraph(format!("Line Number: {source_line}")),
        paragraph(message),
    ))
    .with_inline_style(
        TuiStyle::new()
            .display(Display::Flex)
            .flex_direction(FlexDirection::Column)
            .flex_grow(1.0)
            .min_size(LayoutSize::new(
                Dimension::Auto,
                Dimension::from(Length::cells(0.0)),
            ))
            .gap(Axes::new(Length::cells(0.0), Length::cells(1.0)))
            .overflow(Axes::new(Overflow::Hidden, Overflow::Auto)),
    );
    let diagnostic = div((title, details)).with_inline_style(
        TuiStyle::new()
            .display(Display::Flex)
            .flex_direction(FlexDirection::Column)
            .flex_grow(1.0)
            .min_size(LayoutSize::new(
                Dimension::Auto,
                Dimension::from(Length::cells(0.0)),
            ))
            .gap(Axes::new(Length::cells(0.0), Length::cells(1.0))),
    );
    let controls = div(actions).with_inline_style(
        TuiStyle::new()
            .display(Display::Flex)
            .flex_direction(FlexDirection::Row)
            .flex_shrink(0.0)
            .justify_content(JustifyContent::SpaceEvenly)
            .size(LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::Auto,
            )),
    );
    let content = div((diagnostic, controls)).with_inline_style(
        TuiStyle::new()
            .display(Display::Flex)
            .flex_direction(FlexDirection::Column)
            .flex_grow(1.0)
            .min_size(LayoutSize::new(
                Dimension::Auto,
                Dimension::from(Length::cells(0.0)),
            ))
            .size(LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::Auto,
            ))
            .foreground(Color::Red),
    );

    let clear = AnyView::new(ErrorScreenClear::new())
        .with_inline_style(error_screen_viewport_style().overflow(Axes::all(Overflow::Hidden)));
    let surface = block(content).with_inline_style(
        error_screen_viewport_style()
            .display(Display::Flex)
            .flex_direction(FlexDirection::Column)
            .overflow(Axes::all(Overflow::Hidden))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(TuiSpacing::uniform(1))
            .foreground(Color::Red)
            .background(Color::Reset),
    );

    div((clear, surface))
}

/// Returns the fixed full-terminal style shared by error-screen layers.
///
/// # Returns
///
/// A [`TuiStyle`] covering the complete viewport above application content.
fn error_screen_viewport_style() -> TuiStyle {
    TuiStyle::new()
        .box_sizing(BoxSizing::BorderBox)
        .position(Position::Fixed)
        .inset(Edges::new(
            Length::cells(0.0).into(),
            LengthAuto::Auto,
            LengthAuto::Auto,
            Length::cells(0.0).into(),
        ))
        .size(LayoutSize::new(
            Dimension::from(Length::vw(100.0)),
            Dimension::from(Length::vh(100.0)),
        ))
        .z_index(ZIndex::Integer(i32::MAX))
}

/// Full-viewport layer that clears previously rendered application symbols.
#[derive(Debug)]
struct ErrorScreenClear {
    /// Selector and retained layout metadata for the fixed layer.
    metadata: StyleMetadata,
}

impl ErrorScreenClear {
    /// Creates an unstyled terminal-clear layer.
    ///
    /// # Returns
    ///
    /// An [`ErrorScreenClear`] ready to receive its fixed inline style.
    fn new() -> Self {
        Self {
            metadata: StyleMetadata::new(ViewType::new("ErrorScreenClear")),
        }
    }
}

impl View for ErrorScreenClear {
    /// Clears every terminal cell assigned to this layer.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context covering the retained viewport geometry.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] after clearing the layer.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        ctx.render_widget(Clear);
        Ok(())
    }

    /// Returns this layer's selector and retained layout metadata.
    ///
    /// # Returns
    ///
    /// An optional shared [`StyleMetadata`] reference.
    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }

    /// Returns this layer's mutable selector and retained layout metadata.
    ///
    /// # Returns
    ///
    /// An optional mutable [`StyleMetadata`] reference.
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }

    /// Returns this clear layer for concrete type inspection.
    ///
    /// # Returns
    ///
    /// A shared [`std::any::Any`] reference.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Returns this clear layer mutably for concrete type inspection.
    ///
    /// # Returns
    ///
    /// A mutable [`std::any::Any`] reference.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
/// Unit tests for managed error-screen registration and ownership.
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    use crate::{AppControl, View, context};

    use super::*;

    /// Verifies a fallible view registers a live standalone managed screen.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// managed render context + view error
    /// q -> AppControl::Exit
    /// drop view -> no active screen
    /// ```
    ///
    /// # Assertions
    ///
    /// - Constructing the error view registers its mounted screen boundary.
    /// - The registered screen owns and handles its Quit shortcut.
    /// - Dropping the owning view invalidates the weak registry entry.
    #[test]
    fn view_error_registers_managed_screen_with_its_own_shortcuts() {
        let registry = StandaloneScreenRegistry::new();
        let view = context::hooks::__with_context_scope(|| {
            context::provide_context(registry.clone());
            __view_error(anyhow::anyhow!("managed failure"), "src/managed.rs", 42)
        });
        let mut screen = registry
            .active()
            .expect("constructed error view should register its screen");

        assert_eq!(
            View::handle_event(
                &mut screen,
                Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            )
            .expect("managed Quit shortcut should dispatch"),
            AppControl::Exit,
        );

        drop(screen);
        drop(view);
        assert!(registry.active().is_none());
    }
}
