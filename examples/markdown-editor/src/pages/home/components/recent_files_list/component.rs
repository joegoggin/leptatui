//! Recent-file list, empty state, and persistence warning.

use std::{path::PathBuf, rc::Rc};

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
/// * `base` — Current directory used to shorten displayed paths.
/// * `on_open` — Home-owned callback that records and opens a path.
///
/// # Returns
///
/// A recent-file list with an empty state or warning when applicable.
#[component]
pub(in crate::pages::home) fn RecentFilesList(
    entries: Vec<PathBuf>,
    error: Option<String>,
    base: PathBuf,
    on_open: Rc<dyn Fn(PathBuf)>,
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
            let entry_base = base.clone();
            let entry_on_open = Rc::clone(&on_open);

            view! {
                <RecentFileEntry
                    path=entry_path
                    base=entry_base
                    on_open=entry_on_open
                />
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
