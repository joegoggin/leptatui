//! Explorer page content rendered from page-owned signals.

use std::sync::Arc;

use leptatui::prelude::*;

use crate::{
    pages::shared::{relative_path, routed_page_style},
    services::{DirectoryListing, Workspace},
};

use super::{ExplorerList, ExplorerListProps};

/// Renders the current explorer state inside a stable scroll boundary.
///
/// # Arguments
///
/// * `workspace` — Validated workspace displayed by the page.
/// * `listing` — Page-owned directory listing signal.
/// * `selection` — Page-owned selected index signal.
/// * `error` — Page-owned recoverable error signal.
///
/// # Returns
///
/// Explorer headings and the scrollable listing.
#[component]
pub(in crate::pages::explorer) fn ExplorerContent(
    workspace: Workspace,
    listing: RwSignal<DirectoryListing>,
    selection: RwSignal<Option<usize>>,
    error: RwSignal<Option<Arc<anyhow::Error>>>,
) -> impl IntoView {
    let home_navigate = use_navigate();
    let root = workspace.root().to_path_buf();
    let directory_workspace = workspace.clone();
    let page_style = routed_page_style();

    view! {
        <Div class="page" style=page_style>
            <Text class="page-title">"File explorer"</Text>
            <Text class="path-context">{format!("Root: {}", root.display())}</Text>
            {move || {
                let current = listing.get_untracked();
                let directory =
                    relative_path(directory_workspace.root(), current.directory());
                view! {
                    <Text class="path-context">{format!("Directory: {directory}")}</Text>
                }
            }}
            <Block class="page-content scroll-content">
                {move || {
                    view! {
                        <ExplorerList
                            listing=listing.get_untracked()
                            selection=selection.get_untracked()
                            error=error.get_untracked().map(|error| error.to_string())
                        />
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
