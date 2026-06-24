//! Signal-backed async resource state for terminal apps.
//!
//! Resources connect Leptos reactive source keys to asynchronous read work and
//! expose the latest pending, ready, or error state as a signal-friendly value.

use std::{future::Future, sync::Arc};

use leptos::prelude::{
    Effect, Get, GetUntracked, ReadSignal, Set, SyncStorage, With, WithUntracked, signal,
};

use crate::app::request_redraw;
use crate::executor::{LatestTask, init_tokio_executor};

/// Current state for an asynchronous resource read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceState<T, E> {
    /// The resource is loading for the current source key.
    Pending,
    /// The resource finished successfully.
    Ready(T),
    /// The resource finished with an error.
    Error(E),
}

impl<T, E> ResourceState<T, E> {
    /// Returns whether this state is currently pending.
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns whether this state contains a successful value.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// Returns whether this state contains an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Returns the successful value by reference, when ready.
    pub fn as_ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Pending | Self::Error(_) => None,
        }
    }

    /// Returns the error by reference, when failed.
    pub fn as_error(&self) -> Option<&E> {
        match self {
            Self::Error(error) => Some(error),
            Self::Pending | Self::Ready(_) => None,
        }
    }
}

/// Reactive handle for an async read keyed by a tracked source value.
pub struct Resource<T, E> {
    /// Signal containing the visible resource state.
    state: ReadSignal<ResourceState<T, E>>,
    /// Effect that tracks source-key changes and starts fetch tasks.
    _watcher: Effect<SyncStorage>,
}

impl<T, E> Clone for Resource<T, E> {
    /// Clones the resource signal and watcher handles.
    fn clone(&self) -> Self {
        Self {
            state: self.state,
            _watcher: self._watcher,
        }
    }
}

impl<T, E> Resource<T, E>
where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates a resource from a tracked source key and async fetcher.
    ///
    /// The `source` closure is tracked with a Leptos effect. Whenever the source
    /// key changes, the resource enters [`ResourceState::Pending`] and runs
    /// `fetcher` for the new key. Each fetch receives a monotonically
    /// increasing request id, and only the latest in-flight fetch may update the
    /// state.
    ///
    /// Older fetch tasks are not cancelled. They may still finish, but their
    /// ready or error results are ignored when a newer request has started, so
    /// slower stale fetches cannot overwrite newer results.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime, because the resource watcher
    /// and fetch tasks are scheduled onto Tokio.
    pub fn new<K, F, Fut>(source: impl Fn() -> K + Send + Sync + 'static, fetcher: F) -> Self
    where
        K: Clone + Send + Sync + 'static,
        F: Fn(K) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        init_tokio_executor();

        let (state, set_state) = signal(ResourceState::Pending);
        let latest_request = LatestTask::default();
        let fetcher = Arc::new(fetcher);

        let watcher = {
            let latest_request = latest_request.clone();
            Effect::watch_sync(
                source,
                move |key, _, _| {
                    let request_id = latest_request.next();
                    let _ = set_state.try_set(ResourceState::Pending);
                    request_redraw();

                    let key = key.clone();
                    let set_state = set_state;
                    let fetcher = Arc::clone(&fetcher);
                    let latest_request = latest_request.clone();

                    tokio::spawn(async move {
                        let next = match fetcher(key).await {
                            Ok(value) => ResourceState::Ready(value),
                            Err(error) => ResourceState::Error(error),
                        };

                        if latest_request.is_current(request_id) {
                            let _ = set_state.try_set(next);
                            request_redraw();
                        }
                    });
                },
                true,
            )
        };

        Self {
            state,
            _watcher: watcher,
        }
    }

    /// Returns the read signal containing this resource's state.
    pub fn state(&self) -> ReadSignal<ResourceState<T, E>> {
        self.state
    }

    /// Reads the current resource state reactively by reference.
    pub fn with<R>(&self, read: impl FnOnce(&ResourceState<T, E>) -> R) -> R {
        self.state.with(read)
    }

    /// Reads the current resource state without tracking it.
    pub fn with_untracked<R>(&self, read: impl FnOnce(&ResourceState<T, E>) -> R) -> R {
        self.state.with_untracked(read)
    }

    /// Returns whether the resource is currently pending.
    pub fn is_pending(&self) -> bool {
        self.with(ResourceState::is_pending)
    }
}

