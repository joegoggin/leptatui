//! Recovering markup context and source ranges for tag completion edits.
//!
//! Rust tokenization keeps comments, literals, and embedded expression groups out
//! of markup completion even when an opening or closing tag is unfinished.

use rustc_lexer::TokenKind;
use std::ops::Range;

/// One significant Rust token with byte offsets in the original document.
#[derive(Clone)]
struct Token {
    /// Lexical category before any markup interpretation.
    kind: TokenKind,
    /// Original byte range.
    range: Range<usize>,
}

/// A markup tag name or bare component prefix at the completion cursor.
#[derive(Clone)]
pub(super) struct Context {
    /// Name to replace, including any suffix after the cursor.
    pub(super) name: Range<usize>,
    /// Existing opening angle bracket, if one has been typed.
    pub(super) opening: Option<usize>,
    /// Whether the name belongs to a closing tag.
    pub(super) closing: bool,
    /// End of the opening header to replace, including any closing delimiter.
    pub(super) end: usize,
    /// Existing attributes, excluding the final slash or angle bracket.
    pub(super) attributes: Range<usize>,
    /// End of the containing macro body.
    body_end: usize,
}

impl Context {
    /// Finds a tag completion position within the innermost view macro.
    ///
    /// # Arguments
    ///
    /// * `text` — Current document, including unsaved edits.
    /// * `cursor` — UTF-8 byte offset of the completion cursor.
    ///
    /// # Returns
    ///
    /// An optional markup context, excluding Rust expressions and prop values.
    pub(super) fn find(text: &str, cursor: usize) -> Option<Self> {
        let mut tokens = Vec::new();
        let mut offset = 0;
        for token in rustc_lexer::tokenize(text) {
            let range = offset..offset + token.len;
            offset = range.end;
            match token.kind {
                TokenKind::LineComment | TokenKind::BlockComment { .. }
                    if range.start <= cursor && cursor <= range.end =>
                {
                    return None;
                }
                TokenKind::Literal { .. } if range.start < cursor && cursor < range.end => {
                    return None;
                }
                _ => {}
            }
            if !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment { .. }
            ) {
                tokens.push(Token {
                    kind: token.kind,
                    range,
                });
            }
        }
        let pairs = pairs(&tokens);
        let (start, end) = tokens
            .iter()
            .enumerate()
            .filter_map(|(index, token)| {
                if index < 2
                    || !opens(token.kind)
                    || tokens[index - 1].kind != TokenKind::Not
                    || &text[tokens[index - 2].range.clone()] != "view"
                {
                    return None;
                }
                let end = pairs[index].unwrap_or(tokens.len());
                let end_offset = tokens
                    .get(end)
                    .map_or(text.len(), |token| token.range.start);
                (token.range.end <= cursor && cursor <= end_offset).then_some((index + 1, end))
            })
            .next_back()?;
        let body_end = tokens
            .get(end)
            .map_or(text.len(), |token| token.range.start);
        let mut index = start;
        while index < end {
            let token = &tokens[index];
            if opens(token.kind) {
                let next = pairs[index].unwrap_or(end);
                if token.range.start < cursor
                    && cursor <= tokens.get(next).map_or(body_end, |token| token.range.start)
                {
                    return None;
                }
                index = next + 1;
                continue;
            }
            if token.kind == TokenKind::Lt {
                let opening = token.range.start;
                let mut name_index = index + 1;
                let closing = tokens
                    .get(name_index)
                    .is_some_and(|token| token.kind == TokenKind::Slash);
                if closing {
                    name_index += 1;
                }
                let name = tokens
                    .get(name_index)
                    .filter(|token| matches!(token.kind, TokenKind::Ident | TokenKind::RawIdent));
                let name_range = name.map_or(cursor..cursor, |token| token.range.clone());
                let mut header = name_index + usize::from(name.is_some());
                while header < end {
                    if matches!(tokens[header].kind, TokenKind::Gt | TokenKind::Lt) {
                        break;
                    }
                    if opens(tokens[header].kind) {
                        header = pairs[header].unwrap_or(end);
                    }
                    header += 1;
                }
                let delimiter = tokens
                    .get(header)
                    .filter(|token| token.kind == TokenKind::Gt);
                let last = if delimiter.is_some() {
                    header
                } else {
                    header.min(end).saturating_sub(1)
                };
                let slash = delimiter.is_some()
                    && header > name_index
                    && tokens[header - 1].kind == TokenKind::Slash;
                let attr_end = if slash {
                    tokens[header - 1].range.start
                } else if let Some(token) = delimiter {
                    token.range.start
                } else {
                    tokens
                        .get(last)
                        .map_or(name_range.end, |token| token.range.end)
                        .max(name_range.end)
                };
                let header_end = delimiter.map_or(attr_end, |token| token.range.end);
                if token.range.end <= cursor && cursor <= name_range.end {
                    return Some(Self {
                        name: name_range.clone(),
                        opening: Some(opening),
                        closing,
                        end: header_end,
                        attributes: name_range.end..attr_end,
                        body_end,
                    });
                }
                if cursor > name_range.end && cursor <= header_end {
                    return None;
                }
                index = header + usize::from(delimiter.is_some());
                continue;
            }
            if matches!(token.kind, TokenKind::Ident | TokenKind::RawIdent)
                && token.range.start <= cursor
                && cursor <= token.range.end
            {
                return Some(Self {
                    name: token.range.clone(),
                    opening: None,
                    closing: false,
                    end: token.range.end,
                    attributes: token.range.end..token.range.end,
                    body_end,
                });
            }
            index += 1;
        }
        Some(Self {
            name: cursor..cursor,
            opening: None,
            closing: false,
            end: cursor,
            attributes: cursor..cursor,
            body_end,
        })
    }

    /// Finds an existing closing tag without crossing an enclosing parent's close.
    ///
    /// # Arguments
    ///
    /// * `text` — Original source document.
    /// * `selected` — Completed component name, including any import alias.
    ///
    /// # Returns
    ///
    /// An optional byte range for the matching closing name.
    pub(super) fn matching_close(&self, text: &str, selected: &str) -> Option<Range<usize>> {
        self.opening?;
        if !text[..self.end].ends_with('>') || text[..self.end].ends_with("/>") {
            return None;
        }
        let original = &text[self.name.clone()];
        let mut offset = self.end;
        let tokens: Vec<_> = rustc_lexer::tokenize(&text[self.end..self.body_end])
            .filter_map(|token| {
                let range = offset..offset + token.len;
                offset = range.end;
                (!matches!(
                    token.kind,
                    TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment { .. }
                ))
                .then_some(Token {
                    kind: token.kind,
                    range,
                })
            })
            .collect();
        let pairs = pairs(&tokens);
        let mut stack: Vec<&str> = Vec::new();
        let mut index = 0;
        while index < tokens.len() {
            if opens(tokens[index].kind) {
                index = pairs[index].unwrap_or(tokens.len()) + 1;
                continue;
            }
            if tokens[index].kind != TokenKind::Lt {
                index += 1;
                continue;
            }
            let closing = tokens
                .get(index + 1)
                .is_some_and(|token| token.kind == TokenKind::Slash);
            let name_index = index + 1 + usize::from(closing);
            let name = tokens.get(name_index)?;
            if !matches!(name.kind, TokenKind::Ident | TokenKind::RawIdent) {
                return None;
            }
            let spelling = &text[name.range.clone()];
            if closing {
                if let Some(nested) = stack.pop() {
                    if nested != spelling {
                        return None;
                    }
                } else {
                    return (spelling == original || spelling == selected)
                        .then(|| name.range.clone());
                }
            }
            index = name_index + 1;
            while index < tokens.len() && tokens[index].kind != TokenKind::Gt {
                if opens(tokens[index].kind) {
                    index = pairs[index].unwrap_or(tokens.len());
                }
                index += 1;
            }
            if !closing && index < tokens.len() && tokens[index - 1].kind != TokenKind::Slash {
                stack.push(spelling);
            }
            index += 1;
        }
        None
    }
}

