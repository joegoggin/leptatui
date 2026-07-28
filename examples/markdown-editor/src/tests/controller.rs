//! Controller transition and recoverable-state tests.

use std::{cell::RefCell, ffi::OsString, fs, rc::Rc};

use crate::{
    controller::{Controller, ExplorerActivation},
    editor_process::EditorProcess,
    filesystem::FileSystem,
};

use super::support::{RecordingLauncher, TestEnvironment, TestLaunchOutcome, TestTree};

/// Verifies explorer selection clamps at both listing boundaries.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// ├── alpha.md
/// └── beta.md
/// ```
///
/// # Assertions
///
/// - The first sorted entry is selected initially.
/// - Moving before the first entry keeps index zero selected.
/// - Moving after the last entry keeps the last index selected.
#[test]
fn explorer_selection_clamps_at_listing_boundaries() {
    let tree = TestTree::new("selection-boundaries");
    fs::write(tree.root().join("alpha.md"), "# Alpha")
        .expect("the first Markdown file should be created");
    fs::write(tree.root().join("beta.md"), "# Beta")
        .expect("the second Markdown file should be created");
    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");

    assert_eq!(controller.explorer().selection(), Some(0));
    controller.select_previous();
    assert_eq!(controller.explorer().selection(), Some(0));

    controller.select_next();
    controller.select_next();
    assert_eq!(controller.explorer().selection(), Some(1));
}

/// Verifies directory activation replaces the listing and selects its first entry.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// └── docs/
///     └── guide.md
/// ```
///
/// # Assertions
///
/// - The root directory entry is selected initially.
/// - Activating it browses into `docs`.
/// - The nested Markdown file becomes selected.
/// - Activating the file stores its absolute path for the Markdown view.
/// - Opening the document advances the view revision without an editor error.
#[test]
fn explorer_activation_browses_directories_and_opens_markdown() {
    let tree = TestTree::new("activation");
    let docs = tree.root().join("docs");
    let guide = docs.join("guide.md");
    fs::create_dir(&docs).expect("the docs directory should be created");
    fs::write(&guide, "# Guide").expect("the Markdown file should be created");
    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");

    assert_eq!(
        controller.activate_selected(),
        ExplorerActivation::Directory
    );
    assert_eq!(
        controller.explorer().directory(),
        fs::canonicalize(&docs).expect("the docs directory should canonicalize")
    );
    assert_eq!(controller.explorer().selection(), Some(0));

    assert_eq!(controller.activate_selected(), ExplorerActivation::Document);
    assert_eq!(
        controller.preview().path(),
        Some(
            fs::canonicalize(&guide)
                .expect("the guide should canonicalize")
                .as_path()
        )
    );
    assert_eq!(controller.preview().revision(), 1);
    assert_eq!(controller.preview().editor_error(), None);
}

/// Verifies opening another Markdown file replaces the previous view path.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// ├── alpha.md
/// └── beta.md
/// ```
///
/// # Assertions
///
/// - Activating `alpha.md` stores its canonical path.
/// - Selecting and activating `beta.md` replaces that path.
/// - Each open advances the view revision.
#[test]
fn preview_replaces_document_when_another_file_opens() {
    let tree = TestTree::new("preview-replacement");
    let alpha = tree.root().join("alpha.md");
    let beta = tree.root().join("beta.md");
    fs::write(&alpha, "# Alpha").expect("the first Markdown file should be created");
    fs::write(&beta, "# Beta").expect("the second Markdown file should be created");
    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");

    assert_eq!(controller.activate_selected(), ExplorerActivation::Document);
    assert_eq!(
        controller.preview().path(),
        Some(
            fs::canonicalize(&alpha)
                .expect("the first file should canonicalize")
                .as_path()
        )
    );
    assert_eq!(controller.preview().revision(), 1);

    controller.select_next();
    assert_eq!(controller.activate_selected(), ExplorerActivation::Document);
    assert_eq!(
        controller.preview().path(),
        Some(
            fs::canonicalize(&beta)
                .expect("the second file should canonicalize")
                .as_path()
        )
    );
    assert_eq!(controller.preview().revision(), 2);
}

/// Verifies preview reload invalidates the path-backed Markdown view.
///
/// # Example Under Test
///
/// ```text
/// open guide.md
/// delete guide.md
/// reload
/// recreate guide.md
/// reload
/// ```
///
/// # Assertions
///
/// - The initial preview stores the canonical document path.
/// - Reloading a deleted file retains its path and advances the revision.
/// - Reloading the recreated file advances the revision again.
#[test]
fn preview_reload_invalidates_after_file_changes() {
    let tree = TestTree::new("reload-recovery");
    let guide = tree.root().join("guide.md");
    fs::write(&guide, "# Original").expect("the Markdown file should be created");
    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");

    assert_eq!(controller.activate_selected(), ExplorerActivation::Document);
    let canonical_guide =
        fs::canonicalize(&guide).expect("the original document should canonicalize");
    assert_eq!(controller.preview().revision(), 1);
    fs::remove_file(&guide).expect("the open Markdown file should be removed");
    assert!(controller.reload_preview());
    assert_eq!(controller.preview().path(), Some(canonical_guide.as_path()));
    assert_eq!(controller.preview().revision(), 2);

    fs::write(&guide, "# Restored").expect("the Markdown file should be recreated");
    assert!(controller.reload_preview());
    assert_eq!(controller.preview().path(), Some(canonical_guide.as_path()));
    assert_eq!(controller.preview().revision(), 3);
}

