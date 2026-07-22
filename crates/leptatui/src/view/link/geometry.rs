//! Link-aware wrapping and rendered hit-test geometry.

use std::collections::VecDeque;

use ratatui::{style::Style, text::Text};
use unicode_width::UnicodeWidthStr;

use super::rich_text::InlineLink;
use crate::view::CellAlignment;

/// Wrapping behavior used by one rich-text renderer.
#[derive(Clone, Copy)]
pub(crate) enum RichTextWrapMode {
    /// Ratatui paragraph word wrapping with leading whitespace preserved.
    Word,
    /// Table-cell wrapping at individual grapheme boundaries.
    Grapheme,
}

/// One visual grapheme retained while computing inline-link geometry.
#[derive(Clone, Copy)]
struct LinkedVisualGrapheme {
    /// Embedded link index owning the grapheme.
    link: Option<usize>,
    /// Terminal-cell width of the grapheme.
    width: u16,
    /// Whether Ratatui treats the grapheme as wrappable whitespace.
    whitespace: bool,
}

/// One wrapped visual row containing link-aware graphemes.
struct LinkedVisualRow {
    /// Graphemes rendered on this row.
    graphemes: Vec<LinkedVisualGrapheme>,
    /// Total terminal-cell width of the row.
    width: u16,
}

/// One visual terminal segment occupied by an inline link.
#[derive(Clone, Copy)]
pub(super) struct LinkedVisualSegment {
    /// Source-order index of the embedded link occupying the segment.
    pub(super) link: usize,
    /// Zero-based wrapped visual row containing the segment.
    pub(super) row: u16,
    /// Inclusive terminal-cell offset where the segment begins.
    pub(super) start: u16,
    /// Exclusive terminal-cell offset where the segment ends.
    pub(super) end: u16,
    /// Total terminal-cell width of the wrapped visual row.
    pub(super) line_width: u16,
}

/// Returns link-bearing visual segments using the renderer's wrapping behavior.
///
/// # Arguments
///
/// * `text` — Rich text rendered by the semantic view.
/// * `links` — Inline link metadata associated with text spans.
/// * `width` — Width used to wrap the text.
/// * `wrap_mode` — Wrapping behavior used by the renderer.
///
/// # Returns
///
/// A [`Vec`] containing each visible linked segment in render order.
pub(super) fn linked_visual_segments(
    text: &Text<'static>,
    links: &[InlineLink],
    width: u16,
    wrap_mode: RichTextWrapMode,
) -> Vec<LinkedVisualSegment> {
    let mut segments = Vec::new();
    for (row, visual_row) in linked_visual_rows(text, links, width, wrap_mode)
        .into_iter()
        .enumerate()
    {
        let row = u16::try_from(row).unwrap_or(u16::MAX);
        let mut pending: Option<LinkedVisualSegment> = None;
        let mut column = 0u16;
        for grapheme in visual_row.graphemes {
            if grapheme.width == 0 {
                continue;
            }

            if let Some(link) = grapheme.link {
                if let Some(segment) = pending.as_mut()
                    && segment.link == link
                    && segment.end == column
                {
                    segment.end = column.saturating_add(grapheme.width);
                } else {
                    finish_linked_segment(&mut segments, &mut pending);
                    pending = Some(LinkedVisualSegment {
                        link,
                        row,
                        start: column,
                        end: column.saturating_add(grapheme.width),
                        line_width: visual_row.width,
                    });
                }
            } else {
                finish_linked_segment(&mut segments, &mut pending);
            }

            column = column.saturating_add(grapheme.width);
        }
        finish_linked_segment(&mut segments, &mut pending);
    }

    segments
}

/// Returns wrapped visual rows while retaining each grapheme's link index.
///
/// # Arguments
///
/// * `text` — Rich text rendered by the semantic view.
/// * `links` — Inline link metadata associated with text spans.
/// * `width` — Width used to wrap the text.
/// * `wrap_mode` — Wrapping behavior used by the renderer.
///
/// # Returns
///
/// A [`Vec`] containing link-aware visual rows in render order.
fn linked_visual_rows(
    text: &Text<'static>,
    links: &[InlineLink],
    width: u16,
    wrap_mode: RichTextWrapMode,
) -> Vec<LinkedVisualRow> {
    let link_spans = links
        .iter()
        .enumerate()
        .flat_map(|(link, inline_link)| {
            inline_link
                .spans
                .iter()
                .copied()
                .map(move |span| (span, link))
        })
        .collect::<Vec<_>>();
    let mut rows = Vec::new();

    for (line_index, line) in text.lines.iter().enumerate() {
        let graphemes = line
            .spans
            .iter()
            .enumerate()
            .flat_map(|(span_index, span)| {
                let link = link_spans
                    .iter()
                    .find(|(position, _)| {
                        position.line == line_index && position.span == span_index
                    })
                    .map(|(_, link)| *link);
                span.styled_graphemes(Style::new())
                    .map(move |grapheme| LinkedVisualGrapheme {
                        link,
                        width: u16::try_from(UnicodeWidthStr::width(grapheme.symbol))
                            .unwrap_or(u16::MAX),
                        whitespace: grapheme.is_whitespace(),
                    })
            })
            .collect::<Vec<_>>();

        rows.extend(match wrap_mode {
            RichTextWrapMode::Word => word_wrapped_visual_rows(graphemes, width),
            RichTextWrapMode::Grapheme => grapheme_wrapped_visual_rows(graphemes, width),
        });
    }

    rows
}

