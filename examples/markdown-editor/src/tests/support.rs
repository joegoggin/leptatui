//! Shared fixtures, mocks, and rendering helpers for editor tests.

use std::{
    cell::RefCell,
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{self, Command},
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use leptatui::prelude::{RenderCtx, View};
use ratatui::{Terminal, backend::TestBackend};

use crate::{
    domain::ExplorerEntry,
    editor_process::{EnvironmentReader, ProcessLauncher},
};

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
    pub(super) commands: Rc<RefCell<Vec<RecordedCommand>>>,
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
pub(super) fn draw_editor<V>(terminal: &mut Terminal<TestBackend>, view: &V) -> leptatui::Result<()>
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
pub(super) fn rendered_position(
    terminal: &Terminal<TestBackend>,
    needle: &str,
) -> Option<(usize, usize)> {
    rendered_lines(terminal)
        .iter()
        .enumerate()
        .find_map(|(row, line)| line.find(needle).map(|column| (column, row)))
}
