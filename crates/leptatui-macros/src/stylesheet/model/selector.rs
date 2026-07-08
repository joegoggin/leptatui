//! Selector model for `stylesheet!` syntax.
//!
//! This module parses type, class, id, focus, type-focus, and nested `&:focus`
//! selectors and lowers them into public `StyleSelector` constructor calls.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Error, Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream},
};

/// Parsed terminal stylesheet selector.
pub(super) enum Selector {
    /// View type selector such as `Text`.
    Type(Ident),
    /// Class selector such as `.primary`.
    Class(SelectorName),
    /// Id selector such as `#submit`.
    Id(SelectorName),
    /// Pseudo selector such as `:focus`.
    Pseudo(Ident),
    /// Compound type and pseudo selector such as `Button:focus`.
    TypePseudo {
        /// View type part of the compound selector.
        view_type: Ident,
        /// Pseudo-selector part of the compound selector.
        pseudo: Ident,
    },
    /// Nested parent pseudo selector containing the pseudo identifier from
    /// selectors such as `&:focus`.
    ParentPseudo(Ident),
}

impl Parse for Selector {
    /// Parses a stylesheet selector.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a selector.
    ///
    /// # Returns
    ///
    /// A [`Selector`] containing the parsed selector model.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the selector is not a supported type, class,
    /// id, focus, type-focus, or nested `&:focus` selector.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(Token![&]) {
            input.parse::<Token![&]>()?;

            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                return Ok(Self::ParentPseudo(input.parse()?));
            }

            return Err(
                input.error("stylesheet! parent selector only supports &:focus in nested rules")
            );
        }

        if input.peek(Token![.]) {
            input.parse::<Token![.]>()?;
            return Ok(Self::Class(SelectorName::parse(input)?));
        }

        if input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            return Ok(Self::Id(SelectorName::parse(input)?));
        }

        if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            return Ok(Self::Pseudo(input.parse()?));
        }

        if input.peek(Ident) {
            let view_type = input.parse()?;
            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                let pseudo = input.parse()?;
                return Ok(Self::TypePseudo { view_type, pseudo });
            }

            return Ok(Self::Type(view_type));
        }

        Err(input.error(
            "stylesheet! selector must be a type, .class, #id, :focus, Type:focus, or nested &:focus selector",
        ))
    }
}

impl Selector {
    /// Expands this selector into a `StyleSelector` expression.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a public `StyleSelector` constructor call.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the selector uses an unsupported view type or
    /// pseudo-selector, or if a parent-reference selector is expanded without a
    /// selector path.
    pub(super) fn expand(&self) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();

        match self {
            Self::Type(view_type) => {
                let view_type = Self::expand_view_type(view_type)?;
                Ok(quote! { #leptatui::StyleSelector::view_type(#view_type) })
            }
            Self::Class(class) => {
                let class = class.literal();
                Ok(quote! { #leptatui::StyleSelector::class(#class) })
            }
            Self::Id(id) => {
                let id = id.literal();
                Ok(quote! { #leptatui::StyleSelector::id(#id) })
            }
            Self::Pseudo(pseudo) => Self::expand_pseudo(pseudo),
            Self::TypePseudo { view_type, pseudo } => {
                let view_type = Self::expand_view_type(view_type)?;
                let pseudo = Self::expand_pseudo(pseudo)?;

                Ok(quote! {
                    #leptatui::StyleSelector::compound(::std::vec![
                        #leptatui::StyleSelector::view_type(#view_type),
                        #pseudo,
                    ])
                })
            }
            Self::ParentPseudo(_) => Err(Error::new_spanned(
                self.span_tokens(),
                "stylesheet! parent selector &:focus can only appear inside a nested rule",
            )),
        }
    }

    /// Expands a selector path into a single runtime selector expression.
    ///
    /// # Arguments
    ///
    /// * `path` — Ordered selector path from outermost rule to current rule.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a public `StyleSelector` expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if parent-reference selectors appear without a
    /// parent selector or any selector segment cannot be expanded.
    pub(super) fn expand_path(path: &[&Selector]) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();
        let mut segments: Vec<Vec<TokenStream>> = Vec::new();

        for selector in path {
            match selector {
                Self::ParentPseudo(pseudo) => {
                    let pseudo = Self::expand_pseudo(pseudo)?;
                    let Some(segment) = segments.last_mut() else {
                        return Err(Error::new_spanned(
                            selector.span_tokens(),
                            "stylesheet! parent selector &:focus requires a parent selector",
                        ));
                    };

                    segment.push(pseudo);
                }
                _ => segments.push(vec![selector.expand()?]),
            }
        }

        let Some(target) = segments.pop() else {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                "stylesheet! rule requires a selector",
            ));
        };

        let target = expand_selector_segment(&leptatui, target);
        if segments.is_empty() {
            return Ok(target);
        }

        let ancestors = segments
            .into_iter()
            .map(|segment| expand_selector_segment(&leptatui, segment));

        Ok(quote! {
            #leptatui::StyleSelector::descendant(
                ::std::vec![#(#ancestors),*],
                #target,
            )
        })
    }

