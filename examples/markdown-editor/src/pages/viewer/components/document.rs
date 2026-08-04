//! Path-backed Markdown content and editor diagnostics.

use std::path::PathBuf;

use leptatui::prelude::*;

/// Renders an open path through the existing file-backed Markdown view.
///
/// # Arguments
///
/// * `path` — Canonical or requested document path.
/// * `editor_error` — Recoverable external-editor diagnostic for `path`.
///
/// # Returns
///
/// A path-backed Markdown document, editor error, or empty hint.
#[component]
pub(in crate::pages::viewer) fn ViewerDocument(
    path: Option<PathBuf>,
    editor_error: Option<String>,
) -> impl IntoView {
    let body = if let Some(error) = editor_error {
        view! {
            <Text class="error">{format!("Error: {error}")}</Text>
        }
        .into_view()
    } else if let Some(path) = path {
        view! {
            <Markdown src=path line_numbers=true />
        }
        .into_view()
    } else {
        view! {
            <Text class="empty">"Choose a Markdown file from Home or Explorer"</Text>
        }
        .into_view()
    };
    let content_style = TuiStyle::new()
        .flex_basis(Dimension::from(Length::cells(0.0)))
        .flex_grow(1.0)
        .borders(Borders::ALL)
        .padding(TuiSpacing::horizontal(1))
        .overflow(Axes::new(Overflow::Hidden, Overflow::Auto));

    view! { <Block style=content_style>{body}</Block> }
}
