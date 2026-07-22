//! Resource tests.
//!
//! These tests verify Leptatui's signal-backed async read state transitions.
//!
//! # Modules
//!
//! - [`support`] — Shared rendering and async polling helpers.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend};
use tokio::sync::oneshot;

mod support;

use support::{draw_component, rendered_text, settle_tasks, wait_until};

/// Result returned by the controlled test resource fetcher.
type TestFetchResult = std::result::Result<String, &'static str>;
/// Pending fetch senders keyed by the source value that started each fetch.
type PendingFetches = Arc<Mutex<HashMap<i32, oneshot::Sender<TestFetchResult>>>>;

/// Verifies a resource loads once for its initial source key.
///
/// # Example Under Test
///
/// ```text
/// let (key, _set_key) = signal(7);
/// create_resource(move || key.get(), |key| async move { Ok(format!("item-{key}")) });
/// ```
///
/// # Assertions
///
/// - The resource starts in the pending state.
/// - The resource eventually stores `ResourceState::Ready("item-7")`.
/// - The fetcher is called exactly once for the initial source key.
#[tokio::test(flavor = "current_thread")]
async fn resource_loads_initial_source_key_once() {
    let owner = Owner::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_fetcher = Arc::clone(&calls);

    let resource: Resource<String, &'static str> = owner.with(|| {
        let (key, _set_key) = signal(7);

        create_resource(
            move || key.get(),
            move |key| {
                let calls = Arc::clone(&calls_for_fetcher);

                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(format!("item-{key}"))
                }
            },
        )
    });

    assert!(resource.is_pending());

    wait_until(|| {
        matches!(
            resource.get_untracked(),
            ResourceState::Ready(ref value) if value == "item-7"
        )
    })
    .await;

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

/// Verifies changing the source key starts a new pending load and stores the new result.
///
/// # Example Under Test
///
/// ```text
/// let (key, set_key) = signal(1);
/// let resource = create_resource(move || key.get(), fetch);
/// set_key.set(2);
/// ```
///
/// # Assertions
///
/// - The initial key `1` registers a pending fetch.
/// - Completing key `1` stores `one` as the resource value.
/// - Updating the key to `2` registers a new pending fetch.
/// - Completing key `2` stores `two` as the resource value.
#[tokio::test(flavor = "current_thread")]
async fn source_key_change_triggers_reload() {
    let owner = Owner::new();
    let pending = PendingFetches::default();

    let (resource, set_key) = create_keyed_test_resource(&owner, &pending);

    wait_until(|| has_pending_fetch(&pending, 1)).await;
    assert!(resource.is_pending());

    send_fetch_response(&pending, 1, Ok(String::from("one")));
    wait_until(|| resource.value().as_deref() == Some("one")).await;

    set_key.set(2);

    wait_until(|| has_pending_fetch(&pending, 2)).await;
    assert!(resource.is_pending());

    send_fetch_response(&pending, 2, Ok(String::from("two")));
    wait_until(|| resource.value().as_deref() == Some("two")).await;
}

/// Verifies a slower stale fetch cannot overwrite a newer load result.
///
/// # Example Under Test
///
/// ```text
/// set_key.set(2);
/// send_fetch_response(2, Ok("second"));
/// send_fetch_response(1, Ok("first"));
/// ```
///
/// # Assertions
///
/// - Fetches are registered for keys `1` and `2`.
/// - Completing key `2` first stores `second` as the resource value.
/// - Completing stale key `1` afterward does not replace the newer value.
///
/// # Why
///
/// Slow responses from older source keys must not overwrite fresher UI state.
#[tokio::test(flavor = "current_thread")]
async fn stale_fetch_completion_does_not_overwrite_newer_result() {
    let owner = Owner::new();
    let pending = PendingFetches::default();

    let (resource, set_key) = create_keyed_test_resource(&owner, &pending);

    wait_until(|| has_pending_fetch(&pending, 1)).await;

    set_key.set(2);
    wait_until(|| has_pending_fetch(&pending, 2)).await;

    send_fetch_response(&pending, 2, Ok(String::from("second")));
    wait_until(|| resource.value().as_deref() == Some("second")).await;

    send_fetch_response(&pending, 1, Ok(String::from("first")));
    settle_tasks().await;

    assert_eq!(resource.value().as_deref(), Some("second"));
}

