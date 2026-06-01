use syn::{Expr, Ident, LitStr};

pub(super) struct ViewRoot {
    pub(super) element: Element,
}

pub(super) struct Element {
    pub(super) name: Ident,
    pub(super) attrs: Vec<Attr>,
    pub(super) children: Vec<Child>,
}

pub(super) struct Attr {
    pub(super) name: Ident,
}

pub(super) enum Child {
    Element(Element),
    Text(TextContent),
}

pub(super) enum TextContent {
    Literal(LitStr),
    Expr(Expr),
}
