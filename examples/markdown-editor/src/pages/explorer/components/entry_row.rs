//! Selected and unselected explorer entry presentation.

use leptatui::prelude::*;

use crate::services::{ExplorerEntry, ExplorerEntryKind};

/// Renders one selected or unselected explorer entry.
///
/// # Arguments
///
/// * `entry` — Safe discovered filesystem entry.
/// * `selected` — Whether the entry is highlighted.
///
/// # Returns
///
/// A styled explorer row.
#[component]
pub(in crate::pages::explorer) fn ExplorerEntryRow(
    entry: ExplorerEntry,
    selected: bool,
) -> impl IntoView {
    let (marker, class) = match entry.kind() {
        ExplorerEntryKind::Directory => ("[D]", "directory-entry"),
        ExplorerEntryKind::Markdown => ("[M]", "markdown-entry"),
    };
    let selection_marker = if selected { ">" } else { " " };
    let classes = if selected {
        format!("{class} selected")
    } else {
        String::from(class)
    };

    let label = format!(
        "{selection_marker} {marker} {}",
        entry.name().to_string_lossy()
    );

    view! {
        <Text class={classes}>{label}</Text>
    }
}
