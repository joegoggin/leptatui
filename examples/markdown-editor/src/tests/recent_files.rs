//! Recent-file persistence and controller integration tests.

use std::fs;

use crate::{
    controller::Controller, domain::RECENT_FILE_LIMIT, editor_process::EditorProcess,
    filesystem::FileSystem, recent_files::RecentFilesStore,
};

use super::support::TestTree;

/// Verifies recent files persist in bounded, deduplicated MRU order.
///
/// # Example Under Test
///
/// ```text
/// open file-00.md through file-11.md
/// reopen file-05.md
/// restart controller
/// ```
///
/// # Assertions
///
/// - Successful opens retain at most ten recent paths.
/// - Reopening a path moves it to the front without duplication.
/// - A new controller restores the same ordering from disk.
#[test]
fn recent_files_persist_in_bounded_mru_order() {
    let tree = TestTree::new("recent-mru");
    let store_path = tree.root().join("state").join("recent-files.json");
    let store = RecentFilesStore::at(store_path);
    let paths = (0..12)
        .map(|index| {
            let path = tree.root().join(format!("file-{index:02}.md"));
            fs::write(&path, format!("# File {index}"))
                .expect("each Markdown file should be created");
            fs::canonicalize(path).expect("each Markdown path should canonicalize")
        })
        .collect::<Vec<_>>();
    let mut controller = Controller::initialize_with_store(
        tree.root(),
        FileSystem::new(),
        EditorProcess::new(),
        store.clone(),
    )
    .expect("the workspace should initialize");

    for path in &paths {
        assert!(controller.open_recent(path));
    }
    assert_eq!(controller.recent_files().entries().len(), RECENT_FILE_LIMIT);
    assert_eq!(controller.recent_files().entries()[0], paths[11]);
    assert!(!controller.recent_files().entries().contains(&paths[0]));
    assert!(!controller.recent_files().entries().contains(&paths[1]));

    assert!(controller.open_recent(&paths[5]));
    assert_eq!(controller.recent_files().entries()[0], paths[5]);
    assert_eq!(
        controller
            .recent_files()
            .entries()
            .iter()
            .filter(|path| *path == &paths[5])
            .count(),
        1
    );

    let restored = Controller::initialize_with_store(
        tree.root(),
        FileSystem::new(),
        EditorProcess::new(),
        store,
    )
    .expect("the workspace should restore");
    assert_eq!(
        restored.recent_files().entries(),
        controller.recent_files().entries()
    );
}

/// Verifies startup filters recent entries through the workspace boundary.
///
/// # Example Under Test
///
/// ```text
/// workspace/guide.md
/// workspace/notes.txt
/// workspace/missing.md
/// outside/outside.md
/// ```
///
/// # Assertions
///
/// - The persisted document loads successfully.
/// - Only the existing in-workspace Markdown path remains visible.
/// - Unsupported, missing, and out-of-workspace paths are omitted.
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

    let controller = Controller::initialize_with_store(
        tree.root(),
        FileSystem::new(),
        EditorProcess::new(),
        store,
    )
    .expect("the workspace should initialize");

    assert_eq!(controller.recent_files().entries(), [canonical_guide]);
    assert_eq!(controller.recent_files().error(), None);
}

