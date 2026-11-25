//! Source-spanned Rust expression probes for unfinished stylesheets.

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::quote;

/// Preserves Rust value and import tokens for both complete and unfinished stylesheets.
///
/// # Arguments
///
/// * `input` — Authored stylesheet tokens with original spans.
///
/// # Returns
///
/// A token stream of nonexecuted expression probes.
pub(super) fn references(input: TokenStream) -> TokenStream {
    let leptatui = crate::crate_path::leptatui();
    let tokens: Vec<_> = input.into_iter().collect();
    let mut output = TokenStream::new();
    let mut index = 0;
    while index < tokens.len() {
        if punct(&tokens[index], '@')
            && tokens
                .get(index + 1)
                .is_some_and(|t| matches!(t,TokenTree::Ident(name) if name=="use"))
        {
            index += 2;
            let mut path = TokenStream::new();
            while let Some(t) = tokens.get(index) {
                if punct(t, ';') || matches!(t,TokenTree::Ident(name) if name=="as") {
                    break;
                }
                path.extend([t.clone()]);
                index += 1;
            }
            if !path.is_empty() {
                output.extend(quote! {if false {let _ = #path();}});
            }
            while index < tokens.len() && !punct(&tokens[index], ';') {
                index += 1;
            }
        } else if let TokenTree::Ident(name) = &tokens[index]
            && tokens.get(index + 1).is_some_and(|t| punct(t, ':'))
        {
            let property = properties::PROPERTIES.iter().find(|p| name == p.0);
            let variable = index > 0 && punct(&tokens[index - 1], '$');
            if property.is_some() || variable {
                index += 2;
                let mut value = TokenStream::new();
                let mut stylesheet_value = false;
                while let Some(t) = tokens.get(index) {
                    if punct(t, ';') {
                        break;
                    }
                    if punct(t, '!')
                        && tokens
                            .get(index + 1)
                            .is_some_and(|t| matches!(t,TokenTree::Ident(n) if n=="important"))
                    {
                        break;
                    }
                    if punct(t, '$') {
                        stylesheet_value = true;
                    }
                    value.extend([t.clone()]);
                    index += 1;
                }
                if !value.is_empty() && !stylesheet_value {
                    if let Some((_, _, method, _)) = property {
                        let method = proc_macro2::Ident::new(method, name.span());
                        output.extend(quote!{if false {let _ = #leptatui::StyleDeclarations::new().#method({#value});}});
                    } else {
                        output.extend(quote! {if false {let _ = {#value};}});
                    }
                }
            } else {
                index += 1;
                continue;
            }
        } else if let TokenTree::Group(group) = &tokens[index]
            && group.delimiter() == Delimiter::Brace
        {
            output.extend(references(group.stream()));
        }
        index += 1;
    }
    quote! {
        #[allow(unexpected_cfgs)]
        {
            #[cfg(rust_analyzer)]
            #leptatui::__view_ide_probe! {
                #[allow(unused_variables, dead_code, deprecated)]
                { #output }
            }
        }
    }
}

/// Tests punctuation while preserving authored token spans.
///
/// # Arguments
///
/// * `token` — Source token to inspect.
/// * `expected` — Punctuation character to match.
///
/// # Returns
///
/// A boolean indicating whether the token matches the expected punctuation.
fn punct(token: &TokenTree, expected: char) -> bool {
    matches!(token,TokenTree::Punct(p) if p.as_char()==expected)
}

use super::properties;

#[cfg(test)]
mod tests {
    //! Shared property metadata validation.
    use super::*;

    /// Verifies every editor property remains accepted by the compiler.
    ///
    /// # Example Under Test
    ///
    /// Each property in the shared metadata is parsed in a stylesheet rule.
    ///
    /// # Assertions
    ///
    /// - Property validation accepts every advertised spelling.
    #[test]
    fn property_metadata_matches_parser() {
        for &(name, _, _, _) in properties::PROPERTIES {
            let input = format!("Text {{ {name}: value; }}");
            let root = syn::parse_str::<crate::stylesheet::root::StylesheetRoot>(&input).unwrap();
            assert!(root.expand().is_ok(), "{name}");
        }
    }
}
