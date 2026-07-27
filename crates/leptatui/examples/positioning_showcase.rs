//! Positioning and stacking-context example.
//!
//! This binary demonstrates static, relative, absolute, fixed, and sticky
//! positioning together with containing blocks, scrolling, overlap, clipping,
//! and signed stacking levels.

use leptatui::prelude::*;

/// Root component for the positioning showcase.
#[component]
fn PositioningShowcase() -> impl IntoView {
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
        .scrollport => {
            position: Position::Relative,
            borders: Borders::ALL,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::cells(16.0))
            ),
            overflow: Axes::new(Overflow::Hidden, Overflow::Auto)
        }
        .flow-card => {
            borders: Borders::ALL,
            bg: Color::DarkGray
        }
        .relative-card => {
            position: Position::Relative,
            inset: Edges::new(
                Length::cells(1.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto,
                Length::cells(4.0).into()
            ),
            size: LayoutSize::new(
                Dimension::from(Length::cells(28.0)),
                Dimension::from(Length::cells(3.0))
            ),
            borders: Borders::ALL,
            bg: Color::Blue,
            z_index: ZIndex::Integer(1)
        }
        .absolute-card => {
            position: Position::Absolute,
            inset: Edges::new(
                Length::cells(5.0).into(),
                Length::cells(2.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto
            ),
            size: LayoutSize::new(
                Dimension::from(Length::cells(30.0)),
                Dimension::from(Length::cells(3.0))
            ),
            borders: Borders::ALL,
            bg: Color::Red,
            z_index: ZIndex::Integer(2)
        }
        .fixed-card => {
            position: Position::Fixed,
            inset: Edges::new(
                Length::cells(1.0).into(),
                Length::cells(1.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto
            ),
            size: LayoutSize::new(
                Dimension::from(Length::cells(25.0)),
                Dimension::from(Length::cells(3.0))
            ),
            borders: Borders::ALL,
            bg: Color::Magenta,
            z_index: ZIndex::Integer(4)
        }
        .sticky-card => {
            position: Position::Sticky,
            inset: Edges::new(
                Length::cells(0.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto,
                LengthAuto::Auto
            ),
            borders: Borders::ALL,
            bg: Color::Green,
            z_index: ZIndex::Integer(3)
        }
        .negative-layer => {
            position: Position::Relative,
            inset: Edges::new(
                Length::cells(-1.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto,
                Length::cells(2.0).into()
            ),
            borders: Borders::ALL,
            bg: Color::DarkGray,
            z_index: ZIndex::Integer(-1)
        }
        .hint => { fg: Color::DarkGray }
    }

    view! {
        <Block class="app-shell">
            <Div>
                <Text class="title">"Leptatui positioning showcase"</Text>
                <Text class="hint">
                    "Scroll with j/k or Page Up/Page Down; the fixed card stays viewport-relative."
                </Text>
                <Div class="scrollport">
                    <Div class="flow-card">
                        <Text>"Static: normal flow, authored insets ignored."</Text>
                    </Div>
                    <Div class="sticky-card">
                        <Text>"Sticky: keeps flow space and pins to this scrollport."</Text>
                    </Div>
                    <Text>"Flow row 1"</Text>
                    <Div class="relative-card">
                        <Text>"Relative: offset after layout while its original space remains."</Text>
                    </Div>
                    <Text>"Flow row 2: the relative box still reserves its source position."</Text>
                    <Div class="absolute-card">
                        <Text>"Absolute: leaves flow and uses the relative scrollport."</Text>
                    </Div>
                    <Text>"Flow row 3: the absolute card overlaps without reserving a row."</Text>
                    <Div class="negative-layer">
                        <Text>"z-index -1: behind normal flow, above its context background."</Text>
                    </Div>
                    <Text>"Flow row 4"</Text>
                    <Text>"Flow row 5"</Text>
                    <Text>"Flow row 6"</Text>
                    <Text>"Flow row 7"</Text>
                    <Text>"Flow row 8"</Text>
                    <Text>"Flow row 9"</Text>
                    <Text>"Flow row 10: end of the positioned scrollport."</Text>
                </Div>
                <Div class="fixed-card">
                    <Text>"Fixed: terminal viewport, z-index 4."</Text>
                </Div>
                <Text class="hint">"Press q to quit."</Text>
            </Div>
        </Block>
    }
}

/// Runs the positioning showcase application.
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
    App::new(PositioningShowcase::new()).run().await
}
