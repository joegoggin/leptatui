use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Expr, Result};

use super::ast::{Child, Element, TextContent, ViewRoot};

impl ViewRoot {
    pub(super) fn expand(self) -> Result<TokenStream> {
        self.element.expand()
    }
}

impl Element {
    fn expand(&self) -> Result<TokenStream> {
        self.validate_attrs()?;

        match self.name.to_string().as_str() {
            "Block" => self.expand_single_child("Block", |child| {
                quote! { ::leptatui::block(#child) }
            }),
            "Row" => self.expand_child_list("Row", |children| {
                quote! { ::leptatui::row(::std::vec![#(#children),*]) }
            }),
            "Column" => self.expand_child_list("Column", |children| {
                quote! { ::leptatui::column(::std::vec![#(#children),*]) }
            }),
            "Text" => self.expand_text_like("Text", |content| {
                quote! { ::leptatui::text(#content) }
            }),
            "Button" => self.expand_text_like("Button", |content| {
                quote! { ::leptatui::button(#content) }
            }),
            _ => Err(Error::new_spanned(
                &self.name,
                "unsupported Leptatui element; expected Block, Text, Row, Column, or Button",
            )),
        }
    }

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

        let Child::Element(child) = &self.children[0] else {
            return Err(Error::new_spanned(
                &self.name,
                format!("{element_name} expects an element child"),
            ));
        };

        Ok(wrap(child.expand()?))
    }

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
            let Child::Element(child) = child else {
                return Err(Error::new_spanned(
                    &self.name,
                    format!("{element_name} expects element children"),
                ));
            };

            expanded.push(child.expand()?);
        }

        Ok(wrap(expanded))
    }

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
    fn expand(&self) -> TokenStream {
        match self {
            Self::Literal(value) => quote! { #value },
            Self::Expr(expr) if matches!(expr, Expr::Closure(_)) => quote! { (#expr)() },
            Self::Expr(expr) => quote! { #expr },
        }
    }
}
