//! Interactive terminal controls.
//!
//! # Modules
//!
//! - [`button`] — Focusable action buttons.
//! - [`editable`] — Controlled input and text-area editing.
//! - [`form`] — Submit and cancel boundaries for editable controls.
//! - [`link`] — Focusable URL and filesystem links.
//! - [`progress_bar`] — Clamped terminal gauge views.

use std::rc::Rc;

pub(crate) mod button;
pub(crate) mod editable;
pub(crate) mod form;
pub(crate) mod link;
pub(crate) mod progress_bar;

/// Returns whether optional callbacks are both absent or share allocation identity.
pub(crate) fn actions_equal<T: ?Sized>(left: &Option<Rc<T>>, right: &Option<Rc<T>>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
        _ => false,
    }
}
