//! Syntax tree for parsed `view!` macro input.
//!
//! Each model in this module keeps its parsing and expansion impls next to the
//! type declaration so the model's behavior is easy to audit in one place.

mod attr;
mod child;
mod element;
mod text_content;
mod view_root;

pub(super) use view_root::ViewRoot;

use syn::{
    Expr, Result, braced,
    parse::ParseStream,
};

/// Parses braced content as exactly one Rust expression.
///
/// # Arguments
///
/// * `input` - Macro input stream positioned at a braced expression.
///
/// # Returns
///
/// An [`Expr`] parsed from inside the braces.
///
/// # Errors
///
/// Returns [`syn::Error`] if the braced content is missing, invalid, or contains
/// tokens after the expression.
pub(super) fn parse_braced_expr(input: ParseStream<'_>) -> Result<Expr> {
    let content;
    braced!(content in input);
    let value = content.parse()?;

    if !content.is_empty() {
        return Err(content.error("view! braced content must be a single Rust expression"));
    }

    Ok(value)
}
