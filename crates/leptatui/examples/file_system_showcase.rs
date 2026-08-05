//! Guided showcase for every root-scoped filesystem operation.
//!
//! The example creates an isolated process-specific root below the platform
//! temporary directory. A successful walkthrough executes one operation at a
//! time, retaining every result and preventing later steps from racing their
//! prerequisites. An optional failure walkthrough demonstrates expected
//! containment, decoding, missing-path, and destination-conflict errors.

use std::{io, path::PathBuf};

use leptatui::prelude::*;

/// Directory used for every visible walkthrough fixture.
const WALKTHROUGH_DIR: &str = "walkthrough";

/// Ordered successful filesystem operations.
const SUCCESS_STEPS: &[WalkthroughStep] = &[
    WalkthroughStep::CreateDirectory,
    WalkthroughStep::WriteFile,
    WalkthroughStep::AppendFile,
    WalkthroughStep::ResolvePath,
    WalkthroughStep::GetMetadata,
    WalkthroughStep::ReadDirectory,
    WalkthroughStep::ReadBytes,
    WalkthroughStep::ReadString,
    WalkthroughStep::ReplaceFile,
    WalkthroughStep::CopyFile,
    WalkthroughStep::RenameFile,
    WalkthroughStep::DeleteFile,
    WalkthroughStep::DeleteDirectory,
];

/// Ordered fixture and expected-error operations.
const FAILURE_STEPS: &[WalkthroughStep] = &[
    WalkthroughStep::CreateFailureDirectory,
    WalkthroughStep::WriteInvalidUtf8,
    WalkthroughStep::ReadInvalidUtf8,
    WalkthroughStep::ReadMissingFile,
    WalkthroughStep::ResolveTraversal,
    WalkthroughStep::ResolveHomeOutsideRoot,
    WalkthroughStep::WriteConflictSource,
    WalkthroughStep::WriteConflictDestination,
    WalkthroughStep::CopyToExistingDestination,
    WalkthroughStep::DeleteRoot,
    WalkthroughStep::CleanupFailureDirectory,
];

/// Walkthrough selected by the user.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TourKind {
    /// Ordered successful-operation walkthrough.
    Success,
    /// Ordered expected-failure walkthrough.
    Failure,
}

impl TourKind {
    /// Returns the display name for the walkthrough.
    ///
    /// # Returns
    ///
    /// A static string naming the walkthrough.
    const fn name(self) -> &'static str {
        match self {
            Self::Success => "Successful operation tour",
            Self::Failure => "Expected failure tour",
        }
    }

    /// Returns the ordered steps for the walkthrough.
    ///
    /// # Returns
    ///
    /// A static slice of [`WalkthroughStep`] values.
    const fn steps(self) -> &'static [WalkthroughStep] {
        match self {
            Self::Success => SUCCESS_STEPS,
            Self::Failure => FAILURE_STEPS,
        }
    }
}

/// One operation or fixture step in a guided walkthrough.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WalkthroughStep {
    /// Recursively creates the successful walkthrough directory tree.
    CreateDirectory,
    /// Creates the successful walkthrough text file.
    WriteFile,
    /// Appends bytes to the successful walkthrough text file.
    AppendFile,
    /// Resolves the successful walkthrough text file.
    ResolvePath,
    /// Reads metadata for the successful walkthrough text file.
    GetMetadata,
    /// Lists the successful walkthrough directory.
    ReadDirectory,
    /// Reads the successful walkthrough text file as bytes.
    ReadBytes,
    /// Reads the successful walkthrough text file as UTF-8.
    ReadString,
    /// Replaces the successful walkthrough text file atomically.
    ReplaceFile,
    /// Copies the successful walkthrough text file.
    CopyFile,
    /// Renames the completed copy.
    RenameFile,
    /// Deletes the renamed file.
    DeleteFile,
    /// Recursively deletes the successful walkthrough directory.
    DeleteDirectory,
    /// Creates the expected-failure walkthrough directory.
    CreateFailureDirectory,
    /// Writes deliberately invalid UTF-8 bytes.
    WriteInvalidUtf8,
    /// Reads invalid UTF-8 and expects a decoding error.
    ReadInvalidUtf8,
    /// Reads a missing file and expects a missing-path error.
    ReadMissingFile,
    /// Resolves parent traversal and expects a containment error.
    ResolveTraversal,
    /// Resolves the home directory and expects a containment error.
    ResolveHomeOutsideRoot,
    /// Writes the source fixture for a destination conflict.
    WriteConflictSource,
    /// Writes the existing destination for a destination conflict.
    WriteConflictDestination,
    /// Copies onto an existing destination and expects a conflict.
    CopyToExistingDestination,
    /// Deletes the scoped root and expects root protection.
    DeleteRoot,
    /// Removes fixtures created by the expected-failure walkthrough.
    CleanupFailureDirectory,
}

