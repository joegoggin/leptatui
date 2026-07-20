//! Interactive terminal controls.

use std::rc::Rc;

pub(crate) mod button;
pub(crate) mod editable;
pub(crate) mod form;
pub(crate) mod progress_bar;

/// Returns whether optional callbacks are both absent or share allocation identity.
pub(crate) fn actions_equal<T: ?Sized>(left: &Option<Rc<T>>, right: &Option<Rc<T>>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
        _ => false,
    }
}
