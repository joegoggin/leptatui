//! Link target classification, resolution, and activation.

use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
};

use crate::app::{AppControl, Error, Result};

/// Destination retained by a standalone or embedded link.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkTarget {
    /// Absolute URI passed to the operating system's configured handler.
    Url(String),
    /// Absolute or relative filesystem path passed to its configured application.
    Path(PathBuf),
    /// Markdown file eligible for in-app file-backed navigation.
    Markdown {
        /// Absolute or relative Markdown file path.
        path: PathBuf,
        /// Optional heading fragment to reveal after loading.
        fragment: Option<String>,
    },
    /// Empty or in-document fragment target retained without activation.
    Fragment(String),
}

impl LinkTarget {
    /// Returns whether this target can be activated.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether this is an external or in-app target.
    pub const fn is_actionable(&self) -> bool {
        matches!(self, Self::Url(_) | Self::Path(_) | Self::Markdown { .. })
    }

    /// Resolves a relative filesystem target against a base directory.
    ///
    /// URL, absolute path, and fragment targets remain unchanged. Relative
    /// Markdown paths retain their optional fragment while being resolved.
    ///
    /// # Arguments
    ///
    /// * `base` — Directory used to resolve relative filesystem paths.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget`] containing an absolute or base-relative path.
    pub fn resolve_against(self, base: impl AsRef<Path>) -> Self {
        match self {
            Self::Path(path) if path.is_relative() => Self::Path(base.as_ref().join(path)),
            Self::Markdown { path, fragment } if path.is_relative() => Self::Markdown {
                path: base.as_ref().join(path),
                fragment,
            },
            target => target,
        }
    }

    /// Returns the user-facing destination text.
    ///
    /// # Returns
    ///
    /// A [`String`] containing the URI, path, or fragment.
    pub fn display(&self) -> String {
        match self {
            Self::Url(url) | Self::Fragment(url) => url.clone(),
            Self::Path(path) => path.display().to_string(),
            Self::Markdown { path, fragment } => fragment.as_ref().map_or_else(
                || path.display().to_string(),
                |fragment| format!("{}#{fragment}", path.display()),
            ),
        }
    }

    /// Returns the operating-system argument for an actionable target.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the URL or path as an [`OsStr`].
    fn as_os_str(&self) -> Option<&OsStr> {
        match self {
            Self::Url(url) => Some(OsStr::new(url)),
            Self::Path(path) | Self::Markdown { path, .. } => Some(path.as_os_str()),
            Self::Fragment(_) => None,
        }
    }
}

impl From<String> for LinkTarget {
    /// Classifies owned destination text as a URL, path, or fragment.
    ///
    /// # Arguments
    ///
    /// * `value` — Destination text to classify.
    ///
    /// # Returns
    ///
    /// A classified [`LinkTarget`].
    fn from(value: String) -> Self {
        if value.is_empty() || value.starts_with('#') {
            Self::Fragment(value)
        } else if !has_windows_drive_prefix(&value) && has_uri_scheme(&value) {
            Self::Url(value)
        } else {
            Self::Path(PathBuf::from(value))
        }
    }
}

