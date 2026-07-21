//! Crate path resolution for generated macro code.
//!
//! Procedural macros cannot use `$crate`, so this module resolves the runtime
//! crate path from the calling package's manifest before expansion.

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

/// Returns the runtime crate path to use in generated code.
///
/// # Returns
///
/// A [`TokenStream`] containing `::leptatui`, a renamed dependency path, or a
/// fallback path when manifest lookup fails.
pub(crate) fn leptatui() -> TokenStream {
    match crate_name("leptatui") {
        Ok(FoundCrate::Itself) => quote! { ::leptatui },
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote! { ::#ident }
        }
        Err(_) => quote! { ::leptatui },
    }
}
