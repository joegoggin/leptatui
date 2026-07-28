//! Signal-backed asynchronous resources for terminal apps.
//!
//! Resources mirror Leptos resource semantics: the fetcher can return any
//! value, including an application-owned [`std::result::Result`]. Loading state
//! is separate from the retained value so refetches do not discard previously
//! loaded data.

use std::{future::Future, sync::Arc};

use leptos::prelude::{
    Effect, Get, GetUntracked, ReadSignal, Set, SyncStorage, Update, With, WithUntracked,
    WriteSignal, signal,
};

use crate::{
    app::request_redraw,
    executor::{LatestTask, init_tokio_executor},
};

/// Reactive handle for an asynchronous read keyed by tracked source state.
pub struct Resource<T> {
    /// Signal containing the latest completed value.
    value: ReadSignal<Option<T>>,
    /// Signal indicating whether the current request is running.
    loading: ReadSignal<bool>,
    /// Setter used to trigger a fetch for the current source value.
    refetch: WriteSignal<u64>,
    /// Effect that tracks source changes and starts fetch tasks.
    _watcher: Effect<SyncStorage>,
}

impl<T> Clone for Resource<T> {
    /// Clones the resource signals and watcher handle.
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            loading: self.loading,
            refetch: self.refetch,
            _watcher: self._watcher,
        }
    }
}

impl<T> Resource<T>
where
    T: Send + Sync + 'static,
{
    /// Creates a resource from a tracked source and asynchronous fetcher.
    ///
    /// Source changes and explicit refetches begin a new request. Existing
    /// values remain available while loading, and stale completions cannot
    /// replace the latest value.
    ///
    /// # Arguments
    ///
    /// * `source` — Tracked closure returning the current resource key.
    /// * `fetcher` — Function returning the asynchronous value for a key.
    ///
    /// # Returns
    ///
    /// A [`Resource`] with separate loading and optional-value signals.
    ///
    /// # Panics
    ///
    /// Panics if created outside a Tokio runtime.
    pub fn new<K, F, Fut>(source: impl Fn() -> K + Send + Sync + 'static, fetcher: F) -> Self
    where
        K: Clone + Send + Sync + 'static,
        F: Fn(K) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        init_tokio_executor();

        let (value, set_value) = signal(None);
        let (loading, set_loading) = signal(false);
        let (refetch_version, refetch) = signal(0_u64);
        let latest_request = LatestTask::default();
        let fetcher = Arc::new(fetcher);

        let watcher = {
            let latest_request = latest_request.clone();
            Effect::watch_sync(
                move || (refetch_version.get(), source()),
                move |(_, key), _, _| {
                    let request_id = latest_request.next();
                    let _ = set_loading.try_set(true);
                    request_redraw();

                    let key = key.clone();
                    let fetcher = Arc::clone(&fetcher);
                    let latest_request = latest_request.clone();

                    tokio::spawn(async move {
                        let next = fetcher(key).await;

                        if latest_request.is_current(request_id) {
                            let _ = set_value.try_set(Some(next));
                            let _ = set_loading.try_set(false);
                            request_redraw();
                        }
                    });
                },
                true,
            )
        };

        Self {
            value,
            loading,
            refetch,
            _watcher: watcher,
        }
    }

    /// Requests another fetch using the current source value.
    pub fn refetch(&self) {
        self.refetch.update(|version| *version += 1);
    }

    /// Returns the signal containing the latest completed value.
    ///
    /// # Returns
    ///
    /// A [`ReadSignal<Option<T>>`] that retains its value while refetching.
    pub fn value(&self) -> ReadSignal<Option<T>> {
        self.value
    }

    /// Returns the signal indicating whether a request is running.
    ///
    /// # Returns
    ///
    /// A [`ReadSignal<bool>`] containing the loading state.
    pub fn loading(&self) -> ReadSignal<bool> {
        self.loading
    }

    /// Reads the optional resource value reactively by reference.
    ///
    /// # Arguments
    ///
    /// * `read` — Closure receiving the optional loaded value.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `read`.
    pub fn with<R>(&self, read: impl FnOnce(&Option<T>) -> R) -> R {
        self.value.with(read)
    }

    /// Reads the optional resource value without tracking.
    ///
    /// # Arguments
    ///
    /// * `read` — Closure receiving the optional loaded value.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `read`.
    pub fn with_untracked<R>(&self, read: impl FnOnce(&Option<T>) -> R) -> R {
        self.value.with_untracked(read)
    }

    /// Returns whether a request is currently running.
    ///
    /// # Returns
    ///
    /// A [`bool`] containing the tracked loading state.
    pub fn is_loading(&self) -> bool {
        self.loading.get()
    }

    /// Returns whether a request is running without tracking.
    ///
    /// # Returns
    ///
    /// A [`bool`] containing the untracked loading state.
    pub fn is_loading_untracked(&self) -> bool {
        self.loading.get_untracked()
    }
}

impl<T> Resource<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Returns the optional resource value reactively.
    ///
    /// # Returns
    ///
    /// An [`Option<T>`] containing the latest completed value.
    pub fn get(&self) -> Option<T> {
        self.value.get()
    }

    /// Returns the optional resource value without tracking.
    ///
    /// # Returns
    ///
    /// An [`Option<T>`] containing the latest completed value.
    pub fn get_untracked(&self) -> Option<T> {
        self.value.get_untracked()
    }
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
    /// Resource::new(|| (), |_| async move { Ok("ready") })
    /// ```
    ///
    /// # Assertions
    ///
    /// - Initial loading sends a redraw request.
    /// - Successful completion sends another redraw request.
    /// - The resource stores `Some(Ok("ready"))`.
    #[tokio::test(flavor = "current_thread")]
    async fn successful_completion_requests_redraw() {
        assert_completion_requests_redraw(Ok(String::from("ready"))).await;
    }

    /// Verifies failed resource completion requests a redraw.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Resource::new(|| (), |_| async move { Err("offline") })
    /// ```
    ///
    /// # Assertions
    ///
    /// - Initial loading sends a redraw request.
    /// - Failed completion sends another redraw request.
    /// - The resource stores `Some(Err("offline"))`.
    #[tokio::test(flavor = "current_thread")]
    async fn error_completion_requests_redraw() {
        assert_completion_requests_redraw(Err("offline")).await;
    }

    /// Verifies redraw behavior for one controlled resource response.
    ///
    /// # Arguments
    ///
    /// * `response` — Result sent into the pending fetch task.
    async fn assert_completion_requests_redraw(response: TestFetchResult) {
        let _redraw_guard = redraw_test_lock().await;
        let owner = Owner::new();
        let expected = response.clone();
        let (sender, receiver) = oneshot::channel();
        let receiver = Arc::new(Mutex::new(Some(receiver)));
        let receiver_for_fetcher = Arc::clone(&receiver);
        let mut redraws = subscribe_redraws();

        let resource: Resource<TestFetchResult> = owner.with(|| {
            Resource::new(
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

        assert_eq!(resource.get_untracked(), Some(expected));
    }
}
