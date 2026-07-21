//! Low-level parsers for `view!` braced expressions and closing tags.
//!
//! This module keeps token-stream lookahead and braced-expression validation
//! separate from the parsed syntax model.

use syn::{Expr, Result, Token, braced, parse::ParseStream};

/// Parses braced content as exactly one Rust expression.
///
/// # Arguments
///
/// * `input` — Macro input stream positioned at a braced expression.
///
/// # Returns
///
/// An [`Expr`] parsed from inside the braces.
///
/// # Errors
///
/// Returns [`syn::Error`] if the braced content is missing, invalid, or
/// contains tokens after the expression.
pub(crate) fn parse_braced_expr(input: ParseStream<'_>) -> Result<Expr> {
    let content;
    braced!(content in input);
    let value = content.parse()?;

    if !content.is_empty() {
        return Err(content.error("view! braced content must be a single Rust expression"));
    }

    Ok(value)
}

/// Returns whether the next tokens begin a closing tag.
///
/// # Arguments
///
/// * `input` — Macro input stream to inspect without consuming.
///
/// # Returns
///
/// A [`bool`] indicating whether the stream begins with `</`.
pub(crate) fn next_is_closing_tag(input: ParseStream<'_>) -> bool {
    let fork = input.fork();

    fork.parse::<Token![<]>().is_ok() && fork.parse::<Token![/]>().is_ok()
}

/// Returns whether the next tokens end a self-closing opening tag.
///
/// # Arguments
///
/// * `input` — Macro input stream to inspect without consuming.
///
/// # Returns
///
/// A [`bool`] indicating whether the stream begins with `/>`.
pub(crate) fn next_is_self_closing_tag_end(input: ParseStream<'_>) -> bool {
    let fork = input.fork();

    fork.parse::<Token![/]>().is_ok() && fork.parse::<Token![>]>().is_ok()
}
