//! Anchored filesystem discovery and failure tests.

use std::{ffi::OsString, fs};

use crate::{
    core::Controller,
    core::{ExplorerEntry, ExplorerEntryKind},
    services::EditorProcess,
    services::FileSystem,
};

use super::support::{TestTree, explorer_entry_names, temporary_path};

/// Verifies filesystem validation returns a canonical directory workspace.
///
/// # Example Under Test
///
/// ```text
/// <temporary-directory>/workspace
/// ```
///
/// # Assertions
///
/// - The temporary directory is created successfully.
/// - Root validation succeeds.
/// - The workspace root equals the canonical temporary-directory path.
#[test]
fn explorer_canonicalizes_valid_directory() {
    let tree = TestTree::new("valid-root");
    let expected =
        fs::canonicalize(tree.root()).expect("the temporary directory should canonicalize");

    let workspace = FileSystem::new()
        .validate_root(tree.root())
        .expect("a directory should be a valid browsing root");

    assert_eq!(workspace.root(), expected);
}

/// Verifies filesystem validation rejects regular files as browsing roots.
///
/// # Example Under Test
///
/// ```text
/// <temporary-directory>/not-a-directory.md
/// ```
///
/// # Assertions
///
/// - The temporary file is created successfully.
/// - Validation returns `InvalidInput`.
/// - The diagnostic identifies that the root is not a directory.
#[test]
fn explorer_rejects_regular_file_as_root() {
    let root = temporary_path("regular-file.md");
    fs::write(&root, "# Not a directory").expect("the temporary file should be created");

    let error = FileSystem::new()
        .validate_root(&root)
        .expect_err("a regular file should not be a valid browsing root");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("not a directory"));
    fs::remove_file(&root).expect("the temporary file should be removed");
}

/// Verifies controller initialization rejects a missing root before UI startup.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor <missing-temporary-path>
/// ```
///
/// # Assertions
///
/// - Controller initialization fails with `NotFound`.
/// - The diagnostic contains the missing path.
///
/// # Why
///
/// The binary must complete controller initialization before constructing the
/// Leptatui app, preventing invalid roots from entering managed terminal mode.
#[test]
fn explorer_rejects_missing_root_before_ui_startup() {
    let root = temporary_path("missing-root");

    let error = Controller::initialize(&root, FileSystem::new(), EditorProcess::new())
        .expect_err("a missing root should fail controller initialization");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(error.to_string().contains(&root.display().to_string()));
}

/// Verifies explorer discovery filters and deterministically orders entries.
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
/// - Listing succeeds for the canonical workspace root.
/// - Directories sort before Markdown files.
/// - Names sort case-insensitively inside each kind.
/// - `.md` and `.markdown` extensions match without ASCII case sensitivity.
/// - Non-Markdown files are omitted.
#[test]
fn explorer_lists_directories_before_case_insensitive_markdown_files() {
    let tree = TestTree::new("ordered-listing");
    fs::create_dir(tree.root().join("zeta")).expect("the zeta directory should be created");
    fs::create_dir(tree.root().join("Alpha")).expect("the Alpha directory should be created");
    fs::write(tree.root().join("notes.MD"), "# Notes")
        .expect("the uppercase Markdown file should be created");
    fs::write(tree.root().join("Guide.markdown"), "# Guide")
        .expect("the long-extension Markdown file should be created");
    fs::write(tree.root().join("ignored.txt"), "ignored")
        .expect("the non-Markdown file should be created");

    let filesystem = FileSystem::new();
    let workspace = filesystem
        .validate_root(tree.root())
        .expect("the fixture root should be valid");
    let listing = filesystem
        .list_directory(&workspace, tree.root())
        .expect("the fixture root should be listed");

    assert_eq!(
        explorer_entry_names(listing.entries()),
        ["Alpha", "zeta", "Guide.markdown", "notes.MD"]
    );
    assert_eq!(
        listing
            .entries()
            .iter()
            .map(ExplorerEntry::kind)
            .collect::<Vec<_>>(),
        [
            ExplorerEntryKind::Directory,
            ExplorerEntryKind::Directory,
            ExplorerEntryKind::Markdown,
            ExplorerEntryKind::Markdown,
        ]
    );
}

