//! Behavior tests for the Markdown editor.
//!
//! Coverage spans CLI and root validation, anchored explorer discovery,
//! selection and activation, preview reload failures, keyboard commands, and
//! responsive test-backend rendering.

use std::{
    cell::{Cell, RefCell},
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{self, Command},
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use leptatui::prelude::{KeyCode, KeyControl, KeyEvent, KeyModifiers, RenderCtx, View};
use ratatui::{Terminal, backend::TestBackend};

use crate::{
    cli::Cli,
    controller::Controller,
    domain::{ExplorerEntry, ExplorerEntryKind},
    editor_process::{EditorProcess, EnvironmentReader, ProcessLauncher},
    filesystem::FileSystem,
    ui::app_view,
};

/// Program and argument values captured from one process launch.
type RecordedCommand = (OsString, Vec<OsString>);

/// Result returned by one injected process launch.
#[derive(Clone, Copy, Debug)]
enum TestLaunchOutcome {
    /// Reports a successful child exit.
    Success,
    /// Reports a non-zero child exit.
    NonZero,
    /// Reports a missing executable.
    NotFound,
}

/// Injectable launcher that records commands and supplies deterministic exits.
#[derive(Clone, Debug)]
struct RecordingLauncher {
    /// Commands received by the launcher in call order.
    commands: Rc<RefCell<Vec<RecordedCommand>>>,
    /// Outcome returned for each launch.
    outcome: TestLaunchOutcome,
    /// Optional file replacement performed during a successful edit.
    replacement: Option<(PathBuf, String)>,
}

/// Injectable environment containing optional editor configuration.
#[derive(Clone, Debug, Default)]
struct TestEnvironment {
    /// Value returned for `VISUAL`.
    visual: Option<OsString>,
    /// Value returned for `EDITOR`.
    editor: Option<OsString>,
}

impl EnvironmentReader for TestEnvironment {
    /// Returns a configured test environment value.
    ///
    /// # Arguments
    ///
    /// * `name` — Environment variable name requested by the editor service.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the configured test value.
    fn var_os(&self, name: &str) -> Option<OsString> {
        match name {
            "VISUAL" => self.visual.clone(),
            "EDITOR" => self.editor.clone(),
            _ => None,
        }
    }
}

impl ProcessLauncher for RecordingLauncher {
    /// Records and resolves one prepared command without spawning a process.
    ///
    /// # Arguments
    ///
    /// * `command` — Prepared editor command whose program and arguments are
    ///   captured.
    ///
    /// # Returns
    ///
    /// A boolean matching the configured test outcome.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] for the missing-executable outcome or if a
    /// configured replacement cannot be written.
    fn success(&self, command: &mut Command) -> io::Result<bool> {
        self.commands.borrow_mut().push((
            command.get_program().to_os_string(),
            command.get_args().map(OsString::from).collect(),
        ));

        match self.outcome {
            TestLaunchOutcome::Success => {
                if let Some((path, source)) = &self.replacement {
                    fs::write(path, source)?;
                }
                Ok(true)
            }
            TestLaunchOutcome::NonZero => Ok(false),
            TestLaunchOutcome::NotFound => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "injected missing editor executable",
            )),
        }
    }
}

/// Temporary directory tree removed automatically after an explorer test.
#[derive(Debug)]
struct TestTree {
    /// Root directory owned by this fixture.
    root: PathBuf,
}

impl TestTree {
    /// Creates an empty temporary directory tree.
    ///
    /// # Arguments
    ///
    /// * `label` — Readable scenario name included in the temporary path.
    ///
    /// # Returns
    ///
    /// A [`TestTree`] owning a newly created directory.
    fn new(label: &str) -> Self {
        let root = temporary_path(label);
        fs::create_dir(&root).expect("the temporary directory should be created");
        Self { root }
    }

