//! Selector model for `stylesheet!` syntax.
//!
//! This module parses type, class, id, focus, and type-focus selectors and
//! lowers them into public `StyleSelector` constructor calls.

use proc_macro2::TokenStream;
use quote::quote;
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
    /// id, focus, or type-focus selector.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
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
            "stylesheet! selector must be a type, .class, #id, :focus, or Type:focus selector",
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
    /// pseudo-selector.
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
        }
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
}
