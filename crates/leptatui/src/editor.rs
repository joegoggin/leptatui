//! Restored-terminal external editor integration.
//!
//! The [`use_editor`] hook returns an [`Editor`] that queues synchronous editor
//! processes through the active application runtime. File edits operate on the
//! supplied path. Text edits round-trip a reactive [`RwSignal<String>`] through
//! a temporary Markdown file that is removed before completion is reported.

use std::{
    env,
    ffi::OsString,
    fmt, fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use leptos::prelude::{Get, GetUntracked, RwSignal, Set};
use tempfile::Builder;

use crate::{AppHandle, app::request_redraw, context};

/// Environment variable consulted first for an interactive visual editor.
const VISUAL_ENVIRONMENT_VARIABLE: &str = "VISUAL";
/// Environment variable consulted second for a terminal editor.
const EDITOR_ENVIRONMENT_VARIABLE: &str = "EDITOR";
/// Editor used when neither configured environment variable has a value.
const FALLBACK_EDITOR: &str = "vi";

/// Executes a prepared external-editor command and reports its success state.
trait ProcessLauncher: fmt::Debug + Send + Sync {
    /// Runs a prepared process command to completion.
    ///
    /// # Arguments
    ///
    /// * `command` — Fully configured editor command to execute.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the child exited successfully.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the process cannot be spawned or waited on.
    fn success(&self, command: &mut Command) -> io::Result<bool>;
}

/// Reads editor configuration from the process environment.
trait EnvironmentReader: fmt::Debug + Send + Sync {
    /// Returns one operating-system environment value.
    ///
    /// # Arguments
    ///
    /// * `name` — Environment variable name to read.
    ///
    /// # Returns
    ///
    /// An optional operating-system string containing the configured value.
    fn var_os(&self, name: &str) -> Option<OsString>;
}

/// Operating-system environment reader used by public editor handles.
#[derive(Debug)]
struct SystemEnvironmentReader;

impl EnvironmentReader for SystemEnvironmentReader {
    /// Returns one value from the current process environment.
    ///
    /// # Arguments
    ///
    /// * `name` — Environment variable name to read.
    ///
    /// # Returns
    ///
    /// An optional operating-system string containing the configured value.
    fn var_os(&self, name: &str) -> Option<OsString> {
        env::var_os(name)
    }
}

/// Operating-system process launcher used by public editor handles.
#[derive(Debug)]
struct SystemProcessLauncher;

impl ProcessLauncher for SystemProcessLauncher {
    /// Runs a prepared process through [`Command::status`].
    ///
    /// # Arguments
    ///
    /// * `command` — Fully configured editor command to execute.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the child exited successfully.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the process cannot be spawned or waited on.
    fn success(&self, command: &mut Command) -> io::Result<bool> {
        command.status().map(|status| status.success())
    }
}

/// Process boundary that resolves and launches the preferred editor.
#[derive(Clone, Debug)]
struct EditorProcess {
    /// Boundary used to execute prepared editor commands.
    launcher: Arc<dyn ProcessLauncher>,
    /// Boundary used to read editor environment variables.
    environment: Arc<dyn EnvironmentReader>,
}

impl EditorProcess {
    /// Creates a process boundary backed by the operating system.
    ///
    /// # Returns
    ///
    /// An [`EditorProcess`] using process environment and command services.
    fn system() -> Self {
        Self {
            launcher: Arc::new(SystemProcessLauncher),
            environment: Arc::new(SystemEnvironmentReader),
        }
    }

    /// Opens one path in the preferred editor and waits for it to exit.
    ///
    /// # Arguments
    ///
    /// * `path` — File path supplied to the editor.
    ///
    /// # Returns
    ///
    /// An empty result after the editor exits successfully.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if configuration is invalid, the editor cannot be
    /// launched or waited on, or the editor exits unsuccessfully.
    fn edit(&self, path: &Path) -> io::Result<()> {
        let mut editor = resolve_editor_command(self.environment.as_ref())?;
        let program = editor.remove(0);
        let has_separator = editor.last().is_some_and(|argument| argument == "--");
        let mut command = Command::new(&program);
        command.args(&editor);
        if !has_separator {
            command.arg("--");
        }
        command.arg(path);

        match self.launcher.success(&mut command) {
            Ok(true) => Ok(()),
            Ok(false) => Err(io::Error::other(format!(
                "editor '{}' exited with a non-zero status while editing '{}'",
                program.to_string_lossy(),
                path.display()
            ))),
            Err(source) => Err(io::Error::new(
                source.kind(),
                format!(
                    "failed to launch editor '{}' for '{}': {source}",
                    program.to_string_lossy(),
                    path.display()
                ),
            )),
        }
    }
}

/// Cloneable handle for opening files and reactive text in an external editor.
#[derive(Clone, Debug)]
pub struct Editor {
    /// Runtime handle that temporarily restores the managed terminal.
    app_handle: Option<AppHandle>,
    /// Process boundary used by every requested edit.
    process: EditorProcess,
    /// Shared reactive lifecycle state for external-editor requests.
    status: RwSignal<Option<EditorStatus>>,
}

/// Current lifecycle state of the external editor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditorStatus {
    /// Indicates that an editor request is queued or running.
    Pending,
    /// Indicates that the latest editor request completed successfully.
    Complete,
    /// Indicates that the latest editor request failed with the provided message.
    Error(String),
}

