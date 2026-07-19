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
    /// Returns [`syn::Error`] if attributes, children, ancestry, or the element
    /// name are unsupported.
    pub(super) fn expand(&self) -> Result<TokenStream> {
        self.expand_with_parent(None)
    }

    /// Expands this element with its direct semantic parent context.
    ///
    /// # Arguments
    ///
    /// * `parent` — Direct parent tag name, or [`None`] for a root element.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing view builder calls for this element.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if attributes, children, ancestry, or the element
    /// name are unsupported.
    fn expand_with_parent(&self, parent: Option<&str>) -> Result<TokenStream> {
        self.validate_ancestry(parent)?;

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
            "H1" => self.expand_text_like("H1", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::h1(#content) }
            }),
            "H2" => self.expand_text_like("H2", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::h2(#content) }
            }),
            "H3" => self.expand_text_like("H3", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::h3(#content) }
            }),
            "H4" => self.expand_text_like("H4", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::h4(#content) }
            }),
            "H5" => self.expand_text_like("H5", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::h5(#content) }
            }),
            "H6" => self.expand_text_like("H6", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::h6(#content) }
            }),
            "Paragraph" => self.expand_text_like("Paragraph", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::paragraph(#content) }
            }),
            "Markdown" => self.expand_required_attr_element(
                "Markdown",
                AttrKind::MarkdownSource,
                "source",
                |source| {
                    let leptatui = crate::utils::crate_path::leptatui();
                    quote! { #leptatui::markdown(#source) }
                },
            ),
            "CodeBlock" => self.expand_text_like("CodeBlock", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::code_block(#content) }
            }),
            "OrderedList" => self.expand_element_child_list(
                "OrderedList",
                &["ListItem"],
                |children| {
                    let leptatui = crate::utils::crate_path::leptatui();
                    quote! { #leptatui::ordered_list(::std::vec![#(#children),*]) }
                },
            ),
            "UnorderedList" => self.expand_element_child_list(
                "UnorderedList",
                &["ListItem"],
                |children| {
                    let leptatui = crate::utils::crate_path::leptatui();
                    quote! { #leptatui::unordered_list(::std::vec![#(#children),*]) }
                },
            ),
            "ListItem" => self.expand_child_list("ListItem", |children| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::list_item(::std::vec![#(#children),*]) }
            }),
            "Table" => self.expand_element_child_list(
                "Table",
                &["TableHead", "TableBody"],
                |children| {
                    let leptatui = crate::utils::crate_path::leptatui();
                    quote! { #leptatui::table(::std::vec![#(#children),*]) }
                },
            ),
            "TableHead" => self.expand_element_child_list(
                "TableHead",
                &["TableRow"],
                |children| {
                    let leptatui = crate::utils::crate_path::leptatui();
                    quote! { #leptatui::table_head(::std::vec![#(#children),*]) }
                },
            ),
            "TableBody" => self.expand_element_child_list(
                "TableBody",
                &["TableRow"],
                |children| {
                    let leptatui = crate::utils::crate_path::leptatui();
                    quote! { #leptatui::table_body(::std::vec![#(#children),*]) }
                },
            ),
            "TableRow" => self.expand_element_child_list(
                "TableRow",
                &["TableCell"],
                |children| {
                    let leptatui = crate::utils::crate_path::leptatui();
                    quote! { #leptatui::table_row(::std::vec![#(#children),*]) }
                },
            ),
            "TableCell" => self.expand_text_like("TableCell", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::table_cell(#content) }
            }),
            "Button" => self.expand_text_like("Button", |content| {
                let leptatui = crate::utils::crate_path::leptatui();
                quote! { #leptatui::button(#content) }
            }),
            _ if is_component_name(&self.name) => self.expand_component(),
            _ => {
                return Err(Error::new_spanned(
                    &self.name,
                    "unsupported Leptatui element; expected a built-in view tag or a PascalCase component",
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

    /// Validates the direct parent required by structural document elements.
    ///
    /// # Arguments
    ///
    /// * `parent` — Direct parent tag name, or [`None`] for a root element.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] when the element has valid ancestry.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] when a list item, table section, row, or cell is
    /// used outside its required semantic container.
    fn validate_ancestry(&self, parent: Option<&str>) -> Result<()> {
        let (expected, valid) = match self.name.to_string().as_str() {
            "ListItem" => (
                "OrderedList or UnorderedList",
                matches!(parent, Some("OrderedList" | "UnorderedList")),
            ),
            "TableHead" | "TableBody" => ("Table", parent == Some("Table")),
            "TableRow" => (
                "TableHead or TableBody",
                matches!(parent, Some("TableHead" | "TableBody")),
            ),
            "TableCell" => ("TableRow", parent == Some("TableRow")),
            _ => return Ok(()),
        };

        if valid {
            return Ok(());
        }

        Err(Error::new_spanned(
            &self.name,
            format!("{} must be a direct child of {expected}", self.name),
        ))
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
    fn expand_image(&self) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();
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
    fn expand_progress_bar(&self) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();
        self.expand_required_attr_element(
            "ProgressBar",
            AttrKind::ProgressValue,
            "value",
            |value| quote! { #leptatui::progress_bar(#value) },
        )
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
    fn expand_required_attr_element(
        &self,
        element_name: &str,
        required_kind: AttrKind,
        required_attr_name: &str,
        build: impl FnOnce(TokenStream) -> TokenStream,
    ) -> Result<TokenStream> {
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
        self.expand_attrs(build(value), &attrs)
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
                "source" if element_name == "Markdown" => AttrKind::MarkdownSource,
                "source" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! source attribute is only supported on Markdown",
                    ));
                }
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
                "start" if element_name == "OrderedList" => AttrKind::Start,
                "start" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! start attribute is only supported on OrderedList",
                    ));
                }
                "alignment" if element_name == "TableCell" => AttrKind::Alignment,
                "alignment" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! alignment attribute is only supported on TableCell",
                    ));
                }
                "language" if element_name == "CodeBlock" => AttrKind::Language,
                "language" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! language attribute is only supported on CodeBlock",
                    ));
                }
                "line_numbers" if element_name == "CodeBlock" => AttrKind::LineNumbers,
                "line_numbers" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! line_numbers attribute is only supported on CodeBlock",
                    ));
                }
                "syntax_theme" if element_name == "CodeBlock" => AttrKind::SyntaxTheme,
                "syntax_theme" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! syntax_theme attribute is only supported on CodeBlock",
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
                        "Markdown" => {
                            "unsupported view! attribute; expected class, id, style, or source"
                        }
                        "OrderedList" => {
                            "unsupported view! attribute; expected class, id, style, or start"
                        }
                        "TableCell" => {
                            "unsupported view! attribute; expected class, id, style, or alignment"
                        }
                        "CodeBlock" => {
                            "unsupported view! attribute; expected class, id, style, language, line_numbers, or syntax_theme"
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
    /// Returns [`syn::Error`] for duplicate attributes, invalid callback
    /// literals, string literals used for typed configuration, or unbraced
    /// values where braces are required.
    fn expand_attrs(&self, view: TokenStream, attrs: &[ValidatedAttr<'_>]) -> Result<TokenStream> {
        for (index, current) in attrs.iter().enumerate() {
            if let Some(duplicate) = attrs[index + 1..]
                .iter()
                .find(|candidate| candidate.kind == current.kind)
            {
                return Err(Error::new_spanned(
                    &duplicate.attr.name,
                    format!(
                        "{} expects at most one {} attribute",
                        self.name, duplicate.attr.name
                    ),
                ));
            }
        }

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
                AttrKind::MarkdownSource => expanded,
                AttrKind::Placeholder => quote! { (#expanded).placeholder(#value) },
                AttrKind::Alt => quote! { (#expanded).alt(#value) },
                AttrKind::Label => quote! { (#expanded).label(#value) },
                AttrKind::Start => {
                    reject_literal_typed_attr(attr, "start", "usize")?;
                    quote! { (#expanded).start(#value) }
                }
                AttrKind::Alignment => {
                    reject_literal_typed_attr(attr, "alignment", "CellAlignment")?;
                    quote! { (#expanded).alignment(#value) }
                }
                AttrKind::Language => quote! { (#expanded).language(#value) },
                AttrKind::LineNumbers => {
                    reject_literal_typed_attr(attr, "line_numbers", "bool")?;
                    quote! { (#expanded).line_numbers(#value) }
                }
                AttrKind::SyntaxTheme => {
                    reject_literal_typed_attr(attr, "syntax_theme", "SyntaxTheme")?;
                    quote! { (#expanded).syntax_theme(#value) }
                }
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
                    | AttrKind::MarkdownSource
                    | AttrKind::Style
                    | AttrKind::Start
                    | AttrKind::Alignment
                    | AttrKind::Language
                    | AttrKind::LineNumbers
                    | AttrKind::SyntaxTheme
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
    fn expand_element_child_list(
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
            Child::Element(child) => {
                let parent = self.name.to_string();
                child.expand_with_parent(Some(&parent)).map(Some)
            }
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

/// Rejects a string literal for an attribute requiring a typed expression.
///
/// # Arguments
///
/// * `attr` — Parsed typed attribute to inspect.
/// * `attribute_name` — User-facing attribute name for diagnostics.
/// * `expected_type` — Rust type required by the corresponding builder.
///
/// # Returns
///
/// An empty [`Result`] when the value is an expression.
///
/// # Errors
///
/// Returns [`syn::Error`] if the value is a string literal.
fn reject_literal_typed_attr(attr: &Attr, attribute_name: &str, expected_type: &str) -> Result<()> {
    if attr.value.is_literal() {
        return Err(Error::new_spanned(
            &attr.name,
            format!("view! {attribute_name} attribute must be a {expected_type} expression"),
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
            | "H1"
            | "H2"
            | "H3"
            | "H4"
            | "H5"
            | "H6"
            | "Paragraph"
            | "Markdown"
            | "CodeBlock"
            | "OrderedList"
            | "UnorderedList"
            | "ListItem"
            | "Table"
            | "TableHead"
            | "TableBody"
            | "TableRow"
            | "TableCell"
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
    #[derive(Clone, Copy, Eq, PartialEq)]
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
        /// Required in-memory Markdown source.
        MarkdownSource,
        /// Input placeholder text.
        Placeholder,
        /// Image fallback text.
        Alt,
        /// Progress bar label text.
        Label,
        /// Ordered-list starting marker.
        Start,
        /// Table-cell horizontal alignment.
        Alignment,
        /// Code-block syntax language.
        Language,
        /// Code-block line-number visibility.
        LineNumbers,
        /// Code-block highlighting theme.
        SyntaxTheme,
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
