//! Runner-scoped standalone-screen registration.
//!
//! Error components register weak screen boundaries here so the managed app
//! can suspend its ordinary root while an error screen or file selector owns
//! the terminal and event stream.

use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, ThreadId},
};

use crate::view::{ComponentView, WeakComponentView};

/// Next process-local identifier assigned to a standalone-screen registry.
static NEXT_STANDALONE_SCREEN_REGISTRY_ID: AtomicU64 = AtomicU64::new(1);

/// Retention policy for one active standalone screen.
#[derive(Clone)]
enum ActiveStandaloneScreen {
    /// Screen retained by its ordinary render-tree position.
    Borrowed(WeakComponentView),
    /// Screen retained directly by the application runner.
    Owned(ComponentView),
}

impl ActiveStandaloneScreen {
    /// Returns the mounted component when it remains alive.
    ///
    /// # Returns
    ///
    /// An optional [`ComponentView`] sharing the active screen state.
    fn active(&self) -> Option<ComponentView> {
        match self {
            Self::Borrowed(screen) => screen.upgrade(),
            Self::Owned(screen) => Some(screen.clone()),
        }
    }
}

thread_local! {
    /// Active weak screen boundaries keyed by their runner registry.
    static ACTIVE_STANDALONE_SCREENS: RefCell<HashMap<u64, ActiveStandaloneScreen>> =
        RefCell::new(HashMap::new());
}

/// Thread-safe identity shared by clones of one standalone-screen registry.
struct StandaloneScreenRegistryState {
    /// Process-local identifier used to find same-thread screen state.
    id: u64,
    /// Thread that owns the registered view boundary.
    owner_thread: ThreadId,
}

/// Runner-owned registry for one active standalone screen.
///
/// The cloneable token is safe to provide through component context while the
/// non-thread-safe view handle remains in owner-thread-local storage.
#[derive(Clone)]
pub(crate) struct StandaloneScreenRegistry {
    /// Shared identity for this application runner.
    state: Arc<StandaloneScreenRegistryState>,
}

impl StandaloneScreenRegistry {
    /// Creates an empty standalone-screen registry for the current thread.
    ///
    /// # Returns
    ///
    /// A [`StandaloneScreenRegistry`] with no active screen.
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(StandaloneScreenRegistryState {
                id: NEXT_STANDALONE_SCREEN_REGISTRY_ID.fetch_add(1, Ordering::Relaxed),
                owner_thread: thread::current().id(),
            }),
        }
    }

    /// Registers the first live error screen for the current app state.
    ///
    /// A later failure cannot replace an already-mounted screen. A stale weak
    /// entry is replaced by the newly registered boundary.
    ///
    /// # Arguments
    ///
    /// * `screen` — Mounted style-isolated screen boundary to register.
    pub(crate) fn register(&self, screen: &ComponentView) {
        self.assert_owner_thread();
        ACTIVE_STANDALONE_SCREENS.with(|screens| {
            let mut screens = screens.borrow_mut();
            if screens
                .get(&self.state.id)
                .and_then(ActiveStandaloneScreen::active)
                .is_some()
            {
                return;
            }
            screens.insert(
                self.state.id,
                ActiveStandaloneScreen::Borrowed(screen.downgrade()),
            );
        });
    }

    /// Registers and retains the first live runner-owned standalone screen.
    ///
    /// # Arguments
    ///
    /// * `screen` — Mounted style-isolated screen boundary to retain.
    pub(crate) fn register_owned(&self, screen: ComponentView) {
        self.assert_owner_thread();
        ACTIVE_STANDALONE_SCREENS.with(|screens| {
            let mut screens = screens.borrow_mut();
            if screens
                .get(&self.state.id)
                .and_then(ActiveStandaloneScreen::active)
                .is_none()
            {
                screens.insert(self.state.id, ActiveStandaloneScreen::Owned(screen));
            }
        });
    }

    /// Returns the active mounted standalone screen.
    ///
    /// Stale weak entries are removed when their render-tree boundary has
    /// already been dropped.
    ///
    /// # Returns
    ///
    /// An optional [`ComponentView`] sharing the active screen state.
    pub(crate) fn active(&self) -> Option<ComponentView> {
        self.assert_owner_thread();
        ACTIVE_STANDALONE_SCREENS.with(|screens| {
            let mut screens = screens.borrow_mut();
            let active = screens
                .get(&self.state.id)
                .and_then(ActiveStandaloneScreen::active);
            if active.is_none() {
                screens.remove(&self.state.id);
            }
            active
        })
    }

    /// Removes the active standalone screen from this runner.
    pub(crate) fn dismiss(&self) {
        self.assert_owner_thread();
        ACTIVE_STANDALONE_SCREENS.with(|screens| {
            screens.borrow_mut().remove(&self.state.id);
        });
    }

    /// Verifies a registry operation runs on its owner thread.
    ///
    /// # Panics
    ///
    /// Panics if the current thread does not own this registry.
    fn assert_owner_thread(&self) {
        assert_eq!(
            thread::current().id(),
            self.state.owner_thread,
            "standalone-screen registry must be used on its app runner thread",
        );
    }
}