impl Editor {
    /// Opens a file in the user's preferred editor.
    ///
    /// The operation starts immediately and completes after the editor exits
    /// and Leptatui is ready to redraw the existing application.
    ///
    /// # Arguments
    ///
    /// * `file_path` — Path passed directly to the configured editor.
    pub fn edit_file(&self, file_path: impl Into<PathBuf>) {
        let path = file_path.into();
        self.start_operation(move |process| process.edit(&path));
    }

    /// Edits reactive text through a temporary Markdown file.
    ///
    /// The current signal value is captured when this method is called. A
    /// successful editor exit, UTF-8 read, and temporary-file cleanup replaces
    /// the signal value. Any failure preserves the original signal value.
    ///
    /// # Arguments
    ///
    /// * `text` — Reactive string to round-trip through the external editor.
    pub fn edit_text(&self, text: RwSignal<String>) {
        let initial = text.get_untracked();
        self.start_operation(move |process| edit_text(process, text, &initial));
    }

    /// Returns the current editor lifecycle state reactively.
    ///
    /// Reading this method inside an effect or dynamic view tracks the editor
    /// status as a dependency.
    ///
    /// # Returns
    ///
    /// An optional [`EditorStatus`] for the latest request.
    pub fn status(&self) -> Option<EditorStatus> {
        self.status.get()
    }

    /// Clears the current editor lifecycle state.
    pub fn clear(&self) {
        self.status.set(None);
        request_redraw();
    }

    /// Starts one editor operation through the managed terminal runtime.
    ///
    /// # Arguments
    ///
    /// * `task` — Synchronous editor work executed with normal terminal modes.
    ///
    fn start_operation(&self, task: impl FnOnce(&EditorProcess) -> io::Result<()> + 'static) {
        self.status.set(Some(EditorStatus::Pending));
        request_redraw();

        let Some(app_handle) = &self.app_handle else {
            self.status.set(Some(EditorStatus::Error(String::from(
                "external editing requires a managed Leptatui application",
            ))));
            request_redraw();
            return;
        };

        let process = self.process.clone();
        let status = self.status;
        app_handle.suspend_terminal(move || {
            let completed = match task(&process) {
                Ok(()) => EditorStatus::Complete,
                Err(error) => EditorStatus::Error(error.to_string()),
            };
            let _ = status.try_set(Some(completed));
            request_redraw();
        });
    }
}

/// Returns an editor handle for the current component.
///
/// The hook may be created outside a managed application for headless view
/// construction. Editing through such a handle reports an [`EditorStatus::Error`].
///
/// # Returns
///
/// An [`Editor`] connected to the nearest managed application when available.
pub fn use_editor() -> Editor {
    Editor {
        app_handle: context::use_context::<AppHandle>(),
        process: EditorProcess::system(),
        status: RwSignal::new(None),
    }
}

