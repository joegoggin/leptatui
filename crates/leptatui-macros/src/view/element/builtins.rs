//! Built-in elements with required or specialized attributes.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Result};

use super::{
    Element,
    children::ReactiveTextKind,
    validation::{AttrKind, ValidatedAttr},
};

impl Element {
    /// Expands a controlled editable text element.
    ///
    /// # Arguments
    ///
    /// * `element_name` — Name to use in compile diagnostics.
    /// * `build` — Function that wraps the required value in a builder call.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing an editable text-control builder expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if children are present, `value` is missing,
    /// duplicate `value` attributes are supplied, or control attributes are
    /// invalid.
    pub(super) fn expand_editable_text_control(
        &self,
        element_name: &str,
        build: impl FnOnce(TokenStream) -> TokenStream,
    ) -> Result<TokenStream> {
        self.expand_required_attr_element(element_name, AttrKind::InputValue, "value", build)
    }

    /// Expands a path-backed image element.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing an image builder expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if children are present, `src` is missing,
    /// duplicate `src` attributes are supplied, or image attributes are invalid.
    pub(super) fn expand_image(&self) -> Result<TokenStream> {
        let leptatui = crate::crate_path::leptatui();
        self.expand_required_attr_element("Image", AttrKind::ImageSource, "src", |source| {
            quote! { #leptatui::image(#source) }
        })
    }

    /// Expands a progress bar element.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a progress bar builder expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if children are present, `value` is missing,
    /// duplicate `value` attributes are supplied, or progress bar attributes
    /// are invalid.
    pub(super) fn expand_progress_bar(&self) -> Result<TokenStream> {
        let leptatui = crate::crate_path::leptatui();
        self.expand_required_attr_element(
            "ProgressBar",
            AttrKind::ProgressValue,
            "value",
            |value| quote! { #leptatui::progress_bar(#value) },
        )
    }

    /// Expands an asynchronously loaded path-backed Markdown element.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] loading the Markdown view into its tree position.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if children are present, `src` is missing,
    /// attributes are duplicated, or Markdown options are invalid.
    pub(super) fn expand_markdown(&self) -> Result<TokenStream> {
        let (attrs, source) =
            self.validate_required_attr_element("Markdown", AttrKind::MarkdownSrc, "src")?;
        let leptatui = crate::crate_path::leptatui();
        let mut options = quote! { #leptatui::MarkdownOptions::default() };
        let editable = attrs
            .iter()
            .find(|validated| validated.kind == AttrKind::Editable)
            .map_or_else(
                || quote! { false },
                |validated| validated.attr.value.to_tokens(),
            );

        if let Some(line_numbers) = attrs
            .iter()
            .find(|validated| validated.kind == AttrKind::LineNumbers)
        {
            let value = line_numbers.attr.value.to_tokens();
            options = quote! { (#options).line_numbers(#value) };
        }

        self.expand_attrs(
            quote! {
                #leptatui::__private::__markdown_element(
                    #source,
                    #options,
                    #editable,
                    ::core::file!(),
                    ::core::line!(),
                )
            },
            &attrs,
        )
    }

    /// Expands a rich-text link with one required `href` attribute.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] constructing a standalone link view.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if text content or `href` is missing, attributes
    /// are duplicated, or an unsupported attribute is supplied.
    pub(super) fn expand_link(&self) -> Result<TokenStream> {
        let attrs = self.validate_attrs()?;
        let hrefs = attrs
            .iter()
            .filter(|validated| validated.kind == AttrKind::Href)
            .collect::<Vec<_>>();
        let href = match hrefs.as_slice() {
            [href] => href.attr.value.to_tokens(),
            [] => {
                return Err(Error::new_spanned(
                    &self.name,
                    "Link requires an href attribute",
                ));
            }
            [first, ..] => {
                return Err(Error::new_spanned(
                    &first.attr.name,
                    "Link expects exactly one href attribute",
                ));
            }
        };
        let leptatui = crate::crate_path::leptatui();
        let view = self.expand_text_like("Link", ReactiveTextKind::RichText, |content| {
            quote! { #leptatui::link(#content, (#href).clone()) }
        })?;
        self.expand_attrs(view, &attrs)
    }

    /// Expands a self-contained element with one required attribute.
    ///
    /// # Arguments
    ///
    /// * `element_name` — Name to use in compile diagnostics.
    /// * `required_kind` — Attribute kind that must appear exactly once.
    /// * `required_attr_name` — Attribute name to use in diagnostics.
    /// * `build` — Function that wraps the required attribute value in a
    ///   builder call.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the expanded builder expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if children are present, the required attribute
    /// is missing or duplicated, or other attributes are invalid.
    pub(super) fn expand_required_attr_element(
        &self,
        element_name: &str,
        required_kind: AttrKind,
        required_attr_name: &str,
        build: impl FnOnce(TokenStream) -> TokenStream,
    ) -> Result<TokenStream> {
        let (attrs, value) =
            self.validate_required_attr_element(element_name, required_kind, required_attr_name)?;
        self.expand_attrs(build(value), &attrs)
    }

    /// Validates a self-contained element and returns its required attribute.
    ///
    /// # Arguments
    ///
    /// * `element_name` — Name to use in compile diagnostics.
    /// * `required_kind` — Attribute kind that must appear exactly once.
    /// * `required_attr_name` — Attribute name to use in diagnostics.
    ///
    /// # Returns
    ///
    /// A tuple containing the validated attributes and required value tokens.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if children are present, the required attribute
    /// is missing or duplicated, or another attribute is invalid.
    pub(super) fn validate_required_attr_element(
        &self,
        element_name: &str,
        required_kind: AttrKind,
        required_attr_name: &str,
    ) -> Result<(Vec<ValidatedAttr<'_>>, TokenStream)> {
        if !self.children.is_empty() {
            return Err(Error::new_spanned(
                &self.name,
                format!("{element_name} does not accept children"),
            ));
        }

        let attrs = self.validate_attrs()?;
        let required_attrs = attrs
            .iter()
            .filter(|validated| validated.kind == required_kind)
            .collect::<Vec<_>>();
        let required_attr = match required_attrs.as_slice() {
            [required_attr] => *required_attr,
            [] => {
                return Err(Error::new_spanned(
                    &self.name,
                    format!("{element_name} requires a {required_attr_name} attribute"),
                ));
            }
            [first, ..] => {
                return Err(Error::new_spanned(
                    &first.attr.name,
                    format!("{element_name} expects exactly one {required_attr_name} attribute"),
                ));
            }
        };

        let value = required_attr.attr.value.to_tokens();
        Ok((attrs, value))
    }
}
