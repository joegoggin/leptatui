//! Signal-backed async resource state for terminal apps.
//!
//! Resources connect Leptos reactive source keys to asynchronous read work and
//! expose the latest pending, ready, or error state as a signal-friendly value.

use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use leptos::prelude::{
    Effect, Get, GetUntracked, ReadSignal, Set, SyncStorage, With, WithUntracked, signal,
};

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
    state: ReadSignal<ResourceState<T, E>>,
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
        let latest_request = Arc::new(AtomicU64::new(0));
        let fetcher = Arc::new(fetcher);

        let watcher = {
            let latest_request = Arc::clone(&latest_request);
            Effect::watch_sync(
                source,
                move |key, _, _| {
                    let request_id = latest_request.fetch_add(1, Ordering::AcqRel) + 1;
                    let _ = set_state.try_set(ResourceState::Pending);

                    let key = key.clone();
                    let set_state = set_state;
                    let fetcher = Arc::clone(&fetcher);
                    let latest_request = Arc::clone(&latest_request);

                    tokio::spawn(async move {
                        let next = match fetcher(key).await {
                            Ok(value) => ResourceState::Ready(value),
                            Err(error) => ResourceState::Error(error),
                        };

                        if latest_request.load(Ordering::Acquire) == request_id {
                            let _ = set_state.try_set(next);
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
}

impl<T, E> Resource<T, E>
where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
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
        match self.get() {
            ResourceState::Ready(value) => Some(value),
            ResourceState::Pending | ResourceState::Error(_) => None,
        }
    }

    /// Returns the error, when failed.
    pub fn error(&self) -> Option<E> {
        match self.get() {
            ResourceState::Error(error) => Some(error),
            ResourceState::Pending | ResourceState::Ready(_) => None,
        }
    }
}

/// Creates a resource from a tracked source key and async fetcher.
///
/// Older in-flight fetch tasks are not cancelled when the source key changes.
/// Their results are ignored if a newer request has started.
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

fn init_tokio_executor() {
    let _ = any_spawner::Executor::init_tokio();
}
