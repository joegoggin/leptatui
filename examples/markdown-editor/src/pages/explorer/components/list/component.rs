//! Explorer rows, empty state, and recoverable errors.

use leptatui::prelude::*;

use crate::services::DirectoryListing;

use super::{
    super::{ExplorerEntryRow, ExplorerEntryRowProps},
    style::use_explorer_list_styles,
};

/// Renders explorer rows and any recoverable directory error.
///
/// # Arguments
///
/// * `listing` — Current directory listing snapshot.
/// * `selection` — Current selected entry index.
/// * `error` — Current recoverable navigation error.
///
/// # Returns
///
/// A directory listing component.
#[component]
pub(in crate::pages::explorer) fn ExplorerList(
    listing: DirectoryListing,
    selection: Option<usize>,
    error: Option<String>,
) -> impl IntoView {
    use_explorer_list_styles();

    let mut rows = Vec::new();

    if listing.entries().is_empty() {
        rows.push(
            view! {
                <Text class="explorer-list__empty">"No directories or Markdown files"</Text>
            }
            .into_view(),
        );
    } else {
        rows.extend(
            listing
                .entries()
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, entry)| {
                    view! {
                        <ExplorerEntryRow entry=entry selected={selection == Some(index)} />
                    }
                    .into_view()
                }),
        );
    }

    if let Some(error) = error {
        rows.push(
            view! {
                <Text class="explorer-list__error">{format!("Error: {error}")}</Text>
            }
            .into_view(),
        );
    }

    div(rows).with_classes("explorer-list")
}
