//! Actionable recent-file entry.

use std::path::PathBuf;

use leptatui::prelude::*;

use crate::pages::{shared::relative_path, viewer_location};

/// Renders one actionable recent-file row.
///
/// # Arguments
///
/// * `path` — Canonical recent Markdown path.
/// * `base` — Current directory used to shorten the displayed path.
///
/// # Returns
///
/// A button that opens `path` in Viewer.
#[component]
pub(in crate::pages::home) fn RecentFileEntry(path: PathBuf, base: PathBuf) -> impl IntoView {
    let navigate = use_navigate();
    let label = relative_path(&base, &path);
    let target = viewer_location(&path);

    view! {
        <Button on_press=move || {
            navigate(&target, NavigateOptions::default());
            AppControl::Continue
        }>{label}</Button>
    }
}