impl WalkthroughStep {
    /// Returns the operation call displayed for the step.
    ///
    /// # Returns
    ///
    /// A static string containing the public method and concrete arguments.
    const fn label(self) -> &'static str {
        match self {
            Self::CreateDirectory => "create_dir(\"walkthrough/nested/tree\")",
            Self::WriteFile => "write_file(\"walkthrough/demo.txt\", ...)",
            Self::AppendFile => "append_file(\"walkthrough/demo.txt\", ...)",
            Self::ResolvePath => "resolve_path(\"walkthrough/demo.txt\")",
            Self::GetMetadata => "get_metadata(\"walkthrough/demo.txt\")",
            Self::ReadDirectory => "read_dir(\"walkthrough\")",
            Self::ReadBytes => "read_file_as_bytes(\"walkthrough/demo.txt\")",
            Self::ReadString => "read_file_as_string(\"walkthrough/demo.txt\")",
            Self::ReplaceFile => "write_and_replace_file(\"walkthrough/demo.txt\", ...)",
            Self::CopyFile => "copy_file(\"walkthrough/demo.txt\", \"walkthrough/copy.txt\")",
            Self::RenameFile => "rename(\"walkthrough/copy.txt\", \"walkthrough/moved.txt\")",
            Self::DeleteFile => "delete_file(\"walkthrough/moved.txt\")",
            Self::DeleteDirectory => "delete_dir(\"walkthrough\")",
            Self::CreateFailureDirectory => "create_dir(\"walkthrough\")",
            Self::WriteInvalidUtf8 => "write_file(\"walkthrough/invalid.md\", [0xff, ...])",
            Self::ReadInvalidUtf8 => "read_file_as_string(\"walkthrough/invalid.md\")",
            Self::ReadMissingFile => "read_file_as_string(\"walkthrough/missing.txt\")",
            Self::ResolveTraversal => "resolve_path(\"../escape.txt\")",
            Self::ResolveHomeOutsideRoot => "resolve_path(\"~\")",
            Self::WriteConflictSource => "write_file(\"walkthrough/conflict-source.txt\", ...)",
            Self::WriteConflictDestination => {
                "write_file(\"walkthrough/conflict-destination.txt\", ...)"
            }
            Self::CopyToExistingDestination => {
                "copy_file(\"walkthrough/conflict-source.txt\", \"walkthrough/conflict-destination.txt\")"
            }
            Self::DeleteRoot => "delete_dir(\"\")",
            Self::CleanupFailureDirectory => "delete_dir(\"walkthrough\")",
        }
    }

    /// Returns the user-facing behavior explanation for the step.
    ///
    /// # Returns
    ///
    /// A static string describing the operation and its expected effect.
    const fn explanation(self) -> &'static str {
        match self {
            Self::CreateDirectory | Self::CreateFailureDirectory => {
                "Recursively creates the directory and every missing ancestor."
            }
            Self::WriteFile => "Creates or truncates a file, then writes all supplied bytes.",
            Self::AppendFile => "Creates the file if needed, then adds bytes at its end.",
            Self::ResolvePath => {
                "Expands path syntax, canonicalizes the target, and checks containment."
            }
            Self::GetMetadata => "Reports the target kind, length, permissions, and timestamps.",
            Self::ReadDirectory => "Returns safe contained entries in deterministic order.",
            Self::ReadBytes => "Reads the complete file without decoding its bytes.",
            Self::ReadString => "Reads and UTF-8 decodes the complete text file.",
            Self::ReplaceFile => "Writes a sibling temporary file, then replaces the destination.",
            Self::CopyFile => "Copies the file after the preceding write has completed.",
            Self::RenameFile => "Moves the completed copy without overwriting another entry.",
            Self::DeleteFile => "Removes the renamed file without following symbolic links.",
            Self::DeleteDirectory | Self::CleanupFailureDirectory => {
                "Recursively removes the walkthrough directory and its contents."
            }
            Self::WriteInvalidUtf8 => "Creates the invalid byte fixture used by the next step.",
            Self::ReadInvalidUtf8 => "Demonstrates the expected invalid UTF-8 diagnostic.",
            Self::ReadMissingFile => "Demonstrates the expected missing-file diagnostic.",
            Self::ResolveTraversal => "Demonstrates explicit parent-traversal protection.",
            Self::ResolveHomeOutsideRoot => {
                "Expands ~, then demonstrates that expansion cannot escape the root."
            }
            Self::WriteConflictSource => "Creates the source fixture for a copy conflict.",
            Self::WriteConflictDestination => {
                "Creates the destination fixture that copying must not overwrite."
            }
            Self::CopyToExistingDestination => {
                "Demonstrates the expected non-overwriting destination conflict."
            }
            Self::DeleteRoot => "Demonstrates that recursive deletion protects the scoped root.",
        }
    }

    /// Returns the outcome required to complete the step.
    ///
    /// # Returns
    ///
    /// An [`ExpectedOutcome`] describing success or an expected error kind.
    const fn expected(self) -> ExpectedOutcome {
        match self {
            Self::ReadInvalidUtf8 => ExpectedOutcome::Error(io::ErrorKind::InvalidData),
            Self::ReadMissingFile => ExpectedOutcome::Error(io::ErrorKind::NotFound),
            Self::ResolveTraversal | Self::ResolveHomeOutsideRoot | Self::DeleteRoot => {
                ExpectedOutcome::Error(io::ErrorKind::PermissionDenied)
            }
            Self::CopyToExistingDestination => ExpectedOutcome::Error(io::ErrorKind::AlreadyExists),
            _ => ExpectedOutcome::Success,
        }
    }

    /// Starts the filesystem operation represented by the step.
    ///
    /// # Arguments
    ///
    /// * `filesystem` — Component-local handle used to start the operation.
    ///
    /// # Returns
    ///
    /// A typed [`StepOperation`] that has already been dispatched.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    fn start(self, filesystem: &FileSystem) -> StepOperation {
        match self {
            Self::CreateDirectory => {
                StepOperation::Unit(filesystem.create_dir("walkthrough/nested/tree"))
            }
            Self::WriteFile => StepOperation::Unit(
                filesystem.write_file("walkthrough/demo.txt", b"hello from leptatui"),
            ),
            Self::AppendFile => {
                StepOperation::Unit(filesystem.append_file("walkthrough/demo.txt", b" + append"))
            }
            Self::ResolvePath => {
                StepOperation::Path(filesystem.resolve_path("walkthrough/demo.txt"))
            }
            Self::GetMetadata => {
                StepOperation::Metadata(filesystem.get_metadata("walkthrough/demo.txt"))
            }
            Self::ReadDirectory => StepOperation::Entries(filesystem.read_dir(WALKTHROUGH_DIR)),
            Self::ReadBytes => {
                StepOperation::Bytes(filesystem.read_file_as_bytes("walkthrough/demo.txt"))
            }
            Self::ReadString => {
                StepOperation::Text(filesystem.read_file_as_string("walkthrough/demo.txt"))
            }
            Self::ReplaceFile => StepOperation::Unit(
                filesystem.write_and_replace_file("walkthrough/demo.txt", b"atomic replacement"),
            ),
            Self::CopyFile => StepOperation::ByteCount(
                filesystem.copy_file("walkthrough/demo.txt", "walkthrough/copy.txt"),
            ),
            Self::RenameFile => StepOperation::Unit(
                filesystem.rename("walkthrough/copy.txt", "walkthrough/moved.txt"),
            ),
            Self::DeleteFile => {
                StepOperation::Unit(filesystem.delete_file("walkthrough/moved.txt"))
            }
            Self::DeleteDirectory => StepOperation::Unit(filesystem.delete_dir(WALKTHROUGH_DIR)),
            Self::CreateFailureDirectory => {
                StepOperation::Unit(filesystem.create_dir(WALKTHROUGH_DIR))
            }
            Self::WriteInvalidUtf8 => StepOperation::Unit(
                filesystem.write_file("walkthrough/invalid.md", [0xff, 0xfe, 0xfd]),
            ),
            Self::ReadInvalidUtf8 => {
                StepOperation::Text(filesystem.read_file_as_string("walkthrough/invalid.md"))
            }
            Self::ReadMissingFile => {
                StepOperation::Text(filesystem.read_file_as_string("walkthrough/missing.txt"))
            }
            Self::ResolveTraversal => StepOperation::Path(filesystem.resolve_path("../escape.txt")),
            Self::ResolveHomeOutsideRoot => StepOperation::Path(filesystem.resolve_path("~")),
            Self::WriteConflictSource => StepOperation::Unit(
                filesystem.write_file("walkthrough/conflict-source.txt", b"source"),
            ),
            Self::WriteConflictDestination => StepOperation::Unit(
                filesystem.write_file("walkthrough/conflict-destination.txt", b"destination"),
            ),
            Self::CopyToExistingDestination => StepOperation::ByteCount(filesystem.copy_file(
                "walkthrough/conflict-source.txt",
                "walkthrough/conflict-destination.txt",
            )),
            Self::DeleteRoot => StepOperation::Unit(filesystem.delete_dir("")),
            Self::CleanupFailureDirectory => {
                StepOperation::Unit(filesystem.delete_dir(WALKTHROUGH_DIR))
            }
        }
    }
}

