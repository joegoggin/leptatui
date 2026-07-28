//! Actionable recent-file entry.

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use leptatui::prelude::*;

use crate::{
    core::Controller,
    pages::{shared::relative_path, viewer_location},
};

/// Renders one actionable recent-file row.
///
/// # Arguments
///
/// * `path` — Canonical recent Markdown path.
/// * `root` — Active workspace root.
/// * `controller` — Shared application state.
///
/// # Returns
///
/// A button that opens `path` in Viewer.
#[component]
pub(in crate::pages::home) fn RecentFileEntry(
    path: PathBuf,
    root: PathBuf,
    controller: Rc<RefCell<Controller>>,
) -> impl IntoView {
    let navigate = use_navigate();
    let label = relative_path(&root, &path);
    let target = viewer_location(&root, &path);

    view! {
        <Button on_press=move || {
            controller.borrow_mut().open_recent(&path);
            navigate(&target, NavigateOptions::default());
            AppControl::Continue
        }>{label}</Button>
    }
}
