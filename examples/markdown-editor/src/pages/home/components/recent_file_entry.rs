//! Actionable recent-file entry.

use std::{path::PathBuf, rc::Rc};

use leptatui::prelude::*;

use crate::pages::shared::relative_path;

/// Renders one actionable recent-file row.
///
/// # Arguments
///
/// * `path` — Canonical recent Markdown path.
/// * `base` — Current directory used to shorten the displayed path.
/// * `on_open` — Home-owned callback that records and opens the path.
///
/// # Returns
///
/// A button that opens `path` in Viewer.
#[component]
pub(in crate::pages::home) fn RecentFileEntry(
    path: PathBuf,
    base: PathBuf,
    on_open: Rc<dyn Fn(PathBuf)>,
) -> impl IntoView {
    let label = relative_path(&base, &path);

    view! {
        <Button on_press=move || {
            on_open(path.clone());
            AppControl::Continue
        }>{label}</Button>
    }
}
