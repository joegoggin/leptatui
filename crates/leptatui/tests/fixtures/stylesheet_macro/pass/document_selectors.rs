//! Pass fixture for semantic document type selectors.

use leptatui::prelude::*;

/// Exercises every semantic document selector and its `ViewType` lowering.
fn main() {
    let stylesheet = stylesheet! {
        H1 => { fg: Color::White }
        H2 => { fg: Color::White }
        H3 => { fg: Color::White }
        H4 => { fg: Color::White }
        H5 => { fg: Color::White }
        H6 => { fg: Color::White }
        Paragraph => { fg: Color::White }
        Link => { fg: Color::White }
        CodeBlock => { fg: Color::White }
        OrderedList => { fg: Color::White }
        UnorderedList => { fg: Color::White }
        ListItem => { fg: Color::White }
        Table => { fg: Color::White }
        TableHead => { fg: Color::White }
        TableBody => { fg: Color::White }
        TableRow => { fg: Color::White }
        TableCell => { fg: Color::White }
    };

    let expected_types = [
        ViewType::H1,
        ViewType::H2,
        ViewType::H3,
        ViewType::H4,
        ViewType::H5,
        ViewType::H6,
        ViewType::Paragraph,
        ViewType::Link,
        ViewType::CodeBlock,
        ViewType::OrderedList,
        ViewType::UnorderedList,
        ViewType::ListItem,
        ViewType::Table,
        ViewType::TableHead,
        ViewType::TableBody,
        ViewType::TableRow,
        ViewType::TableCell,
    ];

    let expected = expected_types.into_iter().fold(
        Stylesheet::new(),
        |stylesheet, view_type| {
            stylesheet.rule(
                StyleSelector::view_type(view_type),
                TuiStyle::new().foreground(Color::White),
            )
        },
    );

    assert_eq!(stylesheet, expected);
}
