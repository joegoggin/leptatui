//! CommonMark parsing and semantic document-view conversion.
//!
//! This module converts pulldown-cmark's balanced event stream into the
//! semantic headings, paragraphs, lists, tables, highlighted code blocks, and
//! styled inline spans exposed by [`mod@crate::view`]. Readable styled-block or
//! text fallbacks retain CommonMark content without dedicated semantic views.
//! In-memory and explicit file readers are infallible; file failures become
//! path-aware semantic fallback content. File-backed views navigate local
//! Markdown targets and heading fragments in-app with cached page history.
//! The compatibility promise is core CommonMark plus tables. Optional GFM
//! extensions are deferred. Links retain focusable target metadata, while
//! images become descriptive text without fetching local or remote targets.
//!
//! # Modules
//!
//! - [`block`] — Block event parsing and semantic view construction.
//! - [`code`] — Fenced and indented code-block conversion.
//! - [`inline`] — Inline text and style span collection.
//! - [`inline_events`] — Inline event rendering and fallback handling.
//! - [`list`] — Ordered and unordered list parsing.
//! - [`navigation`] — File-backed navigation, targets, and page history.
//! - [`table`] — Table section, row, cell, and alignment parsing.

mod block;
mod code;
mod inline;
mod inline_events;
mod list;
mod navigation;
mod table;

use std::path::{Path, PathBuf};

use pulldown_cmark::{Options, Parser};

use crate::{AnyView, IntoView, SyntaxTheme, column};

use self::block::parse_blocks;
use self::navigation::MarkdownParseContext;
pub use self::navigation::MarkdownView;

/// Default presentation options applied while converting Markdown documents.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MarkdownOptions {
    /// Bundled syntax theme applied to fenced and indented code blocks.
    pub(super) syntax_theme: SyntaxTheme,
    /// Whether parsed code blocks display one-based line numbers.
    pub(super) line_numbers: bool,
}

impl MarkdownOptions {
    /// Sets the bundled syntax theme for parsed code blocks.
    ///
    /// # Arguments
    ///
    /// * `syntax_theme` — Dark or light bundled theme selection.
    ///
    /// # Returns
    ///
    /// A [`MarkdownOptions`] value with the requested syntax theme.
    pub fn syntax_theme(mut self, syntax_theme: SyntaxTheme) -> Self {
        self.syntax_theme = syntax_theme;
        self
    }

    /// Sets default line-number visibility for parsed code blocks.
    ///
    /// # Arguments
    ///
    /// * `line_numbers` — Whether to display one-based logical line numbers.
    ///
    /// # Returns
    ///
    /// A [`MarkdownOptions`] value with the requested line-number behavior.
    pub fn line_numbers(mut self, line_numbers: bool) -> Self {
        self.line_numbers = line_numbers;
        self
    }
}

/// Converts CommonMark source into a scrollable semantic document view.
///
/// Uses [`MarkdownOptions::default`] and performs no filesystem access.
///
/// # Examples
///
/// ```
/// use leptatui::markdown;
///
/// let document = markdown("# Guide\n\nRead **semantic** terminal documents.");
/// # let _ = document;
/// ```
///
/// # Arguments
///
/// * `source` — CommonMark source text to parse.
///
/// # Returns
///
/// An [`AnyView`] containing a vertical [`LayoutView`](crate::LayoutView) of
/// semantic document blocks separated by empty terminal rows in source order.
pub fn markdown(source: impl AsRef<str>) -> AnyView {
    markdown_with_options(source, MarkdownOptions::default())
}

/// Converts CommonMark source with explicit presentation options.
///
/// Parsing is infallible and performs no filesystem access.
///
/// # Examples
///
/// ```
/// use leptatui::{MarkdownOptions, SyntaxTheme, markdown_with_options};
///
/// let source = "```rust\nfn main() {}\n```";
/// let document = markdown_with_options(
///     source,
///     MarkdownOptions::default()
///         .syntax_theme(SyntaxTheme::Dark)
///         .line_numbers(true),
/// );
/// # let _ = document;
/// ```
///
/// # Arguments
///
/// * `source` — CommonMark source text to parse.
/// * `options` — Code-block presentation defaults for the document.
///
/// # Returns
///
/// An [`AnyView`] containing a vertical [`LayoutView`](crate::LayoutView) of
/// semantic document blocks separated by empty terminal rows in source order.
pub fn markdown_with_options(source: impl AsRef<str>, options: MarkdownOptions) -> AnyView {
    let link_base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    markdown_with_options_and_source(source.as_ref(), options, &link_base, None)
}

/// Converts CommonMark with an explicit link base and optional source path.
fn markdown_with_options_and_source(
    source: &str,
    options: MarkdownOptions,
    link_base: &Path,
    source_path: Option<&Path>,
) -> AnyView {
    let mut parser = Parser::new_ext(source, Options::ENABLE_TABLES);
    let mut context = MarkdownParseContext::new(link_base, source_path);
    column(parse_blocks(&mut parser, None, options, &mut context)).into_view()
}

/// Loads a UTF-8 Markdown file into a scrollable semantic document view.
///
/// Uses [`MarkdownOptions::default`] and performs all filesystem access before
/// returning the view.
///
/// # Examples
///
/// ```
/// use leptatui::markdown_file;
///
/// let document = markdown_file("README.md");
/// # let _ = document;
/// ```
///
/// # Arguments
///
/// * `path` — Path to the UTF-8 Markdown file to load.
///
/// # Returns
///
/// An [`AnyView`] containing a vertical [`LayoutView`](crate::LayoutView) of
/// the parsed document or a path-aware fallback paragraph when the file cannot
/// be read as UTF-8.
pub fn markdown_file(path: impl AsRef<Path>) -> AnyView {
    markdown_file_with_options(path, MarkdownOptions::default())
}

/// Loads a UTF-8 Markdown file with explicit presentation options.
///
/// All filesystem access completes before the returned view enters render
/// traversal.
///
/// # Examples
///
/// ```no_run
/// use leptatui::{MarkdownOptions, SyntaxTheme, markdown_file_with_options};
///
/// let view = markdown_file_with_options(
///     "README.md",
///     MarkdownOptions::default()
///         .syntax_theme(SyntaxTheme::Light)
///         .line_numbers(true),
/// );
/// # let _ = view;
/// ```
///
/// # Arguments
///
/// * `path` — Path to the UTF-8 Markdown file to load.
/// * `options` — Code-block presentation defaults for the document.
///
/// # Returns
///
/// An [`AnyView`] containing a vertical [`LayoutView`](crate::LayoutView) of
/// the parsed document or a path-aware fallback paragraph when the file cannot
/// be read as UTF-8.
pub fn markdown_file_with_options(path: impl AsRef<Path>, options: MarkdownOptions) -> AnyView {
    MarkdownView::new(path.as_ref(), options).into_view()
}

#[cfg(test)]
mod tests;