    /// Returns the fixture root.
    ///
    /// # Returns
    ///
    /// A [`Path`] containing the temporary directory.
    fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for TestTree {
    /// Removes the temporary fixture tree after its test completes.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Returns a process-local temporary path for one filesystem test.
///
/// # Arguments
///
/// * `label` — Readable scenario name included in the temporary path.
///
/// # Returns
///
/// A [`PathBuf`] beneath the operating system temporary directory.
fn temporary_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after the Unix epoch")
        .as_nanos();

    env::temp_dir().join(format!(
        "leptatui-markdown-editor-{label}-{}-{nonce}",
        process::id()
    ))
}

/// Converts explorer entry names into assertion-friendly strings.
///
/// # Arguments
///
/// * `entries` — Discovered entries whose display order should be asserted.
///
/// # Returns
///
/// A [`Vec`] containing lossy entry names in their existing order.
fn explorer_entry_names(entries: &[ExplorerEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| entry.name().to_string_lossy().into_owned())
        .collect()
}

/// Draws a Markdown editor view into a fixed-size test terminal.
///
/// # Arguments
///
/// * `terminal` — Test terminal used as the render target.
/// * `view` — Markdown editor view to render.
///
/// # Returns
///
/// An empty [`leptatui::Result`] after a successful draw.
///
/// # Errors
///
/// Returns [`leptatui::Error::Io`] if terminal drawing or view rendering fails.
fn draw_editor<V>(terminal: &mut Terminal<TestBackend>, view: &V) -> leptatui::Result<()>
where
    V: View,
{
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut context = RenderCtx::new(frame);
        render_result = view.render(&mut context);
    })?;

    render_result
}

/// Returns all rendered terminal rows as plain strings.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered editor.
///
/// # Returns
///
/// A [`Vec`] containing one string per terminal row.
fn rendered_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let area = terminal.backend().buffer().area;
    (0..area.height)
        .map(|row| {
            (0..area.width)
                .map(|column| {
                    terminal.backend().buffer().content()
                        [usize::from(row) * usize::from(area.width) + usize::from(column)]
                    .symbol()
                })
                .collect::<String>()
        })
        .collect()
}

/// Returns the first rendered position of a text fragment.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered editor.
/// * `needle` — Text fragment to locate.
///
/// # Returns
///
/// An [`Option`] containing the fragment's starting column and row.
fn rendered_position(terminal: &Terminal<TestBackend>, needle: &str) -> Option<(usize, usize)> {
    rendered_lines(terminal)
        .iter()
        .enumerate()
        .find_map(|(row, line)| line.find(needle).map(|column| (column, row)))
}

/// Verifies the CLI accepts no browsing root and uses the current directory.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without a positional root.
/// - Root resolution returns the process current directory.
#[test]
fn cli_defaults_to_current_directory() {
    let cli = Cli::try_parse_from(["markdown-editor"])
        .expect("the command should accept an omitted browsing root");

    assert_eq!(
        cli.requested_root()
            .expect("the current directory should be readable"),
        env::current_dir().expect("the current directory should be readable")
    );
}

/// Verifies the CLI accepts exactly one explicit browsing root.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor docs
/// ```
///
/// # Assertions
///
/// - Parsing succeeds with one positional root.
/// - The resolved root equals `docs`.
#[test]
fn cli_accepts_one_explicit_root() {
    let cli = Cli::try_parse_from(["markdown-editor", "docs"])
        .expect("the command should accept one browsing root");

    assert_eq!(
        cli.requested_root()
            .expect("an explicit root should not query the current directory"),
        PathBuf::from("docs")
    );
}

/// Verifies the CLI rejects additional positional roots.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor docs notes
/// ```
///
/// # Assertions
///
/// - Parsing fails when two roots are supplied.
#[test]
fn cli_rejects_additional_roots() {
    let result = Cli::try_parse_from(["markdown-editor", "docs", "notes"]);

    assert!(result.is_err());
}

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
/// - Activating the file loads its source and absolute path into the preview.
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

    assert!(controller.activate_selected());
    assert_eq!(
        controller.explorer().directory(),
        fs::canonicalize(&docs).expect("the docs directory should canonicalize")
    );
    assert_eq!(controller.explorer().selection(), Some(0));

    assert!(controller.activate_selected());
    assert_eq!(
        controller.preview().path(),
        Some(
            fs::canonicalize(&guide)
                .expect("the guide should canonicalize")
                .as_path()
        )
    );
    assert_eq!(controller.preview().source(), Some("# Guide"));
    assert_eq!(controller.preview().error(), None);
}

/// Verifies opening another Markdown file replaces the previous preview.
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
/// - Activating `alpha.md` loads its source.
/// - Selecting and activating `beta.md` replaces the path and source.
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

    assert!(controller.activate_selected());
    assert_eq!(controller.preview().source(), Some("# Alpha"));

    controller.select_next();
    assert!(controller.activate_selected());
    assert_eq!(
        controller.preview().path(),
        Some(
            fs::canonicalize(&beta)
                .expect("the second file should canonicalize")
                .as_path()
        )
    );
    assert_eq!(controller.preview().source(), Some("# Beta"));
}

