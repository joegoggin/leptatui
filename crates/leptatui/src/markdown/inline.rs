//! Owned rich-text accumulation for inline Markdown content.

use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};

use crate::view::{InlineLink, LinkTarget, LinkedSpan, RichText};

/// Accumulates owned rich-text lines and their optional link ranges.
pub(super) struct InlineText {
    lines: Vec<Vec<ParsedInlineSpan>>,
    links: Vec<LinkTarget>,
    has_content: bool,
}

/// One parsed inline span and its optional owning link index.
struct ParsedInlineSpan {
    span: Span<'static>,
    link: Option<usize>,
}

impl InlineText {
    /// Creates an empty rich-text accumulator with one logical line.
    pub(super) fn new() -> Self {
        Self {
            lines: vec![Vec::new()],
            links: Vec::new(),
            has_content: false,
        }
    }

    /// Returns whether any text or line break has been accumulated.
    pub(super) fn has_content(&self) -> bool {
        self.has_content
    }

    /// Appends unlinked styled text.
    pub(super) fn push_text(&mut self, content: &str, style: Style) {
        self.push_text_for_link(content, style, None);
    }

    /// Appends styled text associated with an optional link.
    pub(super) fn push_text_for_link(&mut self, content: &str, style: Style, link: Option<usize>) {
        if content.is_empty() {
            return;
        }

        self.has_content = true;
        let mut parts = content.split('\n').peekable();
        while let Some(part) = parts.next() {
            if !part.is_empty() {
                self.push_span(part, style, link);
            }
            if parts.peek().is_some() {
                self.lines.push(Vec::new());
            }
        }
    }

    /// Appends one explicit logical line break.
    pub(super) fn push_break(&mut self) {
        self.has_content = true;
        self.lines.push(Vec::new());
    }

    /// Registers a link target and returns its source-order index.
    pub(super) fn push_link(&mut self, target: LinkTarget) -> usize {
        let index = self.links.len();
        self.links.push(target);
        index
    }

    /// Returns whether visible text has been assigned to a link index.
    pub(super) fn link_has_text(&self, link: usize) -> bool {
        self.lines
            .iter()
            .flatten()
            .any(|span| span.link == Some(link))
    }

    /// Returns the visible unstyled content represented by the accumulator.
    pub(super) fn plain_text(&self) -> String {
        self.lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|span| span.span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Converts accumulated lines into rich text with focusable links.
    pub(super) fn into_rich_text(self) -> RichText {
        let mut linked_spans = vec![Vec::new(); self.links.len()];
        let lines = self
            .lines
            .into_iter()
            .enumerate()
            .map(|(line_index, line)| {
                Line::from(
                    line.into_iter()
                        .enumerate()
                        .map(|(span_index, parsed)| {
                            if let Some(link_index) = parsed.link
                                && let Some(positions) = linked_spans.get_mut(link_index)
                            {
                                positions.push(LinkedSpan {
                                    line: line_index,
                                    span: span_index,
                                });
                            }
                            parsed.span
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        let links = self
            .links
            .into_iter()
            .zip(linked_spans)
            .map(|(target, spans)| InlineLink::new(target, spans))
            .collect();
        RichText::from_parts(Text::from(lines), links)
    }

    /// Appends one span or merges it with a matching previous span.
    fn push_span(&mut self, content: &str, style: Style, link: Option<usize>) {
        let line = self
            .lines
            .last_mut()
            .expect("inline text always retains one logical line");
        if let Some(span) = line
            .last_mut()
            .filter(|span| span.span.style == style && span.link == link)
        {
            span.span.content.to_mut().push_str(content);
        } else {
            line.push(ParsedInlineSpan {
                span: Span::styled(content.to_owned(), style),
                link,
            });
        }
    }
}

impl Default for InlineText {
    fn default() -> Self {
        Self::new()
    }
}