/// Outcome required before a walkthrough step unlocks its successor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedOutcome {
    /// The operation must complete successfully.
    Success,
    /// The operation must return the specified error kind.
    Error(io::ErrorKind),
}

impl ExpectedOutcome {
    /// Returns whether an observed outcome satisfies the expectation.
    ///
    /// # Arguments
    ///
    /// * `outcome` — Completed operation outcome to compare.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the step passed.
    fn matches(self, outcome: &OperationOutcome) -> bool {
        match (self, outcome) {
            (Self::Success, OperationOutcome::Success(_)) => true,
            (Self::Error(expected), OperationOutcome::Error { kind, .. }) => expected == *kind,
            _ => false,
        }
    }
}

/// Normalized completed result used by walkthrough state and rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
enum OperationOutcome {
    /// Successful operation output formatted for display.
    Success(String),
    /// Failed operation kind and diagnostic formatted for display.
    Error {
        /// Portable error classification.
        kind: io::ErrorKind,
        /// User-facing error message.
        message: String,
    },
}

impl OperationOutcome {
    /// Formats the outcome for one completed status row.
    ///
    /// # Arguments
    ///
    /// * `expected` — Outcome required by the walkthrough step.
    ///
    /// # Returns
    ///
    /// A [`String`] describing a success, expected error, or unexpected error.
    fn display(&self, expected: ExpectedOutcome) -> String {
        match self {
            Self::Success(output) => format!("ok: {output}"),
            Self::Error { kind, message } if expected.matches(self) => {
                format!("expected {kind:?}: {message}")
            }
            Self::Error { kind, message } => format!("unexpected {kind:?}: {message}"),
        }
    }
}

/// Typed filesystem operation started by one walkthrough step.
#[derive(Clone)]
enum StepOperation {
    /// Canonical path output.
    Path(FileOperation<PathBuf>),
    /// Portable metadata output.
    Metadata(FileOperation<FileMetadata>),
    /// Directory entry output.
    Entries(FileOperation<Vec<FileEntry>>),
    /// Raw byte output.
    Bytes(FileOperation<Vec<u8>>),
    /// UTF-8 string output.
    Text(FileOperation<String>),
    /// Copied byte-count output.
    ByteCount(FileOperation<u64>),
    /// Operation with no data output.
    Unit(FileOperation<()>),
}

