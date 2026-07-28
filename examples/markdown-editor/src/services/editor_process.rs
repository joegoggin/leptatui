//! External editor process boundary.
//!
//! The service resolves the user's terminal editor, parses configured arguments
//! without invoking a shell, and delegates execution through injectable
//! environment and process boundaries.

use std::{env, ffi::OsString, fmt::Debug, io, path::Path, process::Command, rc::Rc};

/// Environment variable consulted first for an interactive visual editor.
const VISUAL_ENVIRONMENT_VARIABLE: &str = "VISUAL";
/// Environment variable consulted second for a terminal editor.
const EDITOR_ENVIRONMENT_VARIABLE: &str = "EDITOR";
/// Editor used when neither configured environment variable has a value.
const FALLBACK_EDITOR: &str = "vi";

/// Executes a prepared external-editor command and reports its success state.
pub(crate) trait ProcessLauncher: Debug {
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
pub(crate) trait EnvironmentReader: Debug {
    /// Returns one operating-system environment value.
    ///
    /// # Arguments
    ///
    /// * `name` — Environment variable name to read.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the configured operating-system string.
    fn var_os(&self, name: &str) -> Option<OsString>;
}

/// Operating-system environment reader used outside tests.
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
    /// An [`Option`] containing the configured operating-system string.
    fn var_os(&self, name: &str) -> Option<OsString> {
        env::var_os(name)
    }
}

/// Operating-system process launcher used outside tests.
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

/// Process service that launches the configured terminal editor.
#[derive(Clone, Debug)]
pub(crate) struct EditorProcess {
    /// Injectable boundary used to execute prepared commands.
    launcher: Rc<dyn ProcessLauncher>,
    /// Injectable boundary used to resolve editor environment variables.
    environment: Rc<dyn EnvironmentReader>,
}

impl EditorProcess {
    /// Creates the external editor process service.
    ///
    /// # Returns
    ///
    /// An [`EditorProcess`] backed by the operating-system launcher.
    pub(crate) fn new() -> Self {
        Self {
            launcher: Rc::new(SystemProcessLauncher),
            environment: Rc::new(SystemEnvironmentReader),
        }
    }

    /// Creates an editor process around injected service boundaries.
    ///
    /// # Arguments
    ///
    /// * `launcher` — Test or production process execution boundary.
    /// * `environment` — Test or production environment reader.
    ///
    /// # Returns
    ///
    /// An [`EditorProcess`] that delegates command resolution and execution.
    #[cfg(test)]
    pub(crate) fn with_services(
        launcher: impl ProcessLauncher + 'static,
        environment: impl EnvironmentReader + 'static,
    ) -> Self {
        Self {
            launcher: Rc::new(launcher),
            environment: Rc::new(environment),
        }
    }

    /// Opens one Markdown document in the configured editor and waits for it.
    ///
    /// `VISUAL` takes precedence over `EDITOR`; empty values are skipped and
    /// `vi` is the fallback. Configured arguments use shell-word quoting but
    /// are executed directly without shell expansion. One `--` separator is
    /// placed before the path.
    ///
    /// # Arguments
    ///
    /// * `path` — Canonical absolute Markdown path to edit.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] after the editor exits successfully.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if editor configuration is invalid, the editor
    /// cannot be launched or waited on, or it exits with a non-zero status.
    pub(crate) fn edit(&self, path: &Path) -> io::Result<()> {
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

/// Resolves the configured editor executable and arguments.
///
/// `VISUAL` takes precedence over `EDITOR`. Empty or whitespace-only values are
/// skipped, and `vi` is returned when neither variable selects an editor.
///
/// # Arguments
///
/// * `environment` — Environment boundary supplying editor configuration.
///
/// # Returns
///
/// A non-empty [`Vec`] containing the editor executable followed by arguments.
///
/// # Errors
///
/// Returns [`io::Error`] if the selected value is not Unicode, contains
/// malformed shell-word quoting, or does not name an executable.
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

impl Default for EditorProcess {
    /// Creates the default operating-system editor process.
    ///
    /// # Returns
    ///
    /// An [`EditorProcess`] backed by the operating-system launcher.
    fn default() -> Self {
        Self::new()
    }
}
