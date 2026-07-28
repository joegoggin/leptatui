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
            text("No directories or Markdown files")
                .with_classes("empty")
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
                    ExplorerEntryRow::with_props(
                        ExplorerEntryRowProps::builder()
                            .entry(entry)
                            .selected(state.selection() == Some(index))
                            .build(),
                    )
                    .into_view()
                }),
        );
    }

    if let Some(error) = state.error() {
        rows.push(
            text(format!("Error: {error}"))
                .with_classes("error")
                .into_view(),
        );
    }

    div(rows)
}
