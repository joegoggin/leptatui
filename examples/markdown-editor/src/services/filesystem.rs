//! Markdown-specific policy layered over Leptatui's scoped filesystem.
//!
//! Leptatui owns file selection and path containment. This module retains only
//! Markdown path validation and volume-boundary discovery used by the viewer.

use std::{ffi::OsStr, path::Path};

/// Returns whether a path has a supported Markdown extension.
///
/// # Arguments
///
/// * `path` — Path whose final extension should be checked.
///
/// # Returns
///
/// A boolean indicating whether the extension is `md` or `markdown`.
pub(crate) fn is_markdown_path(path: &Path) -> bool {
    path.file_name().is_some_and(is_markdown_name)
}

/// Returns whether a directory-entry name has a Markdown extension.
///
/// # Arguments
///
/// * `name` — Original filesystem entry name.
///
/// # Returns
///
/// A boolean indicating whether the extension is `md` or `markdown`.
fn is_markdown_name(name: &OsStr) -> bool {
    Path::new(name)
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}
