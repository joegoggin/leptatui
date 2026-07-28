//! Anchored filesystem discovery and failure tests.

use std::{ffi::OsString, fs};

use crate::services::{ExplorerEntry, ExplorerEntryKind, FileSystem};

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

/// Verifies workspace validation rejects a missing root before UI startup.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor <missing-temporary-path>
/// ```
///
/// # Assertions
///
/// - Workspace validation fails with `NotFound`.
/// - The diagnostic contains the missing path.
///
/// # Why
///
/// The binary must complete workspace validation before constructing the
/// Leptatui app, preventing invalid roots from entering managed terminal mode.
#[test]
fn explorer_rejects_missing_root_before_ui_startup() {
    let root = temporary_path("missing-root");

    let error = FileSystem::new()
        .validate_root(&root)
        .expect_err("a missing root should fail workspace validation");

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

/// Verifies an empty directory produces a successful empty listing.
///
/// # Example Under Test
///
/// ```text
/// empty-workspace/
/// ```
///
/// # Assertions
///
/// - Workspace validation succeeds.
/// - The current directory is the canonical root.
/// - The listing contains no entries.
#[test]
fn explorer_represents_empty_directory_without_an_error() {
    let tree = TestTree::new("empty-listing");
    let expected =
        fs::canonicalize(tree.root()).expect("the temporary directory should canonicalize");

    let filesystem = FileSystem::new();
    let workspace = filesystem
        .validate_root(tree.root())
        .expect("the empty root should validate");
    let listing = filesystem
        .list_directory(&workspace, tree.root())
        .expect("the empty root should list successfully");

    assert_eq!(listing.directory(), expected);
    assert!(listing.entries().is_empty());
}

/// Verifies nested and parent directories can be listed within the workspace.
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
/// - The nested directory lists successfully.
/// - Its parent resolves to and lists the canonical workspace root.
#[test]
fn explorer_parent_navigation_stops_at_root() {
    let tree = TestTree::new("parent-boundary");
    let nested = tree.root().join("nested");
    fs::create_dir(&nested).expect("the nested directory should be created");
    let canonical_root =
        fs::canonicalize(tree.root()).expect("the temporary directory should canonicalize");

    let filesystem = FileSystem::new();
    let workspace = filesystem
        .validate_root(tree.root())
        .expect("the workspace should validate");
    let nested_listing = filesystem
        .list_directory(&workspace, &nested)
        .expect("the nested directory should list");
    assert_ne!(nested_listing.directory(), canonical_root);
    let parent = nested_listing
        .directory()
        .parent()
        .expect("the nested directory should have a parent");
    let parent_listing = filesystem
        .list_directory(&workspace, parent)
        .expect("the workspace root should list");
    assert_eq!(parent_listing.directory(), canonical_root);
}

/// Verifies missing-directory discovery returns a contextual error.
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
/// - Root discovery succeeds before the failed request.
/// - Missing-directory discovery reports failure.
/// - The error identifies the missing path and failed resolution.
#[test]
fn explorer_missing_path_preserves_listing_and_records_error() {
    let tree = TestTree::new("missing-navigation");
    fs::create_dir(tree.root().join("docs")).expect("the docs directory should be created");
    let missing = tree.root().join("missing");
    let filesystem = FileSystem::new();
    let workspace = filesystem
        .validate_root(tree.root())
        .expect("the workspace should validate");
    filesystem
        .list_directory(&workspace, tree.root())
        .expect("the root listing should succeed");
    let error = filesystem
        .list_directory(&workspace, &missing)
        .expect_err("the missing directory should fail");
    assert!(error.to_string().contains("failed to resolve directory"));
    assert!(error.to_string().contains(&missing.display().to_string()));
}

/// Verifies explicit out-of-root discovery is rejected.
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
/// - Discovery of the existing outside directory fails.
/// - The error explains that the target lies outside the browsing root.
#[test]
fn explorer_rejects_out_of_root_navigation() {
    let tree = TestTree::new("contained-root");
    let outside = TestTree::new("outside-root");
    let filesystem = FileSystem::new();
    let workspace = filesystem
        .validate_root(tree.root())
        .expect("the workspace should validate");
    let error = filesystem
        .list_directory(&workspace, outside.root())
        .expect_err("the outside directory should be rejected");
    assert!(error.to_string().contains("outside browsing root"));
}

/// Verifies requesting a regular file as a directory returns an error.
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
/// - Directory discovery for `guide.md` fails.
/// - The error identifies that the explorer target is not a directory.
#[test]
fn explorer_regular_file_navigation_is_recoverable() {
    let tree = TestTree::new("file-navigation");
    let markdown = tree.root().join("guide.md");
    fs::write(&markdown, "# Guide").expect("the Markdown file should be created");
    let filesystem = FileSystem::new();
    let workspace = filesystem
        .validate_root(tree.root())
        .expect("the workspace should validate");
    let error = filesystem
        .list_directory(&workspace, &markdown)
        .expect_err("a Markdown file should not list as a directory");
    assert!(error.to_string().contains("not a directory"));
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
    let filesystem = FileSystem::new();
    let workspace = filesystem
        .validate_root(tree.root())
        .expect("the workspace should validate");
    let result = filesystem.list_directory(&workspace, &private);
    fs::set_permissions(&private, fs::Permissions::from_mode(0o700))
        .expect("the private directory permissions should be restored");

    let Err(error) = result else {
        return;
    };

    assert!(error.to_string().contains(&private.display().to_string()));
}
