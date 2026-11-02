//! Runner-scoped visited-link tracking.
//!
//! This module retains successfully activated destinations for one managed
//! [`App`](crate::App) and exposes the current registry while that app renders
//! or dispatches terminal events.

use std::{
    cell::RefCell,
    collections::HashSet,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::view::StyleMetadata;

use super::LinkTarget;

/// Internal key for a visited external, Markdown, filesystem, or router link.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum VisitedLinkKey {
    /// Destination represented by the public link-target model.
    Target(LinkTarget),
    /// Normalized in-app router destination.
    Route(String),
}

thread_local! {
    /// Stack of visited-link registries active on the current thread.
    static VISITED_LINK_STACK: RefCell<Vec<VisitedLinkRegistry>> = const { RefCell::new(Vec::new()) };
}

/// In-memory visited destinations owned by one application runner.
#[derive(Clone, Debug, Default)]
pub(crate) struct VisitedLinkRegistry {
    /// Destinations successfully activated during this application session.
    targets: Arc<Mutex<HashSet<VisitedLinkKey>>>,
}

impl VisitedLinkRegistry {
    /// Creates an empty visited-link registry.
    ///
    /// # Returns
    ///
    /// A [`VisitedLinkRegistry`] containing no destinations.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Runs an operation with this registry active on the current thread.
    ///
    /// # Arguments
    ///
    /// * `operation` — Operation that may query or update visited targets.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `operation`.
    pub(crate) fn with<R>(&self, operation: impl FnOnce() -> R) -> R {
        let _guard = VisitedLinkRegistryGuard::enter(self);
        operation()
    }

    /// Returns whether this registry contains a destination.
    ///
    /// # Arguments
    ///
    /// * `target` — Resolved link destination to inspect.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether `target` was visited.
    fn contains(&self, target: &LinkTarget) -> bool {
        self.targets()
            .contains(&VisitedLinkKey::Target(target.clone()))
    }

    /// Records a visited destination.
    ///
    /// # Arguments
    ///
    /// * `target` — Resolved link destination to retain.
    fn insert(&self, target: LinkTarget) {
        self.targets().insert(VisitedLinkKey::Target(target));
    }

    /// Returns whether this registry contains a normalized router destination.
    ///
    /// # Arguments
    ///
    /// * `target` — Normalized router destination to inspect.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether `target` was visited.
    fn contains_route(&self, target: &str) -> bool {
        self.targets()
            .contains(&VisitedLinkKey::Route(target.to_owned()))
    }

    /// Records a visited router destination.
    ///
    /// # Arguments
    ///
    /// * `target` — Normalized router destination to retain.
    fn insert_route(&self, target: String) {
        self.targets().insert(VisitedLinkKey::Route(target));
    }

