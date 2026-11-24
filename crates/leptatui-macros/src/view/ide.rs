//! Source-preserving semantic references for complete and unfinished markup.
//!
//! These probes never execute. The compiler still receives the strict parser's
//! diagnostics, while rust-analyzer can resolve names in the surviving tokens.

use proc_macro2::{Delimiter, Ident, Span, TokenStream, TokenTree};
use quote::quote;
use syn::parse::{ParseStream, Parser};

use super::attr::Attr;

/// Emits tag and attribute references without requiring well-formed markup.
///
/// # Arguments
///
/// * `input` — Original macro tokens, including their source spans.
///
/// # Returns
///
/// A [`TokenStream`] containing nonexecuted semantic probes.
pub(super) fn references(input: TokenStream) -> TokenStream {
    let leptatui = crate::crate_path::leptatui();
    let tokens: Vec<_> = input.into_iter().collect();
    let mut output = TokenStream::new();
    let mut index = 0;
    while index < tokens.len() {
        if let TokenTree::Group(group) = &tokens[index]
            && group.delimiter() == Delimiter::Brace
        {
            output.extend(expression_reference(group.stream()));
        }
        if !punct(&tokens[index], '<') {
            if let TokenTree::Ident(name) = &tokens[index] {
                output.extend(tag_reference(name, &[]));
            }
            index += 1;
            continue;
        }
        index += 1;
        let closing = tokens.get(index).is_some_and(|token| punct(token, '/'));
        if closing {
            index += 1;
        }
        let Some(TokenTree::Ident(name)) = tokens.get(index) else {
            continue;
        };
        let name = name.clone();
        index += 1;
        let header_start = index;
        let mut expressions = TokenStream::new();
        let mut attrs = Vec::new();
        let mut value = false;
        while let Some(token) = tokens.get(index) {
            if punct(token, '>') || (punct(token, '<') && !value) {
                break;
            }
            if let TokenTree::Ident(ident) = token {
                let assignment = tokens.get(index + 1).is_some_and(|token| punct(token, '='));
                if !closing && (!value || assignment) {
                    attrs.push(ident.clone());
                    value = assignment;
                    index += 1 + usize::from(assignment);
                    continue;
                }
            }
            if let TokenTree::Group(group) = token
                && group.delimiter() == Delimiter::Brace
            {
                expressions.extend(expression_reference(group.stream()));
            }
            if matches!(token, TokenTree::Group(_) | TokenTree::Literal(_)) {
                value = false;
            }
            index += 1;
        }
        let mut header = &tokens[header_start..index];
        if header.last().is_some_and(|token| punct(token, '/')) {
            header = &header[..header.len() - 1];
        }
        let parse_attrs = |input: ParseStream<'_>| {
            let mut attrs = Vec::new();
            while !input.is_empty() {
                attrs.push(input.parse::<Attr>()?);
            }
            Ok(attrs)
        };
        if let Ok(parsed) = parse_attrs.parse2(header.iter().cloned().collect()) {
            attrs = parsed.iter().map(|attr| attr.name.clone()).collect();
            for attr in parsed {
                if !attr.value.is_literal() {
                    output.extend(expression_reference(attr.value.to_tokens()));
                }
            }
        } else {
            output.extend(expressions);
        }
        output.extend(tag_reference(&name, &attrs));
    }
    if output.is_empty() {
        output.extend(tag_reference(
            &Ident::new("__leptatui_incomplete", Span::mixed_site()),
            &[],
        ));
    }
    quote! {
        #[allow(unexpected_cfgs)]
        {
            #[cfg(rust_analyzer)]
            #leptatui::__view_ide_probe! {
                #[allow(unused_imports, unused_variables, dead_code, deprecated)]
                {
                    use #leptatui::__ide::*;
                    #output
                }
            }
        }
    }
}

/// Emits one tag's type reference and source-named prop field accesses.
///
/// # Arguments
///
/// * `name` — Original tag identifier.
/// * `attrs` — Original attribute identifiers.
///
/// # Returns
///
/// A [`TokenStream`] of type and field probes with the original source spans.
fn tag_reference(name: &Ident, attrs: &[Ident]) -> TokenStream {
    let leptatui = crate::crate_path::leptatui();
    // Keep the expansion shape identical for an exact built-in name and the
    // speculative identifier rust-analyzer inserts while requesting completion.
    let ty = quote! { #name };
    let props = quote! { <#ty as #leptatui::__ide::__Component>::Props };
    // Preserve the field-access shape when the first attribute is inserted.
    // The synthetic field has no source identifier to diagnose or rename.
    let fields = if attrs.is_empty() {
        vec![Ident::new("__leptatui_incomplete", Span::mixed_site())]
    } else {
        attrs.to_vec()
    };
    quote! {
        let _ = |_: ::core::option::Option<#ty>| {};
        let _ = |__leptatui_props: &#props| {
            #(let _ = &__leptatui_props.#fields;)*
        };
    }
}

/// Returns whether a token is the requested punctuation character.
///
/// # Arguments
///
/// * `token` — Source token to inspect.
/// * `expected` — Requested punctuation character.
///
/// # Returns
///
/// A [`bool`] indicating whether the token matches.
fn punct(token: &TokenTree, expected: char) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == expected)
}

/// Emits an unfinished expression with a syntactically valid trailing member.
///
/// # Arguments
///
/// * `expression` — Tokens inside a source brace group.
///
/// # Returns
///
/// A [`TokenStream`] preserving the receiver for member completion.
fn expression_reference(mut expression: TokenStream) -> TokenStream {
    if expression
        .clone()
        .into_iter()
        .last()
        .is_some_and(|token| punct(&token, '.'))
    {
        expression.extend([TokenTree::Ident(Ident::new(
            "__leptatui_incomplete",
            Span::mixed_site(),
        ))]);
    }
    quote! { let _ = || { #expression }; }
}
