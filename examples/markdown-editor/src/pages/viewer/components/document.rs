//! Action-loaded Markdown content and editor diagnostics.

use std::path::PathBuf;

use leptatui::prelude::*;

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
pub(in crate::pages::viewer) fn viewer_document(
    path: Option<PathBuf>,
    source: Option<Result<String, String>>,
    loading: bool,
    editor_error: Option<String>,
) -> AnyView {
    let body = if let Some(error) = editor_error {
        view! {
            <Text class="error">{format!("Error: {error}")}</Text>
        }
        .into_view()
    } else if loading {
        text("Loading Markdown file...")
            .with_classes("info")
            .into_view()
    } else if let (Some(path), Some(source)) = (path, source) {
        match source {
            Ok(source) => markdown_source_with_options(
                path,
                source,
                MarkdownOptions::default().line_numbers(true),
            ),
            Err(error) => text(format!("Error: {error}"))
                .with_classes("error")
                .into_view(),
        }
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

    view! { <Block style=content_style>{body}</Block> }.into_view()
}
