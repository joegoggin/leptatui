//! Action-loaded Markdown content and editor diagnostics.

use std::path::PathBuf;

use leptatui::prelude::*;

use super::style::use_viewer_document_styles;

/// Renders an open path through the existing file-backed Markdown view.
///
/// # Arguments
///
/// * `path` — Canonical or requested document path.
/// * `source` — Loaded Markdown source or a contextual read error.
/// * `loading` — Whether the current document read is pending.
/// * `editor_error` — Recoverable external-editor diagnostic for `path`.
///
/// # Returns
///
/// A path-backed Markdown document, editor error, or empty hint.
#[component]
pub(in crate::pages::viewer) fn ViewerDocument(
    path: Option<PathBuf>,
    source: Option<Result<String, String>>,
    loading: bool,
    editor_error: Option<String>,
) -> impl IntoView {
    use_viewer_document_styles();

    let body = if let Some(error) = editor_error {
        view! {
            <Text class="viewer-document__error">{format!("Error: {error}")}</Text>
        }
        .into_view()
    } else if loading {
        text("Loading Markdown file...")
            .with_classes("viewer-document__loading")
            .into_view()
    } else if let (Some(path), Some(source)) = (path, source) {
        match source {
            Ok(source) => markdown_source_with_options(
                path,
                source,
                MarkdownOptions::default().line_numbers(true),
            ),
            Err(error) => text(format!("Error: {error}"))
                .with_classes("viewer-document__error")
                .into_view(),
        }
    } else {
        view! {
            <Text class="viewer-document__empty">
                "Choose a Markdown file from Home or the file selector"
            </Text>
        }
        .into_view()
    };

    view! { <Block class="viewer-document">{body}</Block> }
}
