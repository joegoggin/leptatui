//! Recent-file persistence tests.

use std::fs;

use crate::services::{RECENT_FILE_LIMIT, RecentFilesStore};

use super::support::TestTree;

/// Verifies the store persists bounded, deduplicated MRU file order.
///
/// # Example Under Test
///
/// ```text
/// record file-00.md through file-11.md, then reopen file-05.md
/// ```
///
/// # Assertions
///
/// - Each fixture is created, canonicalized, and recorded successfully.
/// - Recent history loads successfully after initial and promoted records.
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
    for path in &paths {
        store.record(path).expect("each file should be recorded");
    }
    let recent = store
        .load_valid()
        .expect("bounded recent history should load");
    assert_eq!(recent.len(), RECENT_FILE_LIMIT);
    assert_eq!(recent[0], paths[11]);
    assert!(!recent.contains(&paths[0]));
    assert!(!recent.contains(&paths[1]));

    store
        .record(&paths[5])
        .expect("the reopened file should be promoted");
    let expected = store
        .load_valid()
        .expect("promoted recent history should load");
    assert_eq!(expected[0], paths[5]);
    assert_eq!(expected.iter().filter(|path| *path == &paths[5]).count(), 1);

    let restored = RecentFilesStore::at(store_path)
        .load_valid()
        .expect("persisted recent history should load");
    assert_eq!(restored, expected);
}

/// Verifies recent history filters invalid paths but remains global.
///
/// # Example Under Test
///
/// ```text
/// recent files = [guide.md, notes.txt, missing.md, ../outside.md]
/// ```
///
/// # Assertions
///
/// - The fixtures are created, canonicalized, and persisted successfully.
/// - The persisted fixture loads successfully.
/// - Valid Markdown files inside and outside the current directory remain.
/// - Non-Markdown and missing paths are omitted.
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

    let recent = store
        .load_valid()
        .expect("valid persisted history should load");
    assert_eq!(recent, [canonical_guide, canonical_outside]);
}

/// Verifies history records files from different directory trees.
///
/// # Example Under Test
///
/// ```text
/// record first/alpha.md, then second/beta.md
/// ```
///
/// # Assertions
///
/// - Both files are created and recorded successfully.
/// - Recent history loads successfully.
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

    assert_eq!(
        store
            .load_valid()
            .expect("global recent history should load"),
        [canonical_beta, canonical_alpha]
    );
}

/// Verifies malformed recent data propagates through loading and recording.
///
/// # Example Under Test
///
/// ```text
/// recent-files.json = {bad json
/// record guide.md
/// ```
///
/// # Assertions
///
/// - The Markdown and malformed-state fixtures are created successfully.
/// - Loading returns a contextual parse error.
/// - Recording returns the same parse error instead of replacing the document.
#[test]
fn malformed_recent_data_propagates_from_load_and_record() {
    let tree = TestTree::new("recent-malformed");
    let guide = tree.root().join("guide.md");
    let store_path = tree.root().join("recent-files.json");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    fs::write(&store_path, "{bad json").expect("the malformed state should be created");
    let store = RecentFilesStore::at(store_path);

    let load_error = store
        .load_valid()
        .expect_err("malformed recent history should fail to load");
    assert!(
        load_error
            .to_string()
            .contains("failed to parse recent files")
    );

    let record_error = store
        .record(&guide)
        .expect_err("recording should propagate malformed recent history");
    assert!(
        record_error
            .to_string()
            .contains("failed to parse recent files")
    );
}

/// Verifies inaccessible recent-file storage returns a contextual error.
///
/// # Example Under Test
///
/// ```text
/// blocked = regular file
/// storage = blocked/recent-files.json
/// ```
///
/// # Assertions
///
/// - The Markdown fixture and blocked parent are created successfully.
/// - Recording fails when the storage parent is a regular file.
/// - The diagnostic identifies the failed recent-history read.
#[test]
fn recent_storage_parent_failure_is_propagated() {
    let tree = TestTree::new("recent-write-failure");
    let guide = tree.root().join("guide.md");
    let blocked_parent = tree.root().join("blocked");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    fs::write(&blocked_parent, "not a directory").expect("the blocked parent should be created");
    let store = RecentFilesStore::at(blocked_parent.join("recent-files.json"));

    let error = store
        .record(&guide)
        .expect_err("the blocked storage directory should reject writes");
    assert!(error.to_string().contains("failed to read recent files"));
}
