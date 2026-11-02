//! Constant and deferred component data props.

use std::rc::Rc;

/// Readable component data whose source can be constant or reactive.
///
/// `view!` creates deferred props automatically. Reading a prop inside a view
/// expression tracks every signal read by its source. Clones share the source.
pub struct Prop<T> {
    /// Shared source evaluated by tracked readers.
    source: Rc<dyn Fn() -> T>,
}

impl<T> Clone for Prop<T> {
    fn clone(&self) -> Self {
        Self {
            source: Rc::clone(&self.source),
        }
    }
}

impl<T: 'static> Prop<T> {
    /// Creates a constant prop.
    ///
    /// # Arguments
    ///
    /// * `value` — Constant value returned by readers.
    ///
    /// # Returns
    ///
    /// A constant [`Prop`].
    pub fn new(value: T) -> Self
    where
        T: Clone,
    {
        Self::__derived(move || value.clone())
    }

    /// Creates a deferred prop for generated view expressions.
    ///
    /// # Arguments
    ///
    /// * `source` — Repeatable expression evaluated in the reader's tracking scope.
    ///
    /// # Returns
    ///
    /// A deferred [`Prop`].
    #[doc(hidden)]
    pub fn __derived(source: impl Fn() -> T + 'static) -> Self {
        Self {
            source: Rc::new(source),
        }
    }

    /// Returns the current value and tracks its signal dependencies.
    ///
    /// # Returns
    ///
    /// A current `T` value.
    pub fn get(&self) -> T {
        (self.source)()
    }

    /// Reads the current value through a callback, tracking its dependencies.
    ///
    /// # Arguments
    ///
    /// * `read` — Callback borrowing the produced value.
    ///
    /// # Returns
    ///
    /// An `R` value returned by the callback.
    pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        read(&self.get())
    }
}

impl<T: Clone + 'static> Default for Prop<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Clone + 'static> From<T> for Prop<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: crate::IntoView + 'static> crate::IntoView for Prop<T> {
    fn into_view(self) -> crate::AnyView {
        crate::IntoView::into_view(crate::dynamic(move || self.get()))
    }
}
