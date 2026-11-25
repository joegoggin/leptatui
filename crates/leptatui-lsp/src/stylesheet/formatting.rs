//! Comment-preserving stylesheet formatting and document edit composition.
//!
//! # Modules
//!
//! - [`printer`] — Stylesheet structure, spacing, and lossless trivia handling.
//! - [`expressions`] — Batched rustfmt expression formatting and project settings.
//! - [`proxy`] — Revision-safe document-formatting request handling.

mod expressions;
mod printer;
mod proxy;
#[cfg(test)]
mod tests;

use super::syntax::{Block, Kind, Macro, Statement, Syntax};
use serde_json::{Value, json};
use std::{collections::HashMap, ops::Range, path::Path};

/// Layout preferences for a stylesheet document.
#[derive(Clone)]
pub(crate) struct Options {
    /// Spaces represented by one indentation level.
    pub(crate) tab_size: usize,
    /// Whether indentation uses spaces rather than hard tabs.
    pub(crate) spaces: bool,
    /// Maximum preferred line width.
    pub(crate) width: usize,
    /// Edition used when rustfmt parses value expressions.
    pub(crate) edition: String,
}

impl Default for Options {
    /// Returns conventional Rust formatting defaults.
    ///
    /// # Returns
    ///
    /// An option set using four spaces, 100 columns, and Rust 2024.
    fn default() -> Self {
        Self {
            tab_size: 4,
            spaces: true,
            width: 100,
            edition: "2024".into(),
        }
    }
}

impl Options {
    /// Returns one indentation unit.
    ///
    /// # Returns
    ///
    /// A string of spaces or one tab.
    fn indent(&self) -> String {
        if self.spaces {
            " ".repeat(self.tab_size)
        } else {
            "\t".into()
        }
    }
}

/// Returns nonoverlapping stylesheet invocations in source order.
///
/// # Arguments
///
/// * `syntax` — Source token and macro index.
///
/// # Returns
///
/// A list excluding stylesheets nested inside another stylesheet expression.
fn macros<'a>(syntax: &'a Syntax<'_>) -> Vec<&'a Macro> {
    let mut output: Vec<&Macro> = Vec::new();
    for m in syntax.macros.iter().filter(|m| !m.view) {
        if output
            .last()
            .is_none_or(|previous| previous.span.end < m.span.start)
        {
            output.push(m);
        }
    }
    output
}

/// Tests whether a gap contains only whitespace or comments.
///
/// # Arguments
///
/// * `text` — Gap between structural statements.
///
/// # Returns
///
/// A boolean excluding punctuation that a recovering parser may skip.
fn trivia(text: &str) -> bool {
    rustc_lexer::tokenize(text).all(|token| {
        matches!(
            token.kind,
            rustc_lexer::TokenKind::Whitespace
                | rustc_lexer::TokenKind::LineComment
                | rustc_lexer::TokenKind::BlockComment { terminated: true }
        )
    })
}

/// Returns source ranges and importance markers for declaration values.
///
/// # Arguments
///
/// * `syntax` — Current source tokens.
/// * `statement` — Property or variable declaration.
///
/// # Returns
///
/// An optional expression range and authored importance suffix.
fn value(syntax: &Syntax<'_>, statement: &Statement) -> Option<(Range<usize>, String)> {
    if !matches!(statement.kind, Kind::Property | Kind::Variable) {
        return None;
    }
    let colon = statement.head.start
        + if statement.kind == Kind::Variable {
            2
        } else {
            1
        };
    if syntax.at(colon) != ":" {
        return None;
    }
    let end = statement.head.end;
    let important = statement.kind == Kind::Property
        && end >= colon + 3
        && syntax.at(end - 2) == "!"
        && syntax.at(end - 1) == "important";
    let finish = if important {
        syntax.tokens[end - 2].span.start
    } else {
        syntax.tokens.get(end)?.span.start
    };
    let suffix = if important {
        let raw = &syntax.text[finish..syntax.tokens.get(end)?.span.start];
        if printer::comments(raw).is_empty() {
            " !important".into()
        } else {
            format!(" {}", raw.trim())
        }
    } else {
        String::new()
    };
    Some((syntax.tokens[colon].span.end..finish, suffix))
}

