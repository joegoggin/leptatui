//! Inline Markdown event parsing and fallback text conversion.

use pulldown_cmark::{Event, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::Text,
};

use crate::{AnyView, IntoView, paragraph};

use super::inline::InlineText;

/// Converts accumulated direct inline content into a semantic paragraph.
///
/// # Arguments
///
/// * `inline` — Pending styled inline text.
/// * `views` — Destination block sequence for the resulting paragraph.
pub(super) fn flush_inline_paragraph(inline: &mut InlineText, views: &mut Vec<AnyView>) {
    if inline.has_content() {
        views.push(paragraph(std::mem::take(inline).into_text()).into_view());
    }
}

/// Collects styled inline content through a matching closing tag.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned after an opening tag.
/// * `end` — Closing tag that terminates the inline content.
///
/// # Returns
///
/// An owned [`Text`] containing styled spans and retained line breaks.
pub(super) fn parse_inline<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    end: TagEnd,
) -> Text<'static> {
    let mut content = InlineText::new();
    parse_inline_events(events, end, Style::new(), &mut content);
    content.into_text()
}

/// Parses inline events into an accumulator using the inherited span style.
///
/// Recursive calls combine nested emphasis, strong, and link modifiers while
/// retaining a single output accumulator.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside inline content.
/// * `end` — Closing tag that terminates the current inline scope.
/// * `style` — Span style inherited by text in the current scope.
/// * `content` — Destination rich-text accumulator.
fn parse_inline_events<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    end: TagEnd,
    style: Style,
    content: &mut InlineText,
) {
    while let Some(event) = events.next() {
        if event == Event::End(end) {
            break;
        }
        parse_inline_event(events, event, style, content);
    }
}

/// Parses one inline event into an accumulator using an inherited span style.
///
/// Nested inline tags recursively consume their balanced event scopes. Block
/// tags encountered inside inline content are skipped through their closing
/// tags, while unsupported structural events are ignored.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned after the current event.
/// * `event` — Inline or fallback event to convert.
/// * `style` — Span style inherited by the event's content.
/// * `content` — Destination rich-text accumulator.
pub(super) fn parse_inline_event<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    event: Event<'a>,
    style: Style,
    content: &mut InlineText,
) {
    match event {
        Event::Text(text)
        | Event::InlineMath(text)
        | Event::DisplayMath(text)
        | Event::Html(text)
        | Event::InlineHtml(text) => content.push_text(&text, style),
        Event::Code(text) => {
            content.push_text(&text, style.add_modifier(Modifier::REVERSED));
        }
        Event::SoftBreak | Event::HardBreak => content.push_break(),
        Event::Start(Tag::Emphasis) => parse_inline_events(
            events,
            TagEnd::Emphasis,
            style.add_modifier(Modifier::ITALIC),
            content,
        ),
        Event::Start(Tag::Strong) => parse_inline_events(
            events,
            TagEnd::Strong,
            style.add_modifier(Modifier::BOLD),
            content,
        ),
        Event::Start(Tag::Link { dest_url, .. }) => {
            parse_link(events, &dest_url, style, content);
        }
        Event::Start(Tag::Image { dest_url, .. }) => {
            parse_image(events, &dest_url, style, content);
        }
        Event::Start(Tag::CodeBlock(_)) => skip_until(events, TagEnd::CodeBlock),
        Event::Start(Tag::HtmlBlock) => skip_until(events, TagEnd::HtmlBlock),
        Event::FootnoteReference(label) => {
            push_footnote_reference(content, &label, style);
        }
        Event::TaskListMarker(checked) => {
            push_task_list_marker(content, checked, style);
        }
        Event::Start(_) | Event::End(_) | Event::Rule => {}
    }
}

/// Parses a Markdown image into deterministic descriptive terminal text.
///
/// No image view is constructed, so remote and local destinations are never
/// opened. Parsed alt content is flattened to visible text and combined with
/// the source using a stable `Image:` label.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside an image.
/// * `destination` — Parsed image source URL or path.
/// * `style` — Span style inherited from the surrounding inline scope.
/// * `content` — Destination rich-text accumulator.
fn parse_image<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    destination: &str,
    style: Style,
    content: &mut InlineText,
) {
    let mut alt = InlineText::new();
    parse_inline_events(events, TagEnd::Image, style, &mut alt);
    let alt = alt.plain_text();
    let fallback = match (alt.is_empty(), destination.is_empty()) {
        (false, false) => format!("Image: {alt} ({destination})"),
        (false, true) => format!("Image: {alt}"),
        (true, false) => format!("Image: {destination}"),
        (true, true) => "Image".to_owned(),
    };
    content.push_text(&fallback, style);
}

/// Appends a readable footnote reference when such an event is enabled.
///
/// # Arguments
///
/// * `content` — Destination rich-text accumulator.
/// * `label` — Parsed footnote label.
/// * `style` — Span style inherited from the surrounding inline scope.
pub(super) fn push_footnote_reference(content: &mut InlineText, label: &str, style: Style) {
    content.push_text(&format!("[^{label}]"), style);
}

/// Appends a readable checkbox marker when task-list events are enabled.
///
/// # Arguments
///
/// * `content` — Destination rich-text accumulator.
/// * `checked` — Whether the parsed task marker is checked.
/// * `style` — Span style inherited from the surrounding inline scope.
pub(super) fn push_task_list_marker(content: &mut InlineText, checked: bool, style: Style) {
    content.push_text(if checked { "[x] " } else { "[ ] " }, style);
}

/// Parses a Markdown link and appends a terminal-readable destination.
///
/// Link labels are underlined and retain surrounding emphasis or strong
/// modifiers. Non-empty destinations are appended only when the visible label
/// does not already expose the exact URL or an email address without its
/// `mailto:` scheme.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside a link.
/// * `destination` — Parsed link destination.
/// * `style` — Span style inherited from the surrounding inline scope.
/// * `content` — Destination rich-text accumulator.
fn parse_link<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    destination: &str,
    style: Style,
    content: &mut InlineText,
) {
    let link_style = style.add_modifier(Modifier::UNDERLINED);
    let mut link = InlineText::new();
    parse_inline_events(events, TagEnd::Link, link_style, &mut link);

    let label = link.plain_text();
    if !destination.is_empty() && !link_destination_is_visible(&label, destination) {
        if label.is_empty() {
            link.push_text(destination, link_style);
        } else {
            link.push_text(&format!(" ({destination})"), link_style);
        }
    }

    content.append(link);
}

/// Returns whether a link label already displays its destination.
///
/// Email autolinks omit the `mailto:` scheme from their visible label, so that
/// prefix is ignored for the comparison.
///
/// # Arguments
///
/// * `label` — Visible unstyled link-label content.
/// * `destination` — Parsed link destination.
///
/// # Returns
///
/// A boolean indicating whether appending the destination would duplicate it.
fn link_destination_is_visible(label: &str, destination: &str) -> bool {
    label == destination || destination.strip_prefix("mailto:") == Some(label)
}

/// Discards events through a balanced unsupported block's closing tag.
fn skip_until<'a>(events: &mut impl Iterator<Item = Event<'a>>, end: TagEnd) {
    for event in events.by_ref() {
        if event == Event::End(end) {
            break;
        }
    }
}
