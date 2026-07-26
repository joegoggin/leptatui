//! Responsive flexbox application shell example.
//!
//! This binary demonstrates a horizontal navigation bar, growing main content,
//! a fixed-basis sidebar, and media-query rules that stack both layout regions
//! on narrow terminals.

use leptatui::prelude::*;

/// Root component for the responsive flexbox example.
#[component]
fn ResponsiveFlex() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .app-shell => {
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }
        .app-title => {
            fg: Color::LightCyan,
            modifier: Modifier::BOLD
        }
        .nav => {
            display: Display::Flex,
            gap: Axes::new(Length::cells(1.0), Length::cells(0.0)),
            align_items: AlignItems::Center
        }
        .workspace => {
            display: Display::Flex,
            gap: Axes::new(Length::cells(1.0), Length::cells(0.0)),
            align_items: AlignItems::Stretch
        }
        .content => {
            flex_basis: Dimension::from(Length::cells(0.0)),
            flex_grow: 1.0,
            padding: TuiSpacing::uniform(1)
        }
        .sidebar => {
            flex_basis: Dimension::from(Length::cells(24.0)),
            flex_shrink: 0.0,
            padding: TuiSpacing::uniform(1)
        }
        .section-title => {
            fg: Color::Yellow,
            modifier: Modifier::BOLD
        }
        .hint => { fg: Color::DarkGray }

        Button => {
            borders: Borders::ALL,
            padding: TuiSpacing::horizontal(1)
        }

        @media (max-width: 60) {
            .app-shell => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
            .nav => {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch
            }
            .workspace => {
                flex_direction: FlexDirection::Column,
                gap: Axes::new(Length::cells(0.0), Length::cells(1.0))
            }
            .content => { flex_basis: Dimension::Auto }
            .sidebar => {
                flex_basis: Dimension::Auto,
                flex_shrink: 1.0
            }
            Button => { padding: TuiSpacing::ZERO }
        }
    }

    view! {
        <Block class="app-shell">
            <Div>
                <Text class="app-title">"Leptatui responsive flex"</Text>
                <Div class="nav">
                    <Button>"Overview"</Button>
                    <Button>"Guides"</Button>
                    <Button>"API"</Button>
                </Div>
                <Div class="workspace">
                    <Block class="content">
                        <Div>
                            <Text class="section-title">"Build terminal interfaces"</Text>
                            <Text>
                                "The content region grows into free space beside the sidebar."
                            </Text>
                            <Text class="hint">
                                "Resize below 61 columns to stack every region."
                            </Text>
                        </Div>
                    </Block>
                    <Block class="sidebar">
                        <Div>
                            <Text class="section-title">"On this page"</Text>
                            <Text>"Flex containers"</Text>
                            <Text>"Responsive rules"</Text>
                            <Text>"Terminal rounding"</Text>
                        </Div>
                    </Block>
                </Div>
                <Text class="hint">"Press q to quit."</Text>
            </Div>
        </Block>
    }
}

/// Runs the responsive flexbox example application.
///
/// # Returns
///
/// An empty [`Result`] when the application exits successfully.
///
/// # Errors
///
/// Returns [`Error::Io`] if terminal setup, rendering, input, or cleanup fails.
/// Returns [`Error::EventTask`] if the blocking event task fails.
#[tokio::main]
async fn main() -> Result<()> {
    App::new(ResponsiveFlex::new()).run().await
}
