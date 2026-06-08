//! Selector model for `stylesheet!` syntax.
//!
//! This module parses type, class, id, focus, type-focus, and nested `&:focus`
//! selectors and lowers them into public `StyleSelector` constructor calls.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Error, Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream},
};

/// Parsed terminal stylesheet selector.
pub(super) enum Selector {
    /// Node type selector such as `Text`.
    Type(Ident),
    /// Class selector such as `.primary`.
    Class(Ident),
    /// Id selector such as `#submit`.
    Id(Ident),
    /// Pseudo selector such as `:focus`.
    Pseudo(Ident),
    /// Compound type and pseudo selector such as `Button:focus`.
    TypePseudo {
        /// Node type part of the compound selector.
        node_type: Ident,
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
            return Ok(Self::Class(input.parse()?));
        }

        if input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            return Ok(Self::Id(input.parse()?));
        }

        if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            return Ok(Self::Pseudo(input.parse()?));
        }

        if input.peek(Ident) {
            let node_type = input.parse()?;
            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                let pseudo = input.parse()?;
                return Ok(Self::TypePseudo { node_type, pseudo });
            }

            return Ok(Self::Type(node_type));
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
    /// Returns [`syn::Error`] if the selector uses an unsupported node type or
    /// pseudo-selector, or if a parent-reference selector is expanded without a
    /// selector path.
    pub(super) fn expand(&self) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();

        match self {
            Self::Type(node_type) => {
                let node_type = Self::expand_node_type(node_type)?;
                Ok(quote! { #leptatui::StyleSelector::node_type(#node_type) })
            }
            Self::Class(class) => {
                let class = LitStr::new(&class.to_string(), class.span());
                Ok(quote! { #leptatui::StyleSelector::class(#class) })
            }
            Self::Id(id) => {
                let id = LitStr::new(&id.to_string(), id.span());
                Ok(quote! { #leptatui::StyleSelector::id(#id) })
            }
            Self::Pseudo(pseudo) => Self::expand_pseudo(pseudo),
            Self::TypePseudo { node_type, pseudo } => {
                let node_type = Self::expand_node_type(node_type)?;
                let pseudo = Self::expand_pseudo(pseudo)?;

                Ok(quote! {
                    #leptatui::StyleSelector::compound(::std::vec![
                        #leptatui::StyleSelector::node_type(#node_type),
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

    /// Expands a supported node type selector identifier.
    ///
    /// # Arguments
    ///
    /// * `node_type` — Parsed node type identifier to lower.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a public `NodeType` variant.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if `node_type` is not a supported terminal node
    /// type.
    fn expand_node_type(node_type: &Ident) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();

        match node_type.to_string().as_str() {
            "Block" => Ok(quote! { #leptatui::NodeType::Block }),
            "Text" => Ok(quote! { #leptatui::NodeType::Text }),
            "Row" => Ok(quote! { #leptatui::NodeType::Row }),
            "Column" => Ok(quote! { #leptatui::NodeType::Column }),
            "Button" => Ok(quote! { #leptatui::NodeType::Button }),
            _ => Err(Error::new_spanned(
                node_type,
                "unsupported stylesheet type selector; expected Block, Text, Row, Column, or Button",
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
            Self::Type(ident)
            | Self::Class(ident)
            | Self::Id(ident)
            | Self::Pseudo(ident)
            | Self::ParentPseudo(ident) => ident.to_token_stream(),
            Self::TypePseudo { node_type, pseudo } => quote! { #node_type : #pseudo },
        }
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
