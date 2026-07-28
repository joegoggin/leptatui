//! Recent-file list, empty state, and persistence warning.

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use leptatui::prelude::*;

use crate::{core::Controller, core::RecentFilesState};

use super::{RecentFileEntry, RecentFileEntryProps};

/// Renders the recent-file section on Home.
///
/// # Arguments
///
/// * `state` — Recent paths and persistence error.
/// * `root` — Active workspace root.
/// * `controller` — Shared application state used to open a recent path.
///
/// # Returns
///
/// A recent-file list with an empty state or warning when applicable.
#[component]
pub(in crate::pages::home) fn RecentFilesList(
    state: RecentFilesState,
    root: PathBuf,
    controller: Rc<RefCell<Controller>>,
) -> impl IntoView {
    let mut rows = vec![
        text("Recent files")
            .with_classes("section-title")
            .into_view(),
    ];

    if state.entries().is_empty() {
        rows.push(
            text("No recent Markdown files")
                .with_classes("empty")
                .into_view(),
        );
    } else {
        rows.extend(state.entries().iter().cloned().map(|path| {
            RecentFileEntry::with_props(
                RecentFileEntryProps::builder()
                    .path(path)
                    .root(root.clone())
                    .controller(Rc::clone(&controller))
                    .build(),
            )
            .into_view()
        }));
    }

    if let Some(error) = state.error() {
        rows.push(
            text(format!("Recent files warning: {error}"))
                .with_classes("error")
                .into_view(),
        );
    }

    div(rows)
}