impl StepOperation {
    /// Returns the tracked completion version for the operation.
    ///
    /// # Returns
    ///
    /// A completion count that changes after each visible attempt.
    fn version(&self) -> usize {
        match self {
            Self::Path(operation) => operation.version().get(),
            Self::Metadata(operation) => operation.version().get(),
            Self::Entries(operation) => operation.version().get(),
            Self::Bytes(operation) => operation.version().get(),
            Self::Text(operation) => operation.version().get(),
            Self::ByteCount(operation) => operation.version().get(),
            Self::Unit(operation) => operation.version().get(),
        }
    }

    /// Returns whether the latest attempt remains pending.
    ///
    /// # Returns
    ///
    /// A boolean containing the untracked pending state.
    fn is_pending(&self) -> bool {
        match self {
            Self::Path(operation) => operation.is_pending_untracked(),
            Self::Metadata(operation) => operation.is_pending_untracked(),
            Self::Entries(operation) => operation.is_pending_untracked(),
            Self::Bytes(operation) => operation.is_pending_untracked(),
            Self::Text(operation) => operation.is_pending_untracked(),
            Self::ByteCount(operation) => operation.is_pending_untracked(),
            Self::Unit(operation) => operation.is_pending_untracked(),
        }
    }

    /// Retries the operation with its captured arguments.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    fn retry(&self) {
        match self {
            Self::Path(operation) => operation.dispatch(()),
            Self::Metadata(operation) => operation.dispatch(()),
            Self::Entries(operation) => operation.dispatch(()),
            Self::Bytes(operation) => operation.dispatch(()),
            Self::Text(operation) => operation.dispatch(()),
            Self::ByteCount(operation) => operation.dispatch(()),
            Self::Unit(operation) => operation.dispatch(()),
        }
    }

    /// Returns the latest completed operation outcome.
    ///
    /// # Returns
    ///
    /// An optional normalized [`OperationOutcome`].
    fn outcome(&self) -> Option<OperationOutcome> {
        match self {
            Self::Path(operation) => {
                operation_outcome(operation, |path| path.display().to_string())
            }
            Self::Metadata(operation) => operation_outcome(operation, |metadata| {
                format!(
                    "kind={:?}, len={}, readonly={}",
                    metadata.kind(),
                    metadata.len(),
                    metadata.readonly()
                )
            }),
            Self::Entries(operation) => operation_outcome(operation, |entries| {
                let names = entries
                    .iter()
                    .map(|entry| entry.name().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} entries: [{names}]", entries.len())
            }),
            Self::Bytes(operation) => operation_outcome(operation, |bytes| format!("{bytes:?}")),
            Self::Text(operation) => operation_outcome(operation, |source| format!("{source:?}")),
            Self::ByteCount(operation) => {
                operation_outcome(operation, |count| format!("{count} bytes copied"))
            }
            Self::Unit(operation) => operation_outcome(operation, |()| String::from("completed")),
        }
    }
}

/// Normalizes a typed operation's latest result.
///
/// # Arguments
///
/// * `operation` — Completed operation whose value should be inspected.
/// * `format` — Formatter for successful typed output.
///
/// # Returns
///
/// An optional normalized [`OperationOutcome`].
fn operation_outcome<T>(
    operation: &FileOperation<T>,
    format: impl FnOnce(&T) -> String,
) -> Option<OperationOutcome>
where
    T: Send + Sync + 'static,
{
    operation.value().with_untracked(|value| {
        value.as_ref().map(|result| match result {
            Ok(output) => OperationOutcome::Success(format(output)),
            Err(error) => OperationOutcome::Error {
                kind: error.kind(),
                message: error.to_string(),
            },
        })
    })
}

/// Visible status retained for one walkthrough step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StepStatus {
    /// Step remains locked behind an earlier operation.
    Locked,
    /// Step is ready to execute.
    Ready,
    /// Step currently has an in-flight operation.
    Pending,
    /// Step completed with its expected outcome.
    Passed,
    /// Step completed with an unexpected outcome.
    Failed,
}

/// Retained state for one visible walkthrough row.
#[derive(Clone, Debug, Eq, PartialEq)]
struct StepRecord {
    /// Current lifecycle status.
    status: StepStatus,
    /// Number of started attempts.
    attempts: usize,
    /// Latest completed result formatted for display.
    result: Option<String>,
}

impl StepRecord {
    /// Creates a locked walkthrough record.
    ///
    /// # Returns
    ///
    /// A [`StepRecord`] with no attempts or result.
    const fn locked() -> Self {
        Self {
            status: StepStatus::Locked,
            attempts: 0,
            result: None,
        }
    }
}

/// Page-local progress through one guided walkthrough.
#[derive(Clone, Debug, Eq, PartialEq)]
struct WalkthroughState {
    /// Active success or expected-failure tour.
    tour: TourKind,
    /// Row currently selected for inspection.
    selected: usize,
    /// First step that has not passed.
    frontier: usize,
    /// Retained visible state for every tour step.
    records: Vec<StepRecord>,
    /// Whether reset cleanup is in progress.
    resetting: bool,
    /// Latest unexpected reset diagnostic.
    reset_error: Option<String>,
}

