//! External editor hook example.
//!
//! This binary demonstrates editing a reactive Markdown string and, when a
//! command-line path is supplied, opening an existing file. The shared editor
//! status remains reactive while Leptatui restores the terminal around the
//! user's preferred editor.

use std::path::PathBuf;

use leptatui::prelude::*;

/// Renders reactive text and optional path editing controls.
///
/// # Returns
///
/// A [`View`] containing the external-editor demonstration.
#[component]
fn ExternalEditor() -> impl IntoView {
    let editor = use_editor();
    let file_path = std::env::args_os().nth(1).map(PathBuf::from);
    let draft = RwSignal::new(String::from(
        "# External editor draft\n\nPress `t` to edit this Markdown text.\n",
    ));
    let target = RwSignal::new(String::from("No edit requested."));
    let status_editor = editor.clone();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Char('t') => {
                target.set(String::from("Editing the in-memory Markdown draft."));
                editor.edit_text(draft);
                KeyControl::Handled
            }
            KeyCode::Char('f') => {
                if let Some(path) = file_path.clone() {
                    target.set(format!("Editing file: {}", path.display()));
                    editor.edit_file(path);
                } else {
                    target.set(String::from(
                        "No file argument supplied; run with `-- path/to/file`.",
                    ));
                    editor.clear();
                }
                KeyControl::Handled
            }
            KeyCode::Char('c') => {
                target.set(String::from("Editor status cleared."));
                editor.clear();
                KeyControl::Handled
            }
            KeyCode::Char('q') => KeyControl::Exit,
            _ => KeyControl::Pass,
        }
    });

    stylesheet! {
        .editor-shell => {
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)

            @media (max-width: 60) {
                padding: TuiSpacing::ZERO
            }
        }

        .editor-title => {
            fg: Color::LightCyan,
            modifier: Modifier::BOLD
        }

        .editor-status => { fg: Color::LightGreen }
        .editor-error => { fg: Color::LightRed }
        .editor-draft => {
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)
        }
        .editor-help => { fg: Color::Gray }
    }

    view! {
        <Block class="editor-shell">
            <Div>
                <Text class="editor-title">"External editor hook"</Text>
                <Text>{target}</Text>
                {move || match status_editor.status() {
                    None => view! { <Text class="editor-status">"Operation: idle"</Text> },
                    Some(EditorStatus::Pending) => {
                        view! { <Text class="editor-status">"Operation: editor open"</Text> }
                    }
                    Some(EditorStatus::Complete) => {
                        view! { <Text class="editor-status">"Operation: completed"</Text> }
                    }
                    Some(EditorStatus::Error(error)) => {
                        view! {
                            <Text class="editor-error">{format!("Operation failed: {error}")}</Text>
                        }
                    }
                }}
                <Text class="editor-help">
                    "t edit draft | f edit optional file | c clear status | q quit"
                </Text>
                <Text class="editor-title">"Reactive Markdown draft"</Text>
                <Text class="editor-draft">{draft}</Text>
            </Div>
        </Block>
    }
}

/// Runs the external editor example.
///
/// # Returns
///
/// An empty [`Result`] when the application exits successfully.
///
/// # Errors
///
/// Returns [`Error::Io`] if terminal setup, rendering, input, or cleanup fails.
#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    let view = view! { <ExternalEditor /> };
    App::new(view).run().await
}