/// Verifies preview reload reports a missing file and later recovers.
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
/// - The initial preview loads successfully.
/// - Reloading the deleted file replaces the body with a contextual error.
/// - Reloading the recreated file replaces the error with its new source.
#[test]
fn preview_reload_recovers_after_a_missing_file_returns() {
    let tree = TestTree::new("reload-recovery");
    let guide = tree.root().join("guide.md");
    fs::write(&guide, "# Original").expect("the Markdown file should be created");
    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");

    assert!(controller.activate_selected());
    fs::remove_file(&guide).expect("the open Markdown file should be removed");
    assert!(controller.reload_preview());
    assert_eq!(controller.preview().source(), None);
    let error = controller
        .preview()
        .error()
        .expect("the missing file should produce a preview error");
    assert!(error.contains("failed to resolve Markdown file"));
    assert!(error.contains(&guide.display().to_string()));

    fs::write(&guide, "# Restored").expect("the Markdown file should be recreated");
    assert!(controller.reload_preview());
    assert_eq!(controller.preview().source(), Some("# Restored"));
    assert_eq!(controller.preview().error(), None);
}

/// Verifies invalid UTF-8 is presented as recoverable preview state.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// └── invalid.md = [0xff, 0xfe, 0xfd]
/// ```
///
/// # Assertions
///
/// - Activating the selected file keeps its path open.
/// - No Markdown source is exposed.
/// - The preview error identifies the failed file read and invalid encoding.
#[test]
fn preview_invalid_utf8_is_recoverable() {
    let tree = TestTree::new("invalid-utf8");
    let invalid = tree.root().join("invalid.md");
    fs::write(&invalid, [0xff, 0xfe, 0xfd]).expect("the invalid UTF-8 fixture should be created");
    let mut controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize");

    assert!(controller.activate_selected());
    assert_eq!(controller.preview().source(), None);
    let error = controller
        .preview()
        .error()
        .expect("invalid UTF-8 should produce a preview error");
    assert!(error.contains("failed to read Markdown file"));
    assert!(error.contains(&invalid.display().to_string()));
    assert!(error.to_lowercase().contains("valid utf-8"));
}

/// Verifies `VISUAL` takes precedence and quoted arguments are parsed safely.
///
/// # Example Under Test
///
/// ```text
/// VISUAL="custom-editor --wait --option 'two words'"
/// EDITOR=ignored-editor
/// ```
///
/// # Assertions
///
/// - The injected launcher reports a successful edit.
/// - The program comes from `VISUAL`, not `EDITOR`.
/// - Quoted words remain one argument without invoking a shell.
/// - One `--` separator precedes the absolute Markdown path.
#[test]
fn editor_process_prefers_visual_and_parses_arguments() {
    let tree = TestTree::new("editor-command");
    let markdown = tree.root().join("-guide with spaces.md");
    fs::write(&markdown, "# Guide").expect("the Markdown file should be created");
    let absolute_markdown =
        fs::canonicalize(&markdown).expect("the Markdown file should canonicalize");
    let commands = Rc::new(RefCell::new(Vec::new()));
    let process = EditorProcess::with_services(
        RecordingLauncher {
            commands: Rc::clone(&commands),
            outcome: TestLaunchOutcome::Success,
            replacement: None,
        },
        TestEnvironment {
            visual: Some(OsString::from("custom-editor --wait --option 'two words'")),
            editor: Some(OsString::from("ignored-editor")),
        },
    );

    process
        .edit(&absolute_markdown)
        .expect("the injected editor launch should succeed");

    assert_eq!(
        commands.borrow().as_slice(),
        [(
            OsString::from("custom-editor"),
            vec![
                OsString::from("--wait"),
                OsString::from("--option"),
                OsString::from("two words"),
                OsString::from("--"),
                absolute_markdown.into_os_string()
            ]
        )]
    );
}

