//! Recent-file persistence and shared-file-hook integration tests.

use std::{fs, path::Path};

use leptatui::prelude::GetUntracked;
use ratatui::{Terminal, backend::TestBackend};

use crate::{
    pages::viewer_location,
    services::{RECENT_FILE_LIMIT, RecentFilesStore},
};

use super::support::{TestContexts, TestTree, draw_editor};

/// Opens one route so Viewer synchronizes the shared recent-file signals.
///
/// # Arguments
///
/// * `contexts` — Shared application fixture supplied to the Viewer.
/// * `path` — Markdown path represented by the Viewer route.
fn open(contexts: &TestContexts, path: &Path) {
    let route = viewer_location(contexts.workspace.root(), path);
    let view = contexts.view_at(route);
    let mut terminal =
        Terminal::new(TestBackend::new(80, 18)).expect("the terminal should initialize");
    draw_editor(&mut terminal, &view).expect("the Viewer should render");
}

/// Verifies Viewer persists bounded, deduplicated MRU file order.
///
/// # Example Under Test
///
/// ```text
/// view file-00.md through file-11.md
/// view file-05.md
/// rebuild contexts from the same store
/// ```
///
/// # Assertions
///
/// - The visible context retains at most the configured limit.
/// - Reopening moves one path to the front without duplication.
/// - Reinitialized file signals restore the same ordering.
#[test]
fn recent_files_persist_in_bounded_mru_order() {
    let tree = TestTree::new("recent-mru");
    let store = RecentFilesStore::at(tree.root().join("state").join("recent-files.json"));
    let paths = (0..12)
        .map(|index| {
            let path = tree.root().join(format!("file-{index:02}.md"));
            fs::write(&path, format!("# File {index}"))
                .expect("each Markdown file should be created");
            fs::canonicalize(path).expect("each Markdown path should canonicalize")
        })
        .collect::<Vec<_>>();
    let contexts = TestContexts::with_store(tree.root(), store.clone());

    for path in &paths {
        open(&contexts, path);
    }
    let recent = contexts.files.recent_files.get_untracked();
    assert_eq!(recent.len(), RECENT_FILE_LIMIT);
    assert_eq!(recent[0], paths[11]);
    assert!(!recent.contains(&paths[0]));
    assert!(!recent.contains(&paths[1]));

    open(&contexts, &paths[5]);
    let expected = contexts.files.recent_files.get_untracked();
    assert_eq!(expected[0], paths[5]);
    assert_eq!(expected.iter().filter(|path| *path == &paths[5]).count(), 1);

    let restored = TestContexts::with_store(tree.root(), store);
    assert_eq!(restored.files.recent_files.get_untracked(), expected);
}

/// Verifies startup filters invalid persisted paths for the active workspace.
///
/// # Example Under Test
///
/// ```text
/// guide.md, notes.txt, missing.md, ../outside.md
/// ```
///
/// # Assertions
///
/// - Only the valid in-workspace Markdown path remains visible.
/// - Filtering valid persisted data does not create an error.
#[test]
fn recent_files_filter_invalid_and_out_of_workspace_paths() {
    let tree = TestTree::new("recent-filter");
    let outside = TestTree::new("recent-outside");
    let guide = tree.root().join("guide.md");
    let notes = tree.root().join("notes.txt");
    let outside_markdown = outside.root().join("outside.md");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    fs::write(&notes, "notes").expect("the text file should be created");
    fs::write(&outside_markdown, "# Outside").expect("the outside Markdown file should be created");
    let canonical_guide = fs::canonicalize(&guide).expect("guide should canonicalize");
    let store = RecentFilesStore::at(tree.root().join("state.json"));
    store
        .save(&[
            canonical_guide.clone(),
            fs::canonicalize(&notes).expect("notes should canonicalize"),
            tree.root().join("missing.md"),
            fs::canonicalize(&outside_markdown).expect("outside path should canonicalize"),
        ])
        .expect("the recent document should be written");

    let contexts = TestContexts::with_store(tree.root(), store);
    assert_eq!(
        contexts.files.recent_files.get_untracked(),
        [canonical_guide]
    );
    assert_eq!(contexts.files.recent_files_error.get_untracked(), None);
}

