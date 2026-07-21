//! Pass fixture for semantic document elements in `view!`.

use leptatui::prelude::*;

/// Exercises every semantic document tag and supported document attribute.
fn main() {
    let heading_style = TuiStyle::new().foreground(Color::Cyan);
    let source = String::from("fn main() {}\n");
    let markdown_path = String::from("guide.md");
    let markdown_class = String::from("markdown guide");
    let markdown_id = String::from("markdown-guide");
    let markdown_style = TuiStyle::new().foreground(Color::Green);

    let view = view! {
        <Column>
            <H1 class="title primary" id="guide" style={heading_style}>"Guide"</H1>
            <H2>"Overview"</H2>
            <H3>"Details"</H3>
            <H4>"Examples"</H4>
            <H5>"Notes"</H5>
            <H6>"Aside"</H6>
            <Paragraph>{String::from("Semantic content")}</Paragraph>
            <Markdown
                src={markdown_path.clone()}
                class={markdown_class}
                id={markdown_id}
                style={markdown_style}
                line_numbers=true
                syntax_theme={SyntaxTheme::Light}
            />
            <OrderedList start=3>
                <ListItem>
                    <Paragraph>"First"</Paragraph>
                    <UnorderedList>
                        <ListItem><Paragraph>"Nested"</Paragraph></ListItem>
                    </UnorderedList>
                </ListItem>
            </OrderedList>
            <Table>
                <TableHead>
                    <TableRow>
                        <TableCell>"Name"</TableCell>
                        <TableCell alignment={CellAlignment::Center}>"Status"</TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    <TableRow>
                        <TableCell>"Parser"</TableCell>
                        <TableCell alignment={CellAlignment::Right}>"Ready"</TableCell>
                    </TableRow>
                </TableBody>
            </Table>
            <CodeBlock
                language="rust"
                line_numbers=true
                syntax_theme={SyntaxTheme::Light}
            >{source.clone()}</CodeBlock>
        </Column>
    };

    let expected = column((
        h1("Guide")
            .with_classes("title primary")
            .with_id("guide")
            .with_inline_style(heading_style),
        h2("Overview"),
        h3("Details"),
        h4("Examples"),
        h5("Notes"),
        h6("Aside"),
        paragraph(String::from("Semantic content")),
        markdown_file_with_options(
            markdown_path,
            MarkdownOptions::default()
                .syntax_theme(SyntaxTheme::Light)
                .line_numbers(true),
        )
            .with_classes("markdown guide")
            .with_id("markdown-guide")
            .with_inline_style(markdown_style),
        ordered_list([list_item((
            paragraph("First"),
            unordered_list([list_item([paragraph("Nested")])]),
        ))])
        .start(3),
        table([
            table_head([table_row([
                table_cell("Name"),
                table_cell("Status").alignment(CellAlignment::Center),
            ])]),
            table_body([table_row([
                table_cell("Parser"),
                table_cell("Ready").alignment(CellAlignment::Right),
            ])]),
        ]),
        code_block(source)
            .language("rust")
            .line_numbers(true)
            .syntax_theme(SyntaxTheme::Light),
    ));

    assert_eq!(view, expected);
}
