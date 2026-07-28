//! Explorer page content and reactive directory listing.

use std::{cell::RefCell, rc::Rc};

use leptatui::prelude::*;

use crate::{
    core::Controller,
    pages::shared::{relative_path, routed_page_style},
};

use super::{ExplorerList, ExplorerListProps};

/// Renders the current explorer state inside a stable scroll boundary.
///
/// # Arguments
///
/// * `controller` — Shared application state.
///
/// # Returns
///
/// Explorer headings and the scrollable listing.
#[component]
pub(in crate::pages::explorer) fn ExplorerContent(
    controller: Rc<RefCell<Controller>>,
) -> impl IntoView {
    let home_navigate = use_navigate();
    let root = controller.borrow().workspace().root().to_path_buf();
    let directory_controller = Rc::clone(&controller);
    let list_controller = Rc::clone(&controller);
    let page_style = routed_page_style();

    view! {
        <Div class="page" style=page_style>
            <Text class="page-title">"File explorer"</Text>
            <Text class="path-context">{format!("Root: {}", root.display())}</Text>
            {move || {
                let controller = directory_controller.borrow();
                let directory = relative_path(
                    controller.workspace().root(),
                    controller.explorer().directory(),
                );
                view! {
                    <Text class="path-context">{format!("Directory: {directory}")}</Text>
                }
            }}
            <Block class="page-content scroll-content">
                {move || {
                    let state = list_controller.borrow().explorer().clone();

                    view! {
                        <ExplorerList state=state />
                    }
                }}
            </Block>
            <Div class="actions">
                <Button on_press=move || {
                    home_navigate("/", NavigateOptions::default());
                    AppControl::Continue
                }>"Home"</Button>
            </Div>
            <Text class="help">
                "↑/k ↓/j select | Enter open | ←/h parent | Esc home | q quit"
            </Text>
        </Div>
    }
}
