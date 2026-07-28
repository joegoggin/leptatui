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
        view! {
            <Text class="section-title">"Recent files"</Text>
        }
        .into_view(),
    ];

    if state.entries().is_empty() {
        rows.push(
            view! {
                <Text class="empty">"No recent Markdown files"</Text>
            }
            .into_view(),
        );
    } else {
        rows.extend(state.entries().iter().map(|path| {
            let entry_path = PathBuf::clone(path);
            let entry_root = root.clone();
            let entry_controller = Rc::clone(&controller);

            view! {
                <RecentFileEntry
                    path=entry_path
                    root=entry_root
                    controller=entry_controller
                />
            }
            .into_view()
        }));
    }

    if let Some(error) = state.error() {
        rows.push(
            view! {
                <Text class="error">{format!("Recent files warning: {error}")}</Text>
            }
            .into_view(),
        );
    }

    div(rows)
}
