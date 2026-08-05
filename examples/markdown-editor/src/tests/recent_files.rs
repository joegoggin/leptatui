//! Recent-file persistence and Viewer integration tests.

use std::{fs, path::Path};

use ratatui::{Terminal, backend::TestBackend};

use crate::{
    pages::viewer_location,
    services::{RECENT_FILE_LIMIT, RecentFilesStore},
};

use super::support::{TestContexts, TestTree, draw_editor};

/// Opens one Viewer route so a successful read records its path.
///
/// # Arguments
///
/// * `contexts` — Application fixture supplied to the Viewer.
/// * `path` — Markdown path represented by the Viewer route.
fn open(contexts: &TestContexts, path: &Path) {
    let view = contexts.view_at(viewer_location(path));
    let mut terminal =
        Terminal::new(TestBackend::new(80, 18)).expect("the terminal should initialize");
    draw_editor(&mut terminal, &view).expect("the Viewer should render");
}

/// Verifies Viewer persists bounded, deduplicated MRU file order.
///
/// # Assertions
///
/// - Persisted history retains at most the configured limit.
/// - Reopening moves one path to the front without duplication.
/// - A new store instance restores the same ordering.
#[test]
fn recent_files_persist_in_bounded_mru_order() {
    let tree = TestTree::new("recent-mru");
    let store_path = tree.root().join("state").join("recent-files.json");
    let store = RecentFilesStore::at(store_path.clone());
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
    let (recent, error) = store.load_valid();
    assert!(error.is_none());
    assert_eq!(recent.len(), RECENT_FILE_LIMIT);
    assert_eq!(recent[0], paths[11]);
    assert!(!recent.contains(&paths[0]));
    assert!(!recent.contains(&paths[1]));

    open(&contexts, &paths[5]);
    let expected = store.load_valid().0;
    assert_eq!(expected[0], paths[5]);
    assert_eq!(expected.iter().filter(|path| *path == &paths[5]).count(), 1);

    let restored = RecentFilesStore::at(store_path).load_valid().0;
    assert_eq!(restored, expected);
}

/// Verifies recent history filters invalid paths but remains global.
///
/// # Assertions
///
/// - Valid Markdown files inside and outside the current directory remain.
/// - Non-Markdown and missing paths are omitted.
/// - Filtering otherwise valid persisted data does not create an error.
#[test]
fn recent_files_filter_invalid_paths_without_directory_scoping() {
    let tree = TestTree::new("recent-filter");
    let outside = TestTree::new("recent-outside");
    let guide = tree.root().join("guide.md");
    let notes = tree.root().join("notes.txt");
    let outside_markdown = outside.root().join("outside.md");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    fs::write(&notes, "notes").expect("the text file should be created");
    fs::write(&outside_markdown, "# Outside").expect("the outside file should be created");
    let canonical_guide = fs::canonicalize(&guide).expect("guide should canonicalize");
    let canonical_outside =
        fs::canonicalize(&outside_markdown).expect("outside path should canonicalize");
    let store = RecentFilesStore::at(tree.root().join("state.json"));
    store
        .save(&[
            canonical_guide.clone(),
            fs::canonicalize(&notes).expect("notes should canonicalize"),
            tree.root().join("missing.md"),
            canonical_outside.clone(),
        ])
        .expect("the recent document should be written");

    let (recent, error) = store.load_valid();
    assert_eq!(recent, [canonical_guide, canonical_outside]);
    assert!(error.is_none());
}

/// Verifies history records files from different directory trees.
///
/// # Assertions
///
/// - Both valid paths remain in one global MRU list.
/// - The most recently opened path sorts first.
#[test]
fn recent_files_preserve_global_history() {
    let first = TestTree::new("recent-first-directory");
    let second = TestTree::new("recent-second-directory");
    let storage = TestTree::new("recent-shared-state");
    let alpha = first.root().join("alpha.md");
    let beta = second.root().join("beta.md");
    fs::write(&alpha, "# Alpha").expect("the first Markdown file should be created");
    fs::write(&beta, "# Beta").expect("the second Markdown file should be created");
    let canonical_alpha = fs::canonicalize(&alpha).expect("alpha should canonicalize");
    let canonical_beta = fs::canonicalize(&beta).expect("beta should canonicalize");
    let store = RecentFilesStore::at(storage.root().join("recent-files.json"));

    store.record(&alpha).expect("alpha should be recorded");
    store.record(&beta).expect("beta should be recorded");

    assert_eq!(store.load_valid().0, [canonical_beta, canonical_alpha]);
}

/// Verifies malformed recent data is replaced by the next successful record.
///
/// # Assertions
///
/// - Loading exposes the parse warning with an empty list.
/// - Recording a document repairs the persisted history.
#[test]
fn malformed_recent_data_recovers_after_a_successful_open() {
    let tree = TestTree::new("recent-malformed");
    let guide = tree.root().join("guide.md");
    let store_path = tree.root().join("recent-files.json");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    fs::write(&store_path, "{bad json").expect("the malformed state should be created");
    let store = RecentFilesStore::at(store_path);

    let (recent, error) = store.load_valid();
    assert!(recent.is_empty());
    assert!(
        error
            .expect("the parse error should be retained")
            .to_string()
            .contains("failed to parse recent files")
    );

    store
        .record(&guide)
        .expect("a successful record should repair history");
    let (restored, error) = store.load_valid();
    assert_eq!(restored.len(), 1);
    assert!(error.is_none());
}

/// Verifies recent-file write failures return a contextual storage error.
///
/// # Assertions
///
/// - Recording fails when the storage parent is a regular file.
/// - The diagnostic identifies directory creation.
#[test]
fn recent_write_failure_is_recoverable() {
    let tree = TestTree::new("recent-write-failure");
    let guide = tree.root().join("guide.md");
    let blocked_parent = tree.root().join("blocked");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    fs::write(&blocked_parent, "not a directory").expect("the blocked parent should be created");
    let store = RecentFilesStore::at(blocked_parent.join("recent-files.json"));

    let error = store
        .record(&guide)
        .expect_err("the blocked storage directory should reject writes");
    assert!(
        error
            .to_string()
            .contains("failed to create recent-files directory")
    );
}
