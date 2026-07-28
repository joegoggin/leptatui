//! Imported stylesheet module example.
//!
//! This binary demonstrates reusable `stylesheet!` variables and mixins,
//! `@use` imports, namespaced variables, imported mixin includes, and local
//! mixin composition.

use leptatui::prelude::*;

/// Defines reusable palette and layout variables.
fn palette() -> StyleModule {
    stylesheet! {
        $text: Color::White;
        $muted: Color::Gray;
        $surface: Color::Black;
        $primary_fg: Color::Black;
        $primary_bg: Color::LightCyan;
        $secondary_fg: Color::White;
        $secondary_bg: Color::DarkGray;
        $danger_fg: Color::White;
        $danger_bg: Color::LightRed;
        $focus_fg: Color::Black;
        $focus_bg: Color::Yellow;
        $border: Borders::ALL;
        $border_type: BorderType::Rounded;
        $focus_border_type: BorderType::Thick;
        $button_padding: TuiSpacing::horizontal(1);
        $panel_padding: TuiSpacing::uniform(1);
        $strong: Modifier::BOLD;
    }
}

/// Defines reusable button mixins that import variables from [`palette`].
fn button_styles() -> StyleModule {
    stylesheet! {
        @use palette;

        @mixin button_chrome {
            borders: palette.$border,
            border_type: palette.$border_type,
            padding: palette.$button_padding
        }

        @mixin primary {
            @include button_chrome,
            fg: palette.$primary_fg,
            bg: palette.$primary_bg,
            modifier: palette.$strong
        }

        @mixin secondary {
            @include button_chrome,
            fg: palette.$secondary_fg,
            bg: palette.$secondary_bg
        }

        @mixin danger {
            @include button_chrome,
            fg: palette.$danger_fg,
            bg: palette.$danger_bg,
            modifier: palette.$strong
        }

        @mixin focused {
            fg: palette.$focus_fg,
            bg: palette.$focus_bg,
            modifier: palette.$strong,
            border_type: palette.$focus_border_type
        }
    }
}

/// Root component for the stylesheet imports example.
#[component]
fn StylesheetImportsDemo() -> impl IntoView {
    let launches = RwSignal::new(0);

    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        @use palette;
        @use button_styles as buttons;

        .screen => {
            fg: palette.$text,
            bg: palette.$surface,
            borders: palette.$border,
            border_type: palette.$border_type,
            padding: palette.$panel_padding,

            .title => {
                fg: palette.$primary_bg,
                modifier: palette.$strong
            }

            .body => { fg: palette.$muted }

            .primary-action => {
                @include buttons.primary

                &:focus => { @include buttons.focused }
            }

            .secondary-action => {
                @include buttons.secondary

                &:focus => { @include buttons.focused }
            }

            .danger-action => {
                @include buttons.danger

                &:focus => { @include buttons.focused }
            }
        }

        .actions => { display: Display::Flex }

        @media (max-width: 60) {
            .screen => { padding: TuiSpacing::ZERO }
            .actions => { flex_direction: FlexDirection::Column }

            .screen => {
                .primary-action => {
                    padding: TuiSpacing::ZERO
                }

                .secondary-action => {
                    padding: TuiSpacing::ZERO
                }

                .danger-action => {
                    padding: TuiSpacing::ZERO
                }
            }
        }
    }

    view! {
        <Block class="screen">
            <Div>
                <Text class="title">"Stylesheet imports"</Text>
                <Text class="body">
                    "`@use` pulls palette variables into mixins, then imports those mixins here."
                </Text>
                {move || {
                    view! {
                        <Text class="body">
                            {format!("Deploy attempts: {}", launches.get_untracked())}
                        </Text>
                    }
                }}
                <Div class="actions">
                    <Button
                        class="primary-action"
                        on_press=move || {
                            launches.update(|launches| *launches += 1);
                            AppControl::Continue
                        }
                    >
                        "Deploy"
                    </Button>
                    <Button class="secondary-action" on_press=|| AppControl::Continue>
                        "Preview"
                    </Button>
                    <Button class="danger-action" on_press=|| AppControl::Exit>
                        "Quit"
                    </Button>
                </Div>
                <Text class="body">"Tab moves focus. Enter or Space activates. q exits."</Text>
            </Div>
        </Block>
    }
}

/// Runs the stylesheet imports example application.
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
    let view = view! { <StylesheetImportsDemo /> };
    App::new(view).run().await
}
