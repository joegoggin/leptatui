//! Code generation for the `component` attribute macro.
//!
//! This module emits the component wrapper type, optional props model,
//! owner-backed setup, view conversions, constructors, and render implementation
//! for validated component functions.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, ItemFn, Stmt, Visibility};

use super::signature::{self, Prop};

/// Builds the generated component type for a parsed function.
///
/// # Arguments
///
/// * `input_fn` — Parsed component function to validate and expand.
///
/// # Returns
///
/// A [`TokenStream`] containing the generated component item.
///
/// # Errors
///
/// Returns [`syn::Error`] if the function signature is unsupported.
pub(super) fn component(input_fn: ItemFn) -> syn::Result<TokenStream> {
    let props = signature::analyze(&input_fn.sig)?;

    let attrs = input_fn.attrs;
    let vis = input_fn.vis;
    let ident = input_fn.sig.ident;
    let mut body = input_fn.block;
    let leptatui = crate::utils::crate_path::leptatui();

    for statement in &mut body.stmts {
        if let Stmt::Macro(statement) = statement
            && statement.semi_token.is_none()
            && statement
                .mac
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "stylesheet")
        {
            statement.semi_token = Some(Default::default());
        }
    }

    let props_api = expand_props_api(&vis, &ident, &props);
    let constructors = expand_constructors(&vis, &ident, &props);
    let setup_fn = expand_setup_fn(&ident, &props, quote! { #body }, &leptatui);
    let default_impl = expand_default_impl(&ident, &props);

    Ok(quote! {
        #props_api

        #[allow(non_camel_case_types)]
        #(#attrs)*
        #vis struct #ident {
            __leptatui_owner: #leptatui::prelude::Owner,
            __leptatui_view: #leptatui::View,
            __leptatui_key_handlers: #leptatui::__private::KeyHandlerRegistry,
            __leptatui_stylesheet: #leptatui::Stylesheet,
        }

        impl #ident {
            #constructors

            #[doc(hidden)]
            fn __create(__leptatui_setup: impl FnOnce() -> #leptatui::View) -> Self {
                let __leptatui_owner = #leptatui::prelude::Owner::new();
                let __leptatui_key_handlers =
                    #leptatui::__private::KeyHandlerRegistry::new();
                let __leptatui_stylesheets =
                    #leptatui::__private::StylesheetRegistry::new();
                let __leptatui_view = __leptatui_owner.with(|| {
                    #leptatui::__private::__with_key_handler_registry(
                        &__leptatui_key_handlers,
                        || {
                            #leptatui::__private::__with_stylesheet_registry(
                                &__leptatui_stylesheets,
                                __leptatui_setup,
                            )
                        },
                    )
                });
                let __leptatui_stylesheet = __leptatui_stylesheets.stylesheet();

                Self {
                    __leptatui_owner,
                    __leptatui_view,
                    __leptatui_key_handlers,
                    __leptatui_stylesheet,
                }
            }

            #setup_fn

            #[doc = "Converts this component into a Leptatui view."]
            #vis fn into_view(self) -> #leptatui::View {
                #leptatui::component(self)
            }
        }

        #default_impl

        impl ::core::convert::From<#ident> for #leptatui::View {
            #[doc = "Converts the component into a Leptatui view."]
            fn from(component: #ident) -> Self {
                component.into_view()
            }
        }

        impl #leptatui::Component for #ident {
            #[doc = "Renders the component into the provided Leptatui context."]
            fn render(
                &mut self,
                ctx: &mut #leptatui::RenderCtx<'_, '_>,
            ) -> #leptatui::Result<()> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;
                let __leptatui_stylesheet = &self.__leptatui_stylesheet;

                ctx.__with_stylesheet(__leptatui_stylesheet, |ctx| {
                    __leptatui_owner.with(|| {
                        #leptatui::__private::__with_context_scope_if_missing(|| {
                            ctx.render_view(__leptatui_view)
                        })
                    })
                })
            }

            #[doc = "Dispatches events through the component's rendered view tree."]
            fn handle_event(
                &mut self,
                event: #leptatui::__private::Event,
            ) -> #leptatui::Result<#leptatui::AppControl> {
                if let #leptatui::__private::Event::Key(__leptatui_key) = event {
                    return self
                        .handle_key_event(__leptatui_key)
                        .map(::core::convert::Into::into);
                }

                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.handle_event(event)
                })
            }

            #[doc = "Dispatches key events through descendant and local key maps."]
            fn handle_key_event(
                &mut self,
                key: #leptatui::__private::KeyEvent,
            ) -> #leptatui::Result<#leptatui::KeyControl> {
                let __leptatui_control = self.__dispatch_key_event(key.clone())?;
                if __leptatui_control != #leptatui::KeyControl::Pass {
                    return Ok(__leptatui_control);
                }

                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__handle_default_key_event(key)
                })
            }

            #[doc(hidden)]
            fn __dispatch_key_event(
                &mut self,
                key: #leptatui::__private::KeyEvent,
            ) -> #leptatui::Result<#leptatui::KeyControl> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;
                let __leptatui_key_handlers = &self.__leptatui_key_handlers;

                __leptatui_owner.with(|| {
                    let __leptatui_control =
                        __leptatui_view.__dispatch_key_event(key.clone())?;

                    match __leptatui_control {
                        #leptatui::KeyControl::Pass => {
                            Ok(__leptatui_key_handlers.handle(key.clone()))
                        },
                        __leptatui_control => Ok(__leptatui_control),
                    }
                })
            }

            #[doc(hidden)]
            fn __focusable_count(&self) -> usize {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__focusable_count()
                })
            }

            #[doc(hidden)]
            fn __min_height(&self, ctx: &mut #leptatui::RenderCtx<'_, '_>) -> u16 {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;
                let __leptatui_stylesheet = &self.__leptatui_stylesheet;

                ctx.__with_stylesheet(__leptatui_stylesheet, |ctx| {
                    __leptatui_owner.with(|| {
                        #leptatui::__private::__with_context_scope_if_missing(|| {
                            __leptatui_view.__min_height(ctx)
                        })
                    })
                })
            }

            #[doc(hidden)]
            fn __focused_index_inner(&self, index: &mut usize) -> ::core::option::Option<usize> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__focused_index_inner(index)
                })
            }

            #[doc(hidden)]
            fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__set_focus_by_index_inner(target, index);
                });
            }

            #[doc(hidden)]
            fn __focused_button_span(
                &self,
                ctx: &mut #leptatui::RenderCtx<'_, '_>,
            ) -> ::core::option::Option<(u32, u32)> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;
                let __leptatui_stylesheet = &self.__leptatui_stylesheet;

                ctx.__with_stylesheet(__leptatui_stylesheet, |ctx| {
                    __leptatui_owner.with(|| {
                        #leptatui::__private::__with_context_scope_if_missing(|| {
                            __leptatui_view.__focused_button_span(ctx)
                        })
                    })
                })
            }

            #[doc(hidden)]
            fn __activate_focused_button(&self) -> ::core::option::Option<#leptatui::AppControl> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__activate_focused_button()
                })
            }

            #[doc(hidden)]
            fn __handle_focused_input_key(
                &mut self,
                key: #leptatui::__private::KeyEvent,
            ) -> ::core::option::Option<#leptatui::KeyControl> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__handle_focused_input_key(key)
                })
            }

            #[doc(hidden)]
            fn __focused_control(
                &self,
            ) -> ::core::option::Option<#leptatui::__private::FocusedControl> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__focused_control()
                })
            }

            #[doc(hidden)]
            fn __handle_form_key(
                &mut self,
                key: #leptatui::__private::KeyEvent,
            ) -> ::core::option::Option<#leptatui::KeyControl> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__handle_form_key(key)
                })
            }

            #[doc(hidden)]
            fn __scroll_first_overflowing(&mut self, delta: i16) -> bool {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__scroll_first_overflowing(delta)
                })
            }

            #[doc(hidden)]
            fn __scroll_first_overflowing_to_top(&mut self) -> bool {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__scroll_first_overflowing_to_top()
                })
            }

            #[doc(hidden)]
            fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__scroll_first_overflowing_to_bottom()
                })
            }

            #[doc(hidden)]
            fn __has_overflowing_scroll_target(&self) -> bool {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__has_overflowing_scroll_target()
                })
            }
        }
    })
}