impl fmt::Debug for StandaloneScreenRegistry {
    /// Formats whether this runner currently owns a live error screen.
    ///
    /// # Arguments
    ///
    /// * `formatter` — Debug formatter receiving the registry summary.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating whether formatting succeeded.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let active = thread::current().id() == self.state.owner_thread && self.active().is_some();
        formatter
            .debug_struct("StandaloneScreenRegistry")
            .field("active", &active)
            .finish()
    }
}

impl Drop for StandaloneScreenRegistry {
    /// Removes same-thread registry state when the final token is dropped.
    ///
    /// Thread-local teardown may itself drop a retained screen and its registry
    /// token, so cleanup tolerates an unavailable or already borrowed registry.
    fn drop(&mut self) {
        if Arc::strong_count(&self.state) == 1 && thread::current().id() == self.state.owner_thread
        {
            let _ = ACTIVE_STANDALONE_SCREENS.try_with(|screens| {
                if let Ok(mut screens) = screens.try_borrow_mut() {
                    screens.remove(&self.state.id);
                }
            });
        }
    }
}

#[cfg(test)]
/// Unit tests for runner-scoped standalone-screen registration.
mod tests {
    use crate::{text, view::ComponentView};

    use super::*;

    /// Verifies the first live error screen remains active until dismissal.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// register first
    /// register second
    /// dismiss
    /// ```
    ///
    /// # Assertions
    ///
    /// - The first registered component remains active.
    /// - A later live component does not replace it.
    /// - Dismissal removes the active component.
    #[test]
    fn registry_keeps_first_live_screen_until_dismissed() {
        let registry = StandaloneScreenRegistry::new();
        let first = ComponentView::new(text("first"));
        let second = ComponentView::new(text("second"));

        registry.register(&first);
        registry.register(&second);
        assert_eq!(registry.active(), Some(first));

        registry.dismiss();
        assert_eq!(registry.active(), None);
    }

    /// Verifies a dropped screen does not remain active through its weak entry.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// register temporary screen
    /// drop temporary screen
    /// read active screen
    /// ```
    ///
    /// # Assertions
    ///
    /// - The temporary screen is active before it is dropped.
    /// - The stale registry entry resolves to no active screen afterward.
    #[test]
    fn registry_removes_dropped_weak_screen() {
        let registry = StandaloneScreenRegistry::new();
        {
            let screen = ComponentView::new(text("temporary"));
            registry.register(&screen);
            assert!(registry.active().is_some());
        }

        assert_eq!(registry.active(), None);
    }

    /// Verifies runner-owned screens remain mounted until explicit dismissal.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// register_owned selector
    /// drop caller handle
    /// dismiss
    /// ```
    ///
    /// # Assertions
    ///
    /// - The registry retains the owned screen after caller state is dropped.
    /// - Dismissal removes the retained screen.
    #[test]
    fn registry_retains_owned_screens_until_dismissed() {
        let registry = StandaloneScreenRegistry::new();
        let screen = ComponentView::new(text("selector"));
        registry.register_owned(screen);

        assert!(registry.active().is_some());
        registry.dismiss();
        assert_eq!(registry.active(), None);
    }
}
