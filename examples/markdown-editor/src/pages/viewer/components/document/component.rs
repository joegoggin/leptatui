//! Path-backed Markdown content and editor diagnostics.

use std::path::PathBuf;

use leptatui::prelude::*;

use super::style::use_viewer_document_styles;

/// Renders an open path through the existing file-backed Markdown view.
///
/// # Arguments
///
/// * `path` — Canonical or requested document path.
/// * `error` — Recoverable route or external-editor diagnostic for `path`.
///
/// # Returns
///
/// A path-backed Markdown document, editor error, or empty hint.
#[component]
pub(in crate::pages::viewer) fn ViewerDocument(
    path: Option<PathBuf>,
    error: Option<String>,
) -> impl IntoView {
    use_viewer_document_styles();

    let body = if let Some(error) = error {
        view! {
            <Text class="viewer-document__error">{format!("Error: {error}")}</Text>
        }
        .into_view()
    } else if let Some(path) = path {
        view! { <Markdown src=path line_numbers=false /> }.into_view()
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