/// Verifies loading and error states render from a component.
///
/// # Example Under Test
///
/// ```text
/// ResourceStatus { resource: resource.clone() }
/// render_component(&mut terminal, &mut component)
/// ```
///
/// # Assertions
///
/// - Terminal creation succeeds.
/// - Rendering the pending resource succeeds and shows `Loading`.
/// - Completing the fetch with `offline` stores `ResourceState::Error`.
/// - Rendering the failed resource succeeds and shows `Error: offline`.
#[tokio::test(flavor = "current_thread")]
async fn loading_and_error_states_render_in_component() -> Result<()> {
    let owner = Owner::new();
    let (sender, receiver) = oneshot::channel::<TestFetchResult>();
    let receiver = Arc::new(Mutex::new(Some(receiver)));
    let receiver_for_fetcher = Arc::clone(&receiver);

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

    let backend = TestBackend::new(32, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut component = ResourceStatus {
        resource: resource.clone(),
    };

    draw_component(&mut terminal, &mut component)?;
    assert!(rendered_text(&terminal).contains("Loading"));

    sender.send(Err("offline")).expect("send error response");
    wait_until(|| matches!(resource.get_untracked(), ResourceState::Error("offline"))).await;

    draw_component(&mut terminal, &mut component)?;
    assert!(rendered_text(&terminal).contains("Error: offline"));

    Ok(())
}

/// Test component that renders a label for the current resource state.
struct ResourceStatus {
    /// Resource read by the component during render.
    resource: Resource<String, &'static str>,
}

impl View for ResourceStatus {
    /// Renders the current resource state into the terminal frame.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Render context for the current terminal frame.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] when the state label renders successfully.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if rendering the text view fails.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let label = match self.resource.get() {
            ResourceState::Pending => String::from("Loading"),
            ResourceState::Ready(value) => value,
            ResourceState::Error(error) => format!("Error: {error}"),
        };

        View::render(&text(label), ctx)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Creates a controlled resource keyed by an integer signal.
///
/// # Arguments
///
/// * `owner` — Leptos owner that keeps the resource signals alive for the test.
/// * `pending` — Shared map used to control fetch responses by source key.
///
/// # Returns
///
/// A [`Resource`] and [`WriteSignal`] pair for changing the source key.
fn create_keyed_test_resource(
    owner: &Owner,
    pending: &PendingFetches,
) -> (Resource<String, &'static str>, WriteSignal<i32>) {
    let pending_for_fetcher = Arc::clone(pending);

    owner.with(|| {
        let (key, set_key) = signal(1);

        let resource = create_resource(
            move || key.get(),
            move |key| {
                let pending = Arc::clone(&pending_for_fetcher);

                async move {
                    let receiver = insert_pending_fetch(&pending, key);
                    receiver.await.expect("test fetch response")
                }
            },
        );

        (resource, set_key)
    })
}

/// Inserts a pending fetch sender for a source key and returns its receiver.
///
/// # Arguments
///
/// * `pending` — Shared map that stores fetch response senders by key.
/// * `key` — Source key associated with the fetch.
///
/// # Returns
///
/// A [`oneshot::Receiver`] awaited by the test fetcher.
fn insert_pending_fetch(pending: &PendingFetches, key: i32) -> oneshot::Receiver<TestFetchResult> {
    let (sender, receiver) = oneshot::channel();
    pending
        .lock()
        .expect("pending fetch lock")
        .insert(key, sender);
    receiver
}

/// Returns whether a source key has a pending controlled fetch.
///
/// # Arguments
///
/// * `pending` — Shared map inspected for the key.
/// * `key` — Source key to look up.
///
/// # Returns
///
/// A [`bool`] indicating whether the key is waiting for a response.
fn has_pending_fetch(pending: &PendingFetches, key: i32) -> bool {
    pending
        .lock()
        .expect("pending fetch lock")
        .contains_key(&key)
}

/// Sends a controlled fetch response for a source key.
///
/// # Arguments
///
/// * `pending` — Shared map containing the pending sender.
/// * `key` — Source key whose fetch should be completed.
/// * `response` — Result to deliver to the fetcher.
fn send_fetch_response(pending: &PendingFetches, key: i32, response: TestFetchResult) {
    let sender = pending
        .lock()
        .expect("pending fetch lock")
        .remove(&key)
        .expect("pending fetch should exist");
    sender.send(response).expect("send fetch response");
}
