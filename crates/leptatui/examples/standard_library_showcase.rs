//! Standard library showcase example.
//!
//! This binary demonstrates the expanded standard component library in one
//! coherent app. It uses controlled `Input` and `TextArea` fields, `Form`
//! submit and cancel callbacks, a standalone `Link`, stylesheet-sized image
//! fallback rendering, and a progress bar backed by Leptos signals.

use leptatui::prelude::*;

/// Current high-level state of the showcase form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShowcaseStatus {
    /// The form contains an editable draft.
    Editing,
    /// The form has been submitted.
    Submitted,
    /// The form has been canceled.
    Canceled,
}

impl ShowcaseStatus {
    /// Returns the user-facing status line for the current showcase state.
    ///
    /// # Arguments
    ///
    /// * `name` — Current controlled name field value.
    /// * `notes` — Current controlled notes field value.
    /// * `progress` — Current progress ratio.
    ///
    /// # Returns
    ///
    /// A [`String`] describing the latest showcase state.
    fn label(self, name: &str, notes: &str, progress: f64) -> String {
        let percent = (progress * 100.0).round();

        match self {
            Self::Editing => format!("Editing {name}'s draft at {percent:.0}%"),
            Self::Submitted => {
                let line_count = notes.lines().count().max(1);
                format!("Submitted {name} with {line_count} note line(s)")
            }
            Self::Canceled => String::from("Canceled the current draft"),
        }
    }
}

/// Returns a progress value clamped to the supported progress bar range.
///
/// # Arguments
///
/// * `value` — Candidate progress ratio.
///
/// # Returns
///
/// A [`f64`] ratio in the `0.0..=1.0` range.
fn clamp_progress(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Root component for the standard library showcase.
///
/// # Returns
///
/// A [`View`] containing form controls, a link, image fallback, and progress state.
#[component]
fn StandardLibraryShowcase() -> impl IntoView {
    let name = RwSignal::new(String::from("Ada Lovelace"));
    let notes = RwSignal::new(String::from("Sketch the first program."));
    let status = RwSignal::new(ShowcaseStatus::Editing);
    let progress = RwSignal::new(0.35_f64);

    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .showcase-shell => {
            fg: Color::White,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)

            @media (max-width: 60) {
                padding: TuiSpacing::ZERO
            }
        }

        .title => {
            fg: Color::LightCyan,
            modifier: Modifier::BOLD
        }

        .status => { fg: Color::LightGreen }
        .label => { fg: Color::Gray }
        .caption => { fg: Color::DarkGray }
        .help => { fg: Color::DarkGray }
        .meter => { fg: Color::LightGreen, bg: Color::DarkGray }
        .image-small => {
            image_size: TuiSize::new(24, 8)

            @media (max-width: 60) {
                image_size: TuiSize::new(20, 6)
            }
        }
        .image-medium => {
            image_size: TuiSize::new(36, 12)

            @media (max-width: 60) {
                image_size: TuiSize::new(28, 8)
            }
        }
        .image-large => {
            image_size: TuiSize::new(48, 16)

            @media (max-width: 60) {
                image_size: TuiSize::new(36, 10)
            }
        }

        Input => {
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)

            @media (max-width: 60) {
                padding: TuiSpacing::ZERO
            }
        }

        TextArea => {
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)

            @media (max-width: 60) {
                padding: TuiSpacing::ZERO
            }
        }

        Image => {
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)

            @media (max-width: 60) {
                padding: TuiSpacing::ZERO
            }
        }

        Button => {
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)

            @media (max-width: 60) {
                padding: TuiSpacing::ZERO
            }
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

        Link:focus => {
            fg: Color::Black,
            bg: Color::Yellow,
            modifier: Modifier::BOLD | Modifier::UNDERLINED
        }

        .actions => {
            display: Display::Flex

            @media (max-width: 60) {
                flex_direction: FlexDirection::Column
            }
        }
    }

    view! {
        <Block class="showcase-shell">
            <Div>
                <Text class="title">"Standard library showcase"</Text>
                {move || {
                    let current_name = name.get();
                    let current_notes = notes.get();
                    let current_progress = progress.get();
                    text(
                            status
                                .get()
                                .label(&current_name, &current_notes, current_progress),
                        )
                        .with_classes("status")
                }}
                {move || {
                    let name_value = name.get();
                    let notes_value = notes.get();

                    view! {
                        <Form
                            on_submit=move || {
                                status.set(ShowcaseStatus::Submitted);
                                AppControl::Continue
                            }
                            on_cancel=move || {
                                status.set(ShowcaseStatus::Canceled);
                                AppControl::Continue
                            }
                        >
                            <Text class="label">"Name"</Text>
                            <Input
                                value=name_value
                                placeholder="Name"
                                on_input=move |next| {
                                    name.set(next);
                                    status.set(ShowcaseStatus::Editing);
                                    AppControl::Continue
                                }
                            />
                            <Text class="label">"Notes"</Text>
                            <TextArea
                                value=notes_value
                                placeholder="Notes"
                                on_input=move |next| {
                                    notes.set(next);
                                    status.set(ShowcaseStatus::Editing);
                                    AppControl::Continue
                                }
                            />
                            <Div class="actions">
                                <Button on_press=move || {
                                    status.set(ShowcaseStatus::Submitted);
                                    AppControl::Continue
                                }>"Submit"</Button>
                                <Button on_press=move || {
                                    progress
                                        .update(|value| {
                                            *value = clamp_progress(*value + 0.1);
                                        });
                                    AppControl::Continue
                                }>"Advance"</Button>
                                <Button on_press=move || {
                                    progress.set(0.0);
                                    status.set(ShowcaseStatus::Editing);
                                    AppControl::Continue
                                }>"Reset"</Button>
                            </Div>
                        </Form>
                    }
                }}
                <Text class="label">"Standalone link"</Text>
                <Link href="https://github.com/joegoggin/leptatui">
                    "Open Leptatui on GitHub"
                </Link>
                <Text class="label">"Image small"</Text>
                <Image
                    class="image-small"
                    src="crates/leptatui/examples/assets/showcase.jpg"
                    alt="Image fallback: terminal graphics are unavailable in this terminal."
                />
                <Text class="label">"Image medium"</Text>
                <Image
                    class="image-medium"
                    src="crates/leptatui/examples/assets/showcase.jpg"
                    alt="Image fallback: terminal graphics are unavailable in this terminal."
                />
                <Text class="label">"Image large"</Text>
                <Image
                    class="image-large"
                    src="crates/leptatui/examples/assets/showcase.jpg"
                    alt="Image fallback: terminal graphics are unavailable in this terminal."
                />
                <Text class="caption">
                    "Image sizes come from stylesheet classes; supported terminals render graphics and other targets show fallback text."
                </Text>
                {move || {
                    let current_progress = progress.get();
                    progress_bar(current_progress)
                        .label(format!("{:.0}% complete", current_progress * 100.0))
                        .with_classes("meter")
                }}
                <Text class="help">
                    "Tab/Shift+Tab or pointer movement focuses controls. Enter/Space or left click activates links and buttons; the mouse wheel scrolls. i/a/I/A insert, Esc normal/cancel, jk leaves insert, v/V select, x/d/y/p/u/Ctrl+R edit, Ctrl+Enter submits notes. q quits."
                </Text>
            </Div>
        </Block>
    }
}

/// Runs the standard library showcase application.
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
async fn main() -> leptatui::app::Result<()> {
    let view = view! { <StandardLibraryShowcase /> };
    App::new(view).run().await
}
