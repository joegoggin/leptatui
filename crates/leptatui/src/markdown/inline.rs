//! Owned rich-text accumulation for inline Markdown content.

use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};

/// Accumulates owned rich-text lines while parsing inline Markdown events.
///
/// Adjacent content with identical styles is coalesced so parser event
/// boundaries do not leak into the semantic view tree. An explicit content
/// flag distinguishes untouched tight-list content from a deliberately empty
/// line created by a Markdown break.
pub(super) struct InlineText {
    /// Styled spans grouped by logical output line.
    lines: Vec<Vec<Span<'static>>>,
    /// Whether the parser emitted text or a line break into this accumulator.
    has_content: bool,
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

    /// Appends styled text while preserving embedded logical line boundaries.
    ///
    /// Adjacent spans using the same style are merged on each line.
    ///
    /// # Arguments
    ///
    /// * `content` — Text emitted by the Markdown parser.
    /// * `style` — Ratatui style applied to the appended text.
    pub(super) fn push_text(&mut self, content: &str, style: Style) {
        if content.is_empty() {
            return;
        }

        self.has_content = true;
        let mut parts = content.split('\n').peekable();
        while let Some(part) = parts.next() {
            if !part.is_empty() {
                self.push_span(part, style);
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

    /// Appends another rich-text accumulator without losing its first line.
    ///
    /// # Arguments
    ///
    /// * `other` — Parsed inline content to append at the current position.
    pub(super) fn append(&mut self, other: Self) {
        if !other.has_content {
            return;
        }

        self.has_content = true;
        for (index, line) in other.lines.into_iter().enumerate() {
            if index > 0 {
                self.lines.push(Vec::new());
            }
            for span in line {
                self.push_span(&span.content, span.style);
            }
        }
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
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Converts accumulated lines into owned Ratatui rich text.
    ///
    /// # Returns
    ///
    /// A [`Text`] containing every logical line and styled span.
    pub(super) fn into_text(self) -> Text<'static> {
        Text::from(self.lines.into_iter().map(Line::from).collect::<Vec<_>>())
    }

    /// Appends one span or merges it with the matching previous span.
    ///
    /// # Arguments
    ///
    /// * `content` — Non-empty text for the span.
    /// * `style` — Ratatui style applied to the span.
    fn push_span(&mut self, content: &str, style: Style) {
        let line = self
            .lines
            .last_mut()
            .expect("inline text always retains one logical line");
        if let Some(span) = line.last_mut().filter(|span| span.style == style) {
            span.content.to_mut().push_str(content);
        } else {
            line.push(Span::styled(content.to_owned(), style));
        }
    }
}

impl Default for InlineText {
    /// Creates the default empty rich-text accumulator.
    ///
    /// # Returns
    ///
    /// An empty [`InlineText`] with one logical line.
    fn default() -> Self {
        Self::new()
    }
}
