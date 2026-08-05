//! Selected and unselected explorer entry presentation.

use leptatui::prelude::*;

use crate::services::{ExplorerEntry, ExplorerEntryKind};

use super::style::use_explorer_entry_row_styles;

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
    use_explorer_entry_row_styles();

    let (marker, modifier) = match entry.kind() {
        ExplorerEntryKind::Directory => ("[D]", "explorer-entry--directory"),
        ExplorerEntryKind::Markdown => ("[M]", "explorer-entry--markdown"),
    };
    let selection_marker = if selected { ">" } else { " " };
    let classes = if selected {
        format!("explorer-entry {modifier} explorer-entry--selected")
    } else {
        format!("explorer-entry {modifier}")
    };

    let label = format!(
        "{selection_marker} {marker} {}",
        entry.name().to_string_lossy()
    );

    view! {
        <Text class={classes}>{label}</Text>
    }
}