    /// Expands a supported view type selector identifier.
    ///
    /// # Arguments
    ///
    /// * `view_type` — Parsed view type identifier to lower.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a public `ViewType` variant.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if `view_type` is not a supported terminal view
    /// type.
    fn expand_view_type(view_type: &Ident) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();

        match view_type.to_string().as_str() {
            "Block" => Ok(quote! { #leptatui::ViewType::Block }),
            "Text" => Ok(quote! { #leptatui::ViewType::Text }),
            "Row" => Ok(quote! { #leptatui::ViewType::Row }),
            "Column" => Ok(quote! { #leptatui::ViewType::Column }),
            "Form" => Ok(quote! { #leptatui::ViewType::Form }),
            "Button" => Ok(quote! { #leptatui::ViewType::Button }),
            "Input" => Ok(quote! { #leptatui::ViewType::Input }),
            "TextArea" => Ok(quote! { #leptatui::ViewType::TextArea }),
            "Image" => Ok(quote! { #leptatui::ViewType::Image }),
            "ProgressBar" => Ok(quote! { #leptatui::ViewType::ProgressBar }),
            _ => Err(Error::new_spanned(
                view_type,
                "unsupported stylesheet type selector; expected Block, Text, Row, Column, Form, Button, Input, TextArea, Image, or ProgressBar",
            )),
        }
    }

    /// Expands a supported pseudo-selector identifier.
    ///
    /// # Arguments
    ///
    /// * `pseudo` — Parsed pseudo-selector identifier to lower.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a public `StyleSelector` expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if `pseudo` is not supported.
    fn expand_pseudo(pseudo: &Ident) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();

        match pseudo.to_string().as_str() {
            "focus" => Ok(quote! { #leptatui::StyleSelector::focus() }),
            _ => Err(Error::new_spanned(
                pseudo,
                "unsupported stylesheet pseudo-selector; expected :focus",
            )),
        }
    }

    /// Returns tokens that identify this selector in diagnostics.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the selector span source.
    fn span_tokens(&self) -> TokenStream {
        match self {
            Self::Type(ident) | Self::Pseudo(ident) | Self::ParentPseudo(ident) => {
                ident.to_token_stream()
            }
            Self::Class(name) | Self::Id(name) => name.to_token_stream(),
            Self::TypePseudo { view_type, pseudo } => quote! { #view_type : #pseudo },
        }
    }
}

/// Parsed class or id selector name.
pub(super) struct SelectorName {
    value: String,
    span: Span,
}

impl SelectorName {
    /// Parses an identifier name with optional dash-separated identifier segments.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let first: Ident = input.parse()?;
        let span = first.span();
        let mut value = first.to_string();

        while input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            let segment: Ident = input.parse()?;
            value.push('-');
            value.push_str(&segment.to_string());
        }

        Ok(Self { value, span })
    }

    /// Returns this selector name as a string literal for generated code.
    fn literal(&self) -> LitStr {
        LitStr::new(&self.value, self.span)
    }
}

impl ToTokens for SelectorName {
    /// Appends this selector name as a string literal token.
    ///
    /// # Arguments
    ///
    /// * `tokens` — Token stream receiving the selector literal.
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.literal().to_tokens(tokens);
    }
}

/// Expands one selector path segment into a runtime selector expression.
///
/// # Arguments
///
/// * `leptatui` — Token path to the Leptatui crate used in generated code.
/// * `selectors` — Runtime selector expressions in the same path segment.
///
/// # Returns
///
/// A [`TokenStream`] containing either a single selector expression or a
/// compound selector expression.
fn expand_selector_segment(leptatui: &TokenStream, selectors: Vec<TokenStream>) -> TokenStream {
    if selectors.len() == 1 {
        let mut selectors = selectors.into_iter();
        return selectors.next().expect("checked selector segment length");
    }

    quote! { #leptatui::StyleSelector::compound(::std::vec![#(#selectors),*]) }
}