impl WalkthroughState {
    /// Creates fresh state for one walkthrough.
    ///
    /// # Arguments
    ///
    /// * `tour` — Walkthrough whose ordered records should be created.
    ///
    /// # Returns
    ///
    /// A [`WalkthroughState`] with only its first step unlocked.
    fn new(tour: TourKind) -> Self {
        let mut records = vec![StepRecord::locked(); tour.steps().len()];
        if let Some(first) = records.first_mut() {
            first.status = StepStatus::Ready;
        }
        Self {
            tour,
            selected: 0,
            frontier: 0,
            records,
            resetting: false,
            reset_error: None,
        }
    }

    /// Returns whether every step has passed.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the tour is complete.
    fn is_complete(&self) -> bool {
        self.frontier == self.records.len()
    }

    /// Selects the previous visible row without undoing its operation.
    fn select_back(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Moves toward the frontier through already completed history.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether selection moved without executing a step.
    fn select_forward_history(&mut self) -> bool {
        if self.selected < self.frontier.min(self.records.len().saturating_sub(1)) {
            self.selected += 1;
            return true;
        }
        false
    }

    /// Marks the frontier step as pending.
    ///
    /// # Returns
    ///
    /// An optional [`WalkthroughStep`] to start.
    fn begin_frontier_attempt(&mut self) -> Option<WalkthroughStep> {
        if self.resetting || self.is_complete() || self.selected != self.frontier {
            return None;
        }
        let record = self.records.get_mut(self.frontier)?;
        if !matches!(record.status, StepStatus::Ready | StepStatus::Failed) {
            return None;
        }
        record.status = StepStatus::Pending;
        record.attempts += 1;
        record.result = None;
        self.tour.steps().get(self.frontier).copied()
    }

    /// Marks one attempted step with its observed outcome.
    ///
    /// # Arguments
    ///
    /// * `index` — Attempted step index.
    /// * `outcome` — Normalized completed operation outcome.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the observed outcome passed the step.
    fn finish_step(&mut self, index: usize, outcome: &OperationOutcome) -> bool {
        let Some(step) = self.tour.steps().get(index).copied() else {
            return false;
        };
        let expected = step.expected();
        let passed = expected.matches(outcome);
        let Some(record) = self.records.get_mut(index) else {
            return false;
        };
        record.result = Some(outcome.display(expected));
        record.status = if passed {
            StepStatus::Passed
        } else {
            StepStatus::Failed
        };
        self.selected = index;
        if passed && index == self.frontier {
            self.frontier += 1;
            if let Some(next) = self.records.get_mut(self.frontier) {
                next.status = StepStatus::Ready;
                self.selected = self.frontier;
            }
        }
        passed
    }
}

/// Active step attempt or reset cleanup operation.
#[derive(Clone)]
enum ActiveTask {
    /// User-visible walkthrough step.
    Step {
        /// Identifier distinguishing replacement operations.
        token: u64,
        /// Index of the attempted walkthrough step.
        index: usize,
        /// Typed filesystem operation.
        operation: StepOperation,
    },
    /// Cleanup performed before returning to the first success step.
    Reset {
        /// Identifier distinguishing replacement operations.
        token: u64,
        /// Recursive walkthrough-directory deletion.
        operation: StepOperation,
    },
}

impl ActiveTask {
    /// Returns the task identifier.
    ///
    /// # Returns
    ///
    /// A monotonic identifier for the active task.
    const fn token(&self) -> u64 {
        match self {
            Self::Step { token, .. } | Self::Reset { token, .. } => *token,
        }
    }

