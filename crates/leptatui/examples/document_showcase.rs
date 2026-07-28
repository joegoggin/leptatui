//! Semantic document component showcase.
//!
//! This binary demonstrates headings, paragraphs, nested lists, responsive
//! tables, and syntax-highlighted code blocks in one scrollable document.

use leptatui::prelude::*;

/// Renders the semantic document component showcase.
///
/// # Returns
///
/// A [`View`] containing every semantic document component.
#[component]
fn DocumentShowcase() -> impl IntoView {
    let source = String::from(
        "#[component]\nfn Greeting() -> impl IntoView {\n    view! { <Paragraph>\"Hello, terminal!\"</Paragraph> }\n}",
    );

    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .document-shell => {
            fg: Color::White,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }

        .help => { fg: Color::DarkGray }

        H1 => { fg: Color::LightCyan }
        H2 => { fg: Color::LightBlue }
        H3 => { fg: Color::LightGreen }
        H4 => { fg: Color::LightYellow }
        H5 => { fg: Color::LightMagenta }
        H6 => { fg: Color::Gray }
        Paragraph => { fg: Color::White }
        OrderedList => { fg: Color::LightCyan }
        UnorderedList => { fg: Color::LightGreen }
        Table => { fg: Color::White }
        TableHead => { fg: Color::LightCyan }
        CodeBlock => { fg: Color::LightBlue }

        @media (max-width: 60) {
            .document-shell => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
        }
    }

    view! {
        <Block class="document-shell">
            <Div>
                <H1>"Leptatui document views"</H1>
                <Paragraph>
                    "Semantic components provide readable terminal documents that wrap and resize with the viewport."
                </Paragraph>

                <H2>"Heading hierarchy"</H2>
                <H3>"Semantic sections"</H3>
                <H4>"Responsive content"</H4>
                <H5>"Stylesheet selectors"</H5>
                <H6>"The smallest heading"</H6>
                <Paragraph>
                    "Each heading level has its own semantic view type and default emphasis."
                </Paragraph>

                <H2>"Nested lists"</H2>
                <OrderedList start=3>
                    <ListItem>
                        <Paragraph>"Compose documents from block-oriented list items."</Paragraph>
                    </ListItem>
                    <ListItem>
                        <Paragraph>"Nest ordered and unordered structures."</Paragraph>
                        <UnorderedList>
                            <ListItem>
                                <Paragraph>"Markers adapt to the nesting depth."</Paragraph>
                            </ListItem>
                            <ListItem>
                                <Paragraph>"Wrapped lines remain aligned with their content."</Paragraph>
                            </ListItem>
                        </UnorderedList>
                    </ListItem>
                </OrderedList>

                <H2>"Responsive table"</H2>
                <Table>
                    <TableHead>
                        <TableRow>
                            <TableCell>"Component"</TableCell>
                            <TableCell alignment={CellAlignment::Center}>"Layout"</TableCell>
                            <TableCell alignment={CellAlignment::Right}>"Status"</TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        <TableRow>
                            <TableCell>"Lists"</TableCell>
                            <TableCell alignment={CellAlignment::Center}>"Nested"</TableCell>
                            <TableCell alignment={CellAlignment::Right}>"Ready"</TableCell>
                        </TableRow>
                        <TableRow>
                            <TableCell>"Tables"</TableCell>
                            <TableCell alignment={CellAlignment::Center}>"Responsive"</TableCell>
                            <TableCell alignment={CellAlignment::Right}>"Ready"</TableCell>
                        </TableRow>
                    </TableBody>
                </Table>

                <H2>"Highlighted code"</H2>
                <CodeBlock
                    language="rust"
                    line_numbers=true
                    syntax_theme={SyntaxTheme::Dark}
                >{source}</CodeBlock>

                <Paragraph class="help">
                    "Scroll with j/k, Page Up/Page Down, gg, or G. Press q to quit."
                </Paragraph>
            </Div>
        </Block>
    }
}

/// Runs the document showcase application.
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
    let view = view! { <DocumentShowcase /> };
    App::new(view).run().await
}