/// Wraps link-aware graphemes like Ratatui's `WordWrapper` with `trim: false`.
///
/// # Arguments
///
/// * `graphemes` — Link-aware graphemes from one logical text line.
/// * `width` — Maximum terminal-cell width of each visual row.
///
/// # Returns
///
/// A [`Vec`] containing wrapped visual rows in render order.
fn word_wrapped_visual_rows(
    graphemes: Vec<LinkedVisualGrapheme>,
    width: u16,
) -> Vec<LinkedVisualRow> {
    if width == 0 {
        return vec![LinkedVisualRow {
            graphemes: Vec::new(),
            width: 0,
        }];
    }

    let mut rows = Vec::new();
    let mut pending_line = Vec::new();
    let mut pending_word = Vec::new();
    let mut pending_whitespace: VecDeque<LinkedVisualGrapheme> = VecDeque::new();
    let mut line_width = 0u16;
    let mut word_width = 0u16;
    let mut whitespace_width = 0u16;
    let mut non_whitespace_previous = false;

    for grapheme in graphemes {
        if grapheme.width > width {
            continue;
        }

        let word_found = non_whitespace_previous && grapheme.whitespace;
        let untrimmed_overflow = pending_line.is_empty()
            && word_width
                .saturating_add(whitespace_width)
                .saturating_add(grapheme.width)
                > width;

        if word_found || untrimmed_overflow {
            pending_line.extend(pending_whitespace.drain(..));
            line_width = line_width.saturating_add(whitespace_width);
            pending_line.append(&mut pending_word);
            line_width = line_width.saturating_add(word_width);
            whitespace_width = 0;
            word_width = 0;
        }

        let line_full = line_width >= width;
        let pending_word_overflow = grapheme.width > 0
            && line_width
                .saturating_add(whitespace_width)
                .saturating_add(word_width)
                >= width;

        if line_full || pending_word_overflow {
            let mut remaining_width = width.saturating_sub(line_width);
            rows.push(LinkedVisualRow {
                graphemes: std::mem::take(&mut pending_line),
                width: line_width,
            });
            line_width = 0;

            while let Some(whitespace) = pending_whitespace.front() {
                if whitespace.width > remaining_width {
                    break;
                }
                whitespace_width = whitespace_width.saturating_sub(whitespace.width);
                remaining_width = remaining_width.saturating_sub(whitespace.width);
                pending_whitespace.pop_front();
            }

            if grapheme.whitespace && pending_whitespace.is_empty() {
                continue;
            }
        }

        if grapheme.whitespace {
            whitespace_width = whitespace_width.saturating_add(grapheme.width);
            pending_whitespace.push_back(grapheme);
        } else {
            word_width = word_width.saturating_add(grapheme.width);
            pending_word.push(grapheme);
        }

        non_whitespace_previous = !grapheme.whitespace;
    }

    pending_line.extend(pending_whitespace);
    line_width = line_width.saturating_add(whitespace_width);
    pending_line.append(&mut pending_word);
    line_width = line_width.saturating_add(word_width);
    if !pending_line.is_empty() {
        rows.push(LinkedVisualRow {
            graphemes: pending_line,
            width: line_width,
        });
    }
    if rows.is_empty() {
        rows.push(LinkedVisualRow {
            graphemes: Vec::new(),
            width: 0,
        });
    }

    rows
}

/// Wraps link-aware graphemes at individual grapheme boundaries.
///
/// # Arguments
///
/// * `graphemes` — Link-aware graphemes from one logical text line.
/// * `width` — Maximum terminal-cell width of each visual row.
///
/// # Returns
///
/// A [`Vec`] containing grapheme-wrapped visual rows in render order.
fn grapheme_wrapped_visual_rows(
    graphemes: Vec<LinkedVisualGrapheme>,
    width: u16,
) -> Vec<LinkedVisualRow> {
    if width == 0 {
        return vec![LinkedVisualRow {
            graphemes: Vec::new(),
            width: 0,
        }];
    }

    let mut rows = Vec::new();
    let mut current = Vec::new();
    let mut current_width = 0u16;
    for grapheme in graphemes {
        if grapheme.width > width {
            if !current.is_empty() {
                rows.push(LinkedVisualRow {
                    graphemes: std::mem::take(&mut current),
                    width: current_width,
                });
                current_width = 0;
            }
            continue;
        }
        if current_width.saturating_add(grapheme.width) > width && !current.is_empty() {
            rows.push(LinkedVisualRow {
                graphemes: std::mem::take(&mut current),
                width: current_width,
            });
            current_width = 0;
        }
        current_width = current_width.saturating_add(grapheme.width);
        current.push(grapheme);
    }
    rows.push(LinkedVisualRow {
        graphemes: current,
        width: current_width,
    });
    rows
}

/// Completes the pending inline-link segment.
///
/// # Arguments
///
/// * `segments` — Completed segment collection receiving the pending value.
/// * `pending` — Optional segment to remove and append when present.
fn finish_linked_segment(
    segments: &mut Vec<LinkedVisualSegment>,
    pending: &mut Option<LinkedVisualSegment>,
) {
    if let Some(segment) = pending.take() {
        segments.push(segment);
    }
}

/// Returns left padding for a line inside an aligned rich-text area.
///
/// # Arguments
///
/// * `line_width` — Rendered terminal-cell width of the line.
/// * `width` — Total terminal-cell width available to the line.
/// * `alignment` — Horizontal alignment applied within the available width.
///
/// # Returns
///
/// A terminal-cell offset from the area's left edge.
pub(super) fn aligned_line_offset(line_width: u16, width: u16, alignment: CellAlignment) -> u16 {
    match alignment {
        CellAlignment::Left => 0,
        CellAlignment::Center => width.saturating_sub(line_width) / 2,
        CellAlignment::Right => width.saturating_sub(line_width),
    }
}
