//! Explorer route-level component and keyboard behavior.

use std::{cell::RefCell, rc::Rc};

use leptatui::prelude::*;

use crate::{
    core::{Controller, ExplorerActivation},
    pages::viewer_location,
};

use super::components::{ExplorerContent, ExplorerContentProps};

/// Renders the standalone workspace file explorer.
///
/// # Arguments
///
/// * `controller` — Shared application state.
///
/// # Returns
///
/// An Explorer page component.
#[component]
pub(crate) fn ExplorerPage(controller: Rc<RefCell<Controller>>) -> impl IntoView {
    let shortcut_controller = Rc::clone(&controller);
    let shortcut_navigate = use_navigate();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                shortcut_controller.borrow_mut().select_previous();
                KeyControl::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                shortcut_controller.borrow_mut().select_next();
                KeyControl::Handled
            }
            KeyCode::Enter => {
                if shortcut_controller.borrow_mut().activate_selected()
                    == ExplorerActivation::Document
                {
                    let target = {
                        let controller = shortcut_controller.borrow();
                        controller
                            .preview()
                            .path()
                            .map(|path| viewer_location(controller.workspace().root(), path))
                    };
                    if let Some(target) = target {
                        shortcut_navigate(&target, NavigateOptions::default());
                    }
                }
                KeyControl::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                shortcut_controller.borrow_mut().browse_parent();
                KeyControl::Handled
            }
            KeyCode::Esc => {
                shortcut_navigate("/", NavigateOptions::default());
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    });

    view! {
        <ExplorerContent controller=controller />
    }
}
