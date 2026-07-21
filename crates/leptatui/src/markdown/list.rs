//! Markdown list conversion.

use pulldown_cmark::{Event, Tag, TagEnd};

use crate::{AnyView, IntoView, list_item, ordered_list, unordered_list};

use super::{MarkdownOptions, block::parse_blocks};

/// Parses a CommonMark ordered or unordered list.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned after the list opening tag.
/// * `start` — First ordered marker, or [`None`] for an unordered list.
/// * `options` — Code-block presentation defaults for nested list content.
///
/// # Returns
///
/// A semantic ordered or unordered list retaining item order and nesting.
pub(super) fn parse_list<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    start: Option<u64>,
    options: MarkdownOptions,
) -> AnyView {
    let mut items = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::Item) => {
                items.push(list_item(parse_blocks(events, Some(TagEnd::Item), options)));
            }
            Event::End(TagEnd::List(_)) => break,
            _ => {}
        }
    }

    match start {
        Some(start) => {
            let start = usize::try_from(start).unwrap_or(usize::MAX);
            ordered_list(items).start(start)
        }
        None => unordered_list(items),
    }
    .into_view()
}
