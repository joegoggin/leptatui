//! Expansion for declarative router elements.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Expr, Result};

use crate::view::child::Child;

use super::{Element, children::ReactiveTextKind};

impl Element {
    /// Expands a nested route outlet.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] resolving the active outlet factory.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the outlet has attributes or children.
    pub(super) fn expand_outlet(&self) -> Result<TokenStream> {
        if !self.attrs.is_empty() || !self.children.is_empty() {
            return Err(Error::new_spanned(
                &self.name,
                "Outlet does not accept attributes or children",
            ));
        }
        let leptatui = crate::crate_path::leptatui();
        Ok(quote! { #leptatui::__private::__outlet() })
    }

    /// Expands a route-aware anchor.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] constructing an internal route link.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if `href` is missing, attributes are duplicated,
    /// or the anchor does not contain one text child.
    pub(super) fn expand_route_link(&self) -> Result<TokenStream> {
        let href = self.required_attr("href")?;
        let exact = self
            .optional_attr("exact")?
            .unwrap_or_else(|| quote! { false });
        for attr in &self.attrs {
            if !matches!(
                attr.name.to_string().as_str(),
                "href" | "exact" | "class" | "id" | "style"
            ) {
                return Err(Error::new_spanned(
                    &attr.name,
                    "unsupported A attribute; expected href, exact, class, id, or style",
                ));
            }
        }
        let leptatui = crate::crate_path::leptatui();
        let view = self.expand_text_like("A", ReactiveTextKind::RichText, |content| {
            quote! { #leptatui::route_link(#content, (#href).clone(), (#exact).clone()) }
        })?;
        let mut expanded = view;
        for name in ["class", "id", "style"] {
            if let Some(value) = self.optional_attr(name)? {
                expanded = match name {
                    "class" => quote! { (#expanded).with_classes(#value) },
                    "id" => quote! { (#expanded).with_id(#value) },
                    "style" => quote! { (#expanded).with_inline_style(#value) },
                    _ => unreachable!(),
                };
            }
        }
        Ok(expanded)
    }

    /// Expands a route list and its fallback factory.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] constructing the matched route boundary.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if fallback is missing or children are not route
    /// declarations.
    pub(super) fn expand_routes(&self) -> Result<TokenStream> {
        let fallback = self.required_attr("fallback")?;
        if self.attrs.len() != 1 {
            return Err(Error::new_spanned(
                &self.name,
                "Routes only accepts one fallback attribute",
            ));
        }
        let children = self.expand_route_children("Routes")?;
        let fallback = route_factory(fallback)?;
        let leptatui = crate::crate_path::leptatui();
        Ok(quote! {
            #leptatui::__private::__routes(
                ::std::rc::Rc::new(#fallback),
                ::std::vec![#(#leptatui::IntoView::into_view(#children)),*],
            )
        })
    }

    /// Expands a leaf or parent route declaration.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a non-rendering route definition.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if path or view is missing, attributes are
    /// unsupported, or child declaration structure is invalid.
    pub(super) fn expand_route_definition(&self) -> Result<TokenStream> {
        let path = self.required_attr("path")?;
        let route_view = route_factory(self.required_attr("view")?)?;
        if self.attrs.len() != 2 {
            return Err(Error::new_spanned(
                &self.name,
                "Route declarations only accept path and view attributes",
            ));
        }
        let children = if self.name == "ParentRoute" {
            self.expand_route_children("ParentRoute")?
        } else {
            if !self.children.is_empty() {
                return Err(Error::new_spanned(
                    &self.name,
                    "Route does not accept children; use ParentRoute",
                ));
            }
            Vec::new()
        };
        let leptatui = crate::crate_path::leptatui();
        Ok(quote! {
            #leptatui::__private::__route_definition(
                #path,
                ::std::rc::Rc::new(#route_view),
                ::std::vec![#(#leptatui::IntoView::into_view(#children)),*],
            )
        })
    }

    /// Expands direct route-declaration children.
    ///
    /// # Arguments
    ///
    /// * `parent` — Parent name used in diagnostics.
    ///
    /// # Returns
    ///
    /// A vector of expanded route declaration tokens.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a child is not `Route` or `ParentRoute`.
    fn expand_route_children(&self, parent: &str) -> Result<Vec<TokenStream>> {
        if self.children.is_empty() {
            return Err(Error::new_spanned(
                &self.name,
                format!("{parent} expects at least one route declaration"),
            ));
        }
        self.children
            .iter()
            .map(|child| {
                let Child::Element(element) = child else {
                    return Err(Error::new_spanned(
                        &self.name,
                        format!("{parent} only accepts Route or ParentRoute children"),
                    ));
                };
                if !matches!(element.name.to_string().as_str(), "Route" | "ParentRoute") {
                    return Err(Error::new_spanned(
                        &element.name,
                        format!("{parent} only accepts Route or ParentRoute children"),
                    ));
                }
                element.expand_with_parent(Some(parent))
            })
            .collect()
    }

    /// Returns one required attribute value.
    ///
    /// # Arguments
    ///
    /// * `name` — Attribute name to locate.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the attribute value.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the attribute is absent or duplicated.
    fn required_attr(&self, name: &str) -> Result<TokenStream> {
        self.optional_attr(name)?.ok_or_else(|| {
            Error::new_spanned(
                &self.name,
                format!("{} requires a {name} attribute", self.name),
            )
        })
    }

    /// Returns one optional attribute value.
    ///
    /// # Arguments
    ///
    /// * `name` — Attribute name to locate.
    ///
    /// # Returns
    ///
    /// An optional [`TokenStream`] containing the attribute value.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the attribute is duplicated.
    fn optional_attr(&self, name: &str) -> Result<Option<TokenStream>> {
        let values = self
            .attrs
            .iter()
            .filter(|attr| attr.name == name)
            .collect::<Vec<_>>();
        match values.as_slice() {
            [] => Ok(None),
            [value] => Ok(Some(value.value.to_tokens())),
            [duplicate, ..] => Err(Error::new_spanned(
                &duplicate.name,
                format!("{} expects at most one {name} attribute", self.name),
            )),
        }
    }
}

/// Converts a component type or closure expression into an erased view factory.
///
/// # Arguments
///
/// * `tokens` — Attribute tokens naming a component or closure.
///
/// # Returns
///
/// A closure expression returning [`AnyView`](leptatui::AnyView).
///
/// # Errors
///
/// Returns [`syn::Error`] if the expression cannot be used as a route view.
fn route_factory(tokens: TokenStream) -> Result<TokenStream> {
    let expression: Expr = syn::parse2(tokens)?;
    let leptatui = crate::crate_path::leptatui();
    match expression {
        Expr::Path(path) => Ok(quote! {
            move || #leptatui::IntoView::into_view(#path::new())
        }),
        Expr::Closure(closure) if closure.inputs.is_empty() => {
            let capture = closure.capture;
            let body = closure.body;
            Ok(quote! {
                #capture || #leptatui::IntoView::into_view((|| #body)())
            })
        }
        expression => Err(Error::new_spanned(
            expression,
            "route view must be a component type or zero-argument closure",
        )),
    }
}
