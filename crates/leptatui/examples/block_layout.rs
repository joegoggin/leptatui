//! Block flow and terminal box-model example.
//!
//! This binary demonstrates generic [`DivView`] containers, bordered
//! [`BlockView`] containers, content-box and border-box sizing, physical
//! spacing, percentage widths, and cumulative rounding into terminal cells.

use leptatui::prelude::*;

/// Root component for the block-layout example.
#[component]
fn BlockLayout() -> impl IntoView {
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
        .title => {
            fg: Color::LightCyan,
            modifier: Modifier::BOLD
        }
        .section => {
            margin: Edges::symmetric(
                Length::cells(0.0).into(),
                Length::cells(1.0).into()
            )
        }
        .comparison => {
            display: Display::Flex,
            gap: Axes::new(Length::cells(1.0), Length::cells(0.0))
        }
        .panel => {
            borders: Borders::ALL,
            size: LayoutSize::new(
                Dimension::from(Length::percent(50.0)),
                Dimension::from(Length::cells(6.0))
            ),
            padding: TuiSpacing::uniform(1)
        }
        .content-sized => { box_sizing: BoxSizing::ContentBox }
        .border-sized => { box_sizing: BoxSizing::BorderBox }
        .rounding-row => {
            display: Display::Flex,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::cells(3.0))
            )
        }
        .third => {
            flex_basis: Dimension::from(Length::percent(33.333)),
            flex_grow: 1.0,
            borders: Borders::ALL
        }
        .first => { bg: Color::DarkGray }
        .second => { bg: Color::Blue }
        .third-panel => { bg: Color::DarkGray }
        .hint => { fg: Color::DarkGray }

        @media (max-width: 60) {
            .app-shell => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
            .comparison => { flex_direction: FlexDirection::Column }
            .panel => {
                size: LayoutSize::new(
                    Dimension::from(Length::percent(100.0)),
                    Dimension::from(Length::cells(5.0))
                )
            }
        }
    }

    view! {
        <Block class="app-shell">
            <Div>
                <Text class="title">"Leptatui block layout"</Text>
                <Text>
                    "Div creates a generic layout box; Block adds a bordered single-child shell."
                </Text>
                <Div class="section comparison">
                    <Div class="panel content-sized">
                        <Text>"content-box"</Text>
                        <Text>"The authored size excludes padding and borders."</Text>
                    </Div>
                    <Div class="panel border-sized">
                        <Text>"border-box"</Text>
                        <Text>"The authored size includes padding and borders."</Text>
                    </Div>
                </Div>
                <Text>"Three fractional children retain one contiguous terminal row:"</Text>
                <Div class="section rounding-row">
                    <Div class="third first"><Text>"one"</Text></Div>
                    <Div class="third second"><Text>"two"</Text></Div>
                    <Div class="third third-panel"><Text>"three"</Text></Div>
                </Div>
                <Text class="hint">"Resize below 61 columns to stack the sizing comparison."</Text>
                <Text class="hint">"Press q to quit."</Text>
            </Div>
        </Block>
    }
}

/// Runs the block-layout example application.
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
async fn main() -> leptatui::app::Result<()> {
    let view = view! { <BlockLayout /> };
    App::new(view).run().await
}
