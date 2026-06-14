//! Style declaration model for `stylesheet!` syntax.
//!
//! This module parses property declarations inside a stylesheet rule and
//! expands each accepted declaration value into a `StyleDeclarations` builder
//! call.

use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::{
    Error, Ident, Result, Token,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::model::{
    import::StylesheetImports,
    selector::Selector,
    value::{StyleValue, StyleValueKind},
    variable::StylesheetVariables,
};

mod kw {
    syn::custom_keyword!(important);
}

/// Parsed style declaration such as `fg: Color::White`.
pub(super) struct Declaration {
    /// Declaration property name.
    name: Ident,
    /// Value assigned to the declaration.
    value: StyleValue,
    /// Whether this declaration is marked with `!important`.
    important: bool,
}

impl Parse for Declaration {
    /// Parses a stylesheet declaration name and value.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a declaration name.
    ///
    /// # Returns
    ///
    /// A [`Declaration`] containing the parsed property name and value.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the declaration is missing a colon or value.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = parse_value(input)?;
        let important = if input.peek(Token![!]) {
            input.parse::<Token![!]>()?;
            input.parse::<kw::important>()?;
            true
        } else {
            false
        };

        Ok(Self {
            name,
            value,
            important,
        })
    }
}

fn parse_value(input: ParseStream<'_>) -> Result<StyleValue> {
    let mut tokens = TokenStream::new();

    while !input.is_empty()
        && !input.peek(Token![,])
        && !starts_important(input)
        && !starts_nested_rule(input)
    {
        tokens.extend(::std::iter::once(input.parse::<TokenTree>()?));
    }

    if tokens.is_empty() {
        return Err(input.error("stylesheet! declaration requires a value"));
    }

    syn::parse2(tokens)
}

fn starts_important(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Token![!]>().is_ok() && fork.parse::<kw::important>().is_ok()
}

fn starts_nested_rule(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Selector>().is_ok() && fork.peek(Token![=>])
}

impl Declaration {
    /// Appends this declaration to an in-progress `StyleDeclarations` expression.
    ///
    /// # Arguments
    ///
    /// * `style` — Existing `StyleDeclarations` expression to wrap with this
    ///   declaration.
    /// * `variables` — Stylesheet variables available to declaration values.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated style expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the declaration name is unsupported or a
    /// referenced stylesheet variable is unknown.
    pub(super) fn expand(
        &self,
        style: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
    ) -> Result<TokenStream> {
        match self.name.to_string().as_str() {
            "fg" | "foreground" => {
                let value = self
                    .value
                    .expand(variables, imports, StyleValueKind::Color)?;
                if self.important {
                    Ok(quote! { (#style).foreground_important(#value) })
                } else {
                    Ok(quote! { (#style).foreground(#value) })
                }
            }
            "bg" | "background" => {
                let value = self
                    .value
                    .expand(variables, imports, StyleValueKind::Color)?;
                if self.important {
                    Ok(quote! { (#style).background_important(#value) })
                } else {
                    Ok(quote! { (#style).background(#value) })
                }
            }
            "modifier" => {
                let value = self
                    .value
                    .expand(variables, imports, StyleValueKind::Modifier)?;
                if self.important {
                    Ok(quote! { (#style).modifier_important(#value) })
                } else {
                    Ok(quote! { (#style).modifier(#value) })
                }
            }
            "borders" => {
                let value = self
                    .value
                    .expand(variables, imports, StyleValueKind::Borders)?;
                if self.important {
                    Ok(quote! { (#style).borders_important(#value) })
                } else {
                    Ok(quote! { (#style).borders(#value) })
                }
            }
            "border_type" => {
                let value = self
                    .value
                    .expand(variables, imports, StyleValueKind::BorderType)?;
                if self.important {
                    Ok(quote! { (#style).border_type_important(#value) })
                } else {
                    Ok(quote! { (#style).border_type(#value) })
                }
            }
            "padding" => {
                let value = self
                    .value
                    .expand(variables, imports, StyleValueKind::Spacing)?;
                if self.important {
                    Ok(quote! { (#style).padding_important(#value) })
                } else {
                    Ok(quote! { (#style).padding(#value) })
                }
            }
            "direction" => {
                let value =
                    self.value
                        .expand(variables, imports, StyleValueKind::LayoutDirection)?;
                if self.important {
                    Ok(quote! { (#style).direction_important(#value) })
                } else {
                    Ok(quote! { (#style).direction(#value) })
                }
            }
            _ => Err(Error::new_spanned(
                &self.name,
                "unsupported stylesheet declaration; expected fg, foreground, bg, background, modifier, borders, border_type, padding, or direction",
            )),
        }
    }
}
