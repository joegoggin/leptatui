//! Expansion for parsed `view!` macro syntax.
//!
//! This module validates supported element and child combinations, then emits
//! calls to Leptatui node builder functions.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Expr, Result};

use super::ast::{Child, Element, TextContent, ViewRoot};

impl ViewRoot {
    /// Expands the root element into generated node code.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the expanded root node.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the root element is unsupported or malformed.
    pub(super) fn expand(self) -> Result<TokenStream> {
        self.element.expand()
    }
}

impl Element {
    /// Expands this element based on its supported Leptatui tag name.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing node builder calls for this element.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if attributes, children, or the element name are
    /// unsupported.
    fn expand(&self) -> Result<TokenStream> {
        self.validate_attrs()?;
        let leptatui = crate::crate_path::leptatui();

        match self.name.to_string().as_str() {
            "Block" => self.expand_single_child("Block", |child| {
                quote! { #leptatui::block(#child) }
            }),
            "Row" => self.expand_child_list("Row", |children| {
                quote! { #leptatui::row(::std::vec![#(#children),*]) }
            }),
            "Column" => self.expand_child_list("Column", |children| {
                quote! { #leptatui::column(::std::vec![#(#children),*]) }
            }),
            "Text" => self.expand_text_like("Text", |content| {
                quote! { #leptatui::text(#content) }
            }),
            "Button" => self.expand_text_like("Button", |content| {
                quote! { #leptatui::button(#content) }
            }),
            _ => Err(Error::new_spanned(
                &self.name,
                "unsupported Leptatui element; expected Block, Text, Row, Column, or Button",
            )),
        }
    }

    /// Validates that every attribute name is currently accepted by `view!`.
    ///
    /// # Returns
    ///
    /// An empty [`syn::Result`] when all attributes are accepted.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if an attribute is not `class`, `id`, or `style`.
    fn validate_attrs(&self) -> Result<()> {
        for attr in &self.attrs {
            match attr.name.to_string().as_str() {
                "class" | "id" | "style" => {}
                _ => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "unsupported view! attribute; expected class, id, or style",
                    ));
                }
            }
        }

        Ok(())
    }

    /// Expands an element that must contain exactly one node child.
    ///
    /// # Arguments
    ///
    /// * `element_name` — Name to use in compile diagnostics.
    /// * `wrap` — Function that wraps the expanded child in this element's
    ///   builder call.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the wrapped child node.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the element does not have exactly one valid
    /// node child.
    fn expand_single_child(
        &self,
        element_name: &str,
        wrap: impl FnOnce(TokenStream) -> TokenStream,
    ) -> Result<TokenStream> {
        if self.children.len() != 1 {
            return Err(Error::new_spanned(
                &self.name,
                format!("{element_name} expects exactly one child element"),
            ));
        }

        let child = self.expand_node_child(&self.children[0], element_name)?;
        Ok(wrap(child))
    }

    /// Expands an element that must contain one or more node children.
    ///
    /// # Arguments
    ///
    /// * `element_name` — Name to use in compile diagnostics.
    /// * `wrap` — Function that wraps the expanded children in this element's
    ///   builder call.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the wrapped child list.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the element has no children or contains an
    /// invalid node child.
    fn expand_child_list(
        &self,
        element_name: &str,
        wrap: impl FnOnce(Vec<TokenStream>) -> TokenStream,
    ) -> Result<TokenStream> {
        if self.children.is_empty() {
            return Err(Error::new_spanned(
                &self.name,
                format!("{element_name} expects at least one child element"),
            ));
        }

        let mut expanded = Vec::new();
        for child in &self.children {
            expanded.push(self.expand_node_child(child, element_name)?);
        }

        Ok(wrap(expanded))
    }

    /// Expands a child position that expects a node-compatible value.
    ///
    /// # Arguments
    ///
    /// * `child` — Parsed child to expand.
    /// * `element_name` — Parent element name to use in diagnostics.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a node expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the child is a string literal where a node is
    /// required.
    fn expand_node_child(&self, child: &Child, element_name: &str) -> Result<TokenStream> {
        match child {
            Child::Element(child) => child.expand(),
            Child::Text(TextContent::Expr(expr)) => {
                let leptatui = crate::crate_path::leptatui();
                Ok(quote! { ::core::convert::Into::<#leptatui::Node>::into(#expr) })
            }
            Child::Text(TextContent::Literal(_)) => Err(Error::new_spanned(
                &self.name,
                format!("{element_name} expects element children or braced node expressions"),
            )),
        }
    }

    /// Expands an element that must contain exactly one text child.
    ///
    /// # Arguments
    ///
    /// * `element_name` — Name to use in compile diagnostics.
    /// * `wrap` — Function that wraps the expanded content in this element's
    ///   builder call.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the text-like builder call.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the element has the wrong number or kind of
    /// children.
    fn expand_text_like(
        &self,
        element_name: &str,
        wrap: impl FnOnce(TokenStream) -> TokenStream,
    ) -> Result<TokenStream> {
        if self.children.len() != 1 {
            return Err(Error::new_spanned(
                &self.name,
                format!("{element_name} expects exactly one text child"),
            ));
        }

        let Child::Text(content) = &self.children[0] else {
            return Err(Error::new_spanned(
                &self.name,
                format!("{element_name} expects text content, not child elements"),
            ));
        };

        Ok(wrap(content.expand()))
    }
}

impl TextContent {
    /// Expands text content into an expression suitable for text builders.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a literal, expression, or invoked closure.
    fn expand(&self) -> TokenStream {
        match self {
            Self::Literal(value) => quote! { #value },
            Self::Expr(expr) if matches!(expr.as_ref(), Expr::Closure(_)) => quote! { (#expr)() },
            Self::Expr(expr) => quote! { #expr },
        }
    }
}
