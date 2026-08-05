//! Presentation formatting shared by routed pages.

use std::path::Path;

/// Formats a workspace path relative to its root.
///
/// # Arguments
///
/// * `root` — Canonical workspace root.
/// * `path` — Canonical directory or Markdown path to display.
///
/// # Returns
///
/// A [`String`] containing `.`, a relative workspace path, or the original
/// absolute path when it is not below `root`.
pub(super) fn relative_path(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) if relative.as_os_str().is_empty() => String::from("."),
        Ok(relative) => relative.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}
