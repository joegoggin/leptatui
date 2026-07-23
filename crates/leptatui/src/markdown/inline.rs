//! Owned rich-text accumulation for inline Markdown content.

use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};

use crate::view::{InlineLink, LinkTarget, LinkedSpan, RichText};

/// Accumulates owned rich-text lines and their optional link ranges.
pub(super) struct InlineText {
    /// Parsed spans grouped by logical output line.
    lines: Vec<Vec<ParsedInlineSpan>>,
    /// Link targets retained in Markdown source order.
    links: Vec<LinkTarget>,
    /// Whether the parser emitted text or a line break.
    has_content: bool,
}

/// One parsed inline span and its optional owning link index.
struct ParsedInlineSpan {
    /// Styled Ratatui span containing the visible text.
    span: Span<'static>,
    /// Source-order index of the link containing this span, if any.
    link: Option<usize>,
}

impl InlineText {
    /// Creates an empty rich-text accumulator with one logical line.
    ///
    /// # Returns
    ///
    /// An empty [`InlineText`] ready to receive styled content.
    pub(super) fn new() -> Self {
        Self {
            lines: vec![Vec::new()],
            links: Vec::new(),
            has_content: false,
        }
    }

    /// Returns whether any text or line break has been accumulated.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the accumulator contains parsed content.
    pub(super) fn has_content(&self) -> bool {
        self.has_content
    }

    /// Appends unlinked styled text.
    ///
    /// # Arguments
    ///
    /// * `content` — Text emitted by the Markdown parser.
    /// * `style` — Ratatui style applied to the appended text.
    pub(super) fn push_text(&mut self, content: &str, style: Style) {
        self.push_text_for_link(content, style, None);
    }

    /// Appends styled text associated with an optional link.
    ///
    /// # Arguments
    ///
    /// * `content` — Text emitted by the Markdown parser.
    /// * `style` — Ratatui style applied to the appended text.
    /// * `link` — Source-order link index associated with the text, if any.
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
    ///
    /// # Arguments
    ///
    /// * `target` — Destination associated with the parsed Markdown link.
    ///
    /// # Returns
    ///
    /// A zero-based source-order index for associating spans with the link.
    pub(super) fn push_link(&mut self, target: LinkTarget) -> usize {
        let index = self.links.len();
        self.links.push(target);
        index
    }

    /// Returns whether visible text has been assigned to a link index.
    ///
    /// # Arguments
    ///
    /// * `link` — Source-order link index to inspect.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether any parsed span belongs to the link.
    pub(super) fn link_has_text(&self, link: usize) -> bool {
        self.lines
            .iter()
            .flatten()
            .any(|span| span.link == Some(link))
    }

    /// Returns the visible unstyled content represented by the accumulator.
    ///
    /// # Returns
    ///
    /// A [`String`] containing span content separated by logical newlines.
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
    ///
    /// # Returns
    ///
    /// A [`RichText`] value containing the styled lines and linked span ranges.
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
    ///
    /// # Arguments
    ///
    /// * `content` — Non-empty visible text for the span.
    /// * `style` — Ratatui style applied to the span.
    /// * `link` — Source-order link index associated with the span, if any.
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
    /// Creates the default empty inline-text accumulator.
    ///
    /// # Returns
    ///
    /// An empty [`InlineText`] ready to receive styled content.
    fn default() -> Self {
        Self::new()
    }
}
