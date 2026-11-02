//! Recent-file list and empty state.

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
/// * `base` — Current directory used to shorten displayed paths.
/// * `on_open` — Home-owned callback that records and opens a path.
///
/// # Returns
///
/// A recent-file list with an empty state when applicable.
#[component]
pub(in crate::pages::home) fn RecentFilesList(
    entries: Prop<Vec<PathBuf>>,
    base: Prop<PathBuf>,
    #[prop(callback)] on_open: Rc<dyn Fn(PathBuf)>,
) -> impl IntoView {
    use_recent_files_list_styles();

    view! {
        <Div class="recent-files">
            <Text class="recent-files__title">"Recent files"</Text>
            {if entries.get().is_empty() {
                vec![view! { <Text class="recent-files__empty">"No recent Markdown files"</Text> }.into_view()]
            } else {
                entries.get().into_iter().map(|path| {
                    let entry_base = base.clone();
                    let entry_on_open = Rc::clone(&on_open);
                    view! {
                        <RecentFileEntry path=path base={entry_base.get()} on_open=entry_on_open />
                    }.into_view()
                }).collect::<Vec<_>>()
            }}
        </Div>
    }
}
