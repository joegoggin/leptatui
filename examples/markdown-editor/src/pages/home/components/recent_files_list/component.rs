//! Recent-file list, empty state, and persistence warning.

use std::path::PathBuf;

use leptatui::prelude::*;

use super::{
    super::{RecentFileEntry, RecentFileEntryProps},
    style::use_recent_files_list_styles,
};

/// Renders the recent-file section on Home.
///
/// # Arguments
///
/// * `entries` — Recent paths in most-recent-first order.
/// * `error` — Optional recoverable persistence error.
/// * `root` — Active workspace root.
///
/// # Returns
///
/// A recent-file list with an empty state or warning when applicable.
#[component]
pub(in crate::pages::home) fn RecentFilesList(
    entries: Vec<PathBuf>,
    error: Option<String>,
    root: PathBuf,
) -> impl IntoView {
    use_recent_files_list_styles();

    let mut rows = vec![
        view! {
            <Text class="recent-files__title">"Recent files"</Text>
        }
        .into_view(),
    ];

    if entries.is_empty() {
        rows.push(
            view! {
                <Text class="recent-files__empty">"No recent Markdown files"</Text>
            }
            .into_view(),
        );
    } else {
        rows.extend(entries.iter().map(|path| {
            let entry_path = PathBuf::clone(path);
            let entry_root = root.clone();

            view! {
                <RecentFileEntry path=entry_path root=entry_root />
            }
            .into_view()
        }));
    }

    if let Some(error) = error {
        rows.push(
            view! {
                <Text class="recent-files__error">{format!("Recent files warning: {error}")}</Text>
            }
            .into_view(),
        );
    }

    div(rows).with_classes("recent-files")
}
