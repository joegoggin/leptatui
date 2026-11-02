//! Path-backed editable Markdown content and fallible route diagnostics.

use std::path::PathBuf;

use leptatui::prelude::*;

use super::style::use_viewer_document_styles;

/// Renders an open path through the existing file-backed Markdown view.
///
/// # Arguments
///
/// * `path` — Resolved document path or route validation failure.
///
/// # Returns
///
/// A path-backed Markdown document or empty hint.
///
/// # Errors
///
/// Returns [`ViewError`] if route validation fails.
#[component]
pub(in crate::pages::viewer) fn ViewerDocument(
    path: Result<Option<PathBuf>, String>,
) -> ViewResult<impl IntoView> {
    use_viewer_document_styles();

    let path = path.map_err(ViewError::msg)?;

    let body = if let Some(path) = path {
        view! { <Markdown src=path editable=true line_numbers=false /> }.into_view()
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