impl<T, E> Resource<T, E>
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Returns the current resource state reactively.
    pub fn get(&self) -> ResourceState<T, E> {
        self.state.get()
    }

    /// Returns the current resource state without tracking it.
    pub fn get_untracked(&self) -> ResourceState<T, E> {
        self.state.get_untracked()
    }

    /// Returns the successful value, when ready.
    pub fn value(&self) -> Option<T> {
        self.with(|state| state.as_ready().cloned())
    }

    /// Returns the error, when failed.
    pub fn error(&self) -> Option<E> {
        self.with(|state| state.as_error().cloned())
    }
}

/// Creates a resource from a tracked source key and async fetcher.
///
/// Older in-flight fetch tasks are not cancelled when the source key changes.
/// Their results are ignored if a newer request has started.
///
/// # Arguments
///
/// * `source` — Tracked closure that returns the current resource key.
/// * `fetcher` — Async function that loads a value for each source key.
///
/// # Returns
///
/// A [`Resource`] that exposes pending, ready, and error state for the fetcher.
pub fn create_resource<K, T, E, F, Fut>(
    source: impl Fn() -> K + Send + Sync + 'static,
    fetcher: F,
) -> Resource<T, E>
where
    K: Clone + Send + Sync + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
    F: Fn(K) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
{
    Resource::new(source, fetcher)
}

#[cfg(test)]
/// Tests for resource redraw wakeups.
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use leptos::prelude::Owner;
    use tokio::{sync::oneshot, time::timeout};

    use crate::app::{redraw_test_lock, subscribe_redraws};

    use super::*;

    /// Result returned by the controlled test resource fetcher.
    type TestFetchResult = std::result::Result<String, &'static str>;

    /// Verifies successful resource completion requests a redraw.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// create_resource(|| (), |_| async move { Ok("ready") })
    /// ```
    ///
    /// # Assertions
    ///
    /// - The initial pending load sends a redraw request.
    /// - Completing the fetch successfully sends another redraw request.
    /// - The resource stores `ResourceState::Ready("ready")`.
    #[tokio::test(flavor = "current_thread")]
    async fn successful_completion_requests_redraw() {
        assert_completion_requests_redraw(Ok(String::from("ready"))).await;
    }

    /// Verifies failed resource completion requests a redraw.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// create_resource(|| (), |_| async move { Err("offline") })
    /// ```
    ///
    /// # Assertions
    ///
    /// - The initial pending load sends a redraw request.
    /// - Completing the fetch with an error sends another redraw request.
    /// - The resource stores `ResourceState::Error("offline")`.
    #[tokio::test(flavor = "current_thread")]
    async fn error_completion_requests_redraw() {
        assert_completion_requests_redraw(Err("offline")).await;
    }

    /// Verifies resource completion redraw behavior for one controlled response.
    ///
    /// # Arguments
    ///
    /// * `response` — Result sent into the pending resource fetch task.
    async fn assert_completion_requests_redraw(response: TestFetchResult) {
        let _redraw_guard = redraw_test_lock().await;
        let owner = Owner::new();
        let expected = response.clone();
        let (sender, receiver) = oneshot::channel();
        let receiver = Arc::new(Mutex::new(Some(receiver)));
        let receiver_for_fetcher = Arc::clone(&receiver);
        let mut redraws = subscribe_redraws();

        let resource: Resource<String, &'static str> = owner.with(|| {
            create_resource(
                || (),
                move |_| {
                    let receiver = Arc::clone(&receiver_for_fetcher);

                    async move {
                        let receiver = receiver
                            .lock()
                            .expect("test receiver lock")
                            .take()
                            .expect("test receiver should be available");
                        receiver.await.expect("test fetch response")
                    }
                },
            )
        });

        timeout(Duration::from_secs(1), redraws.changed())
            .await
            .expect("pending redraw request should arrive")
            .expect("redraw sender should stay available");
        redraws.borrow_and_update();

        sender.send(response).expect("send fetch response");

        timeout(Duration::from_secs(1), redraws.changed())
            .await
            .expect("completion redraw request should arrive")
            .expect("redraw sender should stay available");

        let state = resource.get_untracked();

        match expected {
            Ok(value) => assert_eq!(state, ResourceState::Ready(value)),
            Err(error) => assert_eq!(state, ResourceState::Error(error)),
        }
    }
}