/// Validates a block before printing its recovering syntax tree.
///
/// # Arguments
///
/// * `syntax` — Source index.
/// * `block` — Block whose statement boundaries must be complete.
/// * `root` — Whether root-only declarations are allowed.
///
/// # Returns
///
/// A boolean indicating whether structural formatting is lossless.
fn valid(syntax: &Syntax<'_>, block: &Block, root: bool) -> bool {
    let mut previous = block.span.start;
    let mut phase = 0;
    for s in &block.statements {
        if !trivia(&syntax.text[previous..s.span.start]) || s.head.is_empty() {
            return false;
        }
        let header = &syntax.tokens[s.head.clone()];
        if matches!(s.kind, Kind::Use | Kind::Variable | Kind::Mixin) && !root {
            return false;
        }
        if s.kind == Kind::Include && !block.declarations {
            return false;
        }
        if root {
            let next = match s.kind {
                Kind::Use => 0,
                Kind::Variable => 1,
                Kind::Mixin => 2,
                _ => 3,
            };
            if next < phase {
                return false;
            }
            phase = next;
        }
        if !matches!(s.kind, Kind::Property | Kind::Variable) && !valid_header(syntax, s) {
            return false;
        }
        if let Some(body) = &s.body {
            if !matches!(s.kind, Kind::Rule | Kind::Media | Kind::Mixin)
                || body.statements.is_empty()
                || !valid(syntax, body, false)
            {
                return false;
            }
            if s.kind == Kind::Rule
                && (header.iter().any(|t| t.text == "@")
                    || header.windows(2).any(|w| w[0].ident && w[1].ident))
            {
                return false;
            }
        } else {
            if !s.terminated || matches!(s.kind, Kind::Rule | Kind::Media | Kind::Mixin) {
                return false;
            }
            if matches!(s.kind, Kind::Property | Kind::Variable) {
                if s.kind == Kind::Property
                    && (!block.declarations
                        || !super::properties::PROPERTIES
                            .iter()
                            .any(|p| p.0 == header[0].text))
                {
                    return false;
                }
                let Some((range, _)) = value(syntax, s) else {
                    return false;
                };
                let raw = syntax.text[range].trim();
                if raw.is_empty()
                    || (syn::parse_str::<syn::Expr>(raw).is_err() && !stylesheet_reference(raw))
                {
                    return false;
                }
            }
            if matches!(s.kind, Kind::Use | Kind::Include) && header.len() < 3 {
                return false;
            }
        }
        previous = s.span.end;
    }
    trivia(&syntax.text[previous..block.span.end])
}

