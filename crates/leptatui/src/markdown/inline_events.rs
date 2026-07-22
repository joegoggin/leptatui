//! Inline Markdown event parsing and linked rich-text conversion.

use pulldown_cmark::{Event, LinkType, Tag, TagEnd};
use ratatui::style::{Modifier, Style};

use crate::{AnyView, IntoView, RichText, paragraph};

use super::{inline::InlineText, navigation::MarkdownParseContext};

/// Converts accumulated direct inline content into a semantic paragraph.
pub(super) fn flush_inline_paragraph(inline: &mut InlineText, views: &mut Vec<AnyView>) {
    if inline.has_content() {
        views.push(paragraph(std::mem::take(inline).into_rich_text()).into_view());
    }
}

/// Collects styled inline content through a matching closing tag.
pub(super) fn parse_inline<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    end: TagEnd,
    context: &MarkdownParseContext<'_>,
) -> RichText {
    let mut content = InlineText::new();
    parse_inline_events(events, end, Style::new(), None, context, &mut content);
    content.into_rich_text()
}

/// Parses one inline event using inherited style and optional link ownership.
pub(super) fn parse_inline_event<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    event: Event<'a>,
    style: Style,
    active_link: Option<usize>,
    context: &MarkdownParseContext<'_>,
    content: &mut InlineText,
) {
    match event {
        Event::Text(text)
        | Event::InlineMath(text)
        | Event::DisplayMath(text)
        | Event::Html(text)
        | Event::InlineHtml(text) => content.push_text_for_link(&text, style, active_link),
        Event::Code(text) => {
            content.push_text_for_link(&text, style.add_modifier(Modifier::REVERSED), active_link)
        }
        Event::SoftBreak | Event::HardBreak => content.push_break(),
        Event::Start(Tag::Emphasis) => parse_inline_events(
            events,
            TagEnd::Emphasis,
            style.add_modifier(Modifier::ITALIC),
            active_link,
            context,
            content,
        ),
        Event::Start(Tag::Strong) => parse_inline_events(
            events,
            TagEnd::Strong,
            style.add_modifier(Modifier::BOLD),
            active_link,
            context,
            content,
        ),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            ..
        }) => parse_link(events, link_type, &dest_url, style, context, content),
        Event::Start(Tag::Image { dest_url, .. }) => {
            parse_image(events, &dest_url, style, active_link, context, content);
        }
        Event::Start(Tag::CodeBlock(_)) => skip_until(events, TagEnd::CodeBlock),
        Event::Start(Tag::HtmlBlock) => skip_until(events, TagEnd::HtmlBlock),
        Event::FootnoteReference(label) => {
            push_footnote_reference(content, &label, style, active_link);
        }
        Event::TaskListMarker(checked) => {
            push_task_list_marker(content, checked, style, active_link);
        }
        Event::Start(_) | Event::End(_) | Event::Rule => {}
    }
}

/// Parses inline events until their balanced closing tag.
fn parse_inline_events<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    end: TagEnd,
    style: Style,
    active_link: Option<usize>,
    context: &MarkdownParseContext<'_>,
    content: &mut InlineText,
) {
    while let Some(event) = events.next() {
        if event == Event::End(end) {
            break;
        }
        parse_inline_event(events, event, style, active_link, context, content);
    }
}

/// Parses an image into deterministic descriptive text without fetching it.
fn parse_image<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    destination: &str,
    style: Style,
    active_link: Option<usize>,
    context: &MarkdownParseContext<'_>,
    content: &mut InlineText,
) {
    let mut alt = InlineText::new();
    parse_inline_events(events, TagEnd::Image, style, None, context, &mut alt);
    let alt = alt.plain_text();
    let fallback = match (alt.is_empty(), destination.is_empty()) {
        (false, false) => format!("Image: {alt} ({destination})"),
        (false, true) => format!("Image: {alt}"),
        (true, false) => format!("Image: {destination}"),
        (true, true) => "Image".to_owned(),
    };
    content.push_text_for_link(&fallback, style, active_link);
}

/// Appends a readable footnote reference.
pub(super) fn push_footnote_reference(
    content: &mut InlineText,
    label: &str,
    style: Style,
    active_link: Option<usize>,
) {
    content.push_text_for_link(&format!("[^{label}]"), style, active_link);
}

/// Appends a readable task-list marker.
pub(super) fn push_task_list_marker(
    content: &mut InlineText,
    checked: bool,
    style: Style,
    active_link: Option<usize>,
) {
    content.push_text_for_link(if checked { "[x] " } else { "[ ] " }, style, active_link);
}

/// Parses a Markdown link into a focusable label range.
fn parse_link<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    link_type: LinkType,
    destination: &str,
    style: Style,
    context: &MarkdownParseContext<'_>,
    content: &mut InlineText,
) {
    let target = context.link_target(link_type, destination);
    let link = content.push_link(target);
    parse_inline_events(events, TagEnd::Link, style, Some(link), context, content);
    if !content.link_has_text(link) && !destination.is_empty() {
        content.push_text_for_link(destination, style, Some(link));
    }
}

/// Discards events through a balanced unsupported block's closing tag.
fn skip_until<'a>(events: &mut impl Iterator<Item = Event<'a>>, end: TagEnd) {
    for event in events.by_ref() {
        if event == Event::End(end) {
            break;
        }
    }
}
