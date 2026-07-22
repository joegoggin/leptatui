//! Controlled form example.
//!
//! This binary demonstrates controlled `Input`, `TextArea`, and `Form` views
//! backed by Leptos signals. The editable controls emit proposed values through
//! callbacks, and the dynamic form subtree reconciles focus, cursor, and
//! Vim state across redraws.

use leptatui::prelude::*;

/// Last high-level form action shown in the demo status line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FormStatus {
    /// The form is being edited.
    Editing,
    /// The form has been submitted.
    Submitted,
    /// The form has been canceled.
    Canceled,
}

impl FormStatus {
    /// Returns the user-facing status line for the current form values.
    ///
    /// # Arguments
    ///
    /// * `name` — Current controlled name value.
    /// * `notes` — Current controlled notes value.
    ///
    /// # Returns
    ///
    /// A [`String`] describing the latest form state.
    fn label(self, name: &str, notes: &str) -> String {
        match self {
            Self::Editing => String::from("Editing draft"),
            Self::Submitted => {
                let line_count = notes.lines().count();
                format!("Submitted {name} with {line_count} note line(s)")
            }
            Self::Canceled => String::from("Canceled draft"),
        }
    }
}

/// Root component for the controlled form example.
///
/// # Returns
///
/// A [`View`] containing the controlled form demo.
#[component]
fn ControlledForm() -> impl IntoView {
    let name = RwSignal::new(String::from("Ada Lovelace"));
    let notes = RwSignal::new(String::from("Sketch the first program."));
    let status = RwSignal::new(FormStatus::Editing);

    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .form-shell => {
            fg: Color::White,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }

        .title => {
            fg: Color::LightCyan,
            modifier: Modifier::BOLD
        }

        .status => { fg: Color::LightGreen }
        .label => { fg: Color::Gray }
        .help => { fg: Color::DarkGray }

        Input => {
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)
        }

        TextArea => {
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)
        }

        Button => {
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)
        }

        Input:focus => {
            fg: Color::Black,
            bg: Color::Yellow,
            modifier: Modifier::BOLD,
            border_type: BorderType::Thick
        }

        TextArea:focus => {
            fg: Color::Black,
            bg: Color::Yellow,
            modifier: Modifier::BOLD,
            border_type: BorderType::Thick
        }

        Button:focus => {
            fg: Color::Black,
            bg: Color::Yellow,
            modifier: Modifier::BOLD,
            border_type: BorderType::Thick
        }

        @media (max-width: 60) {
            .form-shell => {
                padding: TuiSpacing::ZERO
            }

            Input => {
                padding: TuiSpacing::ZERO
            }

            TextArea => {
                padding: TuiSpacing::ZERO
            }

            Button => {
                padding: TuiSpacing::ZERO
            }
        }
    }

    view! {
        <Block class="form-shell">
            <Column>
                <Text class="title">"Controlled form"</Text>
                {move || {
                    text(
                            status
                                .get_untracked()
                                .label(
                                    name.get_untracked().as_str(),
                                    notes.get_untracked().as_str(),
                                ),
                        )
                        .with_classes("status")
                }}
                {move || {
                    let name_value = name.get_untracked();
                    let notes_value = notes.get_untracked();

                    view! {
                        <Form
                            on_submit=move || {
                                status.set(FormStatus::Submitted);
                                AppControl::Continue
                            }
                            on_cancel=move || {
                                status.set(FormStatus::Canceled);
                                AppControl::Continue
                            }
                        >
                            <Text class="label">"Name"</Text>
                            <Input
                                value=name_value
                                placeholder="Name"
                                on_input=move |next| {
                                    name.set(next);
                                    status.set(FormStatus::Editing);
                                    AppControl::Continue
                                }
                            />
                            <Text class="label">"Notes"</Text>
                            <TextArea
                                value=notes_value
                                placeholder="Notes"
                                on_input=move |next| {
                                    notes.set(next);
                                    status.set(FormStatus::Editing);
                                    AppControl::Continue
                                }
                            />
                            <Button on_press=move || {
                                status.set(FormStatus::Submitted);
                                AppControl::Continue
                            }>"Submit"</Button>
                        </Form>
                    }
                }}
                <Text class="help">
                    "Tab/Shift+Tab focus. i/a enters insert mode. Esc cancels from normal mode. Ctrl+Enter submits notes. q quits."
                </Text>
            </Column>
        </Block>
    }
}

/// Runs the controlled form example application.
///
/// # Returns
///
/// An empty [`Result`] when the app exits successfully.
///
/// # Errors
///
/// Returns [`Error::Io`] if terminal setup, rendering, input, or cleanup fails.
/// Returns [`Error::EventTask`] if the blocking event task fails.
#[tokio::main]
async fn main() -> Result<()> {
    App::new(ControlledForm::new()).run().await
}
