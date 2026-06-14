//! Element model for `view!` syntax.
//!
//! This module parses supported XML-like terminal elements and expands them
//! into Leptatui view builder calls.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Error, Ident, Result, Token,
    parse::{Parse, ParseStream},
};

use crate::view::utils::parse::{next_is_closing_tag, next_is_self_closing_tag_end};

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

        while !input.peek(Token![>]) && !next_is_self_closing_tag_end(input) {
            attrs.push(input.parse()?);
        }

        if next_is_self_closing_tag_end(input) {
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;

            return Ok(Self {
                name,
                attrs,
                children: Vec::new(),
            });
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
    /// A [`TokenStream`] containing view builder calls for this element.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if attributes, children, or the element name are
    /// unsupported.
    pub(super) fn expand(&self) -> Result<TokenStream> {
        match self.name.to_string().as_str() {
            "Block" => self.expand_single_child("Block", |child| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::block(#child) }
            }),
            "Row" => self.expand_child_list("Row", |children| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::row(::std::vec![#(#children),*]) }
            }),
            "Column" => self.expand_child_list("Column", |children| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::column(::std::vec![#(#children),*]) }
            }),
            "Text" => self.expand_text_like("Text", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::text(#content) }
            }),
            "Button" => self.expand_text_like("Button", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::button(#content) }
            }),
            _ if is_component_name(&self.name) => self.expand_component(),
            _ => {
                return Err(Error::new_spanned(
                    &self.name,
                    "unsupported Leptatui element; expected Block, Text, Row, Column, Button, or a PascalCase component",
                ));
            }
        }
        .and_then(|view| {
            if is_builtin_element(&self.name) {
                let attrs = self.validate_attrs()?;
                self.expand_attrs(view, &attrs)
            } else {
                Ok(view)
            }
        })
    }

    /// Expands a PascalCase component tag into a component constructor call.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a view expression for the component.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if explicit `children` and nested children are
    /// both supplied.
    fn expand_component(&self) -> Result<TokenStream> {
        if let Some(attr) = self
            .attrs
            .iter()
            .find(|attr| attr.name == "children" && !self.children.is_empty())
        {
            return Err(Error::new_spanned(
                &attr.name,
                "view! component cannot specify a children prop and child content",
            ));
        }

        let leptatui = crate::utils::crate_path::leptatui();
        let name = &self.name;
        let component = if self.attrs.is_empty() && self.children.is_empty() {
            quote! { #name::new() }
        } else {
            let props = format_ident!("{name}Props");
            let attr_setters = self.attrs.iter().map(|attr| {
                let name = &attr.name;
                let value = attr.value.to_tokens();

                quote! { .#name(#value) }
            });
            let children = if self.children.is_empty() {
                TokenStream::new()
            } else {
                let children = self.expand_component_children()?;
                quote! {
                    .children(::std::boxed::Box::new(move || {
                        ::std::vec![#(#children),*]
                    }))
                }
            };

            quote! {
                #name::with_props(
                    #props::builder()
                        #(#attr_setters)*
                        #children
                        .build()
                )
            }
        };

        Ok(quote! {
            #leptatui::__private::__component_factory(move || #component)
        })
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
    /// * `view` — Already-expanded view builder expression.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the view expression wrapped with metadata
    /// setters.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if `style` or `on_press` receives a literal.
    fn expand_attrs(&self, view: TokenStream, attrs: &[ValidatedAttr<'_>]) -> Result<TokenStream> {
        let mut expanded = view;

        for ValidatedAttr { attr, kind } in attrs {
            let value = attr.value.to_tokens();
            expanded = match kind {
                AttrKind::Class => quote! { (#expanded).with_classes(#value) },
                AttrKind::Id => quote! { (#expanded).with_id(#value) },
                AttrKind::Style => {
                    if attr.value.is_literal() {
                        return Err(Error::new_spanned(
                            &attr.name,
                            "view! style attribute must be a TuiStyle expression",
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

            if !matches!(*kind, AttrKind::OnPress | AttrKind::Style)
                && attr.value.is_unbraced_expr()
            {
                return Err(Error::new_spanned(
                    &attr.name,
                    "view! attribute values must be string literals or braced expressions",
                ));
            }
        }

        Ok(expanded)
    }

    /// Expands an element that must contain exactly one view child.
    ///
    /// # Arguments
    ///
    /// * `element_name` — Name to use in compile diagnostics.
    /// * `wrap` — Function that wraps the expanded child in this element's
    ///   builder call.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the wrapped child view.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the element does not have exactly one valid
    /// view child.
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

        let child = self.expand_view_child(&self.children[0], element_name)?;
        Ok(wrap(child))
    }

    /// Expands an element that must contain one or more view children.
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
    /// invalid view child.
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
            expanded.push(self.expand_view_child(child, element_name)?);
        }

        Ok(wrap(expanded))
    }

    /// Expands a child position that expects a view-compatible value.
    ///
    /// # Arguments
    ///
    /// * `child` — Parsed child to expand.
    /// * `element_name` — Parent element name to use in diagnostics.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a view expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the child is a string literal where a view is
    /// required.
    fn expand_view_child(&self, child: &Child, element_name: &str) -> Result<TokenStream> {
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
                Ok(quote! { ::core::convert::Into::<#leptatui::View>::into(#expr) })
            }
            Child::Text(TextContent::Literal(_)) => Err(Error::new_spanned(
                &self.name,
                format!("{element_name} expects element children or braced view expressions"),
            )),
        }
    }

    /// Expands all children passed through a component `children` prop.
    ///
    /// # Returns
    ///
    /// A list of child view expressions.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if any nested element is malformed.
    fn expand_component_children(&self) -> Result<Vec<TokenStream>> {
        self.children
            .iter()
            .map(|child| self.expand_component_child(child))
            .collect()
    }

    /// Expands a child position that is passed through component children.
    fn expand_component_child(&self, child: &Child) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();

        match child {
            Child::Element(child) => child.expand(),
            Child::Text(TextContent::Expr(expr))
                if matches!(expr.as_ref(), syn::Expr::Closure(_)) =>
            {
                Ok(quote! { #leptatui::dynamic(#expr) })
            }
            Child::Text(TextContent::Expr(expr)) => {
                Ok(quote! { ::core::convert::Into::<#leptatui::View>::into(#expr) })
            }
            Child::Text(TextContent::Literal(value)) => {
                Ok(quote! { ::core::convert::Into::<#leptatui::View>::into(#value) })
            }
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

/// Returns whether an identifier is one of the built-in `view!` elements.
fn is_builtin_element(name: &Ident) -> bool {
    matches!(
        name.to_string().as_str(),
        "Block" | "Row" | "Column" | "Text" | "Button"
    )
}

/// Returns whether an identifier should be treated as a component tag.
fn is_component_name(name: &Ident) -> bool {
    name.to_string()
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
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