    /// Locks the retained destination set, recovering from prior panics.
    ///
    /// # Returns
    ///
    /// A [`MutexGuard`] providing mutable access to the destination set.
    fn targets(&self) -> MutexGuard<'_, HashSet<VisitedLinkKey>> {
        self.targets
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

/// Synchronizes one link's metadata from the active app registry.
///
/// Metadata remains unchanged when no managed app registry is active, allowing
/// standalone view rendering and authored [`StyleMetadata::set_visited`] state.
///
/// # Arguments
///
/// * `metadata` — Link metadata receiving the computed visited state.
/// * `target` — Resolved destination used as the session key.
pub(crate) fn sync_visited(metadata: &StyleMetadata, target: &LinkTarget) {
    if let Some(visited) = current_registry().map(|registry| registry.contains(target)) {
        metadata.sync_visited(visited);
    }
}

/// Synchronizes one router anchor's metadata from the active app registry.
///
/// Metadata remains unchanged when no managed app registry is active, allowing
/// standalone view rendering and authored [`StyleMetadata::set_visited`] state.
///
/// # Arguments
///
/// * `metadata` — Anchor metadata receiving the computed visited state.
/// * `target` — Normalized router destination used as the session key.
pub(crate) fn sync_route_visited(metadata: &StyleMetadata, target: &str) {
    if let Some(visited) = current_registry().map(|registry| registry.contains_route(target)) {
        metadata.sync_visited(visited);
    }
}

/// Marks one link and its destination as visited.
///
/// The metadata update supports links rendered without a managed app, while
/// the active registry shares the visit with every matching session link.
///
/// # Arguments
///
/// * `metadata` — Activated link metadata to update immediately.
/// * `target` — Resolved destination to retain for the active app session.
pub(crate) fn mark_visited(metadata: &StyleMetadata, target: &LinkTarget) {
    metadata.sync_visited(true);
    mark_target_visited(target);
}

/// Marks one destination in the active app registry.
///
/// # Arguments
///
/// * `target` — Resolved destination to retain when a registry is active.
pub(crate) fn mark_target_visited(target: &LinkTarget) {
    if let Some(registry) = current_registry() {
        registry.insert(target.clone());
    }
}

/// Marks one router anchor and its destination as visited.
///
/// The metadata update supports anchors rendered without a managed app, while
/// the active registry shares the visit with matching session anchors.
///
/// # Arguments
///
/// * `metadata` — Activated anchor metadata to update immediately.
/// * `target` — Normalized router destination to retain for the active session.
pub(crate) fn mark_route_visited(metadata: &StyleMetadata, target: &str) {
    metadata.sync_visited(true);
    if let Some(registry) = current_registry() {
        registry.insert_route(target.to_owned());
    }
}

/// Returns the registry currently active on this thread.
///
/// # Returns
///
/// An optional [`VisitedLinkRegistry`] cloned from the active registry stack.
fn current_registry() -> Option<VisitedLinkRegistry> {
    VISITED_LINK_STACK.with(|stack| stack.borrow().last().cloned())
}

/// Scope guard that removes one active registry on drop.
struct VisitedLinkRegistryGuard;

impl VisitedLinkRegistryGuard {
    /// Pushes a visited-link registry onto the current thread's stack.
    ///
    /// # Arguments
    ///
    /// * `registry` — Registry to expose during an app operation.
    ///
    /// # Returns
    ///
    /// A [`VisitedLinkRegistryGuard`] that restores the previous stack on drop.
    fn enter(registry: &VisitedLinkRegistry) -> Self {
        VISITED_LINK_STACK.with(|stack| stack.borrow_mut().push(registry.clone()));
        Self
    }
}

impl Drop for VisitedLinkRegistryGuard {
    /// Removes the active registry from the current thread's stack.
    fn drop(&mut self) {
        let popped = VISITED_LINK_STACK.with(|stack| stack.borrow_mut().pop().is_some());
        debug_assert!(popped, "visited-link registry stack underflow");
    }
}

#[cfg(test)]
/// Unit tests for runner-scoped visited-link tracking.
mod tests {
    use crate::view::{StyleMetadata, ViewType};

    use super::*;

    /// Verifies one registry shares link and route visits without leaking sessions.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// session one: visit a LinkTarget::Url and a normalized router destination
    /// session two: synchronize both destinations
    /// ```
    ///
    /// # Assertions
    ///
    /// - A new session initially reports the destination as unvisited.
    /// - Marking a standalone link makes matching inline metadata visited in the
    ///   same session.
    /// - Marking a router anchor makes a matching anchor visited in the same
    ///   session.
    /// - A separate registry resets both matching metadata values to unvisited.
    #[test]
    fn registries_share_matching_visits_without_leaking_sessions() {
        let target = LinkTarget::from("https://example.com");
        let standalone = StyleMetadata::new(ViewType::Link);
        let inline = StyleMetadata::new(ViewType::Link);
        let route = StyleMetadata::new(ViewType::A);
        let matching_route = StyleMetadata::new(ViewType::A);
        let first_session = VisitedLinkRegistry::new();

        first_session.with(|| {
            sync_visited(&standalone, &target);
            assert!(!standalone.is_visited());

            mark_visited(&standalone, &target);
            sync_visited(&inline, &target);
            assert!(standalone.is_visited());
            assert!(inline.is_visited());

            sync_route_visited(&route, "/docs?mode=full");
            assert!(!route.is_visited());

            mark_route_visited(&route, "/docs?mode=full");
            sync_route_visited(&matching_route, "/docs?mode=full");
            assert!(route.is_visited());
            assert!(matching_route.is_visited());
        });

        VisitedLinkRegistry::new().with(|| {
            sync_visited(&inline, &target);
            sync_route_visited(&matching_route, "/docs?mode=full");
            assert!(!inline.is_visited());
            assert!(!matching_route.is_visited());
        });
    }
}
