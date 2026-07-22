//! Link targets and linked rich-text storage.
//!
//! This module classifies URL, filesystem, and fragment targets and retains
//! focusable link ranges inside semantic rich text. Standalone and embedded
//! links share the same target resolution and system-opening behavior.

use std::{
    ffi::OsStr,
    fmt, io,
    path::{Path, PathBuf},
};

use ratatui::text::{Line, Span, Text};

use crate::app::{AppControl, Error, Result};

use super::metadata::{StyleMetadata, ViewType};

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
        } else if has_uri_scheme(&value) {
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

/// Rich text with optional focusable link ranges.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RichText {
    /// Ratatui text retained for measurement and rendering.
    text: Text<'static>,
    /// Link metadata and span locations embedded in the text.
    links: Vec<InlineLink>,
}

impl RichText {
    /// Returns the underlying Ratatui text.
    ///
    /// # Returns
    ///
    /// A [`Text`] reference containing the visible rich text.
    pub const fn text(&self) -> &Text<'static> {
        &self.text
    }

    /// Creates linked rich text from parsed text and inline links.
    ///
    /// # Arguments
    ///
    /// * `text` — Visible Ratatui text.
    /// * `links` — Inline link metadata and span positions.
    ///
    /// # Returns
    ///
    /// A [`RichText`] containing the supplied parsed content.
    pub(crate) fn from_parts(text: Text<'static>, links: Vec<InlineLink>) -> Self {
        Self { text, links }
    }

    /// Returns inline links in source order.
    ///
    /// # Returns
    ///
    /// A slice containing the retained inline links.
    pub(crate) fn links(&self) -> &[InlineLink] {
        &self.links
    }

    /// Returns mutable inline links in source order.
    ///
    /// # Returns
    ///
    /// A mutable slice containing the retained inline links.
    pub(crate) fn links_mut(&mut self) -> &mut [InlineLink] {
        &mut self.links
    }
}

impl From<Text<'static>> for RichText {
    /// Converts Ratatui text into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `text` — Owned Ratatui text to retain.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the visible text.
    fn from(text: Text<'static>) -> Self {
        Self {
            text,
            links: Vec::new(),
        }
    }
}

impl From<Line<'static>> for RichText {
    /// Converts one Ratatui line into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `line` — Owned Ratatui line to retain.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the line.
    fn from(line: Line<'static>) -> Self {
        Self::from(Text::from(line))
    }
}

impl From<Span<'static>> for RichText {
    /// Converts one Ratatui span into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `span` — Owned Ratatui span to retain.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the span.
    fn from(span: Span<'static>) -> Self {
        Self::from(Text::from(Line::from(span)))
    }
}

impl From<String> for RichText {
    /// Converts owned plain text into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `value` — Owned plain text to retain.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the plain text.
    fn from(value: String) -> Self {
        Self::from(Text::raw(value))
    }
}

impl From<&str> for RichText {
    /// Copies borrowed plain text into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `value` — Borrowed plain text to copy.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the copied text.
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

impl From<&String> for RichText {
    /// Copies a borrowed string into rich text without embedded links.
    ///
    /// # Arguments
    ///
    /// * `value` — Borrowed string to copy.
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the copied string.
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}

impl PartialEq<Text<'static>> for RichText {
    /// Compares linked rich text with its visible Ratatui text.
    ///
    /// # Arguments
    ///
    /// * `other` — Ratatui text to compare with the visible content.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether the visible text values are equal.
    fn eq(&self, other: &Text<'static>) -> bool {
        &self.text == other
    }
}

impl fmt::Display for RichText {
    /// Formats only the visible text, excluding link destinations and metadata.
    ///
    /// # Arguments
    ///
    /// * `formatter` — Destination formatter for the visible text.
    ///
    /// # Returns
    ///
    /// An empty [`fmt::Result`] after successful formatting.
    ///
    /// # Errors
    ///
    /// Returns [`fmt::Error`] if the destination formatter fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.text, formatter)
    }
}

/// Position of one linked span in retained Ratatui text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LinkedSpan {
    /// Zero-based logical line index.
    pub(crate) line: usize,
    /// Zero-based span index within the logical line.
    pub(crate) span: usize,
}

/// Focus and target metadata for one link embedded in rich text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InlineLink {
    /// Destination opened on activation.
    target: LinkTarget,
    /// Selector and focus metadata for the inline link.
    metadata: StyleMetadata,
    /// Text spans belonging to the link label.
    spans: Vec<LinkedSpan>,
}

impl InlineLink {
    /// Creates one inline link from parsed target and span locations.
    ///
    /// # Arguments
    ///
    /// * `target` — Destination opened on activation.
    /// * `spans` — Text span positions covered by the link label.
    ///
    /// # Returns
    ///
    /// An [`InlineLink`] with fresh `Link` selector metadata.
    pub(crate) fn new(target: LinkTarget, spans: Vec<LinkedSpan>) -> Self {
        Self {
            target,
            metadata: StyleMetadata::new(ViewType::Link),
            spans,
        }
    }

    /// Returns the link target.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget`] reference for this inline link.
    pub(crate) const fn target(&self) -> &LinkTarget {
        &self.target
    }

    /// Returns selector and focus metadata.
    ///
    /// # Returns
    ///
    /// A [`StyleMetadata`] reference for this inline link.
    pub(crate) const fn metadata(&self) -> &StyleMetadata {
        &self.metadata
    }

    /// Returns mutable selector and focus metadata.
    ///
    /// # Returns
    ///
    /// A mutable [`StyleMetadata`] reference for this inline link.
    pub(crate) fn metadata_mut(&mut self) -> &mut StyleMetadata {
        &mut self.metadata
    }

    /// Returns the retained text-span positions for this link.
    ///
    /// # Returns
    ///
    /// A slice containing the linked span positions.
    pub(crate) fn spans(&self) -> &[LinkedSpan] {
        &self.spans
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
    /// - Relative text becomes a filesystem path.
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
