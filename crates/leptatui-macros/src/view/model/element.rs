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
        if self.name == "Input" {
            return self.expand_editable_text_control("Input", |value| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::input(#value) }
            });
        }

        if self.name == "TextArea" {
            return self.expand_editable_text_control("TextArea", |value| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::text_area(#value) }
            });
        }

        if self.name == "Image" {
            return self.expand_image();
        }

        if self.name == "ProgressBar" {
            return self.expand_progress_bar();
        }

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
            "Form" => self.expand_child_list("Form", |children| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::form(::std::vec![#(#children),*]) }
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
                    "unsupported Leptatui element; expected Block, Text, Row, Column, Form, Button, Input, TextArea, Image, ProgressBar, or a PascalCase component",
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
    fn expand_editable_text_control(
        &self,
        element_name: &str,
        build: impl FnOnce(TokenStream) -> TokenStream,
    ) -> Result<TokenStream> {
        if !self.children.is_empty() {
            return Err(Error::new_spanned(
                &self.name,
                format!("{element_name} does not accept children"),
            ));
        }

        let attrs = self.validate_attrs()?;
        let value_attrs = attrs
            .iter()
            .filter(|validated| matches!(validated.kind, AttrKind::InputValue))
            .collect::<Vec<_>>();
        let value_attr = match value_attrs.as_slice() {
            [value_attr] => *value_attr,
            [] => {
                return Err(Error::new_spanned(
                    &self.name,
                    format!("{element_name} requires a value attribute"),
                ));
            }
            [first, ..] => {
                return Err(Error::new_spanned(
                    &first.attr.name,
                    format!("{element_name} expects exactly one value attribute"),
                ));
            }
        };

        let value = value_attr.attr.value.to_tokens();
        self.expand_attrs(build(value), &attrs)
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
    fn expand_image(&self) -> Result<TokenStream> {
        if !self.children.is_empty() {
            return Err(Error::new_spanned(
                &self.name,
                "Image does not accept children",
            ));
        }

        let attrs = self.validate_attrs()?;
        let source_attrs = attrs
            .iter()
            .filter(|validated| matches!(validated.kind, AttrKind::ImageSource))
            .collect::<Vec<_>>();
        let source_attr = match source_attrs.as_slice() {
            [source_attr] => *source_attr,
            [] => {
                return Err(Error::new_spanned(
                    &self.name,
                    "Image requires a src attribute",
                ));
            }
            [first, ..] => {
                return Err(Error::new_spanned(
                    &first.attr.name,
                    "Image expects exactly one src attribute",
                ));
            }
        };

        let source = source_attr.attr.value.to_tokens();
        let leptatui = crate::utils::crate_path::leptatui();
        self.expand_attrs(quote! { #leptatui::image(#source) }, &attrs)
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
    fn expand_progress_bar(&self) -> Result<TokenStream> {
        if !self.children.is_empty() {
            return Err(Error::new_spanned(
                &self.name,
                "ProgressBar does not accept children",
            ));
        }

        let attrs = self.validate_attrs()?;
        let value_attrs = attrs
            .iter()
            .filter(|validated| matches!(validated.kind, AttrKind::ProgressValue))
            .collect::<Vec<_>>();
        let value_attr = match value_attrs.as_slice() {
            [value_attr] => *value_attr,
            [] => {
                return Err(Error::new_spanned(
                    &self.name,
                    "ProgressBar requires a value attribute",
                ));
            }
            [first, ..] => {
                return Err(Error::new_spanned(
                    &first.attr.name,
                    "ProgressBar expects exactly one value attribute",
                ));
            }
        };

        let value = value_attr.attr.value.to_tokens();
        let leptatui = crate::utils::crate_path::leptatui();
        self.expand_attrs(quote! { #leptatui::progress_bar(#value) }, &attrs)
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
        let preserve_on_reconcile = if self.attrs.is_empty() && self.children.is_empty() {
            quote! { true }
        } else {
            quote! { false }
        };
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
            #leptatui::__private::__component_factory(#preserve_on_reconcile, move || #component)
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
                "src" if element_name == "Image" => AttrKind::ImageSource,
                "src" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! src attribute is only supported on Image",
                    ));
                }
                "alt" if element_name == "Image" => AttrKind::Alt,
                "alt" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! alt attribute is only supported on Image",
                    ));
                }
                "value" if matches!(element_name.as_str(), "Input" | "TextArea") => {
                    AttrKind::InputValue
                }
                "value" if element_name == "ProgressBar" => AttrKind::ProgressValue,
                "value" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! value attribute is only supported on Input, TextArea, or ProgressBar",
                    ));
                }
                "label" if element_name == "ProgressBar" => AttrKind::Label,
                "label" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! label attribute is only supported on ProgressBar",
                    ));
                }
                "placeholder" if matches!(element_name.as_str(), "Input" | "TextArea") => {
                    AttrKind::Placeholder
                }
                "placeholder" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! placeholder attribute is only supported on Input or TextArea",
                    ));
                }
                "on_press" if element_name == "Button" => AttrKind::OnPress,
                "on_press" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! on_press attribute is only supported on Button",
                    ));
                }
                "on_submit" if element_name == "Form" => AttrKind::OnSubmit,
                "on_submit" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! on_submit attribute is only supported on Form",
                    ));
                }
                "on_cancel" if element_name == "Form" => AttrKind::OnCancel,
                "on_cancel" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! on_cancel attribute is only supported on Form",
                    ));
                }
                "on_input" if matches!(element_name.as_str(), "Input" | "TextArea") => {
                    AttrKind::OnInput
                }
                "on_input" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! on_input attribute is only supported on Input or TextArea",
                    ));
                }
                _ => {
                    let message = match element_name.as_str() {
                        "Button" => {
                            "unsupported view! attribute; expected class, id, style, or on_press"
                        }
                        "Form" => {
                            "unsupported view! attribute; expected class, id, style, on_submit, or on_cancel"
                        }
                        "Input" | "TextArea" => {
                            "unsupported view! attribute; expected class, id, style, value, placeholder, or on_input"
                        }
                        "Image" => {
                            "unsupported view! attribute; expected class, id, style, src, or alt"
                        }
                        "ProgressBar" => {
                            "unsupported view! attribute; expected class, id, style, value, or label"
                        }
                        _ => "unsupported view! attribute; expected class, id, or style",
                    };
                    return Err(Error::new_spanned(&attr.name, message));
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
                    reject_literal_callback(attr, "on_press")?;
                    quote! { (#expanded).on_press(#value) }
                }
                AttrKind::OnSubmit => {
                    reject_literal_callback(attr, "on_submit")?;
                    quote! { (#expanded).on_submit(#value) }
                }
                AttrKind::OnCancel => {
                    reject_literal_callback(attr, "on_cancel")?;
                    quote! { (#expanded).on_cancel(#value) }
                }
                AttrKind::InputValue => expanded,
                AttrKind::ImageSource => expanded,
                AttrKind::ProgressValue => expanded,
                AttrKind::Placeholder => quote! { (#expanded).placeholder(#value) },
                AttrKind::Alt => quote! { (#expanded).alt(#value) },
                AttrKind::Label => quote! { (#expanded).label(#value) },
                AttrKind::OnInput => {
                    reject_literal_callback(attr, "on_input")?;
                    quote! { (#expanded).on_input(#value) }
                }
            };

            if !matches!(
                *kind,
                AttrKind::OnPress
                    | AttrKind::OnSubmit
                    | AttrKind::OnCancel
                    | AttrKind::OnInput
                    | AttrKind::InputValue
                    | AttrKind::ImageSource
                    | AttrKind::ProgressValue
                    | AttrKind::Style
            ) && attr.value.is_unbraced_expr()
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
    fn expand_component_children(&self) -> Result<Vec<TokenStream>> {
        self.children
            .iter()
            .map(|child| self.expand_component_child(child))
            .collect()
    }

    /// Expands a child position that is passed through component children.
    fn expand_component_child(&self, child: &Child) -> Result<TokenStream> {
        if let Some(child) = self.expand_view_child_value(child)? {
            return Ok(child);
        }

        let Child::Text(TextContent::Literal(value)) = child else {
            unreachable!("only text literals return None from expand_view_child_value")
        };
        let leptatui = crate::utils::crate_path::leptatui();

        Ok(quote! { ::core::convert::Into::<#leptatui::View>::into(#value) })
    }

    /// Expands a child element or braced expression into a view expression.
    fn expand_view_child_value(&self, child: &Child) -> Result<Option<TokenStream>> {
        let leptatui = crate::utils::crate_path::leptatui();

        match child {
            Child::Element(child) => child.expand().map(Some),
            Child::Text(TextContent::Expr(expr))
                if matches!(expr.as_ref(), syn::Expr::Closure(_)) =>
            {
                Ok(Some(quote! { #leptatui::dynamic(#expr) }))
            }
            Child::Text(TextContent::Expr(expr)) => Ok(Some(
                quote! { ::core::convert::Into::<#leptatui::View>::into(#expr) },
            )),
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

/// Rejects a literal callback attribute value.
///
/// # Arguments
///
/// * `attr` — Parsed callback attribute to inspect.
/// * `attribute_name` — User-facing callback attribute name for diagnostics.
///
/// # Returns
///
/// An empty [`Result`] when the attribute value is not a literal.
///
/// # Errors
///
/// Returns [`syn::Error`] if the callback value is a literal.
fn reject_literal_callback(attr: &Attr, attribute_name: &str) -> Result<()> {
    if attr.value.is_literal() {
        return Err(Error::new_spanned(
            &attr.name,
            format!("view! {attribute_name} attribute must be a callback expression"),
        ));
    }

    Ok(())
}

/// Returns whether an identifier is one of the built-in `view!` elements.
fn is_builtin_element(name: &Ident) -> bool {
    matches!(
        name.to_string().as_str(),
        "Block"
            | "Row"
            | "Column"
            | "Form"
            | "Text"
            | "Button"
            | "Input"
            | "TextArea"
            | "Image"
            | "ProgressBar"
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
        /// Required input value.
        InputValue,
        /// Required image source.
        ImageSource,
        /// Required progress value.
        ProgressValue,
        /// Input placeholder text.
        Placeholder,
        /// Image fallback text.
        Alt,
        /// Progress bar label text.
        Label,
        /// Button activation callback.
        OnPress,
        /// Form submit callback.
        OnSubmit,
        /// Form cancel callback.
        OnCancel,
        /// Input value-change callback.
        OnInput,
    }

    /// Attribute paired with its validated kind.
    pub(super) struct ValidatedAttr<'a> {
        /// Parsed source attribute.
        pub(super) attr: &'a Attr,
        /// Accepted attribute kind.
        pub(super) kind: AttrKind,
    }
}
