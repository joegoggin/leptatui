//! Shared fixtures, mocks, and rendering helpers for editor tests.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
    rc::{Rc, Weak},
    sync::{Mutex, MutexGuard, OnceLock},
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend};
use tokio::{runtime::Runtime, task::LocalSet};

use crate::{
    app::{AppRouter, AppRouterProps},
    services::RecentFilesStore,
};

/// Provides deterministic services to the routed application during tests.
///
/// # Arguments
///
/// * `initial_path` — First router location for the test application.
/// * `recent_files_store` — Isolated recent-file store supplied to routed pages.
///
/// # Returns
///
/// A routed application using the supplied service contexts.
#[component]
fn TestAppRouter(initial_path: String, recent_files_store: RecentFilesStore) -> impl IntoView {
    provide_context(recent_files_store);

    view! { <AppRouter initial_path=initial_path /> }
}

thread_local! {
    /// Whether the current synchronous test worker entered the shared runtime.
    static TEST_RUNTIME_ENTERED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Local executor used by effects spawned from synchronous component tests.
    static TEST_LOCAL_SET: std::cell::OnceCell<&'static LocalSet> =
        const { std::cell::OnceCell::new() };
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

/// Application fixture scoped to one temporary current directory.
pub(super) struct TestContexts {
    /// Lease serializing asynchronous component tests across worker threads.
    _test_lease: Rc<TestLease>,
    /// Process directory restored when this fixture is dropped.
    previous_directory: PathBuf,
    /// Recent-file persistence injected into every constructed application.
    pub(super) recent_files_store: RecentFilesStore,
}

impl TestContexts {
    /// Initializes an application fixture rooted at `root`.
    ///
    /// # Arguments
    ///
    /// * `root` — Directory selected as the process current directory.
    ///
    /// # Returns
    ///
    /// A [`TestContexts`] retaining the scoped current directory.
    pub(super) fn new(root: &Path) -> Self {
        Self::with_store(root, RecentFilesStore::memory())
    }

    /// Initializes an application fixture with an explicit recent-file store.
    ///
    /// # Arguments
    ///
    /// * `root` — Directory selected as the process current directory.
    /// * `recent_files_store` — Persistence service provided to the application.
    ///
    /// # Returns
    ///
    /// A [`TestContexts`] retaining the scoped current directory.
    pub(super) fn with_store(root: &Path, recent_files_store: RecentFilesStore) -> Self {
        let test_lease = acquire_test_lease();
        enter_test_runtime();
        let previous_directory = env::current_dir().expect("the current directory should resolve");
        env::set_current_dir(root).expect("the test current directory should be selected");

        Self {
            _test_lease: test_lease,
            previous_directory,
            recent_files_store,
        }
    }

    /// Creates an application view starting on Home.
    ///
    /// # Returns
    ///
    /// An [`AnyView`] using this fixture's shared values.
    pub(super) fn view(&self) -> AnyView {
        self.view_at("/")
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
        TestAppRouter::with_props(
            TestAppRouterProps::builder()
                .initial_path(path.into())
                .recent_files_store(self.recent_files_store.clone())
                .build(),
        )
        .into_view()
    }
}

impl Drop for TestContexts {
    /// Restores the process current directory before releasing the test lease.
    fn drop(&mut self) {
        env::set_current_dir(&self.previous_directory)
            .expect("the previous current directory should be restored");
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

/// Temporary directory tree removed automatically after an application test.
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
        drive_test_local_tasks();
        thread::sleep(Duration::from_millis(20));
        terminal.draw(|frame| {
            let mut context = RenderCtx::new(frame);
            render_result = view.render(&mut context);
        })?;
        drive_test_local_tasks();
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
        let _ = Box::leak(Box::new(test_local_set().enter()));
    });
}

/// Returns the persistent local executor for the current test worker.
///
/// # Returns
///
/// A leaked [`LocalSet`] whose lifetime matches the synchronous test worker.
fn test_local_set() -> &'static LocalSet {
    TEST_LOCAL_SET.with(|local_set| *local_set.get_or_init(|| Box::leak(Box::new(LocalSet::new()))))
}

/// Runs locally spawned component effects until they have yielded once.
fn drive_test_local_tasks() {
    let runtime = *TEST_RUNTIME
        .get()
        .expect("filesystem test runtime should be entered before rendering");
    runtime.block_on(test_local_set().run_until(async {
        tokio::task::yield_now().await;
    }));
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