/// Parses structural headers without recovering or merging invalid tokens.
///
/// # Arguments
///
/// * `syntax` — Authored token ranges.
/// * `statement` — Selector, directive, media query, or mixin header.
///
/// # Returns
///
/// A boolean accepting only a complete supported header.
fn valid_header(syntax: &Syntax<'_>, statement: &Statement) -> bool {
    use syn::{
        Ident, Token,
        parse::{ParseStream, Parser},
    };
    let tokens = &syntax.tokens[statement.head.clone()];
    let start = if statement.kind == Kind::Rule {
        tokens[0].span.start
    } else {
        tokens.get(2).map_or(tokens[0].span.end, |t| t.span.start)
    };
    let end = tokens.last().unwrap().span.end;
    let raw = &syntax.text[start..end];
    let parser = |input: ParseStream<'_>| -> syn::Result<()> {
        match statement.kind {
            Kind::Use => {
                input.parse::<syn::Path>()?;
                if input.peek(Token![as]) {
                    input.parse::<Token![as]>()?;
                    input.parse::<Ident>()?;
                }
            }
            Kind::Include => {
                input.parse::<Ident>()?;
                if input.peek(Token![.]) {
                    input.parse::<Token![.]>()?;
                    input.parse::<Ident>()?;
                }
            }
            Kind::Mixin => {
                input.parse::<Ident>()?;
            }
            Kind::Media => loop {
                let content;
                syn::parenthesized!(content in input);
                let first: Ident = content.parse()?;
                content.parse::<Token![-]>()?;
                let second: Ident = content.parse()?;
                if !matches!(first.to_string().as_str(), "min" | "max")
                    || !matches!(second.to_string().as_str(), "width" | "height")
                {
                    return Err(content.error("unsupported viewport feature"));
                }
                content.parse::<Token![:]>()?;
                content.parse::<syn::LitInt>()?.base10_parse::<u16>()?;
                if !content.is_empty() {
                    return Err(content.error("unexpected viewport value"));
                }
                if input.is_empty() {
                    break;
                }
                let and: Ident = input.parse()?;
                if and != "and" {
                    return Err(input.error("expected and"));
                }
            },
            Kind::Rule => {
                if input.peek(Token![&]) {
                    input.parse::<Token![&]>()?;
                    if input.peek(Token![:]) {
                        pseudo(input)?;
                    } else if input.peek(Token![-]) {
                        input.parse::<Token![-]>()?;
                        input.parse::<Token![-]>()?;
                        selector_name(input)?;
                    } else {
                        let name: Ident = input.parse()?;
                        if !name.to_string().starts_with("__") || name.to_string().len() <= 2 {
                            return Err(input.error("expected parent suffix"));
                        }
                    }
                } else if input.peek(Token![.]) {
                    input.parse::<Token![.]>()?;
                    selector_name(input)?;
                } else if input.peek(Token![#]) {
                    input.parse::<Token![#]>()?;
                    selector_name(input)?;
                } else if input.peek(Token![:]) {
                    pseudo(input)?;
                } else {
                    input.parse::<Ident>()?;
                    if input.peek(Token![:]) {
                        pseudo(input)?;
                    }
                }
            }
            _ => {}
        }
        if !input.is_empty() {
            return Err(input.error("unexpected stylesheet header tokens"));
        }
        Ok(())
    };
    parser.parse_str(raw).is_ok()
}

/// Parses a supported pseudo-class header suffix.
///
/// # Arguments
///
/// * `input` — Header parse stream beginning at a colon.
///
/// # Returns
///
/// An empty result for a supported pseudo-class.
///
/// # Errors
///
/// Returns [`syn::Error`] for malformed or unsupported pseudo-classes.
fn pseudo(input: syn::parse::ParseStream<'_>) -> syn::Result<()> {
    input.parse::<syn::Token![:]>()?;
    let name: syn::Ident = input.parse()?;
    if !matches!(
        name.to_string().as_str(),
        "focus" | "active" | "insert" | "visual" | "visited"
    ) {
        return Err(input.error("unsupported pseudo-class"));
    }
    Ok(())
}

/// Parses a dash-separated class or identifier selector name.
///
/// # Arguments
///
/// * `input` — Header parse stream positioned at a name.
///
/// # Returns
///
/// An empty result for a complete name.
///
/// # Errors
///
/// Returns [`syn::Error`] for a missing name or segment.
fn selector_name(input: syn::parse::ParseStream<'_>) -> syn::Result<()> {
    input.parse::<syn::Ident>()?;
    while input.peek(syn::Token![-]) {
        input.parse::<syn::Token![-]>()?;
        input.parse::<syn::Ident>()?;
    }
    Ok(())
}

/// Recognizes stylesheet variable values without parsing them as Rust.
///
/// # Arguments
///
/// * `text` — Authored value expression.
///
/// # Returns
///
/// A boolean identifying local or imported stylesheet references.
fn stylesheet_reference(text: &str) -> bool {
    let syntax = Syntax::new(text);
    let tokens = &syntax.tokens;
    (tokens.len() == 2 && tokens[0].text == "$" && tokens[1].ident)
        || (tokens.len() == 4
            && tokens[0].ident
            && tokens[1].text == "."
            && tokens[2].text == "$"
            && tokens[3].ident)
}

/// Tests whether a macro has complete delimiters and printable grammar.
///
/// # Arguments
///
/// * `syntax` — Current source index.
/// * `m` — Candidate invocation.
///
/// # Returns
///
/// A boolean allowing only complete, nonempty stylesheet bodies.
fn printable(syntax: &Syntax<'_>, m: &Macro) -> bool {
    m.end < syntax.tokens.len()
        && syntax.text[syntax.tokens[m.open].span.start..syntax.tokens[m.end].span.end]
            .parse::<proc_macro2::TokenStream>()
            .is_ok()
        && !syntax.parse(m).statements.is_empty()
        && valid(syntax, &syntax.parse(m), true)
}

/// Applies protocol edits to an immutable source snapshot.
///
/// # Arguments
///
/// * `text` — Source against which ranges were calculated.
/// * `edits` — Upstream formatting edits.
///
/// # Returns
///
/// An optional edited document; malformed or overlapping edits are rejected.
pub(crate) fn apply(text: &str, edits: &Value) -> Option<String> {
    if edits.is_null() {
        return Some(text.into());
    }
    let mut ranges = Vec::new();
    for edit in edits.as_array()? {
        let start = crate::source::offset(text, &edit["range"]["start"])?;
        let end = crate::source::offset(text, &edit["range"]["end"])?;
        if start > end {
            return None;
        }
        ranges.push((start..end, edit["newText"].as_str()?));
    }
    ranges.sort_by_key(|(r, _)| r.start);
    if ranges.windows(2).any(|w| w[0].0.end > w[1].0.start) {
        return None;
    }
    let mut result = text.to_owned();
    for (range, replacement) in ranges.into_iter().rev() {
        result.replace_range(range, replacement);
    }
    Some(result)
}

/// Formats stylesheets after ordinary Rust formatting while protecting invalid input.
///
/// # Arguments
///
/// * `original` — Unformatted source overlay.
/// * `upstream` — Document after upstream Rust and optional view formatting.
/// * `options` — Layout preferences.
/// * `path` — Source path for project rustfmt configuration.
///
/// # Returns
///
/// A fully composed document without file-system writes.
pub(crate) fn document(
    original: &str,
    upstream: &str,
    options: &Options,
    path: Option<&Path>,
) -> String {
    let source = Syntax::new(original);
    let formatted = Syntax::new(upstream);
    let before = macros(&source);
    let after = macros(&formatted);
    if before.len() != after.len() {
        return original.into();
    }
    let mut protected = upstream.to_owned();
    for (old, new) in before.iter().zip(&after).rev() {
        if !printable(&source, old) {
            protected.replace_range(new.span.clone(), &original[old.span.clone()]);
        }
    }
    let syntax = Syntax::new(&protected);
    let invocations: Vec<_> = macros(&syntax)
        .into_iter()
        .filter(|m| printable(&syntax, m))
        .collect();
    let expressions = expressions::format(&syntax, &invocations, options, path);
    let mut result = protected.clone();
    for m in invocations.into_iter().rev() {
        let line = syntax.text[..syntax.tokens[m.open].span.start]
            .rsplit('\n')
            .next()
            .unwrap_or("");
        let indent: String = line
            .chars()
            .take_while(|c| matches!(c, ' ' | '\t'))
            .collect();
        let body = printer::block(&syntax, &syntax.parse(m), &indent, options, &expressions);
        let replacement = format!("\n{body}{indent}");
        let replacement = if upstream.contains("\r\n") {
            line_endings(&replacement)
        } else {
            replacement
        };
        result.replace_range(m.span.clone(), &replacement);
    }
    result
}

/// Converts generated code newlines to CRLF while preserving literal and comment bytes.
///
/// # Arguments
///
/// * `text` — Printed macro interior.
///
/// # Returns
///
/// A macro interior using CRLF outside literal and block-comment tokens.
fn line_endings(text: &str) -> String {
    let mut offset = 0;
    let mut output = String::new();
    for token in rustc_lexer::tokenize(text) {
        let raw = &text[offset..offset + token.len];
        offset += token.len;
        if matches!(
            token.kind,
            rustc_lexer::TokenKind::Literal { .. } | rustc_lexer::TokenKind::BlockComment { .. }
        ) {
            output.push_str(raw);
        } else {
            output.push_str(&raw.replace("\r\n", "\n").replace('\n', "\r\n"));
        }
    }
    output
}

/// Returns one whole-document edit when formatting changes the source.
///
/// # Arguments
///
/// * `before` — Original source overlay.
/// * `after` — Composed formatting output.
///
/// # Returns
///
/// A list with zero or one UTF-16 source-ranged edit.
fn edits(before: &str, after: &str) -> Value {
    if before == after {
        json!([])
    } else {
        json!([{"range":super::range(before,0..before.len()),"newText":after}])
    }
}
