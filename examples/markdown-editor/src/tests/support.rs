//! Shared fixtures, mocks, and rendering helpers for editor tests.

use std::{
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{self, Command},
    rc::{Rc, Weak},
    sync::{Arc, Mutex, MutexGuard, OnceLock},
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use leptatui::prelude::{AnyView, Owner, RenderCtx, use_file_system};
use ratatui::{Terminal, backend::TestBackend};
use tokio::runtime::Runtime;

use crate::{
    app::{app_view, app_view_at_path},
    hooks::{Files, WorkspaceContext},
    services::{
        EditorProcess, EditorSession, EnvironmentReader, ExplorerEntry, ProcessLauncher,
        RecentFilesStore, Workspace,
    },
};

thread_local! {
    /// Whether the current synchronous test worker entered the shared runtime.
    static TEST_RUNTIME_ENTERED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Weak lease used to make the test lock reentrant on one worker thread.
    static TEST_CONTEXT_LEASE: std::cell::RefCell<Weak<TestLease>> =
        const { std::cell::RefCell::new(Weak::new()) };
}

/// Shared background runtime used by synchronous rendering tests.
static TEST_RUNTIME: OnceLock<&'static Runtime> = OnceLock::new();
/// Lock serializing tests that mount asynchronous filesystem components.
static TEST_CONTEXT_LOCK: Mutex<()> = Mutex::new(());

/// Reentrant lease for the asynchronous component-test lock.
struct TestLease {
    /// Global lock guard retained while any same-thread contexts remain alive.
    _guard: MutexGuard<'static, ()>,
}

/// Shared-context fixture that keeps its signal owner alive.
pub(super) struct TestContexts {
    /// Lease serializing asynchronous component tests across worker threads.
    _test_lease: Rc<TestLease>,
    /// Owner backing every arena-allocated shared file signal.
    _owner: Owner,
    /// Validated workspace resources provided through the shared hook.
    pub(super) workspace: WorkspaceContext,
    /// File-related signals and persistence provided through the shared hook.
    pub(super) files: Files,
}

impl TestContexts {
    /// Initializes a memory-backed shared file hook for `root`.
    ///
    /// # Arguments
    ///
    /// * `root` — Workspace root used to initialize the signals.
    ///
    /// # Returns
    ///
    /// A [`TestContexts`] retaining the shared file signals.
    pub(super) fn new(root: &Path) -> Self {
        Self::with_store(root, RecentFilesStore::memory())
    }

    /// Initializes shared file signals with an explicit recent-file store.
    ///
    /// # Arguments
    ///
    /// * `root` — Workspace root used to initialize the signals.
    /// * `recent_files_store` — Persistence service bundled with file signals.
    ///
    /// # Returns
    ///
    /// A [`TestContexts`] retaining the shared file signals.
    pub(super) fn with_store(root: &Path, recent_files_store: RecentFilesStore) -> Self {
        let test_lease = acquire_test_lease();
        enter_test_runtime();
        let owner = Owner::new();
        let filesystem = use_file_system(root).expect("the workspace should initialize");
        let workspace = Workspace::new(filesystem.root().to_path_buf());
        let (recent_paths, stored_paths, recent_error) =
            recent_files_store.load_for_workspace(&filesystem, &workspace);
        let recent_error = recent_error.map(|error| Arc::new(anyhow::Error::new(error)));
        let workspace = WorkspaceContext::new(workspace);
        let files =
            owner.with(|| Files::new(recent_paths, stored_paths, recent_error, recent_files_store));

        Self {
            _test_lease: test_lease,
            _owner: owner,
            workspace,
            files,
        }
    }

    /// Creates an application view starting on Home.
    ///
    /// # Returns
    ///
    /// An [`AnyView`] using this fixture's shared values.
    pub(super) fn view(&self) -> AnyView {
        app_view(self.workspace.clone(), self.files.clone())
    }

    /// Creates an application view starting at an explicit route.
    ///
    /// # Arguments
    ///
    /// * `path` — Initial router location.
    ///
    /// # Returns
    ///
    /// An [`AnyView`] using this fixture's shared values.
    pub(super) fn view_at(&self, path: impl Into<String>) -> AnyView {
        app_view_at_path(
            self.workspace.clone(),
            self.files.clone(),
            EditorSession::deferred(EditorProcess::new()),
            path,
        )
    }

    /// Creates an application view with an immediate injected editor process.
    ///
    /// # Arguments
    ///
    /// * `editor_process` — Process boundary executed without a real terminal.
    ///
    /// # Returns
    ///
    /// An [`AnyView`] whose Viewer completes editor requests synchronously.
    pub(super) fn view_with_editor(&self, editor_process: EditorProcess) -> AnyView {
        app_view_at_path(
            self.workspace.clone(),
            self.files.clone(),
            EditorSession::immediate(editor_process),
            "/",
        )
    }
}

/// Acquires or reuses the asynchronous component-test lock on this thread.
///
/// # Returns
///
/// A shared [`TestLease`] retained by the constructed test context.
fn acquire_test_lease() -> Rc<TestLease> {
    TEST_CONTEXT_LEASE.with(|slot| {
        if let Some(lease) = slot.borrow().upgrade() {
            return lease;
        }
        let guard = TEST_CONTEXT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let lease = Rc::new(TestLease { _guard: guard });
        *slot.borrow_mut() = Rc::downgrade(&lease);
        lease
    })
}

/// Program and argument values captured from one process launch.
pub(super) type RecordedCommand = (OsString, Vec<OsString>);

/// Result returned by one injected process launch.
#[derive(Clone, Copy, Debug)]
pub(super) enum TestLaunchOutcome {
    /// Reports a successful child exit.
    Success,
    /// Reports a non-zero child exit.
    NonZero,
    /// Reports a missing executable.
    NotFound,
}

/// Injectable launcher that records commands and supplies deterministic exits.
#[derive(Clone, Debug)]
pub(super) struct RecordingLauncher {
    /// Commands received by the launcher in call order.
    pub(super) commands: Arc<Mutex<Vec<RecordedCommand>>>,
    /// Outcome returned for each launch.
    pub(super) outcome: TestLaunchOutcome,
    /// Optional file replacement performed during a successful edit.
    pub(super) replacement: Option<(PathBuf, String)>,
}

/// Injectable environment containing optional editor configuration.
#[derive(Clone, Debug, Default)]
pub(super) struct TestEnvironment {
    /// Value returned for `VISUAL`.
    pub(super) visual: Option<OsString>,
    /// Value returned for `EDITOR`.
    pub(super) editor: Option<OsString>,
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
        self.commands
            .lock()
            .expect("recorded commands should not be poisoned")
            .push((
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
pub(super) struct TestTree {
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
    pub(super) fn new(label: &str) -> Self {
        let root = temporary_path(label);
        fs::create_dir(&root).expect("the temporary directory should be created");
        Self { root }
    }

    /// Returns the fixture root.
    ///
    /// # Returns
    ///
    /// A [`Path`] containing the temporary directory.
    pub(super) fn root(&self) -> &Path {
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
pub(super) fn temporary_path(label: &str) -> PathBuf {
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
pub(super) fn explorer_entry_names(entries: &[ExplorerEntry]) -> Vec<String> {
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
pub(super) fn draw_editor(
    terminal: &mut Terminal<TestBackend>,
    view: &AnyView,
) -> leptatui::Result<()> {
    let mut render_result = Ok(());

    for _ in 0..3 {
        thread::sleep(Duration::from_millis(20));
        terminal.draw(|frame| {
            let mut context = RenderCtx::new(frame);
            render_result = view.render(&mut context);
        })?;
    }

    render_result
}

/// Enters a shared background Tokio runtime on the current test thread.
///
/// Synchronous rendering tests predate filesystem operations. Retaining the entry
/// guard lets those tests dispatch real actions while the background workers
/// complete I/O between draws.
fn enter_test_runtime() {
    TEST_RUNTIME_ENTERED.with(|entered| {
        if entered.replace(true) {
            return;
        }
        let runtime = *TEST_RUNTIME.get_or_init(|| {
            Box::leak(Box::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("filesystem test runtime should initialize"),
            ))
        });
        let _ = Box::leak(Box::new(runtime.enter()));
    });
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
pub(super) fn rendered_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
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
