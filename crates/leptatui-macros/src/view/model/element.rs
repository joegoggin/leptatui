//! Element model for `view!` syntax.
//!
//! This module parses supported XML-like terminal elements and expands them
//! into Leptatui node builder calls.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Ident, Result, Token,
    parse::{Parse, ParseStream},
};

use crate::view::utils::parse::next_is_closing_tag;

use self::attr_validation::{AttrKind, ValidatedAttr};

use super::{attr::Attr, child::Child, text_content::TextContent};

/// Parsed terminal element with attributes and children.
pub(super) struct Element {
    /// Element tag name, such as `Text` or `Column`.
    pub(super) name: Ident,
    /// Attribute names attached to the element.
    pub(super) attrs: Vec<Attr>,
    /// Child elements or text content nested inside this element.
    pub(super) children: Vec<Child>,
}

impl Parse for Element {
    /// Parses an opening tag, children, and matching closing tag.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at an opening `<`.
    ///
    /// # Returns
    ///
    /// An [`Element`] containing the tag name, attributes, and children.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the element starts with a closing tag, has
    /// invalid syntax, or closes with a mismatched tag name.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![<]>()?;

        if input.peek(Token![/]) {
            return Err(input.error("view! element cannot start with a closing tag"));
        }

        let name: Ident = input.parse()?;
        let mut attrs = Vec::new();

        while !input.peek(Token![>]) {
            attrs.push(input.parse()?);
        }

        input.parse::<Token![>]>()?;

        let mut children = Vec::new();
        while !input.is_empty() && !next_is_closing_tag(input) {
            children.push(input.parse()?);
        }

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        let closing_name: Ident = input.parse()?;
        input.parse::<Token![>]>()?;

        if closing_name != name {
            return Err(Error::new_spanned(
                closing_name,
                format!("expected closing tag </{}>", name),
            ));
        }

        Ok(Self {
            name,
            attrs,
            children,
        })
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
    pub(super) fn expand(&self) -> Result<TokenStream> {
        let attrs = self.validate_attrs()?;
        let leptatui = crate::utils::crate_path::leptatui();

        let node = match self.name.to_string().as_str() {
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
        }?;

        self.expand_attrs(node, &attrs)
    }

    /// Validates and classifies every attribute name accepted by `view!`.
    ///
    /// # Returns
    ///
    /// Attribute references paired with their accepted kinds.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if an attribute is unsupported for the element.
    fn validate_attrs(&self) -> Result<Vec<ValidatedAttr<'_>>> {
        let element_name = self.name.to_string();
        let mut attrs = Vec::with_capacity(self.attrs.len());

        for attr in &self.attrs {
            let kind = match attr.name.to_string().as_str() {
                "class" => AttrKind::Class,
                "id" => AttrKind::Id,
                "style" => AttrKind::Style,
                "on_press" if element_name == "Button" => AttrKind::OnPress,
                "on_press" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! on_press attribute is only supported on Button",
                    ));
                }
                _ => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "unsupported view! attribute; expected class, id, style, or button on_press",
                    ));
                }
            };

            attrs.push(ValidatedAttr { attr, kind });
        }

        Ok(attrs)
    }

    /// Expands supported attributes into selector metadata setters.
    ///
    /// # Arguments
    ///
    /// * `node` — Already-expanded node builder expression.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the node expression wrapped with metadata
    /// setters.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if `style` or `on_press` receives a literal.
    fn expand_attrs(&self, node: TokenStream, attrs: &[ValidatedAttr<'_>]) -> Result<TokenStream> {
        let mut expanded = node;

        for ValidatedAttr { attr, kind } in attrs {
            let value = attr.value.to_tokens();
            expanded = match kind {
                AttrKind::Class => quote! { (#expanded).with_classes(#value) },
                AttrKind::Id => quote! { (#expanded).with_id(#value) },
                AttrKind::Style => {
                    if attr.value.is_literal() {
                        return Err(Error::new_spanned(
                            &attr.name,
                            "view! style attribute must be a braced TuiStyle expression",
                        ));
                    }

                    quote! { (#expanded).with_inline_style(#value) }
                }
                AttrKind::OnPress => {
                    if attr.value.is_literal() {
                        return Err(Error::new_spanned(
                            &attr.name,
                            "view! on_press attribute must be a callback expression",
                        ));
                    }

                    quote! { (#expanded).on_press(#value) }
                }
            };
        }

        Ok(expanded)
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
            Child::Text(TextContent::Expr(expr))
                if matches!(expr.as_ref(), syn::Expr::Closure(_)) =>
            {
                let leptatui = crate::utils::crate_path::leptatui();
                Ok(quote! { #leptatui::dynamic(#expr) })
            }
            Child::Text(TextContent::Expr(expr)) => {
                let leptatui = crate::utils::crate_path::leptatui();
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

/// Internal attribute validation output for `Element` expansion.
mod attr_validation {
    use super::Attr;

    /// Supported `view!` attribute kinds after validation.
    #[derive(Clone, Copy)]
    pub(super) enum AttrKind {
        /// `class` selector metadata.
        Class,
        /// `id` selector metadata.
        Id,
        /// Inline `style` override.
        Style,
        /// Button activation callback.
        OnPress,
    }

    /// Attribute paired with its validated kind.
    pub(super) struct ValidatedAttr<'a> {
        /// Parsed source attribute.
        pub(super) attr: &'a Attr,
        /// Accepted attribute kind.
        pub(super) kind: AttrKind,
    }
}