/// Resolves the preferred editor executable and configured arguments.
///
/// `VISUAL` takes precedence over `EDITOR`. Empty values are skipped, and `vi`
/// is returned when neither environment variable selects an editor.
///
/// # Arguments
///
/// * `environment` — Environment boundary supplying editor configuration.
///
/// # Returns
///
/// A non-empty vector containing the executable followed by its arguments.
///
/// # Errors
///
/// Returns [`io::Error`] if the selected value is not Unicode, has malformed
/// shell-word quoting, or does not name an executable.
fn resolve_editor_command(environment: &dyn EnvironmentReader) -> io::Result<Vec<OsString>> {
    for name in [VISUAL_ENVIRONMENT_VARIABLE, EDITOR_ENVIRONMENT_VARIABLE] {
        let Some(value) = environment.var_os(name) else {
            continue;
        };
        let value = value.into_string().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must contain valid Unicode"),
            )
        })?;
        if value.trim().is_empty() {
            continue;
        }

        let command = shlex::split(&value).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} contains malformed shell-word quoting"),
            )
        })?;
        if command.first().is_none_or(|program| program.is_empty()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must name an editor executable"),
            ));
        }

        return Ok(command.into_iter().map(OsString::from).collect());
    }

    Ok(vec![OsString::from(FALLBACK_EDITOR)])
}

/// Round-trips one string signal through a temporary Markdown file.
///
/// # Arguments
///
/// * `process` — Editor process used to open the temporary file.
/// * `text` — Reactive string updated after the temporary file is removed.
/// * `initial` — Signal snapshot written before the editor opens.
///
/// # Returns
///
/// An empty result after editing, reading, cleanup, and signal replacement.
///
/// # Errors
///
/// Returns [`io::Error`] if temporary-file I/O, editor execution, UTF-8
/// decoding, or temporary-file cleanup fails.
fn edit_text(process: &EditorProcess, text: RwSignal<String>, initial: &str) -> io::Result<()> {
    let mut temporary = Builder::new()
        .prefix("leptatui-editor-")
        .suffix(".md")
        .tempfile()?;
    temporary.write_all(initial.as_bytes())?;
    temporary.flush()?;

    let temporary = temporary.into_temp_path();
    let edit_result = process
        .edit(&temporary)
        .and_then(|()| fs::read_to_string(&temporary));
    let cleanup_result = temporary.close();

    match (edit_result, cleanup_result) {
        (Ok(updated), Ok(())) => {
            if text.try_set(updated).is_some() {
                Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "reactive text signal was disposed before editing completed",
                ))
            } else {
                Ok(())
            }
        }
        (Err(source), Ok(())) => Err(source),
        (Ok(_), Err(cleanup)) => Err(io::Error::new(
            cleanup.kind(),
            format!("failed to remove temporary editor file: {cleanup}"),
        )),
        (Err(source), Err(cleanup)) => Err(io::Error::new(
            source.kind(),
            format!("{source}; additionally failed to remove temporary editor file: {cleanup}"),
        )),
    }
}

#[cfg(test)]
/// Unit tests for editor resolution, process execution, and text round-trips.
mod tests {
    use std::{collections::VecDeque, sync::Mutex};

    use leptos::prelude::Owner;

    use super::*;

    /// Program and argument values captured from one process launch.
    type RecordedCommand = (OsString, Vec<OsString>);

    /// Injected editor process paired with its recorded command state.
    struct TestProcess {
        /// Process configured with deterministic test boundaries.
        process: EditorProcess,
        /// Commands received by the injected launcher.
        commands: Arc<Mutex<Vec<RecordedCommand>>>,
        /// Paths supplied as the final editor argument.
        paths: Arc<Mutex<Vec<PathBuf>>>,
        /// File bytes observed before each injected editor replacement.
        initial_contents: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    /// Result returned by an injected editor process.
    #[derive(Clone, Copy, Debug)]
    enum LaunchOutcome {
        /// Reports a successful editor exit.
        Success,
        /// Reports a non-zero editor exit.
        NonZero,
        /// Reports a missing editor executable.
        NotFound,
    }

