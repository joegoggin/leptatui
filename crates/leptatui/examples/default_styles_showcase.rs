//! Default view-style showcase example.
//!
//! This binary presents every visible built-in view except Markdown in one
//! scrollable application. Its stylesheet controls only layout and sizing so
//! the library's default colors, borders, modifiers, and interaction states
//! remain visible for review.

use leptatui::prelude::*;

/// Root component for the default view-style showcase.
///
/// # Returns
///
/// A [`View`] containing the visible built-in views with their default styles.
#[component]
fn DefaultStylesShowcase() -> impl IntoView {
    let name = RwSignal::new(String::from("Ada Lovelace"));
    let notes = RwSignal::new(String::from(
        "Press i for yellow insert mode.\nPress v or V for magenta visual mode.",
    ));
    let form_status = RwSignal::new(String::from("Editing the example form."));
    let progress = RwSignal::new(0.42_f64);

    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .showcase => {
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            ),
            overflow: Axes::new(Overflow::Hidden, Overflow::Auto),
            padding: TuiSpacing::uniform(1),
            box_sizing: BoxSizing::BorderBox

            @media (max-width: 60) {
                padding: TuiSpacing::ZERO
            }
        }

        .row => {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            gap: Axes::new(Length::cells(1.0), Length::cells(0.0))

            @media (max-width: 60) {
                flex_direction: FlexDirection::Column
            }
        }

        .text-area => {
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::cells(6.0))
            ),
            box_sizing: BoxSizing::BorderBox
        }

        .showcase-image => {
            image_size: TuiSize::new(32, 10)

            @media (max-width: 60) {
                image_size: TuiSize::new(20, 6)
            }
        }
    }

    view! {
        <Router initial_path="/">
            <Div class="showcase">
                <H1>"Leptatui default styles"</H1>
                <Paragraph>
                    "This page adds layout and sizing only. Colors, borders, modifiers, focus, active, insert, visual, and visited states come from the views themselves."
                </Paragraph>

                <H2>"Text and containers"</H2>
                <Text>"Text: plain terminal content."</Text>
                <Block>
                    <Text>"Block: a child inside the block's default border."</Text>
                </Block>
                <Div>
                    <Text>"Div: a generic layout container."</Text>
                    <Text>"Its children remain ordinary Text views."</Text>
                </Div>

                <H2>"Heading hierarchy"</H2>
                <H1>"H1 — Primary heading"</H1>
                <H2>"H2 — Secondary heading"</H2>
                <H3>"H3 — Tertiary heading"</H3>
                <H4>"H4 — Fourth-level heading"</H4>
                <H5>"H5 — Fifth-level heading"</H5>
                <H6>"H6 — Sixth-level heading"</H6>
                <Paragraph>
                    "Paragraph: semantic prose that wraps to the available terminal width."
                </Paragraph>

                <H2>"Lists"</H2>
                <OrderedList start=3>
                    <ListItem><Paragraph>"Ordered list item starting at three."</Paragraph></ListItem>
                    <ListItem>
                        <Paragraph>"Ordered item containing a nested unordered list."</Paragraph>
                        <UnorderedList>
                            <ListItem><Paragraph>"First unordered item."</Paragraph></ListItem>
                            <ListItem><Paragraph>"Second unordered item."</Paragraph></ListItem>
                        </UnorderedList>
                    </ListItem>
                </OrderedList>

                <H2>"Table"</H2>
                <Table>
                    <TableHead>
                        <TableRow>
                            <TableCell>"View"</TableCell>
                            <TableCell alignment={CellAlignment::Center}>"State"</TableCell>
                            <TableCell alignment={CellAlignment::Right}>"Default"</TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        <TableRow>
                            <TableCell>"TableHead"</TableCell>
                            <TableCell alignment={CellAlignment::Center}>"Visible"</TableCell>
                            <TableCell alignment={CellAlignment::Right}>"Styled"</TableCell>
                        </TableRow>
                        <TableRow>
                            <TableCell>"TableBody"</TableCell>
                            <TableCell alignment={CellAlignment::Center}>"Visible"</TableCell>
                            <TableCell alignment={CellAlignment::Right}>"Inherited"</TableCell>
                        </TableRow>
                    </TableBody>
                </Table>

                <H2>"Code block"</H2>
                <CodeBlock language="rust" line_numbers=true>
                    {"#[component]\nfn Greeting() -> impl IntoView {\n    view! { <Text>\"Hello\"</Text> }\n}\n"}
                </CodeBlock>

                <H2>"Form controls"</H2>
                {move || {
                    let name_value = name.get_untracked();
                    let notes_value = notes.get_untracked();

                    view! {
                        <Form
                            on_submit=move || {
                                form_status.set(String::from("Submitted the example form."));
                                AppControl::Continue
                            }
                            on_cancel=move || {
                                form_status.set(String::from("Canceled the example form."));
                                AppControl::Continue
                            }
                        >
                            <Text>"Input"</Text>
                            <Input
                                value=name_value
                                placeholder="Enter a name"
                                on_input=move |next| {
                                    name.set(next);
                                    form_status.set(String::from("Editing the example form."));
                                    AppControl::Continue
                                }
                            />
                            <Text>"TextArea"</Text>
                            <TextArea
                                class="text-area"
                                value=notes_value
                                placeholder="Enter multiple lines"
                                on_input=move |next| {
                                    notes.set(next);
                                    form_status.set(String::from("Editing the example form."));
                                    AppControl::Continue
                                }
                            />
                            <Button on_press=move || {
                                form_status.set(String::from("Pressed the default Button."));
                                AppControl::Continue
                            }>"Default Button"</Button>
                        </Form>
                    }
                }}
                {move || text(form_status.get_untracked())}

                <H2>"Progress"</H2>
                {move || {
                    let current = progress.get_untracked();
                    progress_bar(current).label(format!("{:.0}% complete", current * 100.0))
                }}
                <Div class="row">
                    <Button on_press=move || {
                        progress.update(|value| *value = (*value + 0.1).min(1.0));
                        AppControl::Continue
                    }>"Advance progress"</Button>
                    <Button on_press=move || {
                        progress.set(0.0);
                        AppControl::Continue
                    }>"Reset progress"</Button>
                </Div>

                <H2>"Links"</H2>
                <Link href="https://github.com/joegoggin/leptatui">
                    "Standalone Link — blue initially, magenta after opening"
                </Link>
                <Paragraph>
                    "The two router anchors below start blue. Activating one marks it magenta and switches which anchor receives the bold active style."
                </Paragraph>
                <Div class="row">
                    <A href="/" exact=true>"A — Overview route"</A>
                    <A href="/alternate" exact=true>"A — Alternate route"</A>
                </Div>

                <H2>"Image"</H2>
                <Image
                    class="showcase-image"
                    src="crates/leptatui/examples/assets/showcase.jpg"
                    alt="Image fallback: terminal graphics are unavailable in this terminal."
                />

                <H2>"Interaction guide"</H2>
                <Paragraph>
                    "Tab and Shift+Tab move focus. Press i on Input or TextArea for yellow insert mode; press v or V for magenta visual modes; Esc returns to normal mode. Enter or Space activates buttons and links; successfully activated links and router anchors remain magenta. Scroll with j/k, Page Up/Page Down, or the mouse wheel. Press q to quit."
                </Paragraph>
            </Div>
        </Router>
    }
}

/// Runs the default view-style showcase application.
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
    let view = view! { <DefaultStylesShowcase /> };
    App::new(view).run().await
}
