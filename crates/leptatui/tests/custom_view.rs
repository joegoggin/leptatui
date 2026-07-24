//! Integration tests for application-defined `View` implementations.

use std::{any::Any, cell::Cell, rc::Rc};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};

/// Application-owned styleable and interactive view.
struct Badge {
    /// Text rendered by the custom node.
    label: String,
    /// Shared count updated by the public key-event hook.
    key_presses: Rc<Cell<usize>>,
    /// Application-owned selector and inline-style metadata.
    metadata: StyleMetadata,
}

impl Badge {
    /// Creates a badge with application-owned selector identity.
    ///
    /// # Arguments
    ///
    /// * `label` — Text rendered by the badge.
    /// * `key_presses` — Shared counter updated for handled key events.
    ///
    /// # Returns
    ///
    /// A badge using the open `Badge` selector identity.
    fn new(label: impl Into<String>, key_presses: Rc<Cell<usize>>) -> Self {
        Self {
            label: label.into(),
            key_presses,
            metadata: StyleMetadata::new(ViewType::new("Badge")),
        }
    }
}

impl StyledView for Badge {
    fn metadata(&self) -> &StyleMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut StyleMetadata {
        &mut self.metadata
    }
}

impl View for Badge {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let style = ctx.resolve_style(&self.metadata);
        ctx.render_widget(Paragraph::new(self.label.clone()).style(style.to_ratatui_style()));
        Ok(())
    }

    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }

    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn on_key_event(&mut self, _key: KeyEvent) -> Result<KeyControl> {
        self.key_presses.set(self.key_presses.get() + 1);
        Ok(KeyControl::Handled)
    }
}

/// Verifies custom views compose, style, erase, downcast, render, and receive events.
///
/// # Example Under Test
///
/// ```text
/// div((Badge::new("Custom"), text("Built in")))
/// Badge => { fg: Color::LightMagenta }
/// Enter
/// ```
///
/// # Assertions
///
/// - A custom view composes with a built-in sibling through a tuple.
/// - Type erasure preserves downcasting and application-owned selector metadata.
/// - A custom selector resolves through the standard stylesheet cascade.
/// - The custom view renders with the resolved color.
/// - Public key-event hooks participate in subtree event traversal.
#[test]
fn custom_views_participate_in_the_full_view_protocol() -> Result<()> {
    let key_presses = Rc::new(Cell::new(0));
    let badge = Badge::new("Custom", Rc::clone(&key_presses)).with_classes("status");
    let mut view = div((badge, text("Built in")));

    assert!(view.children()[0].is::<Badge>());
    assert_eq!(
        view.children()[0]
            .style_metadata()
            .map(StyleMetadata::view_type),
        Some(ViewType::new("Badge")),
    );

    let stylesheet = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::new("Badge")),
        TuiStyle::new().foreground(Color::LightMagenta),
    );
    let mut terminal = Terminal::new(TestBackend::new(16, 2))?;
    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let custom_cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "C")
        .expect("custom view text should render");
    assert_eq!(custom_cell.fg, Color::LightMagenta);

    let control = view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;
    assert_eq!(control, KeyControl::Handled);
    assert_eq!(key_presses.get(), 1);

    Ok(())
}