/// Verifies `EDITOR` and `vi` provide deterministic fallback commands.
///
/// # Example Under Test
///
/// ```text
/// VISUAL="   " EDITOR="configured-editor --wait --"
/// VISUAL unset, EDITOR unset
/// ```
///
/// # Assertions
///
/// - A whitespace-only `VISUAL` value is skipped in favor of `EDITOR`.
/// - An existing trailing `--` separator is not duplicated.
/// - Unset editor variables fall back to `vi`.
/// - Both commands receive the same canonical absolute path.
#[test]
fn editor_process_uses_editor_then_vi_fallback() {
    let tree = TestTree::new("editor-fallbacks");
    let markdown = tree.root().join("guide.md");
    fs::write(&markdown, "# Guide").expect("the Markdown file should be created");
    let absolute_markdown =
        fs::canonicalize(&markdown).expect("the Markdown file should canonicalize");
    let commands = Rc::new(RefCell::new(Vec::new()));
    let launcher = RecordingLauncher {
        commands: Rc::clone(&commands),
        outcome: TestLaunchOutcome::Success,
        replacement: None,
    };
    let configured = EditorProcess::with_services(
        launcher.clone(),
        TestEnvironment {
            visual: Some(OsString::from("   ")),
            editor: Some(OsString::from("configured-editor --wait --")),
        },
    );
    let fallback = EditorProcess::with_services(launcher, TestEnvironment::default());

    configured
        .edit(&absolute_markdown)
        .expect("the configured editor should succeed");
    fallback
        .edit(&absolute_markdown)
        .expect("the fallback editor should succeed");

    assert_eq!(
        commands.borrow().as_slice(),
        [
            (
                OsString::from("configured-editor"),
                vec![
                    OsString::from("--wait"),
                    OsString::from("--"),
                    absolute_markdown.clone().into_os_string()
                ]
            ),
            (
                OsString::from("vi"),
                vec![OsString::from("--"), absolute_markdown.into_os_string()]
            )
        ]
    );
}

/// Verifies malformed editor configuration fails without running a shell.
///
/// # Example Under Test
///
/// ```text
/// VISUAL="editor 'unterminated"
/// ```
///
/// # Assertions
///
/// - Editing returns an invalid-input error identifying `VISUAL`.
/// - No process command reaches the injected launcher.
#[test]
fn editor_process_rejects_malformed_configuration() {
    let tree = TestTree::new("editor-malformed");
    let markdown = tree.root().join("guide.md");
    fs::write(&markdown, "# Guide").expect("the Markdown file should be created");
    let commands = Rc::new(RefCell::new(Vec::new()));
    let process = EditorProcess::with_services(
        RecordingLauncher {
            commands: Rc::clone(&commands),
            outcome: TestLaunchOutcome::Success,
            replacement: None,
        },
        TestEnvironment {
            visual: Some(OsString::from("editor 'unterminated")),
            editor: None,
        },
    );

    let error = process
        .edit(&markdown)
        .expect_err("malformed editor configuration should fail");

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("VISUAL"));
    assert!(commands.borrow().is_empty());
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
/// - The injected edit succeeds and replaces the preview source from disk.
/// - The explorer directory and selected index remain unchanged.
/// - The preview retains the same canonical absolute path without an error.
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

    assert!(controller.activate_selected());
    controller.select_next();
    assert!(controller.activate_selected());
    let expected_directory = controller.explorer().directory().to_path_buf();
    let expected_selection = controller.explorer().selection();

    assert!(controller.edit_preview());
    assert_eq!(controller.preview().path(), Some(canonical_beta.as_path()));
    assert_eq!(controller.preview().source(), Some("# After"));
    assert_eq!(controller.preview().error(), None);
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
/// - Each failure removes stale source and renders a contextual error.
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
        assert!(controller.activate_selected());

        assert!(controller.edit_preview());
        assert_eq!(
            controller.preview().path(),
            Some(canonical_markdown.as_path())
        );
        assert_eq!(controller.preview().source(), None);
        let error = controller
            .preview()
            .error()
            .expect("the editor failure should be visible");
        assert!(error.contains(expected_error));
        if path_in_error {
            assert!(error.contains(&canonical_markdown.display().to_string()));
        }
    }
}

