//! Markdown-specific filtering tests over Leptatui filesystem results.

use std::{fs, time::Duration};

use leptatui::prelude::{GetUntracked, Owner, WithUntracked, use_file_system};
use tokio::{task::yield_now, time::timeout};

use crate::services::{DirectoryListing, ExplorerEntryKind};

use super::support::{TestTree, explorer_entry_names};

/// Verifies generic filesystem entries become ordered Markdown explorer entries.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// ├── Alpha/
/// ├── zeta/
/// ├── Guide.markdown
/// ├── notes.MD
/// └── ignored.txt
/// ```
///
/// # Assertions
///
/// - Directories sort before Markdown files.
/// - Names sort case-insensitively inside each kind.
/// - Both supported Markdown extensions are retained.
/// - The unrelated text file is omitted.
#[tokio::test(flavor = "current_thread")]
async fn explorer_filters_and_orders_generic_file_entries() {
    let tree = TestTree::new("ordered-listing");
    fs::create_dir(tree.root().join("zeta")).expect("zeta directory should be created");
    fs::create_dir(tree.root().join("Alpha")).expect("Alpha directory should be created");
    fs::write(tree.root().join("notes.MD"), "# Notes")
        .expect("uppercase Markdown file should be created");
    fs::write(tree.root().join("Guide.markdown"), "# Guide")
        .expect("long-extension Markdown file should be created");
    fs::write(tree.root().join("ignored.txt"), "ignored")
        .expect("unrelated file should be created");
    let filesystem = use_file_system(tree.root()).expect("fixture root should initialize");
    let owner = Owner::new();
    let action = owner.with(|| filesystem.read_dir(""));
    timeout(Duration::from_secs(2), async {
        while action.version().get_untracked() == 0 {
            yield_now().await;
        }
    })
    .await
    .expect("directory action should complete");

    action.value().with_untracked(|result| {
        let entries = result
            .as_ref()
            .expect("directory action should retain a result")
            .as_ref()
            .expect("directory listing should succeed")
            .clone();
        let listing = DirectoryListing::from_file_entries(filesystem.root().to_path_buf(), entries);
        assert_eq!(
            explorer_entry_names(listing.entries()),
            ["Alpha", "zeta", "Guide.markdown", "notes.MD"]
        );
        assert_eq!(
            listing
                .entries()
                .iter()
                .map(|entry| entry.kind())
                .collect::<Vec<_>>(),
            [
                ExplorerEntryKind::Directory,
                ExplorerEntryKind::Directory,
                ExplorerEntryKind::Markdown,
                ExplorerEntryKind::Markdown,
            ]
        );
    });
}

/// Verifies an empty generic listing remains a successful empty explorer state.
///
/// # Example Under Test
///
/// ```text
/// empty-workspace/
/// ```
///
/// # Assertions
///
/// - The listing retains the canonical directory.
/// - No explorer entries are produced.
#[test]
fn explorer_represents_empty_directory() {
    let tree = TestTree::new("empty-listing");
    let canonical = fs::canonicalize(tree.root()).expect("empty root should canonicalize");
    let listing = DirectoryListing::from_file_entries(canonical.clone(), Vec::new());

    assert_eq!(listing.directory(), canonical);
    assert!(listing.entries().is_empty());
}
