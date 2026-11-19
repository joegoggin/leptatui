//! Signal binding tests shared by single-line and multiline editable controls.

mod support;

use std::{cell::RefCell, rc::Rc};

use crossterm::event::Event;
use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend};
use support::{draw_view, key, rendered_text};

/// Verifies both fluent controls edit their signals without input callbacks.
///
/// # Example Under Test
///
/// ```text
/// input(value) and text_area(value), focused in insert mode.
/// x, y, Backspace, Esc, u, Ctrl+R, external value.set("external").
/// ```
///
/// # Assertions
///
/// - Consecutive edits and deletion update the bound signal immediately.
/// - Undo and redo work without an input callback.
/// - External writes appear in the accessor and rendered output.
#[test]
fn fluent_controls_bind_without_callbacks() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let title = RwSignal::new(String::from("a"));
        let body = RwSignal::new(String::from("a"));
        let mut input = input(title).with_focus(true);
        input.editable_state_mut().set_mode(VimMode::Insert);
        let mut area = text_area(body).with_focus(true);
        area.editable_state_mut().set_mode(VimMode::Insert);
        for (value, control) in [
            (title, &mut input as &mut dyn View),
            (body, &mut area as &mut dyn View),
        ] {
            for code in [KeyCode::Char('x'), KeyCode::Char('y'), KeyCode::Backspace] {
                View::handle_event(control, key(code))?;
            }
            assert_eq!(value.get_untracked(), "ax");
            View::handle_event(control, key(KeyCode::Esc))?;
            View::handle_event(control, key(KeyCode::Char('u')))?;
            assert_eq!(value.get_untracked(), "axy");
            View::handle_event(
                control,
                Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            )?;
            assert_eq!(value.get_untracked(), "ax");
            value.set(String::from("external"));
            let mut terminal = Terminal::new(TestBackend::new(30, 5))?;
            draw_view(&mut terminal, control)?;
            assert!(rendered_text(&terminal).contains("external"));
        }
        assert_eq!(input.value(), "external");
        assert_eq!(area.value(), "external");
        Ok(())
    })
}

/// Verifies multiline notifications read committed text and may replace it.
///
/// # Example Under Test
///
/// ```text
/// TextArea value=body, on_input reads body and uppercases it.
/// Tab, i, x, Enter, y; external body.set("loaded").
/// ```
///
/// # Assertions
///
/// - Notifications observe the complete edited text, including inserted newlines.
/// - Callback writes are used by subsequent edits without recursive notifications.
/// - External writes update the view without invoking the callback.
#[test]
fn multiline_notifications_observe_committed_values() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let body = RwSignal::new(String::new());
        let observed = Rc::new(RefCell::new(Vec::new()));
        let values = observed.clone();
        let mut view = view! {
            <TextArea value=body on_input=move || {
                let value = body.get_untracked();
                values.borrow_mut().push(value.clone());
                body.set(value.to_uppercase());
            } />
        };
        for code in [
            KeyCode::Tab,
            KeyCode::Char('i'),
            KeyCode::Char('x'),
            KeyCode::Enter,
            KeyCode::Char('y'),
        ] {
            View::handle_event(&mut view, key(code))?;
        }
        assert_eq!(body.get_untracked(), "X\nY");
        assert_eq!(*observed.borrow(), ["x", "X\n", "X\ny"]);
        body.set(String::from("loaded"));
        let mut terminal = Terminal::new(TestBackend::new(30, 5))?;
        draw_view(&mut terminal, &view)?;
        assert!(rendered_text(&terminal).contains("loaded"));
        assert_eq!(observed.borrow().len(), 3);
        Ok(())
    })
}
