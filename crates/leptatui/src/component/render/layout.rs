//! Transient layout state carried by rendering contexts.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{LayoutGeometry, View};

use super::RenderCtx;

/// Current stage of the transient root layout pass.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum LayoutPhase {
    /// No layout snapshot is currently being built.
    #[default]
    Inactive,
    /// Structural traversal is mirroring visible views into Taffy.
    Build,
    /// Taffy is requesting intrinsic leaf measurements.
    Measure,
    /// Painting is consuming a completed snapshot.
    Paint,
}

/// Layout state inherited by child rendering contexts.
#[derive(Clone)]
pub(super) struct LayoutState {
    /// Current root layout stage.
    phase: LayoutPhase,
    /// Whether an erased view should adopt its retained absolute geometry.
    honors_geometry: bool,
    /// Retained geometry for views that do not expose selector metadata.
    unstyled_geometry: Rc<RefCell<HashMap<usize, LayoutGeometry>>>,
}

impl Default for LayoutState {
    /// Creates inactive layout state that honors retained geometry.
    ///
    /// # Returns
    ///
    /// A [`LayoutState`] with empty unstyled-view geometry.
    fn default() -> Self {
        Self {
            phase: LayoutPhase::Inactive,
            honors_geometry: true,
            unstyled_geometry: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

impl LayoutState {
    /// Creates child state for a parent-assigned render area.
    ///
    /// # Returns
    ///
    /// A cloned [`LayoutState`] that does not reapply retained absolute geometry.
    pub(super) fn for_assigned_area(&self) -> Self {
        let mut assigned = self.clone();
        assigned.honors_geometry = false;
        assigned
    }

    /// Disables retained absolute geometry for this child state.
    pub(super) fn disable_retained_geometry(&mut self) {
        self.honors_geometry = false;
    }
}

impl RenderCtx<'_, '_> {
    /// Returns the current transient layout stage.
    ///
    /// # Returns
    ///
    /// A [`LayoutPhase`] describing whether layout is inactive, building,
    /// measuring, or painting.
    pub(crate) const fn layout_phase(&self) -> LayoutPhase {
        self.layout_state.phase
    }

    /// Returns whether erased views should adopt retained absolute geometry.
    ///
    /// # Returns
    ///
    /// `true` when retained layout geometry should replace the assigned area.
    pub(crate) const fn honors_layout_geometry(&self) -> bool {
        self.layout_state.honors_geometry
    }

    /// Returns retained geometry for a view without selector metadata.
    ///
    /// # Arguments
    ///
    /// * `view` — View whose transient geometry is queried.
    ///
    /// # Returns
    ///
    /// The retained [`LayoutGeometry`] when the current layout pass assigned one.
    pub(crate) fn unstyled_layout_geometry(&self, view: &dyn View) -> Option<LayoutGeometry> {
        self.layout_state
            .unstyled_geometry
            .borrow()
            .get(&view_identity(view))
            .copied()
    }

    /// Retains geometry for a view without selector metadata.
    ///
    /// # Arguments
    ///
    /// * `view` — View receiving transient geometry.
    /// * `geometry` — Rounded boxes assigned by the layout engine.
    pub(crate) fn set_unstyled_layout_geometry(&self, view: &dyn View, geometry: LayoutGeometry) {
        self.layout_state
            .unstyled_geometry
            .borrow_mut()
            .insert(view_identity(view), geometry);
    }

    /// Replaces the current transient layout stage.
    ///
    /// # Arguments
    ///
    /// * `phase` — Layout stage to store for descendant contexts.
    pub(crate) fn set_layout_phase(&mut self, phase: LayoutPhase) {
        self.layout_state.phase = phase;
    }
}

/// Returns a stable data-pointer identity for one retained view.
///
/// # Arguments
///
/// * `view` — View whose allocation identity is requested.
///
/// # Returns
///
/// A process-local key suitable for the current render pass.
fn view_identity(view: &dyn View) -> usize {
    std::ptr::from_ref(view).cast::<()>() as usize
}
