//! Explorer page content rendered from page-owned signals.

use std::{path::PathBuf, sync::Arc};

use leptatui::prelude::*;

use crate::{pages::shared::relative_path, services::DirectoryListing};

use super::{
    super::{ExplorerList, ExplorerListProps},
    style::use_explorer_content_styles,
};

/// Renders the current explorer state inside a stable scroll boundary.
///
/// # Arguments
///
/// * `initial_directory` — Process directory where browsing began.
/// * `listing` — Page-owned directory listing signal.
/// * `selection` — Page-owned selected index signal.
/// * `error` — Page-owned recoverable error signal.
///
/// # Returns
///
/// Explorer headings and the scrollable listing.
#[component]
pub(in crate::pages::explorer) fn ExplorerContent(
    initial_directory: PathBuf,
    listing: ArcRwSignal<DirectoryListing>,
    selection: ArcRwSignal<Option<usize>>,
    error: ArcRwSignal<Option<Arc<anyhow::Error>>>,
) -> impl IntoView {
    let home_navigate = use_navigate();
    let directory_base = initial_directory.clone();
    let directory_listing = listing.clone();
    let list_listing = listing.clone();
    let list_selection = selection.clone();
    let list_error = error.clone();

    use_explorer_content_styles();

    view! {
        <Div class="explorer-page">
            <Text class="explorer-page__title">"File explorer"</Text>
            <Text class="explorer-page__path">
                {format!("Started in: {}", initial_directory.display())}
            </Text>
            {move || {
                let directory = directory_listing
                    .try_get_untracked()
                    .map(|current| {
                        relative_path(&directory_base, current.directory())
                    })
                    .unwrap_or_default();
                view! {
                    <Text class="explorer-page__path">{format!("Directory: {directory}")}</Text>
                }
            }}
            <Block class="explorer-page__content">
                {move || {
                    let Some(listing) = list_listing.try_get_untracked() else {
                        return div(()).into_view();
                    };
                    let selection = list_selection.try_get_untracked().flatten();
                    let error = list_error
                        .try_get_untracked()
                        .flatten()
                        .map(|error| error.to_string());
                    view! { <ExplorerList listing=listing selection=selection error=error /> }
                        .into_view()
                }}
            </Block>
            <Div class="explorer-page__actions">
                <Button on_press=move || {
                    home_navigate("/", NavigateOptions::default());
                    AppControl::Continue
                }>"Home"</Button>
            </Div>
            <Text class="explorer-page__help">
                "↑/k ↓/j select | Enter open | ←/h parent | Esc home | q quit"
            </Text>
        </Div>
    }
}
