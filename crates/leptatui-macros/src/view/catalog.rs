//! Built-in markup vocabulary shared by validation and editor metadata.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Built-in element name, documentation, and its element-specific attributes.
///
/// Common styling attributes are added by [`attributes`] except for routing
/// declarations, which deliberately accept a smaller vocabulary.
pub(super) const ELEMENTS: &[(&str, &str, &[&str])] = &[
    ("Block", "A bordered single-child container.", &[]),
    ("Div", "A multi-child layout container.", &[]),
    (
        "Form",
        "A form with submit and cancel callbacks.",
        &["on_submit", "on_cancel"],
    ),
    ("Text", "Plain text content.", &[]),
    ("H1", "A level-one heading.", &[]),
    ("H2", "A level-two heading.", &[]),
    ("H3", "A level-three heading.", &[]),
    ("H4", "A level-four heading.", &[]),
    ("H5", "A level-five heading.", &[]),
    ("H6", "A level-six heading.", &[]),
    ("Paragraph", "Rich paragraph content.", &[]),
    ("Link", "A standalone hyperlink.", &["href"]),
    ("A", "A route-aware navigation link.", &["href", "exact"]),
    (
        "Routes",
        "A matched route boundary with a fallback factory.",
        &["fallback"],
    ),
    ("Route", "A leaf route declaration.", &["path", "view"]),
    (
        "ParentRoute",
        "A route declaration containing nested routes.",
        &["path", "view"],
    ),
    ("Outlet", "The active nested route view.", &[]),
    (
        "Markdown",
        "A Markdown document loaded from a file.",
        &["src", "line_numbers", "editable"],
    ),
    (
        "CodeBlock",
        "A syntax-highlighted code block.",
        &["language", "line_numbers"],
    ),
    (
        "OrderedList",
        "An ordered list of ListItem children.",
        &["start"],
    ),
    (
        "UnorderedList",
        "An unordered list of ListItem children.",
        &[],
    ),
    (
        "ListItem",
        "A direct child of an ordered or unordered list.",
        &[],
    ),
    ("Table", "A semantic table of head and body sections.", &[]),
    ("TableHead", "A table heading section containing rows.", &[]),
    ("TableBody", "A table body section containing rows.", &[]),
    ("TableRow", "A table row containing cells.", &[]),
    ("TableCell", "A rich-text table cell.", &["alignment"]),
    ("Button", "An actionable text button.", &["on_press"]),
    (
        "Input",
        "A single-line editable signal.",
        &["value", "placeholder", "on_input"],
    ),
    (
        "TextArea",
        "A multiline editable signal.",
        &["value", "placeholder", "on_input"],
    ),
    (
        "Image",
        "A terminal image with fallback text.",
        &["src", "alt"],
    ),
    (
        "ProgressBar",
        "A progress indicator with an optional label.",
        &["value", "label"],
    ),
];

/// Returns whether a name belongs to the built-in markup vocabulary.
///
/// # Arguments
///
/// * `name` — Source tag name.
///
/// # Returns
///
/// A [`bool`] indicating whether the name is built in.
pub(super) fn is_builtin(name: &str) -> bool {
    ELEMENTS.iter().any(|(candidate, _, _)| *candidate == name)
}

/// Returns the complete attribute vocabulary accepted by a built-in tag.
///
/// # Arguments
///
/// * `name` — Source tag name.
///
/// # Returns
///
/// A vector of accepted attribute names, or an empty vector for unknown tags.
pub(super) fn attributes(name: &str) -> Vec<&'static str> {
    let Some((_, _, attrs)) = ELEMENTS.iter().find(|(candidate, _, _)| *candidate == name) else {
        return Vec::new();
    };
    let mut result = attrs.to_vec();
    if !matches!(name, "Routes" | "Route" | "ParentRoute" | "Outlet") {
        result.extend(["class", "id", "style"]);
    }
    result
}

/// Expands documentation-bearing built-in types used only for semantic probes.
///
/// # Returns
///
/// A [`TokenStream`] defining every built-in tag and its typed attribute fields.
pub(crate) fn expand() -> TokenStream {
    let leptatui = crate::crate_path::leptatui();
    let elements = ELEMENTS.iter().map(|(name, doc, _)| {
        let ident = format_ident!("{name}");
        let fields = attributes(name).into_iter().map(|attr| {
            let ident = format_ident!("{attr}");
            let (ty, doc) = match attr {
                "style" => (
                    quote!(#leptatui::TuiStyle),
                    "Inline style applied to this element.",
                ),
                "on_press" => (
                    quote!(fn()),
                    "Callback invoked when the button is activated.",
                ),
                "on_submit" => (quote!(fn()), "Callback invoked when the form is submitted."),
                "on_cancel" => (quote!(fn()), "Callback invoked when the form is cancelled."),
                "on_input" => (
                    quote!(fn()),
                    "Callback invoked when the input value changes.",
                ),
                "value" if matches!(*name, "Input" | "TextArea") => (
                    quote!(#leptatui::prelude::RwSignal<String>),
                    "Writable signal containing the editable text.",
                ),
                "value" => (quote!(f64), "Progress value between zero and one."),
                "src" if *name == "Image" => (
                    quote!(#leptatui::ImageSource),
                    "Image source, including supported string conversions.",
                ),
                "src" => (quote!(String), "Path to the Markdown document."),
                "alignment" => (
                    quote!(#leptatui::CellAlignment),
                    "Horizontal alignment of this table cell.",
                ),
                "start" => (quote!(usize), "Starting number for the ordered list."),
                "line_numbers" => (quote!(bool), "Whether to display line numbers."),
                "editable" => (
                    quote!(bool),
                    "Whether the document can open in the external editor.",
                ),
                "exact" => (
                    quote!(bool),
                    "Whether the navigation link requires an exact route match.",
                ),
                "view" | "fallback" => (
                    quote!(fn() -> #leptatui::AnyView),
                    "Component type or zero-argument view factory.",
                ),
                "class" => (quote!(String), "Space-separated stylesheet classes."),
                "id" => (quote!(String), "Stylesheet identifier for this element."),
                "href" => (quote!(String), "Link destination."),
                "placeholder" => (quote!(String), "Text displayed while the input is empty."),
                "alt" => (quote!(String), "Fallback text for the image."),
                "label" => (quote!(String), "Text label for the progress indicator."),
                "language" => (quote!(String), "Syntax-highlighting language."),
                "path" => (quote!(String), "Route path pattern."),
                _ => unreachable!("catalog attribute must have semantic metadata"),
            };
            quote! { #[doc = #doc] pub #ident: #ty }
        });
        quote! { #[doc = #doc] pub struct #ident { #(#fields,)* } }
    });
    quote! { #(#elements)* }
}
