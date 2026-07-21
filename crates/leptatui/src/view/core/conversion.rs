//! Conversions into concrete and type-erased view trees.

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