/// Verifies an empty directory produces a successful empty explorer state.
///
/// # Example Under Test
///
/// ```text
/// empty-workspace/
/// ```
///
/// # Assertions
///
/// - Controller initialization succeeds.
/// - The current directory is the canonical root.
/// - The explorer contains no entries, selection, or error.
#[test]
fn explorer_represents_empty_directory_without_an_error() {
    let tree = TestTree::new("empty-listing");
    let expected =
        fs::canonicalize(tree.root()).expect("the temporary directory should canonicalize");

    let controller = Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
        .expect("an empty root should initialize the controller");

    assert_eq!(controller.explorer().directory(), expected);
    assert!(controller.explorer().entries().is_empty());
    assert_eq!(controller.explorer().selection(), None);
    assert_eq!(controller.explorer().error(), None);
}

/// Verifies parent navigation reaches but never crosses the workspace root.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// └── nested/
/// ```
///
/// # Assertions
///
/// - Navigation into the nested directory succeeds.
/// - Parent navigation returns to the canonical root.
/// - A second parent request is a no-op at the root.
#[test]
fn explorer_parent_navigation_stops_at_root() {
    let tree = TestTree::new("parent-boundary");
    let nested = tree.root().join("nested");
    fs::create_dir(&nested).expect("the nested directory should be created");
    let canonical_root =
        fs::canonicalize(tree.root()).expect("the temporary directory should canonicalize");

    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");

    assert!(controller.browse(&nested));
    assert_ne!(controller.explorer().directory(), canonical_root);
    assert!(controller.browse_parent());
    assert_eq!(controller.explorer().directory(), canonical_root);
    assert!(!controller.browse_parent());
    assert_eq!(controller.explorer().directory(), canonical_root);
}

/// Verifies failed navigation preserves the last valid listing.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// ├── docs/
/// └── missing/
/// ```
///
/// The `missing` directory is requested but does not exist.
///
/// # Assertions
///
/// - Navigation reports failure.
/// - The root directory, entries, and selection remain current.
/// - The error identifies the missing path and failed resolution.
#[test]
fn explorer_missing_path_preserves_listing_and_records_error() {
    let tree = TestTree::new("missing-navigation");
    fs::create_dir(tree.root().join("docs")).expect("the docs directory should be created");
    let missing = tree.root().join("missing");
    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");
    let expected_directory = controller.explorer().directory().to_path_buf();
    let expected_entries = controller.explorer().entries().to_vec();
    let expected_selection = controller.explorer().selection();

    assert!(!controller.browse(&missing));
    assert_eq!(controller.explorer().directory(), expected_directory);
    assert_eq!(controller.explorer().entries(), expected_entries);
    assert_eq!(controller.explorer().selection(), expected_selection);
    let error = controller
        .explorer()
        .error()
        .expect("failed navigation should record an error");
    assert!(error.contains("failed to resolve directory"));
    assert!(error.contains(&missing.display().to_string()));
}

/// Verifies explicit out-of-root navigation is rejected without losing state.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// outside/
/// ```
///
/// # Assertions
///
/// - Navigation to the existing outside directory fails.
/// - The explorer remains at the workspace root.
/// - The error explains that the target lies outside the browsing root.
#[test]
fn explorer_rejects_out_of_root_navigation() {
    let tree = TestTree::new("contained-root");
    let outside = TestTree::new("outside-root");
    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");
    let expected_directory = controller.explorer().directory().to_path_buf();

    assert!(!controller.browse(outside.root()));
    assert_eq!(controller.explorer().directory(), expected_directory);
    assert!(
        controller
            .explorer()
            .error()
            .expect("out-of-root navigation should record an error")
            .contains("outside browsing root")
    );
}

/// Verifies requesting a regular file as a directory is recoverable.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// └── guide.md
/// ```
///
/// # Assertions
///
/// - Navigation to `guide.md` fails.
/// - The last valid root listing remains available.
/// - The error identifies that the explorer target is not a directory.
#[test]
fn explorer_regular_file_navigation_is_recoverable() {
    let tree = TestTree::new("file-navigation");
    let markdown = tree.root().join("guide.md");
    fs::write(&markdown, "# Guide").expect("the Markdown file should be created");
    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");
    let expected_entries = controller.explorer().entries().to_vec();

    assert!(!controller.browse(&markdown));
    assert_eq!(controller.explorer().entries(), expected_entries);
    assert!(
        controller
            .explorer()
            .error()
            .expect("file navigation should record an error")
            .contains("not a directory")
    );
}