/// Expands the generated props struct and typed builder for prop components.
fn expand_props_api(vis: &Visibility, component: &Ident, props: &[Prop]) -> TokenStream {
    if props.is_empty() {
        return TokenStream::new();
    }

    let props_ident = props_ident(component);
    let builder_ident = builder_ident(component);
    let missing_ident = missing_ident(component);
    let field_defs = props.iter().map(|prop| {
        let attrs = &prop.attrs;
        let ident = &prop.ident;
        let ty = &prop.ty;

        quote! {
            #(#attrs)*
            #vis #ident: #ty
        }
    });

    let state_idents = props
        .iter()
        .map(|prop| prop.state_ident(component))
        .collect::<Vec<_>>();
    let initial_state_types = props.iter().map(|prop| match &prop.default {
        Some(_) => {
            let ty = &prop.ty;
            quote! { #ty }
        }
        None => quote! { #missing_ident },
    });
    let initial_values = props.iter().map(|prop| {
        let ident = &prop.ident;
        let value = match &prop.default {
            Some(default) => default.initial_value(prop.ty.as_ref()),
            None => quote! { #missing_ident },
        };

        quote! { #ident: #value }
    });
    let builder_fields = props.iter().zip(&state_idents).map(|(prop, state)| {
        let ident = &prop.ident;
        quote! { #ident: #state }
    });
    let setters = props
        .iter()
        .enumerate()
        .map(|(index, prop)| expand_prop_setter(vis, component, props, index, prop));
    let build_args = props.iter().map(|prop| {
        let ty = &prop.ty;
        quote! { #ty }
    });
    let build_fields = props.iter().map(|prop| {
        let ident = &prop.ident;
        quote! { #ident: self.#ident }
    });

    quote! {
        #[doc = "Props for the generated component."]
        #vis struct #props_ident {
            #(#field_defs,)*
        }

        #[doc = "Builder for generated component props."]
        #vis struct #builder_ident<#(#state_idents),*> {
            #(#builder_fields,)*
        }

        #[doc(hidden)]
        #vis struct #missing_ident;

        impl #props_ident {
            #[doc = "Creates a builder for component props."]
            #vis fn builder() -> #builder_ident<#(#initial_state_types),*> {
                #builder_ident {
                    #(#initial_values,)*
                }
            }
        }

        #(#setters)*

        impl #builder_ident<#(#build_args),*> {
            #[doc = "Builds component props."]
            #vis fn build(self) -> #props_ident {
                #props_ident {
                    #(#build_fields,)*
                }
            }
        }
    }
}