/// Verifies opening files in one workspace preserves history for another.
///
/// # Example Under Test
///
/// ```text
/// workspace-a: open alpha.md
/// workspace-b: open beta.md
/// workspace-a: restart
/// ```
///
/// # Assertions
///
/// - Workspace B does not display Workspace A's recent path.
/// - Opening a file in Workspace B retains both paths in persisted storage.
/// - Restarting Workspace A restores its original recent file.
#[test]
fn recent_files_preserve_entries_for_other_workspaces() {
    let first = TestTree::new("recent-first-workspace");
    let second = TestTree::new("recent-second-workspace");
    let state = TestTree::new("recent-shared-state");
    let alpha = first.root().join("alpha.md");
    let beta = second.root().join("beta.md");
    fs::write(&alpha, "# Alpha").expect("the first Markdown file should be created");
    fs::write(&beta, "# Beta").expect("the second Markdown file should be created");
    let canonical_alpha = fs::canonicalize(&alpha).expect("alpha should canonicalize");
    let store = RecentFilesStore::at(state.root().join("recent-files.json"));

    let mut first_controller = Controller::initialize_with_store(
        first.root(),
        FileSystem::new(),
        EditorProcess::new(),
        store.clone(),
    )
    .expect("the first workspace should initialize");
    assert!(first_controller.open_recent(&alpha));

    let mut second_controller = Controller::initialize_with_store(
        second.root(),
        FileSystem::new(),
        EditorProcess::new(),
        store.clone(),
    )
    .expect("the second workspace should initialize");
    assert!(second_controller.recent_files().entries().is_empty());
    assert!(second_controller.open_recent(&beta));

    let restored_first = Controller::initialize_with_store(
        first.root(),
        FileSystem::new(),
        EditorProcess::new(),
        store,
    )
    .expect("the first workspace should restore");
    assert_eq!(restored_first.recent_files().entries(), [canonical_alpha]);
}

/// Verifies malformed recent data is recoverable and replaced after an open.
///
/// # Example Under Test
///
/// ```text
/// recent-files.json = "{bad json"
/// open guide.md
/// ```
///
/// # Assertions
///
/// - Controller initialization succeeds with an empty recent list.
/// - Home-facing recent state contains a parse warning.
/// - A successful open records the document and clears the warning.
/// - The repaired recent document can be loaded by a new controller.
#[test]
fn malformed_recent_data_recovers_after_a_successful_open() {
    let tree = TestTree::new("recent-malformed");
    let guide = tree.root().join("guide.md");
    let store_path = tree.root().join("recent-files.json");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    fs::write(&store_path, "{bad json").expect("the malformed state should be created");
    let store = RecentFilesStore::at(store_path);
    let mut controller = Controller::initialize_with_store(
        tree.root(),
        FileSystem::new(),
        EditorProcess::new(),
        store.clone(),
    )
    .expect("the workspace should initialize despite malformed recents");

    assert!(controller.recent_files().entries().is_empty());
    assert!(
        controller
            .recent_files()
            .error()
            .expect("the parse error should be retained")
            .contains("failed to parse recent files")
    );

    assert!(controller.open_recent(&guide));
    assert_eq!(controller.recent_files().error(), None);
    assert_eq!(controller.recent_files().entries().len(), 1);

    let restored = Controller::initialize_with_store(
        tree.root(),
        FileSystem::new(),
        EditorProcess::new(),
        store,
    )
    .expect("the repaired recent document should load");
    assert_eq!(restored.recent_files().entries().len(), 1);
}

/// Verifies recent-file write failures do not block document opening.
///
/// # Example Under Test
///
/// ```text
/// store parent path = regular file
/// open guide.md
/// ```
///
/// # Assertions
///
/// - The Markdown document opens successfully.
/// - The path remains in in-memory recent state.
/// - Recent state exposes a recoverable storage warning.
#[test]
fn recent_write_failure_preserves_in_memory_history() {
    let tree = TestTree::new("recent-write-failure");
    let guide = tree.root().join("guide.md");
    let blocked_parent = tree.root().join("blocked");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    fs::write(&blocked_parent, "not a directory").expect("the blocked parent should be created");
    let store = RecentFilesStore::at(blocked_parent.join("recent-files.json"));
    let mut controller = Controller::initialize_with_store(
        tree.root(),
        FileSystem::new(),
        EditorProcess::new(),
        store,
    )
    .expect("the workspace should initialize");

    assert!(controller.open_recent(&guide));
    assert_eq!(controller.preview().source(), Some("# Guide"));
    assert_eq!(controller.recent_files().entries().len(), 1);
    assert!(
        controller
            .recent_files()
            .error()
            .expect("the write error should be retained")
            .contains("failed to create recent-files directory")
    );
}
