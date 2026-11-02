//! Element child validation and expansion.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Result};

use crate::view::{child::Child, text_content::TextContent};

use super::Element;

/// Owned content produced by one reactive text-like element.
pub(super) enum ReactiveTextKind {
    /// Content accepted through `Into<String>`.
    String,
    /// Content accepted through `Into<leptatui::RichText>`.
    RichText,
}

impl Element {
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
    pub(super) fn expand_single_child(
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
    pub(super) fn expand_child_list(
        &self,
        element_name: &str,
        wrap: impl FnOnce(TokenStream) -> TokenStream,
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

        Ok(wrap(Self::flatten_child_values(&expanded)))
    }

    /// Expands child expressions into one ordered, flattened view list.
    ///
    /// Scalar expressions contribute one view, while vector expressions splice
    /// all of their views into the surrounding child position.
    pub(super) fn flatten_child_values(children: &[TokenStream]) -> TokenStream {
        let leptatui = crate::crate_path::leptatui();

        quote! {{
            let mut __leptatui_children = ::std::vec::Vec::new();
            #(
                __leptatui_children.extend(
                    #leptatui::__private::__into_view_children(#children)
                );
            )*
            __leptatui_children
        }}
    }

    /// Expands a non-empty list of specifically named element children.
    ///
    /// # Arguments
    ///
    /// * `element_name` — Parent tag name used in diagnostics and ancestry.
    /// * `allowed_children` — Direct child tag names accepted by the parent.
    /// * `wrap` — Function wrapping expanded children in a builder call.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the wrapped child list.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the parent is empty or contains a child with
    /// an unsupported kind.
    pub(super) fn expand_element_child_list(
        &self,
        element_name: &str,
        allowed_children: &[&str],
        wrap: impl FnOnce(Vec<TokenStream>) -> TokenStream,
    ) -> Result<TokenStream> {
        if self.children.is_empty() {
            return Err(Error::new_spanned(
                &self.name,
                format!("{element_name} expects at least one child element"),
            ));
        }

        let mut expanded = Vec::with_capacity(self.children.len());
        for child in &self.children {
            let Child::Element(child) = child else {
                return Err(Error::new_spanned(
                    &self.name,
                    format!(
                        "{element_name} only accepts {} children",
                        allowed_children.join(" or ")
                    ),
                ));
            };

            if !allowed_children
                .iter()
                .any(|allowed| child.name == *allowed)
            {
                return Err(Error::new_spanned(
                    &child.name,
                    format!(
                        "{element_name} only accepts {} children",
                        allowed_children.join(" or ")
                    ),
                ));
            }

            expanded.push(child.expand_with_parent(Some(element_name))?);
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
    pub(super) fn expand_view_child(
        &self,
        child: &Child,
        element_name: &str,
    ) -> Result<TokenStream> {
        self.expand_view_child_value(child)?.ok_or_else(|| {
            Error::new_spanned(
                &self.name,
                format!("{element_name} expects element children or braced view expressions"),
            )
        })
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
    pub(super) fn expand_component_children(&self) -> Result<Vec<TokenStream>> {
        self.children
            .iter()
            .map(|child| self.expand_component_child(child))
            .collect()
    }

    /// Expands a child position that is passed through component children.
    pub(super) fn expand_component_child(&self, child: &Child) -> Result<TokenStream> {
        if let Some(child) = self.expand_view_child_value(child)? {
            return Ok(child);
        }

        let Child::Text(TextContent::Literal(value)) = child else {
            unreachable!("only text literals return None from expand_view_child_value")
        };
        let leptatui = crate::crate_path::leptatui();

        Ok(quote! { #leptatui::IntoView::into_view(#value) })
    }

    /// Expands a child element or braced expression into a view expression.
    pub(super) fn expand_view_child_value(&self, child: &Child) -> Result<Option<TokenStream>> {
        let leptatui = crate::crate_path::leptatui();

        match child {
            Child::Element(child) => {
                let parent = self.name.to_string();
                child.expand_with_parent(Some(&parent)).map(Some)
            }
            Child::Text(TextContent::Expr(expr))
                if matches!(expr.as_ref(), syn::Expr::Closure(_)) =>
            {
                Ok(Some(quote! { #leptatui::dynamic(#expr) }))
            }
            Child::Text(TextContent::Expr(expr)) => Ok(Some(quote! { #expr })),
            Child::Text(TextContent::Literal(_)) => Ok(None),
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
    pub(super) fn expand_text_like(
        &self,
        element_name: &str,
        kind: ReactiveTextKind,
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

        let leptatui = crate::crate_path::leptatui();
        let expanded = match content {
            TextContent::Literal(value) => return Ok(wrap(quote! { #value })),
            TextContent::Expr(expr) => match kind {
                ReactiveTextKind::String => {
                    let view = wrap(quote! { __leptatui_content });
                    quote! {
                        #leptatui::__private::__into_string_view(
                            #expr,
                            move |__leptatui_content| #view,
                        )
                    }
                }
                ReactiveTextKind::RichText => {
                    let view = wrap(quote! { __leptatui_content });
                    quote! {
                        #leptatui::__private::__into_rich_text_view(
                            #expr,
                            move |__leptatui_content| #view,
                        )
                    }
                }
            },
        };

        Ok(expanded)
    }
}
