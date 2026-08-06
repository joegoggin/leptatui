//! CommonMark parsing and semantic document-view conversion.
//!
//! This module converts pulldown-cmark's balanced event stream into the
//! semantic headings, paragraphs, lists, tables, highlighted code blocks, and
//! styled inline spans exposed by [`mod@crate::view`]. Readable styled-block or
//! text fallbacks retain CommonMark content without dedicated semantic views.
//! In-memory and explicit file readers are infallible; file failures become
//! path-aware semantic fallback content. File-backed views navigate local
//! Markdown targets and heading fragments in-app with cached page history.
//! Declarative `Markdown` tags may opt into external editing and source reload
//! shortcuts with `editable=true`.
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

use std::path::{Component, Path, PathBuf};

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use leptos::prelude::{Effect, Get, GetUntracked, RwSignal, Set, WithUntracked};
use pulldown_cmark::{Options, Parser};

use crate::{
    AnyView, EditorStatus, IntoView, KeyControl, ViewError, div, dynamic,
    file_system::use_file_system, keyed, text, use_editor, view::error::__view_error,
};

use self::block::parse_blocks;
use self::navigation::MarkdownParseContext;
pub use self::navigation::MarkdownView;

/// Default presentation options applied while converting Markdown documents.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MarkdownOptions {
    /// Whether parsed code blocks display one-based line numbers.
    pub(super) line_numbers: bool,
}