impl From<&str> for LinkTarget {
    /// Classifies borrowed destination text as a URL, path, or fragment.
    ///
    /// # Arguments
    ///
    /// * `value` — Destination text to classify and copy.
    ///
    /// # Returns
    ///
    /// A classified [`LinkTarget`].
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

impl From<PathBuf> for LinkTarget {
    /// Converts an explicit path buffer into a filesystem link target.
    ///
    /// # Arguments
    ///
    /// * `value` — Filesystem path to open.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget::Path`] containing `value`.
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<&Path> for LinkTarget {
    /// Converts an explicit borrowed path into a filesystem link target.
    ///
    /// # Arguments
    ///
    /// * `value` — Filesystem path to copy.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget::Path`] containing the copied path.
    fn from(value: &Path) -> Self {
        Self::Path(value.to_path_buf())
    }
}

impl From<&PathBuf> for LinkTarget {
    /// Converts an explicit borrowed path buffer into a filesystem link target.
    ///
    /// # Arguments
    ///
    /// * `value` — Filesystem path to copy.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget::Path`] containing the copied path.
    fn from(value: &PathBuf) -> Self {
        Self::Path(value.clone())
    }
}

/// Returns whether destination text begins with an RFC-style URI scheme.
///
/// # Arguments
///
/// * `value` — Destination text to inspect.
///
/// # Returns
///
/// A [`bool`] indicating whether a valid scheme precedes the first colon.
fn has_uri_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

/// Returns whether destination text begins with a Windows drive prefix.
///
/// # Arguments
///
/// * `value` — Destination text to inspect.
///
/// # Returns
///
/// A [`bool`] indicating whether the text begins with a drive letter, colon,
/// and path separator.
fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

/// Opens one actionable target with the operating system's default handler.
///
/// # Arguments
///
/// * `target` — URL or filesystem path to open.
///
/// # Returns
///
/// An [`AppControl::Continue`] value after the handler starts successfully.
///
/// # Errors
///
/// Returns [`Error::LinkOpen`] if a local target is missing or the system
/// handler cannot be started.
pub(crate) fn open_link_target(target: &LinkTarget) -> Result<AppControl> {
    open_link_target_with(target, |argument| open::that(argument))
}

/// Opens one link through an injected launcher.
///
/// # Arguments
///
/// * `target` — URL or filesystem path to validate and open.
/// * `opener` — Launcher receiving the target as an operating-system string.
///
/// # Returns
///
/// An [`AppControl::Continue`] value after the launcher succeeds.
///
/// # Errors
///
/// Returns [`Error::LinkOpen`] if the path is missing, the target is inactive,
/// or `opener` returns an I/O error.
fn open_link_target_with(
    target: &LinkTarget,
    opener: impl FnOnce(&OsStr) -> io::Result<()>,
) -> Result<AppControl> {
    let display = target.display();
    if let LinkTarget::Path(path) = target
        && !path.exists()
    {
        return Err(Error::LinkOpen {
            target: display,
            source: io::Error::new(io::ErrorKind::NotFound, "link target does not exist"),
        });
    }
    let argument = target.as_os_str().ok_or_else(|| Error::LinkOpen {
        target: display.clone(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "link target is not actionable"),
    })?;
    opener(argument).map_err(|source| Error::LinkOpen {
        target: display,
        source,
    })?;
    Ok(AppControl::Continue)
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, io, path::PathBuf};

    use super::{LinkTarget, open_link_target_with};

    /// Verifies string targets distinguish fragments, paths, and absolute URIs.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// #section
    /// guide.md
    /// https://example.com
    /// mailto:team@example.com
    /// ```
    ///
    /// # Assertions
    ///
    /// - Empty and hash-prefixed targets become inactive fragments.
    /// - Relative and Windows drive-prefixed text become filesystem paths.
    /// - HTTP and mail targets become URLs.
    #[test]
    fn string_targets_are_classified() {
        assert_eq!(LinkTarget::from(""), LinkTarget::Fragment(String::new()));
        assert_eq!(
            LinkTarget::from("#section"),
            LinkTarget::Fragment("#section".to_owned())
        );
        assert_eq!(
            LinkTarget::from("guide.md"),
            LinkTarget::Path(PathBuf::from("guide.md"))
        );
        assert_eq!(
            LinkTarget::from("C:/guide.md"),
            LinkTarget::Path(PathBuf::from("C:/guide.md"))
        );
        assert_eq!(
            LinkTarget::from(r"C:\guide.md"),
            LinkTarget::Path(PathBuf::from(r"C:\guide.md"))
        );
        assert_eq!(
            LinkTarget::from("https://example.com"),
            LinkTarget::Url("https://example.com".to_owned())
        );
        assert_eq!(
            LinkTarget::from("mailto:team@example.com"),
            LinkTarget::Url("mailto:team@example.com".to_owned())
        );
    }

    /// Verifies launcher success and failure remain deterministic in tests.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// https://example.com
    /// ```
    ///
    /// # Assertions
    ///
    /// - A successful injected launcher receives the URL and continues.
    /// - An injected I/O failure becomes a target-aware link-open error.
    #[test]
    fn link_opening_uses_injected_launcher() {
        let called = Cell::new(false);
        let target = LinkTarget::from("https://example.com");
        let result = open_link_target_with(&target, |argument| {
            assert_eq!(argument, "https://example.com");
            called.set(true);
            Ok(())
        });
        assert_eq!(result.unwrap(), crate::AppControl::Continue);
        assert!(called.get());

        let error = open_link_target_with(&target, |_| Err(io::Error::other("launcher failed")))
            .unwrap_err();
        assert!(error.to_string().contains("https://example.com"));
    }
}
