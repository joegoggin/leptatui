//! Syntax tree for parsed `view!` macro input.
//!
//! The AST keeps only the element names, accepted attribute names, and child
//! content needed by the expansion step.

use syn::{Expr, Ident, LitStr};

/// Root node for a `view!` invocation.
pub(super) struct ViewRoot {
    /// Single root element required by the macro.
    pub(super) element: Element,
}

/// Parsed terminal element with attributes and children.
pub(super) struct Element {
    /// Element tag name, such as `Text` or `Column`.
    pub(super) name: Ident,
    /// Attribute names attached to the element.
    pub(super) attrs: Vec<Attr>,
    /// Child elements or text content nested inside this element.
    pub(super) children: Vec<Child>,
}

/// Parsed element attribute.
pub(super) struct Attr {
    /// Attribute name accepted by validation.
    pub(super) name: Ident,
}

/// Parsed child node inside an element.
pub(super) enum Child {
    /// Nested element child.
    Element(Element),
    /// Text literal or expression child.
    Text(TextContent),
}

/// Parsed text-like content inside `Text` or `Button` elements.
pub(super) enum TextContent {
    /// String literal content.
    Literal(LitStr),
    /// Braced Rust expression content.
    Expr(Box<Expr>),
}