impl MarkdownOptions {
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
/// An [`AnyView`] containing a vertical [`DivView`](crate::DivView) of
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
/// use leptatui::{MarkdownOptions, markdown_with_options};
///
/// let source = "```rust\nfn main() {}\n```";
/// let document = markdown_with_options(
///     source,
///     MarkdownOptions::default().line_numbers(true),
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
/// An [`AnyView`] containing a vertical [`DivView`](crate::DivView) of
/// semantic document blocks separated by empty terminal rows in source order.
pub fn markdown_with_options(source: impl AsRef<str>, options: MarkdownOptions) -> AnyView {
    let link_base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    markdown_with_options_and_source(source.as_ref(), options, &link_base, None)
}

/// Converts CommonMark with an explicit link base and optional source path.
///
/// # Arguments
///
/// * `source` — CommonMark source text to parse.
/// * `options` — Code-block presentation defaults for the document.
/// * `link_base` — Directory used to resolve relative local links.
/// * `source_path` — Current Markdown file path used for in-app navigation.
///
/// # Returns
///
/// An [`AnyView`] containing semantic document blocks with resolved links and
/// file-navigation anchors when a source path is present.
fn markdown_with_options_and_source(
    source: &str,
    options: MarkdownOptions,
    link_base: &Path,
    source_path: Option<&Path>,
) -> AnyView {
    let mut parser = Parser::new_ext(source, Options::ENABLE_TABLES);
    let mut context = MarkdownParseContext::new(link_base, source_path);
    div(parse_blocks(&mut parser, None, options, &mut context)).into_view()
}

/// Loads a UTF-8 Markdown file into a navigable semantic document view.
///
/// Uses [`MarkdownOptions::default`] and reads the initial file before
/// returning. Activating an in-app Markdown link reads its target during event
/// handling.
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
/// An [`AnyView`] containing a [`MarkdownView`] with the parsed document or a
/// path-aware fallback page when the file cannot be read as UTF-8.
pub fn markdown_file(path: impl AsRef<Path>) -> AnyView {
    markdown_file_with_options(path, MarkdownOptions::default())
}

/// Loads a UTF-8 Markdown file with explicit presentation options.
///
/// The initial file is read before this function returns. Activating an in-app
/// Markdown link reads its target during event handling.
///
/// # Examples
///
/// ```no_run
/// use leptatui::{MarkdownOptions, markdown_file_with_options};
///
/// let view = markdown_file_with_options(
///     "README.md",
///     MarkdownOptions::default().line_numbers(true),
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
/// An [`AnyView`] containing a [`MarkdownView`] with the parsed document or a
/// path-aware fallback page when the file cannot be read as UTF-8.
pub fn markdown_file_with_options(path: impl AsRef<Path>, options: MarkdownOptions) -> AnyView {
    MarkdownView::new(path.as_ref(), options).into_view()
}

/// Builds the asynchronously loaded view used by the declarative `Markdown` tag.
///
/// This generated-code entry point resolves the source against the process
/// working directory, scopes filesystem access to its volume, and converts
/// initialization, read, or editor failures into Leptatui's standard
/// view-error screen. Editable elements open their original source with `e`
/// and refetch it with `r` or after a successful editor session.
///
/// # Arguments
///
/// * `path` — Markdown file path supplied through `src`.
/// * `options` — Presentation options applied to each loaded document.
/// * `editable` — Whether unmodified `e` and `r` edit and reload `path`.
/// * `source_file` — Rust source file containing the declarative element.
/// * `source_line` — Rust source line containing the declarative element.
///
/// # Returns
///
/// An [`AnyView`] containing the loaded Markdown document or standard error
/// screen.
#[doc(hidden)]
pub fn __markdown_element(
    path: impl AsRef<Path>,
    options: MarkdownOptions,
    editable: bool,
    source_file: &'static str,
    source_line: u32,
) -> AnyView {
    let path = match absolute_markdown_path(path.as_ref()) {
        Ok(path) => path,
        Err(error) => {
            return markdown_element_error(error.to_string(), source_file, source_line);
        }
    };
    let filesystem = match use_file_system(volume_root(&path)) {
        Ok(filesystem) => filesystem,
        Err(error) => {
            return markdown_element_error(error.to_string(), source_file, source_line);
        }
    };
    let operation = filesystem.read_file_as_string(path.clone());
    let pending = operation.pending();
    let value = operation.value();
    let version = operation.version();
    let editor_error = RwSignal::new(None::<String>);
    let keyed_pending = pending.clone();
    let keyed_error = editor_error;
    let child_pending = pending;
    let child_error = editor_error;
    let child_path = path.clone();

    let view = keyed(
        move || (version.get(), keyed_pending.get(), keyed_error.get()),
        move || {
            if let Some(error) = child_error.get_untracked() {
                return __view_error(ViewError::msg(error), source_file, source_line);
            }
            if child_pending.get_untracked() {
                return text("Loading Markdown file...").into_view();
            }

            value.with_untracked(|result| match result {
                Some(Ok(source)) => markdown_source_with_options(&child_path, source, options),
                Some(Err(error)) => {
                    __view_error(ViewError::msg(error.to_string()), source_file, source_line)
                }
                None => text("Loading Markdown file...").into_view(),
            })
        },
    );

    if !editable {
        return view.into_view();
    }

    let editor = use_editor();
    let status_editor = editor.clone();
    let clear_editor = editor.clone();
    let completed_operation = operation.clone();
    let completed_error = editor_error;
    Effect::watch_sync(
        move || status_editor.status(),
        move |status, _, _| match status {
            Some(EditorStatus::Complete) => {
                completed_error.set(None);
                completed_operation.dispatch(());
                clear_editor.clear();
            }
            Some(EditorStatus::Error(error)) => {
                completed_error.set(Some(error.clone()));
                clear_editor.clear();
            }
            Some(EditorStatus::Pending) | None => {}
        },
        true,
    );

    let edit_editor = editor;
    let edit_path = path.clone();
    let edit_error = editor_error;
    let reload_operation = operation;
    view.on_key_event(move |key| {
        if key.kind != KeyEventKind::Press || key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Char('e') => {
                edit_error.set(None);
                edit_editor.edit_file(edit_path.clone());
                KeyControl::Handled
            }
            KeyCode::Char('r') => {
                edit_error.set(None);
                reload_operation.dispatch(());
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    })
    .into_view()
}

/// Creates a reactive standard error screen for Markdown setup failures.
///
/// # Arguments
///
/// * `message` — Diagnostic displayed by the standard error screen.
/// * `source_file` — Rust source file containing the declarative element.
/// * `source_line` — Rust source line containing the declarative element.
///
/// # Returns
///
/// An [`AnyView`] containing the standard error screen.
fn markdown_element_error(message: String, source_file: &'static str, source_line: u32) -> AnyView {
    dynamic(move || __view_error(ViewError::msg(message.clone()), source_file, source_line))
        .into_view()
}

/// Resolves a Markdown source path without requiring the source to exist.
fn absolute_markdown_path(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();

    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if normalized.file_name().is_some() {
                    normalized.pop();
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }

    Ok(normalized)
}

/// Returns the filesystem or drive root containing a Markdown source path.
fn volume_root(path: &Path) -> PathBuf {
    path.ancestors().last().unwrap_or(path).to_path_buf()
}

/// Creates a navigable Markdown view from already-loaded UTF-8 source.
///
/// The supplied path establishes relative-link resolution and navigation
/// identity without reading the initial page from the filesystem. Activating a
/// later local Markdown link continues to use the file-backed navigation
/// behavior of [`MarkdownView`].
///
/// # Arguments
///
/// * `path` — Path represented by the loaded source.
/// * `source` — UTF-8 CommonMark source that has already been read.
/// * `options` — Code-block presentation defaults for the document.
///
/// # Returns
///
/// An [`AnyView`] containing a navigable [`MarkdownView`].
pub fn markdown_source_with_options(
    path: impl AsRef<Path>,
    source: impl AsRef<str>,
    options: MarkdownOptions,
) -> AnyView {
    MarkdownView::from_source(path.as_ref(), source.as_ref(), options).into_view()
}

#[cfg(test)]
mod tests;