    /// Returns the typed filesystem operation.
    ///
    /// # Returns
    ///
    /// A shared reference to the active [`StepOperation`].
    const fn operation(&self) -> &StepOperation {
        match self {
            Self::Step { operation, .. } | Self::Reset { operation, .. } => operation,
        }
    }
}

/// Renders and controls the guided filesystem walkthrough.
///
/// # Arguments
///
/// * `root` — Isolated path used as the component's filesystem boundary.
///
/// # Returns
///
/// A Leptatui view containing the guided operation list, or an initialization
/// error.
#[component]
fn FileSystemShowcase(root: PathBuf) -> ViewResult<impl IntoView> {
    let filesystem =
        use_file_system_with_options(root, FileSystemOptions::new().create_root(true))?;
    let state = ArcRwSignal::new(WalkthroughState::new(TourKind::Success));
    let active = ArcRwSignal::new(None::<ActiveTask>);
    let next_token = ArcRwSignal::new(0_u64);

    let completion_version_active = active.clone();
    let completion_result_active = active.clone();
    let completion_state = state.clone();
    let completion_clear = active.clone();
    Effect::watch_sync(
        move || {
            completion_version_active
                .get()
                .map(|task| (task.token(), task.operation().version()))
                .unwrap_or_default()
        },
        move |(_, version), _, _| {
            if *version == 0 {
                return;
            }
            let Some(task) = completion_result_active.get_untracked() else {
                return;
            };
            let Some(outcome) = task.operation().outcome() else {
                return;
            };
            match task {
                ActiveTask::Step { index, .. } => {
                    let mut passed = false;
                    completion_state.update(|state| {
                        passed = state.finish_step(index, &outcome);
                    });
                    if passed {
                        completion_clear.set(None);
                    }
                }
                ActiveTask::Reset { .. } => {
                    let reset_succeeded = matches!(outcome, OperationOutcome::Success(_))
                        || matches!(
                            outcome,
                            OperationOutcome::Error {
                                kind: io::ErrorKind::NotFound,
                                ..
                            }
                        );
                    if reset_succeeded {
                        completion_state.set(WalkthroughState::new(TourKind::Success));
                        completion_clear.set(None);
                    } else {
                        completion_state.update(|state| {
                            state.resetting = false;
                            state.reset_error = Some(outcome.display(ExpectedOutcome::Success));
                        });
                    }
                }
            }
        },
        false,
    );

    let command_filesystem = filesystem.clone();
    let command_state = state.clone();
    let command_active = active.clone();
    let command_token = next_token.clone();
    use_key_event(KeyEventKind::Press, move |key| {
        let reset_key = key.code == KeyCode::Char('R') && key.modifiers == KeyModifiers::SHIFT;
        if key.modifiers != KeyModifiers::NONE && !reset_key {
            return KeyControl::Pass;
        }
        match key.code {
            KeyCode::Char('q') => KeyControl::Exit,
            KeyCode::Char('b') => {
                command_state.update(WalkthroughState::select_back);
                KeyControl::Handled
            }
            KeyCode::Char('n') | KeyCode::Enter => {
                start_or_advance(
                    &command_filesystem,
                    &command_state,
                    &command_active,
                    &command_token,
                );
                KeyControl::Handled
            }
            KeyCode::Char('r') => {
                retry_failed_step(&command_state, &command_active);
                KeyControl::Handled
            }
            KeyCode::Char('R') => {
                reset_walkthrough(
                    &command_filesystem,
                    &command_state,
                    &command_active,
                    &command_token,
                );
                KeyControl::Handled
            }
            KeyCode::Char('f') => {
                start_failure_tour(&command_state, &command_active);
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    });

    stylesheet! {
        .screen => {
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            ),
            borders: Borders::ALL,
            padding: TuiSpacing::uniform(1),
            overflow: Axes::new(Overflow::Hidden, Overflow::Auto)

            @media (max-width: 60) {
                padding: TuiSpacing::ZERO
            }
        }
        .title => { fg: Color::LightCyan, modifier: Modifier::BOLD }
        .root => { fg: Color::LightGreen }
        .help => { fg: Color::Gray }
        .selected => { fg: Color::LightCyan, modifier: Modifier::BOLD }
        .passed => { fg: Color::LightGreen }
        .failed => { fg: Color::LightRed }
        .pending => { fg: Color::Yellow }
        .locked => { fg: Color::DarkGray }
    }

    let root = filesystem.root().display().to_string();
    let view_state = state.clone();
    let view_active = active.clone();
    let walkthrough = dynamic(move || render_walkthrough(&view_state, &view_active));

    view! {
        <Div class="screen">
            <Text class="title">"Guided root-scoped filesystem showcase"</Text>
            <Text class="root">{format!("Root: {root}")}</Text>
            <Text class="help">
                "Each component-local method starts immediately and returns FileOperation<T>."
            </Text>
            <Text class="help">
                "n/Enter next or run | b inspect previous | r retry failure | R reset | f failure tour | q quit"
            </Text>
            {walkthrough}
        </Div>
    }
}

/// Executes the frontier step or moves forward through completed history.
///
/// # Arguments
///
/// * `filesystem` — Component-local filesystem handle.
/// * `state` — Shared guided walkthrough progress.
/// * `active` — Shared active operation slot.
/// * `next_token` — Monotonic active-task identifier.
fn start_or_advance(
    filesystem: &FileSystem,
    state: &ArcRwSignal<WalkthroughState>,
    active: &ArcRwSignal<Option<ActiveTask>>,
    next_token: &ArcRwSignal<u64>,
) {
    if active
        .get_untracked()
        .is_some_and(|task| task.operation().is_pending())
    {
        return;
    }
    let mut step = None;
    let mut index = 0;
    state.update(|state| {
        if state.select_forward_history() {
            return;
        }
        index = state.frontier;
        step = state.begin_frontier_attempt();
    });
    let Some(step) = step else {
        return;
    };
    let token = increment_token(next_token);
    active.set(Some(ActiveTask::Step {
        token,
        index,
        operation: step.start(filesystem),
    }));
}

/// Retries an unexpectedly failed frontier operation.
///
/// # Arguments
///
/// * `state` — Shared guided walkthrough progress.
/// * `active` — Shared active operation slot.
fn retry_failed_step(
    state: &ArcRwSignal<WalkthroughState>,
    active: &ArcRwSignal<Option<ActiveTask>>,
) {
    let Some(ActiveTask::Step {
        index, operation, ..
    }) = active.get_untracked()
    else {
        return;
    };
    let should_retry = state.with_untracked(|state| {
        index == state.frontier
            && state
                .records
                .get(index)
                .is_some_and(|record| record.status == StepStatus::Failed)
    });
    if !should_retry || operation.is_pending() {
        return;
    }
    state.update(|state| {
        if let Some(record) = state.records.get_mut(index) {
            record.status = StepStatus::Pending;
            record.attempts += 1;
            record.result = None;
        }
    });
    operation.retry();
}

/// Starts or retries cleanup before returning to the first successful step.
///
/// # Arguments
///
/// * `filesystem` — Component-local filesystem handle.
/// * `state` — Shared guided walkthrough progress.
/// * `active` — Shared active operation slot.
/// * `next_token` — Monotonic active-task identifier.
fn reset_walkthrough(
    filesystem: &FileSystem,
    state: &ArcRwSignal<WalkthroughState>,
    active: &ArcRwSignal<Option<ActiveTask>>,
    next_token: &ArcRwSignal<u64>,
) {
    if let Some(task) = active.get_untracked() {
        if task.operation().is_pending() {
            return;
        }
        if matches!(task, ActiveTask::Reset { .. }) {
            state.update(|state| {
                state.resetting = true;
                state.reset_error = None;
            });
            task.operation().retry();
            return;
        }
    }
    state.update(|state| {
        state.resetting = true;
        state.reset_error = None;
    });
    active.set(Some(ActiveTask::Reset {
        token: increment_token(next_token),
        operation: StepOperation::Unit(filesystem.delete_dir(WALKTHROUGH_DIR)),
    }));
}

/// Switches a completed successful walkthrough to the expected-failure tour.
///
/// # Arguments
///
/// * `state` — Shared guided walkthrough progress.
/// * `active` — Shared active operation slot.
fn start_failure_tour(
    state: &ArcRwSignal<WalkthroughState>,
    active: &ArcRwSignal<Option<ActiveTask>>,
) {
    let available = state.with_untracked(|state| {
        state.tour == TourKind::Success && state.is_complete() && !state.resetting
    });
    if !available {
        return;
    }
    state.set(WalkthroughState::new(TourKind::Failure));
    active.set(None);
}

/// Increments and returns a task identifier.
///
/// # Arguments
///
/// * `next_token` — Shared monotonic token signal.
///
/// # Returns
///
/// A non-zero identifier for a newly started task.
fn increment_token(next_token: &ArcRwSignal<u64>) -> u64 {
    next_token.update(|token| *token = token.wrapping_add(1));
    next_token.get_untracked()
}

/// Renders retained walkthrough progress and current instructions.
///
/// # Arguments
///
/// * `state` — Shared guided walkthrough progress.
/// * `active` — Shared active operation slot.
///
/// # Returns
///
/// An [`AnyView`] containing the tour summary, selected detail, and step rows.
fn render_walkthrough(
    state: &ArcRwSignal<WalkthroughState>,
    active: &ArcRwSignal<Option<ActiveTask>>,
) -> AnyView {
    let state = state.get_untracked();
    let steps = state.tour.steps();
    let pending = active
        .get_untracked()
        .is_some_and(|task| task.operation().is_pending());
    let mut views = Vec::new();
    views.push(
        text(format!(
            "{} · {}/{} completed",
            state.tour.name(),
            state.frontier,
            steps.len()
        ))
        .with_classes("selected")
        .into_view(),
    );
    if let Some(step) = steps.get(state.selected) {
        views.push(
            text(format!(
                "Selected: {} — {}",
                step.label(),
                step.explanation()
            ))
            .with_classes("help")
            .into_view(),
        );
    }
    if state.resetting {
        views.push(
            text("Resetting the walkthrough directory...")
                .with_classes("pending")
                .into_view(),
        );
    } else if let Some(error) = &state.reset_error {
        views.push(
            text(format!("Reset failed: {error} · press R to retry"))
                .with_classes("failed")
                .into_view(),
        );
    } else if state.is_complete() && state.tour == TourKind::Success {
        views.push(
            text("Success tour complete · press f for the expected failure tour or R to restart")
                .with_classes("passed")
                .into_view(),
        );
    } else if state.is_complete() {
        views.push(
            text("Expected failure tour complete · press R to restart")
                .with_classes("passed")
                .into_view(),
        );
    } else if pending {
        views.push(
            text("Current operation is pending; advancement waits for its result.")
                .with_classes("pending")
                .into_view(),
        );
    } else if state
        .records
        .get(state.frontier)
        .is_some_and(|record| record.status == StepStatus::Failed)
    {
        views.push(
            text("The current step returned an unexpected result · press r to retry")
                .with_classes("failed")
                .into_view(),
        );
    } else {
        views.push(
            text("Press n or Enter to run the selected unlocked step.")
                .with_classes("help")
                .into_view(),
        );
    }

    for (index, (step, record)) in steps.iter().zip(&state.records).enumerate() {
        let marker = match record.status {
            StepStatus::Locked => "·",
            StepStatus::Ready => ">",
            StepStatus::Pending => "…",
            StepStatus::Passed => "✓",
            StepStatus::Failed => "!",
        };
        let selected = if index == state.selected { "▶" } else { " " };
        let attempts = (record.attempts > 1).then(|| format!(" · attempt {}", record.attempts));
        let result = record
            .result
            .as_ref()
            .map(|result| format!(" · {result}"))
            .unwrap_or_default();
        let line = format!(
            "{selected} {marker} {:02}. {}{}{}",
            index + 1,
            step.label(),
            attempts.unwrap_or_default(),
            result
        );
        let class = if index == state.selected {
            "selected"
        } else {
            match record.status {
                StepStatus::Locked => "locked",
                StepStatus::Ready => "selected",
                StepStatus::Pending => "pending",
                StepStatus::Passed => "passed",
                StepStatus::Failed => "failed",
            }
        };
        views.push(text(line).with_classes(class).into_view());
    }
    div(views).into_view()
}

/// Chooses an isolated showcase root and runs the terminal application.
///
/// # Returns
///
/// An empty [`leptatui::Result`] after a clean application exit.
///
/// # Errors
///
/// Returns [`leptatui::Error::Io`] if component initialization or the terminal
/// runtime fails.
#[tokio::main]
async fn main() -> leptatui::Result<()> {
    let root = std::env::temp_dir().join(format!(
        "leptatui-file-system-showcase-{}",
        std::process::id()
    ));
    let view = view! { <FileSystemShowcase root=root /> };
    App::new(view).run().await
}

#[cfg(test)]
/// Tests for guided filesystem walkthrough ordering and outcomes.
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::Duration,
    };

