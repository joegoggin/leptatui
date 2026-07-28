//! Home route-level component and keyboard behavior.

use std::{cell::RefCell, rc::Rc};

use leptatui::prelude::*;

use crate::{core::Controller, pages::shared::routed_page_style};

use super::components::{RecentFilesList, RecentFilesListProps};

/// Renders the landing page and its recent-file actions.
///
/// # Arguments
///
/// * `controller` — Shared application state.
///
/// # Returns
///
/// A Home page component.
#[component]
pub(crate) fn HomePage(controller: Rc<RefCell<Controller>>) -> impl IntoView {
    let shortcut_navigate = use_navigate();
    let button_navigate = use_navigate();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('o') && key.modifiers == KeyModifiers::NONE {
            shortcut_navigate("/files", NavigateOptions::default());
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    let recent_controller = Rc::clone(&controller);
    let recent_state = recent_controller.borrow().recent_files().clone();
    let root = controller.borrow().workspace().root().to_path_buf();
    let recent_root = root.clone();
    let page_style = routed_page_style();

    view! {
        <Div class="page" style=page_style>
            <Text class="page-title">"Markdown editor"</Text>
            <Text class="path-context">{format!("Root: {}", root.display())}</Text>
            <Div class="actions">
                <Button on_press=move || {
                    button_navigate("/files", NavigateOptions::default());
                    AppControl::Continue
                }>"Open file"</Button>
            </Div>
            <Block class="page-content scroll-content">
                {RecentFilesList::with_props(
                    RecentFilesListProps::builder()
                        .state(recent_state)
                        .root(recent_root)
                        .controller(recent_controller)
                        .build(),
                )}
            </Block>
            <Text class="help">"o open file | Tab/Enter actions | q quit"</Text>
        </Div>
    }
}