/// Verifies in-root symlinks resolve to safe explorer targets.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// ├── docs/
/// ├── guide.md
/// ├── linked-docs -> docs/
/// └── linked-guide.MD -> guide.md
/// ```
///
/// # Assertions
///
/// - Listing succeeds.
/// - Both symlink names appear with their target kinds.
/// - Symlink entry paths are the canonical in-root targets.
#[cfg(unix)]
#[test]
fn explorer_follows_in_root_symlinks() {
    use std::os::unix::fs::symlink;

    let tree = TestTree::new("in-root-symlinks");
    let docs = tree.root().join("docs");
    let guide = tree.root().join("guide.md");
    fs::create_dir(&docs).expect("the docs directory should be created");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    symlink(&docs, tree.root().join("linked-docs"))
        .expect("the directory symlink should be created");
    symlink(&guide, tree.root().join("linked-guide.MD"))
        .expect("the file symlink should be created");

    let filesystem = FileSystem::new();
    let workspace = filesystem
        .validate_root(tree.root())
        .expect("the fixture root should be valid");
    let listing = filesystem
        .list_directory(&workspace, tree.root())
        .expect("in-root symlinks should be listed");

    assert!(listing.entries().contains(&ExplorerEntry::new(
        OsString::from("linked-docs"),
        fs::canonicalize(&docs).expect("the docs target should canonicalize"),
        ExplorerEntryKind::Directory,
    )));
    assert!(listing.entries().contains(&ExplorerEntry::new(
        OsString::from("linked-guide.MD"),
        fs::canonicalize(&guide).expect("the guide target should canonicalize"),
        ExplorerEntryKind::Markdown,
    )));
}

/// Verifies broken and escaping symlinks are hidden from the explorer.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// ├── broken -> missing/
/// ├── outside-docs -> ../outside/
/// └── outside-guide.md -> ../outside/guide.md
/// ```
///
/// # Assertions
///
/// - Listing remains successful.
/// - Broken and root-escaping symlinks do not appear.
/// - A valid in-root Markdown file remains visible.
#[cfg(unix)]
#[test]
fn explorer_hides_broken_and_escaping_symlinks() {
    use std::os::unix::fs::symlink;

    let tree = TestTree::new("escaping-symlinks");
    let outside = TestTree::new("symlink-targets");
    let outside_markdown = outside.root().join("guide.md");
    fs::write(&outside_markdown, "# Outside").expect("the outside file should be created");
    fs::write(tree.root().join("inside.md"), "# Inside")
        .expect("the inside file should be created");
    symlink(tree.root().join("missing"), tree.root().join("broken"))
        .expect("the broken symlink should be created");
    symlink(outside.root(), tree.root().join("outside-docs"))
        .expect("the escaping directory symlink should be created");
    symlink(&outside_markdown, tree.root().join("outside-guide.md"))
        .expect("the escaping file symlink should be created");

    let filesystem = FileSystem::new();
    let workspace = filesystem
        .validate_root(tree.root())
        .expect("the fixture root should be valid");
    let listing = filesystem
        .list_directory(&workspace, tree.root())
        .expect("unsafe symlinks should not fail the listing");

    assert_eq!(explorer_entry_names(listing.entries()), ["inside.md"]);
}

/// Verifies unreadable directories become contextual explorer errors.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// └── private/ (mode 000)
/// ```
///
/// # Assertions
///
/// - Navigation fails when the platform enforces the removed permissions.
/// - The previous root listing remains current.
/// - The error identifies the unreadable directory.
///
/// # Why
///
/// Privileged Unix test processes may bypass mode bits, so the assertions are
/// conditional on the operating system denying the directory read.
#[cfg(unix)]
#[test]
fn explorer_unreadable_directory_is_recoverable() {
    use std::os::unix::fs::PermissionsExt;

    let tree = TestTree::new("unreadable-directory");
    let private = tree.root().join("private");
    fs::create_dir(&private).expect("the private directory should be created");
    fs::set_permissions(&private, fs::Permissions::from_mode(0o000))
        .expect("the private directory permissions should be removed");
    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");
    let expected_directory = controller.explorer().directory().to_path_buf();

    let navigated = controller.browse(&private);
    fs::set_permissions(&private, fs::Permissions::from_mode(0o700))
        .expect("the private directory permissions should be restored");

    if navigated {
        return;
    }

    assert_eq!(controller.explorer().directory(), expected_directory);
    assert!(
        controller
            .explorer()
            .error()
            .expect("the denied read should record an error")
            .contains(&private.display().to_string())
    );
}