/// Verifies a successful external edit reloads content without moving context.
///
/// # Example Under Test
///
/// ```text
/// workspace/docs/beta.md = "# Before"
/// configured-editor -- /absolute/workspace/docs/beta.md
/// workspace/docs/beta.md = "# After"
/// ```
///
/// # Assertions
///
/// - Directory activation and file opening succeed.
/// - The injected edit succeeds and replaces the file contents.
/// - The explorer directory and selected index remain unchanged.
/// - The preview retains the same canonical path and advances its revision.
/// - No external-editor error remains.
#[test]
fn editor_success_reloads_preview_and_preserves_browsing_context() {
    let tree = TestTree::new("editor-success");
    let docs = tree.root().join("docs");
    let alpha = docs.join("alpha.md");
    let beta = docs.join("beta.md");
    fs::create_dir(&docs).expect("the docs directory should be created");
    fs::write(&alpha, "# Alpha").expect("the first Markdown file should be created");
    fs::write(&beta, "# Before").expect("the edited Markdown file should be created");
    let canonical_beta =
        fs::canonicalize(&beta).expect("the edited Markdown file should canonicalize");
    let commands = Rc::new(RefCell::new(Vec::new()));
    let editor_process = EditorProcess::with_services(
        RecordingLauncher {
            commands,
            outcome: TestLaunchOutcome::Success,
            replacement: Some((canonical_beta.clone(), String::from("# After"))),
        },
        TestEnvironment {
            visual: Some(OsString::from("configured-editor")),
            editor: None,
        },
    );
    let mut controller = Controller::initialize(tree.root(), FileSystem::new(), editor_process)
        .expect("the workspace should initialize");

    assert_eq!(
        controller.activate_selected(),
        ExplorerActivation::Directory
    );
    controller.select_next();
    assert_eq!(controller.activate_selected(), ExplorerActivation::Document);
    let expected_directory = controller.explorer().directory().to_path_buf();
    let expected_selection = controller.explorer().selection();
    let previous_revision = controller.preview().revision();

    assert!(controller.edit_preview());
    assert_eq!(controller.preview().path(), Some(canonical_beta.as_path()));
    assert_eq!(
        fs::read_to_string(&canonical_beta).expect("the edited file should remain readable"),
        "# After"
    );
    assert_eq!(controller.preview().revision(), previous_revision + 1);
    assert_eq!(controller.preview().editor_error(), None);
    assert_eq!(controller.explorer().directory(), expected_directory);
    assert_eq!(controller.explorer().selection(), expected_selection);
}

/// Verifies editor launch and exit failures become recoverable preview errors.
///
/// # Example Under Test
///
/// ```text
/// configured editor missing
/// configured editor exits non-zero
/// VISUAL contains malformed quoting
/// ```
///
/// # Assertions
///
/// - Every editor attempt finds an open preview path.
/// - Each failure records a contextual external-editor error.
/// - Configuration, missing-executable, and non-zero diagnostics remain
///   distinguishable.
/// - The absolute preview path is retained so editing can be retried.
#[test]
fn editor_failures_are_visible_and_retain_the_open_path() {
    for (label, outcome, visual, editor, expected_error, path_in_error) in [
        (
            "editor-missing",
            TestLaunchOutcome::NotFound,
            None,
            Some("configured-editor"),
            "failed to launch editor 'configured-editor'",
            true,
        ),
        (
            "editor-non-zero",
            TestLaunchOutcome::NonZero,
            None,
            Some("configured-editor"),
            "editor 'configured-editor' exited with a non-zero status",
            true,
        ),
        (
            "editor-malformed",
            TestLaunchOutcome::Success,
            Some("editor 'unterminated"),
            None,
            "VISUAL contains malformed shell-word quoting",
            false,
        ),
    ] {
        let tree = TestTree::new(label);
        let markdown = tree.root().join("guide.md");
        fs::write(&markdown, "# Guide").expect("the Markdown file should be created");
        let canonical_markdown =
            fs::canonicalize(&markdown).expect("the Markdown file should canonicalize");
        let editor_process = EditorProcess::with_services(
            RecordingLauncher {
                commands: Rc::new(RefCell::new(Vec::new())),
                outcome,
                replacement: None,
            },
            TestEnvironment {
                visual: visual.map(OsString::from),
                editor: editor.map(OsString::from),
            },
        );
        let mut controller = Controller::initialize(tree.root(), FileSystem::new(), editor_process)
            .expect("the workspace should initialize");
        assert_eq!(controller.activate_selected(), ExplorerActivation::Document);

        assert!(controller.edit_preview());
        assert_eq!(
            controller.preview().path(),
            Some(canonical_markdown.as_path())
        );
        let error = controller
            .preview()
            .editor_error()
            .expect("the editor failure should be visible");
        assert!(error.contains(expected_error));
        if path_in_error {
            assert!(error.contains(&canonical_markdown.display().to_string()));
        }
    }
}