    use leptatui::prelude::{FileSystemOptions, use_file_system_with_options};
    use tokio::{task::yield_now, time::timeout};

    use super::*;

    /// Counter used to create distinct temporary test roots.
    static TEST_ROOT_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Temporary directory removed when one walkthrough test completes.
    struct TestRoot {
        /// Owned path to the temporary directory.
        path: PathBuf,
    }

    impl TestRoot {
        /// Creates a distinct temporary test directory.
        ///
        /// # Arguments
        ///
        /// * `label` — Readable label included in the directory name.
        ///
        /// # Returns
        ///
        /// A [`TestRoot`] owning the newly created directory.
        fn new(label: &str) -> Self {
            let sequence = TEST_ROOT_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "leptatui-showcase-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temporary walkthrough root should be created");
            Self { path }
        }

        /// Returns the temporary directory path.
        ///
        /// # Returns
        ///
        /// A [`Path`] borrowed from the fixture.
        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestRoot {
        /// Removes the temporary directory and its remaining contents.
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// Waits for one already-started step operation to complete.
    ///
    /// # Arguments
    ///
    /// * `operation` — Step operation whose first completion should be observed.
    ///
    /// # Returns
    ///
    /// A normalized [`OperationOutcome`] retained by the completed operation.
    async fn wait_for_outcome(operation: &StepOperation) -> OperationOutcome {
        timeout(Duration::from_secs(2), async {
            while operation.version() == 0 {
                yield_now().await;
            }
        })
        .await
        .expect("walkthrough operation should complete");
        operation
            .outcome()
            .expect("completed operation should retain an outcome")
    }

    /// Verifies the successful tour executes every dependent operation in order.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// create -> write -> append -> inspect -> replace -> copy -> rename -> delete
    /// ```
    ///
    /// # Assertions
    ///
    /// - Every successful-tour operation returns its expected outcome.
    /// - Copy completes before rename starts.
    /// - The final recursive deletion removes the walkthrough subtree.
    #[tokio::test(flavor = "current_thread")]
    async fn successful_tour_executes_without_dependency_races() {
        let root = TestRoot::new("success");
        let filesystem =
            use_file_system_with_options(root.path(), FileSystemOptions::new().create_root(true))
                .expect("walkthrough filesystem should initialize");

        for step in SUCCESS_STEPS {
            let outcome = wait_for_outcome(&step.start(&filesystem)).await;
            assert!(
                step.expected().matches(&outcome),
                "{} returned {outcome:?}",
                step.label()
            );
        }

        assert!(!root.path().join(WALKTHROUGH_DIR).exists());
    }

    /// Verifies the failure tour distinguishes expected errors from regressions.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// invalid UTF-8, missing path, traversal, home escape, copy conflict, root deletion
    /// ```
    ///
    /// # Assertions
    ///
    /// - Every fixture-creation step succeeds.
    /// - Every intentional failure returns its documented error kind.
    /// - Cleanup removes the failure-tour subtree.
    #[tokio::test(flavor = "current_thread")]
    async fn failure_tour_observes_documented_error_kinds() {
        let root = TestRoot::new("failure");
        let filesystem =
            use_file_system_with_options(root.path(), FileSystemOptions::new().create_root(true))
                .expect("walkthrough filesystem should initialize");

        for step in FAILURE_STEPS {
            let outcome = wait_for_outcome(&step.start(&filesystem)).await;
            assert!(
                step.expected().matches(&outcome),
                "{} returned {outcome:?}",
                step.label()
            );
        }

        assert!(!root.path().join(WALKTHROUGH_DIR).exists());
    }

    /// Verifies history navigation and reset state never rerun completed steps.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// finish step 1 -> Back -> Next -> reset
    /// ```
    ///
    /// # Assertions
    ///
    /// - Passing a frontier step unlocks and selects its successor.
    /// - Back selects completed history without changing the frontier.
    /// - Forward history navigation returns to the frontier without an attempt.
    /// - Fresh reset state restores only the first step to ready.
    #[test]
    fn history_navigation_and_reset_preserve_execution_order() {
        let mut state = WalkthroughState::new(TourKind::Success);
        assert_eq!(
            state.begin_frontier_attempt(),
            Some(WalkthroughStep::CreateDirectory)
        );
        assert!(state.finish_step(0, &OperationOutcome::Success(String::from("completed"))));
        assert_eq!(state.frontier, 1);
        assert_eq!(state.selected, 1);

        state.select_back();
        assert_eq!(state.selected, 0);
        assert_eq!(state.frontier, 1);
        assert!(state.select_forward_history());
        assert_eq!(state.selected, 1);
        assert_eq!(state.records[1].attempts, 0);

        let reset = WalkthroughState::new(TourKind::Success);
        assert_eq!(reset.frontier, 0);
        assert_eq!(reset.records[0].status, StepStatus::Ready);
        assert!(
            reset
                .records
                .iter()
                .skip(1)
                .all(|record| record.status == StepStatus::Locked)
        );
    }
}
