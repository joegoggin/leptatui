//! Block-level Markdown traversal and fallback construction.

use pulldown_cmark::{Event, Tag, TagEnd};
use ratatui::style::Style;

use crate::{AnyView, Borders, IntoView, TuiSpacing, TuiStyle, block, column, paragraph};

use super::{
    MarkdownOptions,
    code::{heading, parse_code_block},
    inline::InlineText,
    inline_events::{
        flush_inline_paragraph, parse_inline, parse_inline_event, push_footnote_reference,
        push_task_list_marker,
    },
    list::parse_list,
    navigation::MarkdownParseContext,
    table::parse_table,
};

/// Parses semantic blocks until the requested closing tag or end of input.
///
/// Direct inline events are collected into a paragraph because pulldown-cmark
/// omits paragraph tags inside tight list items.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside a block.
/// * `end` — Optional closing tag that terminates the current block sequence.
/// * `options` — Code-block presentation defaults for the document.
///
/// # Returns
///
/// A [`Vec`] containing converted semantic views with empty paragraphs between
/// blocks in event order.
pub(super) fn parse_blocks<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    end: Option<TagEnd>,
    options: MarkdownOptions,
    context: &mut MarkdownParseContext<'_>,
) -> Vec<AnyView> {
    let mut views = Vec::new();
    let mut inline = InlineText::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(paragraph(parse_inline(events, TagEnd::Paragraph, context)).into_view());
            }
            Event::Start(Tag::Heading { level, .. }) => {
                flush_inline_paragraph(&mut inline, &mut views);
                let content = parse_inline(events, TagEnd::Heading(level), context);
                let slug = context
                    .has_source_path()
                    .then(|| context.heading_slug(&content));
                let mut view = heading(level, content);
                if let Some(slug) = slug {
                    view = view.with_id(slug);
                }
                views.push(view);
            }
            Event::Start(Tag::List(start)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_list(events, start, options, context));
            }
            Event::Start(Tag::Table(alignments)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_table(events, &alignments, context));
            }
            Event::Start(Tag::BlockQuote(kind)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(block_quote(parse_blocks(
                    events,
                    Some(TagEnd::BlockQuote(kind)),
                    options,
                    context,
                )));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_code_block(events, kind, options));
            }
            Event::Start(Tag::HtmlBlock) => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(parse_html_block(events));
            }
            Event::Rule => {
                flush_inline_paragraph(&mut inline, &mut views);
                views.push(thematic_break());
            }
            Event::End(tag) if Some(tag) == end => {
                flush_inline_paragraph(&mut inline, &mut views);
                break;
            }
            event => parse_inline_event(events, event, Style::new(), None, context, &mut inline),
        }
    }

    flush_inline_paragraph(&mut inline, &mut views);
    separate_blocks(views)
}

/// Inserts one empty terminal row between Markdown blocks.
///
/// CommonMark blank lines delimit blocks without producing parser events. An
/// empty semantic paragraph restores that document spacing while keeping the
/// surrounding column's existing measurement and scrolling behavior.
///
/// # Arguments
///
/// * `blocks` — Parsed Markdown blocks in source order.
///
/// # Returns
///
/// A [`Vec`] containing the blocks separated by empty paragraphs.
pub(super) fn separate_blocks(blocks: Vec<AnyView>) -> Vec<AnyView> {
    let mut separated = Vec::with_capacity(blocks.len().saturating_mul(2).saturating_sub(1));

    for block in blocks {
        if !separated.is_empty() {
            separated.push(paragraph("").into_view());
        }
        separated.push(block);
    }

    separated
}

/// Wraps parsed quote children in a visible, wrap-aware terminal prefix.
///
/// A left border marks every rendered quote row, including wrapped content.
/// One cell of padding separates the marker from the content, and nested quote
/// blocks naturally stack their prefixes.
///
/// # Arguments
///
/// * `children` — Semantic and fallback views parsed inside the blockquote.
///
/// # Returns
///
/// A left-bordered [`crate::BlockView`] containing the quote children.
pub(super) fn block_quote(children: Vec<AnyView>) -> AnyView {
    block(column(children))
        .with_inline_style(
            TuiStyle::new()
                .borders(Borders::LEFT)
                .padding(TuiSpacing::new(1, 0, 0, 0)),
        )
        .into_view()
}

/// Creates a width-responsive horizontal terminal rule.
///
/// # Returns
///
/// A one-row [`crate::BlockView`] whose top border fills the available width.
pub(super) fn thematic_break() -> AnyView {
    block(column(Vec::<AnyView>::new()))
        .with_inline_style(TuiStyle::new().borders(Borders::TOP))
        .into_view()
}

/// Collects one raw HTML block as literal terminal text.
///
/// Pulldown-cmark includes source line endings in block HTML events. Feeding
/// those payloads through [`InlineText`] retains the exact logical line shape
/// without interpreting tags or entities inside the raw block.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside an HTML block.
///
/// # Returns
///
/// A semantic paragraph containing the literal HTML block payload.
fn parse_html_block<'a>(events: &mut impl Iterator<Item = Event<'a>>) -> AnyView {
    let mut content = InlineText::new();

    for event in events.by_ref() {
        match event {
            Event::End(TagEnd::HtmlBlock) => break,
            Event::Text(text)
            | Event::Code(text)
            | Event::InlineMath(text)
            | Event::DisplayMath(text)
            | Event::Html(text)
            | Event::InlineHtml(text) => content.push_text(&text, Style::new()),
            Event::FootnoteReference(label) => {
                push_footnote_reference(&mut content, &label, Style::new(), None);
            }
            Event::SoftBreak | Event::HardBreak => content.push_break(),
            Event::TaskListMarker(checked) => {
                push_task_list_marker(&mut content, checked, Style::new(), None);
            }
            Event::Start(_) | Event::End(_) | Event::Rule => {}
        }
    }

    paragraph(content.into_rich_text()).into_view()
}
