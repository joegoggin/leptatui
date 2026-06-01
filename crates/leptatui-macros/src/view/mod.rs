mod ast;
mod expand;
mod parse;

use proc_macro::TokenStream;

use ast::ViewRoot;
use syn::Error;

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    syn::parse::<ViewRoot>(input)
        .and_then(ViewRoot::expand)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