    /// Injectable editor environment used by resolution tests.
    #[derive(Debug, Default)]
    struct TestEnvironment {
        /// Value returned for `VISUAL`.
        visual: Option<OsString>,
        /// Value returned for `EDITOR`.
        editor: Option<OsString>,
    }

    impl EnvironmentReader for TestEnvironment {
        /// Returns one configured environment value.
        ///
        /// # Arguments
        ///
        /// * `name` — Environment variable requested by the editor resolver.
        ///
        /// # Returns
        ///
        /// An optional configured operating-system string.
        fn var_os(&self, name: &str) -> Option<OsString> {
            match name {
                VISUAL_ENVIRONMENT_VARIABLE => self.visual.clone(),
                EDITOR_ENVIRONMENT_VARIABLE => self.editor.clone(),
                _ => None,
            }
        }
    }

    /// Injectable launcher that records commands and optionally replaces files.
    #[derive(Debug)]
    struct RecordingLauncher {
        /// Commands received by the launcher in call order.
        commands: Arc<Mutex<Vec<RecordedCommand>>>,
        /// Paths supplied as the final editor argument.
        paths: Arc<Mutex<Vec<PathBuf>>>,
        /// File bytes observed before each optional replacement.
        initial_contents: Arc<Mutex<Vec<Vec<u8>>>>,
        /// Outcomes returned after optional replacement in call order.
        outcomes: Mutex<VecDeque<LaunchOutcome>>,
        /// Bytes written to the edited path before returning.
        replacement: Option<Vec<u8>>,
    }

