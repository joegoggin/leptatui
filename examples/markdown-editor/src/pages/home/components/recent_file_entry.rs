//! Actionable recent-file entry.

use std::path::PathBuf;

use leptatui::prelude::*;

use crate::pages::{shared::relative_path, viewer_location};

/// Renders one actionable recent-file row.
///
/// # Arguments
///
/// * `path` — Canonical recent Markdown path.
/// * `root` — Active workspace root.
///
/// # Returns
///
/// A button that opens `path` in Viewer.
#[component]
pub(in crate::pages::home) fn RecentFileEntry(path: PathBuf, root: PathBuf) -> impl IntoView {
    let navigate = use_navigate();
    let label = relative_path(&root, &path);
    let target = viewer_location(&root, &path);

    view! {
        <Button on_press=move || {
            navigate(&target, NavigateOptions::default());
            AppControl::Continue
        }>{label}</Button>
    }
}
