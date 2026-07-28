//! Path-backed Markdown content and editor diagnostics.

use leptatui::prelude::*;

use crate::core::PreviewState;

/// Renders an open path through the existing file-backed Markdown view.
///
/// # Arguments
///
/// * `preview` — Controller-owned document snapshot.
///
/// # Returns
///
/// A path-backed Markdown document, editor error, or empty hint.
#[component]
pub(in crate::pages::viewer) fn ViewerDocument(preview: PreviewState) -> impl IntoView {
    let body = if let Some(error) = preview.editor_error() {
        view! {
            <Text class="error">{format!("Error: {error}")}</Text>
        }
        .into_view()
    } else if let Some(path) = preview.path() {
        view! {
            <Markdown src=path syntax_theme=SyntaxTheme::Dark line_numbers=true />
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