/// Returns whether a token opens a Rust delimiter group.
///
/// # Arguments
///
/// * `kind` — Lexical token category.
///
/// # Returns
///
/// A [`bool`] identifying opening braces, brackets, and parentheses.
fn opens(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::OpenBrace | TokenKind::OpenParen | TokenKind::OpenBracket
    )
}

/// Maps opening delimiter tokens to their closing token indices.
///
/// # Arguments
///
/// * `tokens` — Significant source tokens.
///
/// # Returns
///
/// A vector retaining unmatched openings as [`None`] for recovery.
fn pairs(tokens: &[Token]) -> Vec<Option<usize>> {
    let mut result = vec![None; tokens.len()];
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if opens(token.kind) {
            stack.push(index);
        } else {
            let expected = match token.kind {
                TokenKind::CloseBrace => TokenKind::OpenBrace,
                TokenKind::CloseParen => TokenKind::OpenParen,
                TokenKind::CloseBracket => TokenKind::OpenBracket,
                _ => continue,
            };
            if let Some(position) = stack
                .iter()
                .rposition(|opening| tokens[*opening].kind == expected)
            {
                let opening = stack[position];
                stack.truncate(position);
                result[opening] = Some(index);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    //! Completion boundaries in incomplete markup and embedded Rust.

    use super::*;

    /// Finds the context at a test cursor without changing any other source bytes.
    ///
    /// # Arguments
    ///
    /// * `marked` — Source containing one `¦` cursor marker.
    ///
    /// # Returns
    ///
    /// A source string and its optional completion context.
    fn at(marked: &str) -> (String, Option<Context>) {
        let cursor = marked.find('¦').unwrap();
        let text = marked.replace('¦', "");
        let context = Context::find(&text, cursor);
        (text, context)
    }

    /// Verifies partial tags and bare names have precise replacement ranges.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// view! { <For¦ /> }
    /// view! { <Div> "🙂" Inp¦ </Div> }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Delimiters are included only for the opening tag being completed.
    /// - Bare names and unfinished Rust braces still produce a markup context.
    #[test]
    fn recovers_partial_and_bare_tags() {
        let (text, context) = at("view! { <For¦ /> }");
        let context = context.unwrap();
        assert_eq!(&text[context.opening.unwrap()..context.end], "<For />");
        let (text, context) = at("view! { <Div> \"🙂\" Inp¦ </Div> }");
        let context = context.unwrap();
        assert_eq!(&text[context.name], "Inp");
        assert!(context.opening.is_none());
        assert!(at("fn main() { view! { <For¦").1.is_some());
        let (text, context) = at("view! { <For¦ class=(value } tail();");
        assert!(context.unwrap().end <= text.find('}').unwrap());
    }

    /// Verifies snippets cannot replace Rust expressions or attribute content.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// view! { <Input value={For¦} /> }
    /// view! { {view! { <For¦ }} }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Values, literals, comments, and ordinary Rust return no markup context.
    /// - A nested view macro inside Rust retains its own markup context.
    #[test]
    fn excludes_non_markup_positions() {
        for text in [
            "view! { <Input value={For¦} /> }",
            "view! { <Input value=For¦ /> }",
            "view! { <Text>{person.For¦}</Text> }",
            "view! { <Text>r#\"<For¦>\"#</Text> }",
            "view! { // <For¦\n }",
            "view! { /* <For¦ */ }",
            "fn main() { For¦ }",
        ] {
            assert!(at(text).1.is_none(), "{text}");
        }
        assert!(at("view! { {view! { <For¦ }} }").1.is_some());
    }

    /// Verifies matching closes respect nesting and surrounding containers.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// view! { <For¦ class="x"><Text>"Body"</Text></For> }
    /// view! { <Div><For¦></Div> }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Existing attributes and matching closing names are identified precisely.
    /// - A surrounding parent's closing tag is never reused for the new tag.
    #[test]
    fn preserves_attributes_and_nested_children() {
        let (text, context) = at("view! { <For¦ class=\"x\"><Text>\"Body\"</Text></For> }");
        let context = context.unwrap();
        assert_eq!(&text[context.attributes.clone()], " class=\"x\"");
        let close = context.matching_close(&text, "Form").unwrap();
        assert_eq!(&text[close], "For");
        let (text, context) = at("view! { <Div><For¦></Div> }");
        assert!(context.unwrap().matching_close(&text, "Form").is_none());
    }
}
