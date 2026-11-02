//! Nested two-axis overflow example.
//!
//! This binary demonstrates automatic and forced scrollbars, independent
//! overflow axes, nested clipping, pointer-targeted wheel scrolling, and
//! scroll bubbling when an inner container reaches its boundary.

use leptatui::prelude::*;

/// Root component for the nested-overflow example.
#[component]
fn NestedOverflow() -> impl IntoView {
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
        .outer-scroll => {
            borders: Borders::ALL,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::cells(14.0))
            ),
            overflow: Axes::new(Overflow::Hidden, Overflow::Auto),
            padding: TuiSpacing::uniform(1)
        }
        .inner-scroll => {
            borders: Borders::ALL,
            size: LayoutSize::new(
                Dimension::from(Length::cells(52.0)),
                Dimension::from(Length::cells(7.0))
            ),
            overflow: Axes::new(Overflow::Auto, Overflow::Scroll)
        }
        .wide-content => {
            size: LayoutSize::new(
                Dimension::from(Length::cells(78.0)),
                Dimension::Auto
            )
        }
        .inner-title => {
            fg: Color::Yellow,
            modifier: Modifier::BOLD
        }
        .hint => { fg: Color::DarkGray }
    }

    view! {
        <Block class="app-shell">
            <Div>
                <Text class="title">"Leptatui nested overflow"</Text>
                <Text class="hint">
                    "Use j/k or Page Up/Page Down for vertical scrolling."
                </Text>
                <Text class="hint">
                    "Point at a pane and use vertical or horizontal wheel events."
                </Text>
                <Div class="outer-scroll">
                    <Text>"Outer row 1: the outer pane adds a scrollbar only when needed."</Text>
                    <Text>"Outer row 2: its horizontal overflow is clipped without scrolling."</Text>
                    <Div class="inner-scroll">
                        <Div class="wide-content">
                            <Text class="inner-title">
                                "Inner pane: 78 cells wide with forced vertical scrolling"
                            </Text>
                            <Text>"Inner row 1: horizontal wheel events reveal this long line → 01 02 03 04 05 06 07 08 09 10"</Text>
                            <Text>"Inner row 2: automatic horizontal overflow reserves a gutter only when needed."</Text>
                            <Text>"Inner row 3: the vertical scrollbar is always present."</Text>
                            <Text>"Inner row 4: wheel events target the pane under the pointer."</Text>
                            <Text>"Inner row 5: scrolling bubbles outward at an inner boundary."</Text>
                            <Text>"Inner row 6: nested clips compose before painting and hit testing."</Text>
                            <Text>"Inner row 7: content remains addressable on both axes."</Text>
                            <Text>"Inner row 8: end of the inner pane."</Text>
                        </Div>
                    </Div>
                    <Text>"Outer row 3: content continues below the nested pane."</Text>
                    <Text>"Outer row 4: keep scrolling to exercise the outer boundary."</Text>
                    <Text>"Outer row 5: hidden overflow still retains scroll-container semantics."</Text>
                    <Text>"Outer row 6: automatic overflow clips only while content exceeds the viewport."</Text>
                    <Text>"Outer row 7: scrollbar gutters reduce the content viewport."</Text>
                    <Text>"Outer row 8: end of the outer pane."</Text>
                </Div>
                <Text class="hint">"Press q to quit."</Text>
            </Div>
        </Block>
    }
}

/// Runs the nested-overflow example application.
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
    let view = view! { <NestedOverflow /> };
    App::new(view).run().await
}
