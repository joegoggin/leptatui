//! Explorer rows, empty state, and recoverable errors.

use leptatui::prelude::*;

use crate::core::ExplorerState;

use super::{ExplorerEntryRow, ExplorerEntryRowProps};

/// Renders explorer rows and any recoverable directory error.
///
/// # Arguments
///
/// * `state` — Current explorer snapshot.
///
/// # Returns
///
/// A directory listing component.
#[component]
pub(in crate::pages::explorer) fn ExplorerList(state: ExplorerState) -> impl IntoView {
    let mut rows = Vec::new();

    if state.entries().is_empty() {
        rows.push(
            view! {
                <Text class="empty">"No directories or Markdown files"</Text>
            }
            .into_view(),
        );
    } else {
        rows.extend(
            state
                .entries()
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, entry)| {
                    let selected = state.selection() == Some(index);

                    view! {
                        <ExplorerEntryRow entry=entry selected=selected />
                    }
                    .into_view()
                }),
        );
    }

    if let Some(error) = state.error() {
        rows.push(
            view! {
                <Text class="error">{format!("Error: {error}")}</Text>
            }
            .into_view(),
        );
    }

    div(rows)
}