    impl ProcessLauncher for RecordingLauncher {
        /// Records and resolves one prepared command without spawning a process.
        ///
        /// # Arguments
        ///
        /// * `command` — Prepared editor command captured by the launcher.
        ///
        /// # Returns
        ///
        /// A boolean matching the configured successful or non-zero outcome.
        ///
        /// # Errors
        ///
        /// Returns [`io::ErrorKind::NotFound`] for the missing-editor outcome or
        /// if a configured replacement cannot be written.
        fn success(&self, command: &mut Command) -> io::Result<bool> {
            let arguments: Vec<OsString> = command.get_args().map(OsString::from).collect();
            let path = arguments
                .last()
                .map(PathBuf::from)
                .expect("the editor command should contain a path");
            self.commands
                .lock()
                .expect("recorded commands should not be poisoned")
                .push((command.get_program().to_os_string(), arguments));
            self.paths
                .lock()
                .expect("recorded paths should not be poisoned")
                .push(path.clone());
            self.initial_contents
                .lock()
                .expect("recorded contents should not be poisoned")
                .push(fs::read(&path).unwrap_or_default());

            if let Some(replacement) = &self.replacement {
                fs::write(path, replacement)?;
            }

            let outcome = self
                .outcomes
                .lock()
                .expect("configured outcomes should not be poisoned")
                .pop_front()
                .expect("each recorded launch should have a configured outcome");
            match outcome {
                LaunchOutcome::Success => Ok(true),
                LaunchOutcome::NonZero => Ok(false),
                LaunchOutcome::NotFound => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "injected missing editor executable",
                )),
            }
        }
    }

    /// Creates an injected editor process and its recorded state.
    ///
    /// # Arguments
    ///
    /// * `environment` — Environment values used for command resolution.
    /// * `outcomes` — Exit or launch outcomes returned in call order.
    /// * `replacement` — Optional bytes written to the edited path.
    ///
    /// # Returns
    ///
    /// A [`TestProcess`] containing the process and recorded state.
    fn test_process(
        environment: TestEnvironment,
        outcomes: Vec<LaunchOutcome>,
        replacement: Option<Vec<u8>>,
    ) -> TestProcess {
        let commands = Arc::new(Mutex::new(Vec::new()));
        let paths = Arc::new(Mutex::new(Vec::new()));
        let initial_contents = Arc::new(Mutex::new(Vec::new()));
        let process = EditorProcess {
            launcher: Arc::new(RecordingLauncher {
                commands: Arc::clone(&commands),
                paths: Arc::clone(&paths),
                initial_contents: Arc::clone(&initial_contents),
                outcomes: Mutex::new(outcomes.into()),
                replacement,
            }),
            environment: Arc::new(environment),
        };
        TestProcess {
            process,
            commands,
            paths,
            initial_contents,
        }
    }

    /// Verifies editor resolution honors precedence, quoting, and fallbacks.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// VISUAL="nvim --wait --cmd 'set textwidth=80'"
    /// EDITOR="ignored"
    /// VISUAL="   " EDITOR="nano --wait --"
    /// VISUAL unset, EDITOR unset
    /// ```
    ///
    /// # Assertions
    ///
    /// - `VISUAL` wins and preserves one quoted argument.
    /// - Empty `VISUAL` falls through to `EDITOR`.
    /// - Missing configuration resolves to `vi`.
    #[test]
    fn editor_resolution_honors_precedence_quoting_and_fallbacks() {
        let visual = resolve_editor_command(&TestEnvironment {
            visual: Some(OsString::from("nvim --wait --cmd 'set textwidth=80'")),
            editor: Some(OsString::from("ignored")),
        })
        .expect("quoted VISUAL configuration should resolve");
        let editor = resolve_editor_command(&TestEnvironment {
            visual: Some(OsString::from("   ")),
            editor: Some(OsString::from("nano --wait --")),
        })
        .expect("EDITOR fallback should resolve");
        let fallback = resolve_editor_command(&TestEnvironment::default())
            .expect("the vi fallback should resolve");

        assert_eq!(
            visual,
            ["nvim", "--wait", "--cmd", "set textwidth=80"].map(OsString::from)
        );
        assert_eq!(editor, ["nano", "--wait", "--"].map(OsString::from));
        assert_eq!(fallback, ["vi"].map(OsString::from));
    }

    /// Verifies malformed editor configuration is rejected before launch.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// VISUAL="editor 'unterminated"
    /// ```
    ///
    /// # Assertions
    ///
    /// - Resolution returns an invalid-input error.
    /// - The diagnostic identifies `VISUAL`.
    #[test]
    fn editor_resolution_rejects_malformed_configuration() {
        let error = resolve_editor_command(&TestEnvironment {
            visual: Some(OsString::from("editor 'unterminated")),
            editor: None,
        })
        .expect_err("malformed editor configuration should fail");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("VISUAL"));
    }

    /// Verifies file operations queue the configured editor and expose shared state.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// VISUAL="configured-editor --wait"
    /// edit_file("-guide with spaces.md")
    /// ```
    ///
    /// # Assertions
    ///
    /// - The editor status begins pending.
    /// - Executing the suspended task appends `--` and the exact path.
    /// - The editor and its clone observe completion.
    /// - Clearing either clone resets the shared status.
    #[test]
    fn edit_file_queues_command_and_completes_reactive_state() {
        let owner = Owner::new();
        owner.with(|| {
            let TestProcess {
                process, commands, ..
            } = test_process(
                TestEnvironment {
                    visual: Some(OsString::from("configured-editor --wait")),
                    editor: None,
                },
                vec![LaunchOutcome::Success],
                None,
            );
            let app_handle = AppHandle::new();
            let editor = Editor {
                app_handle: Some(app_handle.clone()),
                process,
                status: RwSignal::new(None),
            };
            let cloned_editor = editor.clone();
            let path = PathBuf::from("-guide with spaces.md");
            editor.edit_file(path.clone());

            assert_eq!(editor.status(), Some(EditorStatus::Pending));
            assert_eq!(cloned_editor.status(), Some(EditorStatus::Pending));
            for task in app_handle.take_suspended_tasks() {
                task();
            }

            assert_eq!(editor.status(), Some(EditorStatus::Complete));
            assert_eq!(cloned_editor.status(), Some(EditorStatus::Complete));
            cloned_editor.clear();
            assert_eq!(editor.status(), None);
            assert_eq!(
                commands
                    .lock()
                    .expect("recorded commands should not be poisoned")
                    .as_slice(),
                [(
                    OsString::from("configured-editor"),
                    vec![
                        OsString::from("--wait"),
                        OsString::from("--"),
                        path.into_os_string(),
                    ],
                )]
            );
        });
    }

    /// Verifies the last queued editor completion determines the shared status.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// edit_file("first.md") -> non-zero exit
    /// edit_file("second.md") -> successful exit
    /// ```
    ///
    /// # Assertions
    ///
    /// - Both editor tasks execute in submission order.
    /// - The final shared status reports the second task's successful completion.
    #[test]
    fn last_queued_editor_result_wins() {
        let owner = Owner::new();
        owner.with(|| {
            let TestProcess {
                process, commands, ..
            } = test_process(
                TestEnvironment {
                    visual: Some(OsString::from("configured-editor")),
                    editor: None,
                },
                vec![LaunchOutcome::NonZero, LaunchOutcome::Success],
                None,
            );
            let app_handle = AppHandle::new();
            let editor = Editor {
                app_handle: Some(app_handle.clone()),
                process,
                status: RwSignal::new(None),
            };

            editor.edit_file("first.md");
            editor.edit_file("second.md");
            for task in app_handle.take_suspended_tasks() {
                task();
            }

            assert_eq!(editor.status(), Some(EditorStatus::Complete));
            let paths: Vec<PathBuf> = commands
                .lock()
                .expect("recorded commands should not be poisoned")
                .iter()
                .map(|(_, arguments)| {
                    arguments
                        .last()
                        .map(PathBuf::from)
                        .expect("each command should contain a path")
                })
                .collect();
            assert_eq!(
                paths,
                [PathBuf::from("first.md"), PathBuf::from("second.md")]
            );
        });
    }

    /// Verifies a configured trailing separator is not duplicated.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// EDITOR="configured-editor --wait --"
    /// edit_file("guide.md")
    /// ```
    ///
    /// # Assertions
    ///
    /// - The editor process exits successfully.
    /// - Exactly one `--` appears before the file path.
    #[test]
    fn editor_process_preserves_configured_trailing_separator() {
        let TestProcess {
            process, commands, ..
        } = test_process(
            TestEnvironment {
                visual: None,
                editor: Some(OsString::from("configured-editor --wait --")),
            },
            vec![LaunchOutcome::Success],
            None,
        );

        process
            .edit(Path::new("guide.md"))
            .expect("the configured editor should succeed");

        assert_eq!(
            commands
                .lock()
                .expect("recorded commands should not be poisoned")
                .as_slice(),
            [(
                OsString::from("configured-editor"),
                vec![
                    OsString::from("--wait"),
                    OsString::from("--"),
                    OsString::from("guide.md"),
                ],
            )]
        );
    }

    /// Verifies editing without a managed application returns a visible error.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Editor without an AppHandle calls edit_file("guide.md")
    /// ```
    ///
    /// # Assertions
    ///
    /// - The status changes immediately from pending to error.
    /// - The diagnostic explains that a managed application is required.
    #[test]
    fn edit_without_managed_application_completes_with_error() {
        let owner = Owner::new();
        owner.with(|| {
            let editor = Editor {
                app_handle: None,
                process: EditorProcess::system(),
                status: RwSignal::new(None),
            };
            editor.edit_file("guide.md");

            assert_eq!(
                editor.status(),
                Some(EditorStatus::Error(String::from(
                    "external editing requires a managed Leptatui application"
                )))
            );
        });
    }

    /// Verifies text editing applies UTF-8 changes and removes its Markdown file.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// signal = "# Before"
    /// editor writes "# After"
    /// ```
    ///
    /// # Assertions
    ///
    /// - The editor receives a `.md` path containing the original signal value.
    /// - Successful UTF-8 replacement updates the signal.
    /// - The temporary path no longer exists after completion.
    #[test]
    fn edit_text_updates_signal_and_removes_temporary_markdown_file() {
        let owner = Owner::new();
        owner.with(|| {
            let TestProcess {
                process,
                paths,
                initial_contents,
                ..
            } = test_process(
                TestEnvironment {
                    visual: Some(OsString::from("configured-editor")),
                    editor: None,
                },
                vec![LaunchOutcome::Success],
                Some(b"# After\n".to_vec()),
            );
            let text = RwSignal::new(String::from("# Before\n"));

            edit_text(&process, text, &text.get_untracked())
                .expect("successful text editing should complete");

            assert_eq!(text.get_untracked(), "# After\n");
            assert_eq!(
                initial_contents
                    .lock()
                    .expect("recorded contents should not be poisoned")
                    .as_slice(),
                [b"# Before\n".to_vec()]
            );
            let paths = paths.lock().expect("recorded paths should not be poisoned");
            assert_eq!(paths.len(), 1);
            assert_eq!(
                paths[0].extension().and_then(|value| value.to_str()),
                Some("md")
            );
            assert!(!paths[0].exists());
        });
    }

    /// Verifies unsuccessful editor exits preserve text and remove temporary files.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// signal = "# Before"
    /// editor writes "# Unsaved" and exits non-zero
    /// ```
    ///
    /// # Assertions
    ///
    /// - The edit returns a non-zero-exit diagnostic.
    /// - The original signal remains unchanged.
    /// - The temporary path is removed despite the failure.
    #[test]
    fn edit_text_preserves_signal_and_cleans_up_after_non_zero_exit() {
        let owner = Owner::new();
        owner.with(|| {
            let TestProcess { process, paths, .. } = test_process(
                TestEnvironment {
                    visual: Some(OsString::from("configured-editor")),
                    editor: None,
                },
                vec![LaunchOutcome::NonZero],
                Some(b"# Unsaved\n".to_vec()),
            );
            let text = RwSignal::new(String::from("# Before\n"));

            let error = edit_text(&process, text, &text.get_untracked())
                .expect_err("a non-zero editor exit should fail");

            assert!(error.to_string().contains("non-zero status"));
            assert_eq!(text.get_untracked(), "# Before\n");
            let path = paths.lock().expect("recorded paths should not be poisoned")[0].clone();
            assert!(!path.exists());
        });
    }

    /// Verifies invalid edited bytes preserve text and remove temporary files.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// signal = "# Before"
    /// editor writes [0xff, 0xfe]
    /// ```
    ///
    /// # Assertions
    ///
    /// - Reading the edited file returns an invalid-data error.
    /// - The original signal remains unchanged.
    /// - The temporary path is removed after the read failure.
    #[test]
    fn edit_text_preserves_signal_and_cleans_up_after_invalid_utf8() {
        let owner = Owner::new();
        owner.with(|| {
            let TestProcess { process, paths, .. } = test_process(
                TestEnvironment {
                    visual: Some(OsString::from("configured-editor")),
                    editor: None,
                },
                vec![LaunchOutcome::Success],
                Some(vec![0xff, 0xfe]),
            );
            let text = RwSignal::new(String::from("# Before\n"));

            let error = edit_text(&process, text, &text.get_untracked())
                .expect_err("invalid UTF-8 should fail");

            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert_eq!(text.get_untracked(), "# Before\n");
            let path = paths.lock().expect("recorded paths should not be poisoned")[0].clone();
            assert!(!path.exists());
        });
    }

    /// Verifies text-edit launch errors preserve state and clean up temporary files.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// VISUAL="missing-editor"
    /// edit_text(signal)
    /// ```
    ///
    /// # Assertions
    ///
    /// - The process returns a not-found error.
    /// - The diagnostic identifies the failed editor launch.
    /// - The signal remains unchanged and the temporary file is removed.
    #[test]
    fn edit_text_preserves_launch_error_context_and_cleans_up() {
        let owner = Owner::new();
        owner.with(|| {
            let TestProcess { process, paths, .. } = test_process(
                TestEnvironment {
                    visual: Some(OsString::from("missing-editor")),
                    editor: None,
                },
                vec![LaunchOutcome::NotFound],
                None,
            );
            let text = RwSignal::new(String::from("# Before\n"));

            let error = edit_text(&process, text, &text.get_untracked())
                .expect_err("a missing editor should fail");

            assert_eq!(error.kind(), io::ErrorKind::NotFound);
            assert!(error.to_string().contains("failed to launch editor"));
            assert!(error.to_string().contains("missing-editor"));
            assert_eq!(text.get_untracked(), "# Before\n");
            let path = paths.lock().expect("recorded paths should not be poisoned")[0].clone();
            assert!(!path.exists());
        });
    }
}