/// Verifies interactive keys update selection, preview, reload, and scrolling.
///
/// # Example Under Test
///
/// ```text
/// Down
/// Enter
/// PageDown
/// r
/// ```
///
/// # Assertions
///
/// - The initial selected row is `alpha.md`.
/// - `Down` moves the selected marker to `beta.md`.
/// - `Enter` opens and renders `beta.md`.
/// - `PageDown` is handled and changes the overflowing preview viewport.
/// - `r` is handled without closing the open preview.
#[test]
fn editor_keys_drive_selection_preview_reload_and_scroll() -> leptatui::Result<()> {
    let tree = TestTree::new("editor-keys");
    fs::write(tree.root().join("alpha.md"), "# Alpha")
        .expect("the first Markdown file should be created");
    let beta_source = (0..24)
        .map(|index| format!("## Beta line {index}\n"))
        .collect::<String>();
    fs::write(tree.root().join("beta.md"), beta_source)
        .expect("the long Markdown file should be created");
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let mut view = app_view(controller, Rc::new(Cell::new(false)));
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;

    draw_editor(&mut terminal, &view)?;
    let initial_render = rendered_lines(&terminal).join("\n");
    assert!(initial_render.contains("> [M] alpha.md"));
    assert!(initial_render.contains("e edit"));
    assert!(initial_render.contains("q quit"));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("> [M] beta.md")
    );

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    let before_scroll = rendered_lines(&terminal);
    assert!(before_scroll.join("\n").contains("Beta line 0"));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert_ne!(rendered_lines(&terminal), before_scroll);

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(rendered_lines(&terminal).join("\n").contains("Open: "));

    Ok(())
}

/// Verifies the edit key exits only to edit an open preview.
///
/// # Example Under Test
///
/// ```text
/// e
/// Enter
/// e
/// ```
///
/// # Assertions
///
/// - `e` is handled without exiting when no preview is open.
/// - `Enter` opens the selected Markdown document.
/// - `e` then requests editing and exits the managed TUI loop.
/// - `q` exits without setting a separate edit request.
#[test]
fn editor_edit_key_requests_an_external_session_only_for_an_open_preview() -> leptatui::Result<()> {
    let tree = TestTree::new("editor-edit-key");
    fs::write(tree.root().join("guide.md"), "# Guide")
        .expect("the Markdown file should be created");
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let edit_requested = Rc::new(Cell::new(false));
    let mut view = app_view(Rc::clone(&controller), Rc::clone(&edit_requested));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert!(!edit_requested.get());
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))?,
        KeyControl::Exit
    );
    assert!(edit_requested.get());

    let quit_requested = Rc::new(Cell::new(false));
    let mut quit_view = app_view(controller, Rc::clone(&quit_requested));
    assert_eq!(
        quit_view.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))?,
        KeyControl::Exit
    );
    assert!(!quit_requested.get());

    Ok(())
}

/// Verifies the workspace switches from side-by-side to stacked panes.
///
/// # Example Under Test
///
/// ```text
/// viewport = 100x30
/// viewport = 50x30
/// ```
///
/// # Assertions
///
/// - Wide rendering places Explorer and Preview headings on the same row.
/// - Narrow rendering places Preview below Explorer.
/// - Both layouts render the selected Markdown document.
#[test]
fn editor_renders_wide_and_narrow_pane_layouts() -> leptatui::Result<()> {
    let tree = TestTree::new("responsive-layout");
    fs::write(tree.root().join("guide.md"), "# Responsive guide")
        .expect("the Markdown file should be created");

    let mut wide_controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the wide controller should initialize");
    assert!(wide_controller.activate_selected());
    let wide_view = app_view(
        Rc::new(RefCell::new(wide_controller)),
        Rc::new(Cell::new(false)),
    );
    let mut wide_terminal = Terminal::new(TestBackend::new(100, 30))?;
    draw_editor(&mut wide_terminal, &wide_view)?;
    let (_, wide_explorer_row) =
        rendered_position(&wide_terminal, "Explorer").expect("Explorer should render");
    let (_, wide_preview_row) =
        rendered_position(&wide_terminal, "Preview").expect("Preview should render");
    assert_eq!(wide_explorer_row, wide_preview_row);
    assert!(
        rendered_lines(&wide_terminal)
            .join("\n")
            .contains("Responsive guide")
    );

    let mut narrow_controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the narrow controller should initialize");
    assert!(narrow_controller.activate_selected());
    let narrow_view = app_view(
        Rc::new(RefCell::new(narrow_controller)),
        Rc::new(Cell::new(false)),
    );
    let mut narrow_terminal = Terminal::new(TestBackend::new(50, 30))?;
    draw_editor(&mut narrow_terminal, &narrow_view)?;
    let (_, narrow_explorer_row) =
        rendered_position(&narrow_terminal, "Explorer").expect("Explorer should render");
    let (_, narrow_preview_row) =
        rendered_position(&narrow_terminal, "Preview").expect("Preview should render");
    assert!(narrow_preview_row > narrow_explorer_row);
    let narrow_text = rendered_lines(&narrow_terminal).join("\n");
    assert!(
        narrow_text.contains("Responsive guide"),
        "narrow rendering: {narrow_text:?}"
    );

    Ok(())
}
