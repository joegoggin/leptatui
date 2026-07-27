//! External editor process boundary.
//!
//! This service is wired into the controller now so Neovim integration can be
//! added without moving process management into filesystem or UI code.

/// Process service reserved for launching the configured Markdown editor.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EditorProcess;

impl EditorProcess {
    /// Creates the external editor process service.
    ///
    /// # Returns
    ///
    /// A stateless [`EditorProcess`] service.
    pub(crate) const fn new() -> Self {
        Self
    }
}
