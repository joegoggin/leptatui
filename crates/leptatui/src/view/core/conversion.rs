//! Conversions into concrete and type-erased view trees.

use leptos::{
    prelude::{ArcMemo, ArcReadSignal, ArcRwSignal, Get, Memo, ReadSignal, RwSignal, Signal},
    reactive::owner::Storage,
};

use super::{any_view::AnyView, contract::View};

/// Converts a concrete value into a type-erased terminal view.
pub trait IntoView {
    /// Converts this value into an [`AnyView`].
    ///
    /// # Returns
    ///
    /// An [`AnyView`] owning the converted node.
    fn into_view(self) -> AnyView;
}

impl<V> IntoView for V
where
    V: View,
{
    fn into_view(self) -> AnyView {
        AnyView::new(self)
    }
}

impl IntoView for AnyView {
    fn into_view(self) -> AnyView {
        self
    }
}

impl IntoView for String {
    fn into_view(self) -> AnyView {
        crate::view::content::text::text(self).into_view()
    }
}

impl IntoView for &str {
    fn into_view(self) -> AnyView {
        crate::view::content::text::text(self).into_view()
    }
}

/// Converts one readable reactive value into a tracked dynamic boundary.
///
/// # Arguments
///
/// * `source` — Reactive value whose tracked reads invalidate the boundary.
///
/// # Returns
///
/// An [`AnyView`] that rebuilds from the latest source value.
fn reactive_into_view<S, V>(source: S) -> AnyView
where
    S: Get<Value = V> + 'static,
    V: IntoView + 'static,
{
    crate::view::dynamic(move || source.get()).into_view()
}

macro_rules! impl_arena_signal_into_view {
    ($signal:ident) => {
        impl<V, S> IntoView for $signal<V, S>
        where
            $signal<V, S>: Get<Value = V> + 'static,
            V: IntoView + 'static,
        {
            fn into_view(self) -> AnyView {
                reactive_into_view(self)
            }
        }
    };
}

impl_arena_signal_into_view!(RwSignal);
impl_arena_signal_into_view!(ReadSignal);

macro_rules! impl_stored_signal_into_view {
    ($signal:ident) => {
        impl<V, S> IntoView for $signal<V, S>
        where
            S: Storage<V> + 'static,
            $signal<V, S>: Get<Value = V> + 'static,
            V: IntoView + 'static,
        {
            fn into_view(self) -> AnyView {
                reactive_into_view(self)
            }
        }
    };
}

impl_stored_signal_into_view!(Memo);
impl_stored_signal_into_view!(Signal);
impl_stored_signal_into_view!(ArcMemo);

macro_rules! impl_arc_signal_into_view {
    ($signal:ident) => {
        impl<V> IntoView for $signal<V>
        where
            $signal<V>: Get<Value = V> + 'static,
            V: IntoView + 'static,
        {
            fn into_view(self) -> AnyView {
                reactive_into_view(self)
            }
        }
    };
}

impl_arc_signal_into_view!(ArcRwSignal);
impl_arc_signal_into_view!(ArcReadSignal);

/// Converts a homogeneous or tuple-shaped child collection into a view list.
pub trait IntoViews {
    /// Converts this value into type-erased children.
    ///
    /// # Returns
    ///
    /// A [`Vec`] of [`AnyView`] values in source order.
    fn into_views(self) -> Vec<AnyView>;
}

impl<V> IntoViews for Vec<V>
where
    V: IntoView,
{
    fn into_views(self) -> Vec<AnyView> {
        self.into_iter().map(IntoView::into_view).collect()
    }
}

impl<V, const N: usize> IntoViews for [V; N]
where
    V: IntoView,
{
    fn into_views(self) -> Vec<AnyView> {
        self.into_iter().map(IntoView::into_view).collect()
    }
}

impl IntoViews for () {
    fn into_views(self) -> Vec<AnyView> {
        Vec::new()
    }
}

macro_rules! impl_into_views_tuple {
    ($($name:ident),+) => {
        impl<$($name),+> IntoViews for ($($name,)+)
        where
            $($name: IntoView),+
        {
            #[allow(non_snake_case)]
            fn into_views(self) -> Vec<AnyView> {
                let ($($name,)+) = self;
                vec![$($name.into_view()),+]
            }
        }
    };
}

impl_into_views_tuple!(A);
impl_into_views_tuple!(A, B);
impl_into_views_tuple!(A, B, C);
impl_into_views_tuple!(A, B, C, D);
impl_into_views_tuple!(A, B, C, D, E);
impl_into_views_tuple!(A, B, C, D, E, F);
impl_into_views_tuple!(A, B, C, D, E, F, G);
impl_into_views_tuple!(A, B, C, D, E, F, G, H);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_into_views_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U
);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V
);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
impl_into_views_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);