/// Expands one setter method for the generated props builder.
fn expand_prop_setter(
    vis: &Visibility,
    component: &Ident,
    props: &[Prop],
    index: usize,
    prop: &Prop,
) -> TokenStream {
    let builder_ident = builder_ident(component);
    let missing_ident = missing_ident(component);
    let state_idents = props
        .iter()
        .map(|prop| prop.state_ident(component))
        .collect::<Vec<_>>();
    let ident = &prop.ident;
    let ty = &prop.ty;
    let setter_ty = if prop.into {
        quote! { impl ::core::convert::Into<#ty> }
    } else {
        quote! { #ty }
    };
    let setter_value = if prop.into {
        quote! { ::core::convert::Into::into(#ident) }
    } else {
        quote! { #ident }
    };
    let impl_generics = state_idents
        .iter()
        .enumerate()
        .filter_map(|(arg_index, state)| {
            (arg_index != index || prop.default.is_some()).then_some(state)
        })
        .collect::<Vec<_>>();
    let impl_generics = if impl_generics.is_empty() {
        TokenStream::new()
    } else {
        quote! { <#(#impl_generics),*> }
    };
    let impl_args = state_idents.iter().enumerate().map(|(arg_index, state)| {
        if arg_index == index && prop.default.is_none() {
            quote! { #missing_ident }
        } else {
            quote! { #state }
        }
    });
    let return_args = state_idents.iter().enumerate().map(|(arg_index, state)| {
        if arg_index == index {
            quote! { #ty }
        } else {
            quote! { #state }
        }
    });
    let fields = props.iter().enumerate().map(|(field_index, field)| {
        let field_ident = &field.ident;
        if field_index == index {
            quote! { #field_ident: #setter_value }
        } else {
            quote! { #field_ident: self.#field_ident }
        }
    });

    quote! {
        impl #impl_generics #builder_ident<#(#impl_args),*> {
            #[doc = "Sets a component prop value."]
            #vis fn #ident(self, #ident: #setter_ty) -> #builder_ident<#(#return_args),*> {
                #builder_ident {
                    #(#fields,)*
                }
            }
        }
    }
}

/// Expands generated constructors for a component.
fn expand_constructors(vis: &Visibility, component: &Ident, props: &[Prop]) -> TokenStream {
    if props.is_empty() {
        return quote! {
            #[doc = "Creates a component value."]
            #vis fn new() -> Self {
                Self::__create(Self::__setup_tree)
            }
        };
    }

    let props_ident = props_ident(component);
    let default_new = if props_can_default(props) {
        quote! {
            #[doc = "Creates a component value with default props."]
            #vis fn new() -> Self {
                Self::with_props(#props_ident::builder().build())
            }
        }
    } else {
        TokenStream::new()
    };

    quote! {
        #default_new

        #[doc = "Creates a component value with explicit props."]
        #vis fn with_props(__leptatui_props: #props_ident) -> Self {
            Self::__create(move || Self::__setup_tree(__leptatui_props))
        }
    }
}

/// Expands the hidden setup function that runs the original component body.
fn expand_setup_fn(
    component: &Ident,
    props: &[Prop],
    body: TokenStream,
    leptatui: &TokenStream,
) -> TokenStream {
    let setup_body = quote! {
        {
            let view: #leptatui::View = (|| #body)().into();
            view
        }
    };

    if props.is_empty() {
        return quote! {
            #[doc(hidden)]
            fn __setup_tree() -> #leptatui::View {
                #setup_body
            }
        };
    }

    let props_ident = props_ident(component);
    let field_names = props.iter().map(|prop| &prop.ident);

    quote! {
        #[doc(hidden)]
        fn __setup_tree(__leptatui_props: #props_ident) -> #leptatui::View {
            let #props_ident {
                #(#field_names,)*
            } = __leptatui_props;

            #setup_body
        }
    }
}

/// Expands `Default` when the generated component can be created without props.
fn expand_default_impl(component: &Ident, props: &[Prop]) -> TokenStream {
    if !props_can_default(props) {
        return TokenStream::new();
    }

    quote! {
        impl ::core::default::Default for #component {
            #[doc = "Creates the default component value."]
            fn default() -> Self {
                Self::new()
            }
        }
    }
}

/// Returns whether every prop can be omitted.
fn props_can_default(props: &[Prop]) -> bool {
    props.iter().all(|prop| prop.default.is_some())
}

/// Returns the generated props struct identifier.
fn props_ident(component: &Ident) -> Ident {
    format_ident!("{component}Props")
}

/// Returns the generated props builder identifier.
fn builder_ident(component: &Ident) -> Ident {
    format_ident!("{component}PropsBuilder")
}

/// Returns the generated missing-prop marker identifier.
fn missing_ident(component: &Ident) -> Ident {
    format_ident!("{component}PropsMissing")
}
