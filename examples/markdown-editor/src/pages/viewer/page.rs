//! Viewer route-level component and route synchronization.

use std::path::{Path, PathBuf};

use leptatui::prelude::*;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

use crate::services::is_markdown_path;

use super::{
    components::{ViewerDocument, ViewerDocumentProps},
    style::use_viewer_page_styles,
};

/// Characters encoded inside one viewer route path segment.
const ROUTE_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

/// Renders the standalone Markdown viewer and document actions.
///
/// The route identifies the open document. The declarative Markdown element
/// owns external editing and explicit reloads.
///
/// # Returns
///
/// A Viewer page component or a current-directory resolution error.
#[component]
pub(crate) fn ViewerPage() -> ViewResult<impl IntoView> {
    let route_params = use_params_map();
    let current_directory = std::env::current_dir()?;
    let document_directory = current_directory.clone();
    let document_path = Memo::new(move |_| {
        let route_path = route_params.get().get("path").map(str::to_owned);
        resolve_viewer_path(route_path.as_deref(), &document_directory)
    });
    let shortcut_navigate = use_navigate();
    let home_navigate = use_navigate();
    let browse_navigate = use_navigate();
    let open_path = route_params
        .get_untracked()
        .get("path")
        .map_or_else(|| String::from("none"), str::to_owned);
    let document = keyed(
        move || document_path.get(),
        move || {
            let path = document_path.get_untracked();
            view! { <ViewerDocument path=path /> }.into_view()
        },
    );

    use_viewer_page_styles();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Char('h') | KeyCode::Char('b') => {
                shortcut_navigate("/", NavigateOptions::default());
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    });

    view! {
        <Div class="viewer-page">
            <Text class="viewer-page__title">"Markdown viewer"</Text>
            <Text class="viewer-page__path">{format!("Open: {open_path}")}</Text>
            {document}
            <Div class="viewer-page__actions">
                <Button on_press=move || {
                    home_navigate("/", NavigateOptions::default());
                    AppControl::Continue
                }>"Home"</Button>
                <Button on_press=move || {
                    browse_navigate("/", NavigateOptions::default());
                    AppControl::Continue
                }>"Browse files"</Button>
            </Div>
            <Text class="viewer-page__help">
                "PgUp/Dn scroll | e edit | r reload | h home | b browse | q quit"
            </Text>
        </Div>
    }
}

/// Resolves and validates the Markdown path represented by a Viewer route.
fn resolve_viewer_path(
    route_path: Option<&str>,
    current_directory: &Path,
) -> Result<Option<PathBuf>, String> {
    let Some(route_path) = route_path.filter(|path| !path.is_empty()) else {
        return Ok(None);
    };
    let route_path = PathBuf::from(route_path);
    let requested = if route_path.is_absolute() {
        route_path
    } else {
        current_directory.join(route_path)
    };
    if !is_markdown_path(&requested) {
        return Err(format!(
            "preview path is not a Markdown file: {}",
            requested.display()
        ));
    }
    Ok(Some(requested))
}

/// Creates an encoded viewer location for an absolute Markdown path.
///
/// # Arguments
///
/// * `path` — Absolute Markdown path to encode.
///
/// # Returns
///
/// A [`String`] containing `/view/` and one encoded absolute path.
pub(crate) fn viewer_location(path: &Path) -> String {
    let path = path.as_os_str().to_string_lossy();
    let encoded = utf8_percent_encode(&path, ROUTE_SEGMENT_ENCODE_SET);
    format!("/view/{encoded}")
}
