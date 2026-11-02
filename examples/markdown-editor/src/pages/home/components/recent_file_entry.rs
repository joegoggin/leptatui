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
    path: Prop<PathBuf>,
    base: Prop<PathBuf>,
    #[prop(callback)] on_open: Rc<dyn Fn(PathBuf)>,
) -> impl IntoView {
    let open_path = path.clone();

    view! {
        <Button on_press=move || {
            on_open(open_path.get());
            AppControl::Continue
        }>{relative_path(&base.get(), &path.get())}</Button>
    }
}
