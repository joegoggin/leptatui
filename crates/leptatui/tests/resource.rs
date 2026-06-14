//! Resource tests.
//!
//! These tests verify Leptatui's signal-backed async read state transitions.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend};
use tokio::{sync::oneshot, task::yield_now, time::timeout};

type TestFetchResult = std::result::Result<String, &'static str>;
type PendingFetches = Arc<Mutex<HashMap<i32, oneshot::Sender<TestFetchResult>>>>;

/// Verifies a resource loads once for its initial source key.
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

/// Verifies changing the source key starts a new pending load and stores the
/// new result.
#[tokio::test(flavor = "current_thread")]
async fn source_key_change_triggers_reload() {
    let owner = Owner::new();
    let pending = PendingFetches::default();
    let pending_for_fetcher = Arc::clone(&pending);

    let (resource, set_key): (Resource<String, &'static str>, WriteSignal<i32>) =
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
        });

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
#[tokio::test(flavor = "current_thread")]
async fn stale_fetch_completion_does_not_overwrite_newer_result() {
    let owner = Owner::new();
    let pending = PendingFetches::default();
    let pending_for_fetcher = Arc::clone(&pending);

    let (resource, set_key): (Resource<String, &'static str>, WriteSignal<i32>) =
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
        });

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

    render_component(&mut terminal, &mut component)?;
    assert!(terminal_text(&terminal).contains("Loading"));

    sender.send(Err("offline")).expect("send error response");
    wait_until(|| matches!(resource.get_untracked(), ResourceState::Error("offline"))).await;

    render_component(&mut terminal, &mut component)?;
    assert!(terminal_text(&terminal).contains("Error: offline"));

    Ok(())
}

struct ResourceStatus {
    resource: Resource<String, &'static str>,
}

impl Component for ResourceStatus {
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let label = match self.resource.get() {
            ResourceState::Pending => String::from("Loading"),
            ResourceState::Ready(value) => value,
            ResourceState::Error(error) => format!("Error: {error}"),
        };

        ctx.render_view(&text(label))
    }
}

fn insert_pending_fetch(pending: &PendingFetches, key: i32) -> oneshot::Receiver<TestFetchResult> {
    let (sender, receiver) = oneshot::channel();
    pending
        .lock()
        .expect("pending fetch lock")
        .insert(key, sender);
    receiver
}

fn has_pending_fetch(pending: &PendingFetches, key: i32) -> bool {
    pending
        .lock()
        .expect("pending fetch lock")
        .contains_key(&key)
}

fn send_fetch_response(pending: &PendingFetches, key: i32, response: TestFetchResult) {
    let sender = pending
        .lock()
        .expect("pending fetch lock")
        .remove(&key)
        .expect("pending fetch should exist");
    sender.send(response).expect("send fetch response");
}

fn render_component<C>(terminal: &mut Terminal<TestBackend>, component: &mut C) -> Result<()>
where
    C: Component,
{
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = Component::render(component, &mut ctx);
    })?;

    render_result
}

fn terminal_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

async fn wait_until(mut predicate: impl FnMut() -> bool) {
    timeout(Duration::from_secs(1), async {
        while !predicate() {
            yield_now().await;
        }
    })
    .await
    .expect("condition should become true");
}

async fn settle_tasks() {
    for _ in 0..10 {
        yield_now().await;
    }
}
