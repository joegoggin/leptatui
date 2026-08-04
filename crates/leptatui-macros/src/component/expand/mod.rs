//! Code generation for the `component` attribute macro.
//!
//! This module emits the component wrapper type, optional props model,
//! owner-backed setup, view conversions, constructors, and render implementation
//! for validated component functions.
//!
//! # Modules
//!
//! - [`constructors`] — Generated constructors, setup, and defaults.
//! - [`props`] — Generated props types and builder APIs.

mod constructors;
mod props;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, ItemFn, Stmt};

use super::signature;

use self::{
    constructors::{expand_constructors, expand_default_impl, expand_setup_fn},
    props::expand_props_api,
};

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
    let fallible_error = signature::fallible_error(&input_fn.sig);

    let attrs = input_fn.attrs;
    let vis = input_fn.vis;
    let ident = input_fn.sig.ident;
    let mut body = input_fn.block;
    let leptatui = crate::crate_path::leptatui();

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
    let setup_body = if fallible_error.is_some() {
        expand_fallible_body(&mut body.stmts, &leptatui)?
    } else {
        quote! { #body }
    };
    let setup_fn = expand_setup_fn(
        &ident,
        &props,
        setup_body,
        fallible_error.as_ref(),
        &leptatui,
    );
    let default_impl = expand_default_impl(&ident, &props);

    Ok(quote! {
        #props_api

        #[allow(non_camel_case_types)]
        #(#attrs)*
        #vis struct #ident {
            __leptatui_owner: #leptatui::prelude::Owner,
            __leptatui_view: #leptatui::AnyView,
            __leptatui_key_handlers: #leptatui::__private::KeyHandlerRegistry,
            __leptatui_stylesheet: #leptatui::Stylesheet,
        }

        impl #ident {
            #constructors

            #[doc(hidden)]
            fn __create(__leptatui_setup: impl FnOnce() -> #leptatui::AnyView) -> Self {
                let __leptatui_owner = #leptatui::prelude::Owner::new();
                let __leptatui_key_handlers =
                    #leptatui::__private::KeyHandlerRegistry::new();
                let __leptatui_stylesheets =
                    #leptatui::__private::StylesheetRegistry::new();
                let __leptatui_view = __leptatui_owner.with(|| {
                    #leptatui::__private::__with_component_setup_context(|| {
                        #leptatui::__private::__with_key_handler_registry(
                            &__leptatui_key_handlers,
                            || {
                                #leptatui::__private::__with_stylesheet_registry(
                                    &__leptatui_stylesheets,
                                    __leptatui_setup,
                                )
                            },
                        )
                    })
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
            #vis fn into_view(self) -> #leptatui::AnyView {
                #leptatui::component(self)
            }
        }

        #default_impl

        impl #leptatui::View for #ident {
            #[doc = "Renders the component into the provided Leptatui context."]
            fn render(
                &self,
                ctx: &mut #leptatui::RenderCtx<'_, '_>,
            ) -> #leptatui::Result<()> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;
                let __leptatui_stylesheet = &self.__leptatui_stylesheet;

                ctx.__with_stylesheet(__leptatui_stylesheet, |ctx| {
                    __leptatui_owner.with(|| {
                        #leptatui::__private::__with_context_scope_if_missing(|| {
                            __leptatui_view.render(ctx)
                        })
                    })
                })
            }

            #[doc(hidden)]
            fn __dispatch_event(
                &mut self,
                event: &#leptatui::__private::Event,
            ) -> #leptatui::Result<#leptatui::AppControl> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__dispatch_event(event)
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
            fn __handle_mouse_event(
                &mut self,
                mouse: #leptatui::__private::MouseEvent,
            ) -> #leptatui::Result<#leptatui::AppControl> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__handle_mouse_event(mouse)
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
            fn measure(
                &self,
                known_dimensions: #leptatui::LayoutSize<::core::option::Option<f32>>,
                available_space: #leptatui::LayoutSize<#leptatui::AvailableSpace>,
                ctx: &mut #leptatui::RenderCtx<'_, '_>,
            ) -> #leptatui::LayoutSize<f32> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;
                let __leptatui_stylesheet = &self.__leptatui_stylesheet;

                ctx.__with_stylesheet(__leptatui_stylesheet, |ctx| {
                    __leptatui_owner.with(|| {
                        #leptatui::__private::__with_context_scope_if_missing(|| {
                            __leptatui_view.measure(
                                known_dimensions,
                                available_space,
                                ctx,
                            )
                        })
                    })
                })
            }

            #[doc(hidden)]
            fn __visit_layout_children(
                &self,
                ctx: &mut #leptatui::RenderCtx<'_, '_>,
                visitor: &mut dyn FnMut(
                    &#leptatui::AnyView,
                    &mut #leptatui::RenderCtx<'_, '_>,
                ),
            ) {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;
                let __leptatui_stylesheet = &self.__leptatui_stylesheet;

                ctx.__with_stylesheet(__leptatui_stylesheet, |ctx| {
                    __leptatui_owner.with(|| {
                        #leptatui::__private::__with_context_scope_if_missing(|| {
                            visitor(__leptatui_view, ctx);
                        })
                    })
                });
            }

            #[doc(hidden)]
            fn __is_layout_transparent(&self) -> bool {
                true
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
            fn __clear_hit_areas(&self) {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__clear_hit_areas();
                });
            }

            #[doc(hidden)]
            fn __focusable_index_at_position_inner(
                &self,
                column: u16,
                row: u16,
                index: &mut usize,
            ) -> ::core::option::Option<(usize, u64)> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__focusable_index_at_position_inner(column, row, index)
                })
            }

            #[doc(hidden)]
            fn __focused_control_span(
                &self,
                ctx: &mut #leptatui::RenderCtx<'_, '_>,
            ) -> ::core::option::Option<(u32, u32)> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;
                let __leptatui_stylesheet = &self.__leptatui_stylesheet;

                ctx.__with_stylesheet(__leptatui_stylesheet, |ctx| {
                    __leptatui_owner.with(|| {
                        #leptatui::__private::__with_context_scope_if_missing(|| {
                            __leptatui_view.__focused_control_span(ctx)
                        })
                    })
                })
            }

            #[doc(hidden)]
            fn __activate_focused_button(
                &self,
            ) -> #leptatui::Result<::core::option::Option<#leptatui::AppControl>> {
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
            fn __flush_pending_input(
                &mut self,
            ) -> ::core::option::Option<#leptatui::AppControl> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__flush_pending_input()
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
            fn __scroll_first_overflowing(&mut self, delta: #leptatui::Axes<i16>) -> bool {
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

            #[doc(hidden)]
            fn __focus_control_at_position(&mut self, column: u16, row: u16) -> bool {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__focus_control_at_position(column, row)
                })
            }

            #[doc(hidden)]
            fn __scroll_overflowing_at_position(
                &mut self,
                column: u16,
                row: u16,
                delta: #leptatui::Axes<i16>,
            ) -> bool {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__scroll_overflowing_at_position(column, row, delta)
                })
            }

            #[doc(hidden)]
            fn __scroll_target_at_position(
                &self,
                column: u16,
                row: u16,
                delta: #leptatui::Axes<i16>,
            ) -> ::core::option::Option<u64> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__scroll_target_at_position(column, row, delta)
                })
            }

            #[doc(hidden)]
            fn __scroll_target_by_paint_order(
                &mut self,
                order: u64,
                delta: #leptatui::Axes<i16>,
            ) -> bool {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__scroll_target_by_paint_order(order, delta)
                })
            }

            #[doc(hidden)]
            fn __set_scroll_to_top_key_pending(&self, pending: bool) -> bool {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__set_scroll_to_top_key_pending(pending)
                })
            }

            #[doc(hidden)]
            fn __take_scroll_to_top_key_pending(&self) -> bool {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__take_scroll_to_top_key_pending()
                })
            }

            #[doc(hidden)]
            fn __navigate_markdown_history(&mut self, back: bool) -> bool {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_view = &mut self.__leptatui_view;

                __leptatui_owner.with(|| {
                    __leptatui_view.__navigate_markdown_history(back)
                })
            }

            fn as_any(&self) -> &dyn ::core::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
                self
            }
        }
    })
}

/// Wraps a fallible component's final view expression in `Result::Ok`.
///
/// # Arguments
///
/// * `statements` — Parsed statements from the original component body.
/// * `leptatui` — Resolved path to the public runtime crate.
///
/// # Returns
///
/// A [`TokenStream`] containing the transformed fallible setup body.
///
/// # Errors
///
/// Returns [`syn::Error`] if the component has no final bare expression.
fn expand_fallible_body(
    statements: &mut Vec<Stmt>,
    leptatui: &TokenStream,
) -> syn::Result<TokenStream> {
    let Some(tail) = statements.pop() else {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "fallible #[component] functions must end with a view expression",
        ));
    };

    let view = match tail {
        Stmt::Expr(expression, None) => quote! { #expression },
        Stmt::Macro(statement) if statement.semi_token.is_none() => {
            let invocation = statement.mac;
            quote! { #invocation }
        }
        statement => {
            return Err(Error::new_spanned(
                statement,
                "fallible #[component] functions must end with a bare view expression",
            ));
        }
    };

    Ok(quote! {{
        #(#statements)*
        ::core::result::Result::Ok(#leptatui::IntoView::into_view(#view))
    }})
}