/// Verifies persisted history retains entries belonging to other workspaces.
///
/// # Example Under Test
///
/// ```text
/// workspace-a views alpha.md
/// workspace-b views beta.md
/// workspace-a reloads shared storage
/// ```
///
/// # Assertions
///
/// - Workspace B initially hides Workspace A's entry.
/// - Viewing in Workspace B preserves the shared persisted history.
/// - Reinitialized Workspace A file signals restore `alpha.md`.
#[test]
fn recent_files_preserve_entries_for_other_workspaces() {
    let first = TestTree::new("recent-first-workspace");
    let second = TestTree::new("recent-second-workspace");
    let storage = TestTree::new("recent-shared-state");
    let alpha = first.root().join("alpha.md");
    let beta = second.root().join("beta.md");
    fs::write(&alpha, "# Alpha").expect("the first Markdown file should be created");
    fs::write(&beta, "# Beta").expect("the second Markdown file should be created");
    let canonical_alpha = fs::canonicalize(&alpha).expect("alpha should canonicalize");
    let store = RecentFilesStore::at(storage.root().join("recent-files.json"));

    let first_contexts = TestContexts::with_store(first.root(), store.clone());
    open(&first_contexts, &alpha);

    let second_contexts = TestContexts::with_store(second.root(), store.clone());
    assert!(
        second_contexts
            .files
            .recent_files
            .get_untracked()
            .is_empty()
    );
    open(&second_contexts, &beta);

    let restored = TestContexts::with_store(first.root(), store);
    assert_eq!(
        restored.files.recent_files.get_untracked(),
        [canonical_alpha]
    );
}

/// Verifies malformed recent data recovers after Viewer opens a document.
///
/// # Example Under Test
///
/// ```text
/// recent-files.json = "{bad json"
/// view guide.md
/// ```
///
/// # Assertions
///
/// - Initialization exposes the parse warning with an empty list.
/// - Viewing a document clears the warning and stores the path.
/// - Reinitialized file signals load the repaired document.
#[test]
fn malformed_recent_data_recovers_after_a_successful_open() {
    let tree = TestTree::new("recent-malformed");
    let guide = tree.root().join("guide.md");
    let store_path = tree.root().join("recent-files.json");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    fs::write(&store_path, "{bad json").expect("the malformed state should be created");
    let store = RecentFilesStore::at(store_path);
    let contexts = TestContexts::with_store(tree.root(), store.clone());

    assert!(contexts.files.recent_files.get_untracked().is_empty());
    assert!(
        contexts
            .files
            .recent_files_error
            .get_untracked()
            .expect("the parse error should be retained")
            .contains("failed to parse recent files")
    );

    open(&contexts, &guide);
    assert_eq!(contexts.files.recent_files_error.get_untracked(), None);
    assert_eq!(contexts.files.recent_files.get_untracked().len(), 1);

    let restored = TestContexts::with_store(tree.root(), store);
    assert_eq!(restored.files.recent_files.get_untracked().len(), 1);
}

/// Verifies recent-file write failures preserve in-memory shared signals.
///
/// # Example Under Test
///
/// ```text
/// storage parent is a regular file
/// view guide.md
/// ```
///
/// # Assertions
///
/// - The visible recent context contains the document.
/// - The recent-file error context exposes the storage warning.
#[test]
fn recent_write_failure_preserves_in_memory_history() {
    let tree = TestTree::new("recent-write-failure");
    let guide = tree.root().join("guide.md");
    let blocked_parent = tree.root().join("blocked");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    fs::write(&blocked_parent, "not a directory").expect("the blocked parent should be created");
    let store = RecentFilesStore::at(blocked_parent.join("recent-files.json"));
    let contexts = TestContexts::with_store(tree.root(), store);

    open(&contexts, &guide);
    assert_eq!(contexts.files.recent_files.get_untracked().len(), 1);
    assert!(
        contexts
            .files
            .recent_files_error
            .get_untracked()
            .expect("the write error should be retained")
            .contains("failed to create recent-files directory")
    );
}
