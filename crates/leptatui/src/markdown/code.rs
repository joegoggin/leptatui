//! Markdown code-block and heading conversion.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, TagEnd};
use ratatui::text::Text;

use crate::{AnyView, IntoView, code_block, h1, h2, h3, h4, h5, h6};

use super::MarkdownOptions;

/// Converts one fenced or indented Markdown code block into a code-block view.
///
/// Fenced blocks use only the first whitespace-delimited info-string token as
/// their displayed language and highlighter selector. Indented and empty-info
/// blocks remain unlabeled and use plain retained source lines.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside the code block.
/// * `kind` — Parsed fenced info string or indented-block marker.
/// * `options` — Presentation defaults applied to the parsed code block.
///
/// # Returns
///
/// An [`AnyView`] containing a [`CodeBlockView`](crate::CodeBlockView) that
/// retains the parsed source and language selection.
pub(super) fn parse_code_block<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    kind: CodeBlockKind<'a>,
    options: MarkdownOptions,
) -> AnyView {
    let mut source = String::new();
    for event in events.by_ref() {
        match event {
            Event::Text(text) | Event::Code(text) => source.push_str(&text),
            Event::SoftBreak | Event::HardBreak => source.push('\n'),
            Event::End(TagEnd::CodeBlock) => break,
            _ => {}
        }
    }

    let language = match kind {
        CodeBlockKind::Indented => None,
        CodeBlockKind::Fenced(info) => info.split_whitespace().next().map(str::to_owned),
    };
    let view = code_block(source)
        .line_numbers(options.line_numbers)
        .syntax_theme(options.syntax_theme);
    match language {
        Some(language) => view.language(language),
        None => view,
    }
    .into_view()
}

/// Creates the semantic heading matching a CommonMark heading level.
///
/// # Arguments
///
/// * `level` — Parsed CommonMark heading level.
/// * `content` — Owned unstyled heading content.
///
/// # Returns
///
/// A semantic H1 through H6 [`crate::HeadingView`].
pub(super) fn heading(level: HeadingLevel, content: Text<'static>) -> AnyView {
    match level {
        HeadingLevel::H1 => h1(content),
        HeadingLevel::H2 => h2(content),
        HeadingLevel::H3 => h3(content),
        HeadingLevel::H4 => h4(content),
        HeadingLevel::H5 => h5(content),
        HeadingLevel::H6 => h6(content),
    }
    .into_view()
}
