//! Retained-state reconciliation for compatible view trees.

use super::core::contract::View;

/// Reconciles compatible retained state between two view nodes.
pub(crate) fn reconcile_views(next: &mut dyn View, previous: &dyn View) {
    if !next.can_reconcile_from(previous) {
        return;
    }

    if let (Some(next), Some(previous)) = (next.style_metadata_mut(), previous.style_metadata()) {
        next.reconcile_runtime_state(previous);
    }

    next.reconcile(previous);
    for (next, previous) in next.children_mut().iter_mut().zip(previous.children()) {
        next.reconcile_from(previous);
    }
}
