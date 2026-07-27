//! Leptatui views and input handling for the Markdown editor.
//!
//! The initial shell proves that validated controller state reaches the
//! terminal. Explorer, preview, responsive layout, and editor controls will
//! extend this module in later phases.

use leptatui::prelude::*;

use crate::controller::Controller;

/// Creates the root Markdown editor view.
///
/// # Arguments
///
/// * `controller` — Initialized application state and service boundaries.
///
/// # Returns
///
/// A Leptatui view containing the initial application shell.
pub(crate) fn app_view(controller: Controller) -> impl IntoView {
    MarkdownEditor::with_props(
        MarkdownEditorProps::builder()
            .controller(controller)
            .build(),
    )
}

/// Renders the validated workspace root and initial control help.
///
/// # Arguments
///
/// * `controller` — Initialized application state and service boundaries.
///
/// # Returns
///
/// A Leptatui view that exits when the user presses `q`.
#[component]
fn MarkdownEditor(controller: Controller) -> impl IntoView {
    let root = controller.workspace().root().display().to_string();

    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .editor-shell => {
            fg: Color::White,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }

        .title => {
            fg: Color::LightCyan,
            modifier: Modifier::BOLD
        }

        .root => { fg: Color::LightGreen }
        .help => { fg: Color::Gray }

        @media (max-width: 60) {
            .editor-shell => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
        }
    }

    view! {
        <Block class="editor-shell">
            <Div>
                <Text class="title">"Markdown editor"</Text>
                <Text class="root">{format!("Root: {root}")}</Text>
                <Text class="help">"Explorer and preview coming next | q quit"</Text>
            </Div>
        </Block>
    }
}
